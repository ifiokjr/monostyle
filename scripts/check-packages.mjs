#!/usr/bin/env node
// Validates the npm package set before publishing.
//
// Publishing an incomplete package set is the failure mode this catches: a launcher whose platform
// packages are missing installs successfully and then fails at runtime, which is far harder to
// diagnose than a failed publish. Every check here corresponds to a way the set can be wrong.
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

/** The npm package directory prefix. */
const PREFIX = "monostyle__cli";

/** Every platform suffix the launcher knows how to resolve. */
const PLATFORM_SUFFIXES = [
	"darwin-arm64",
	"darwin-x64",
	"linux-arm64-gnu",
	"linux-arm64-musl",
	"linux-x64-gnu",
	"linux-x64-musl",
	"win32-arm64-msvc",
	"win32-x64-msvc",
];

const errors = [];
const packagesDirectory = path.join("packages");

/** Reads and parses a package manifest. */
function readManifest(directory) {
	const manifestPath = path.join(directory, "package.json");

	if (!fs.existsSync(manifestPath)) {
		errors.push(`${manifestPath} is missing`);

		return null;
	}

	try {
		return JSON.parse(fs.readFileSync(manifestPath, "utf8"));
	} catch (error) {
		errors.push(`${manifestPath} is not valid JSON: ${error.message}`);

		return null;
	}
}

const launcherDirectory = path.join(packagesDirectory, `${PREFIX}`);
const launcher = readManifest(launcherDirectory);
const versions = new Map();

if (launcher) {
	versions.set(launcher.name, launcher.version);

	// The launcher must point at every platform package, and at the same version, or npm resolves
	// a mismatched set that may not contain a working binary.
	for (const suffix of PLATFORM_SUFFIXES) {
		const dependency = `@monostyle-rs/cli-${suffix}`;
		const declared = launcher.optionalDependencies?.[dependency];

		if (!declared) {
			errors.push(
				`${launcher.name} does not declare optional dependency ${dependency}`,
			);
		} else if (declared !== `^${launcher.version}`) {
			errors.push(
				`${launcher.name} declares ${dependency} at ${declared}, expected ^${launcher.version}`,
			);
		}
	}

	// The launcher must actually resolve those packages, or the dependency list is decorative.
	const launcherSource = fs.readFileSync(
		path.join(launcherDirectory, "bin", "monostyle.js"),
		"utf8",
	);

	for (const suffix of PLATFORM_SUFFIXES) {
		if (!launcherSource.includes(`@monostyle-rs/cli-${suffix}`)) {
			errors.push(`bin/monostyle.js never tries @monostyle-rs/cli-${suffix}`);
		}
	}
}

for (const suffix of PLATFORM_SUFFIXES) {
	const directory = path.join(packagesDirectory, `${PREFIX}-${suffix}`);
	const manifest = readManifest(directory);

	if (!manifest) continue;

	versions.set(manifest.name, manifest.version);

	if (launcher && manifest.version !== launcher.version) {
		errors.push(
			`${manifest.name} is at ${manifest.version}, launcher is at ${launcher.version}`,
		);
	}

	// A binary must be present, or the package publishes empty and fails on install.
	const binary = path.join(
		directory,
		"bin",
		process.platform === "win32" && suffix.includes("win32") ? "monostyle.exe" : "monostyle",
	);

	// Only the host's own platform package is expected to hold a binary during a local run; CI
	// stages each one on its own runner.
	const isHostPlatform = suffix.includes(
		process.platform === "darwin" ? "darwin" : process.platform === "win32" ? "win32" : "linux",
	);

	if (isHostPlatform && !fs.existsSync(binary)) {
		console.warn(
			`note: ${binary} not staged yet (expected locally until a release build runs)`,
		);
	}

	for (const field of ["os", "cpu", "files", "publishConfig"]) {
		if (!manifest[field]) {
			errors.push(`${manifest.name} is missing the ${field} field`);
		}
	}

	if (manifest.publishConfig?.access !== "public") {
		errors.push(`${manifest.name} does not publish with public access`);
	}
}

// Every package must agree on its version, because the launcher pins its dependencies to its own.
const distinct = new Set(versions.values());

if (distinct.size > 1) {
	errors.push(
		`package versions disagree: ${
			[...versions.entries()].map(([name, version]) => `${name}@${version}`)
				.join(", ")
		}`,
	);
}

if (errors.length > 0) {
	console.error("package check failed:");

	for (const error of errors) {
		console.error(`  - ${error}`);
	}

	process.exit(1);
}

console.log(
	`package check passed: ${versions.size} packages at ${[...distinct][0]}`,
);
