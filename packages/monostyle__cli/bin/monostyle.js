#!/usr/bin/env node
// Launcher for the platform-specific monostyle binary.
//
// npm resolves optional dependencies by platform, so the correct binary package is usually the
// only one installed. The candidate list exists for the cases where it is not: Alpine and other
// musl systems can end up with either the gnu or musl package, and a container built on one host
// may be run on another.
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";

const require = createRequire(import.meta.url);

/** Platform packages to try, most likely first. */
const PLATFORM_PACKAGES = {
	darwin: {
		arm64: ["@monostyle-rs/cli-darwin-arm64"],
		x64: ["@monostyle-rs/cli-darwin-x64"],
	},
	linux: {
		arm64: ["@monostyle-rs/cli-linux-arm64-gnu", "@monostyle-rs/cli-linux-arm64-musl"],
		x64: ["@monostyle-rs/cli-linux-x64-gnu", "@monostyle-rs/cli-linux-x64-musl"],
	},
	win32: {
		arm64: ["@monostyle-rs/cli-win32-arm64-msvc"],
		x64: ["@monostyle-rs/cli-win32-x64-msvc"],
	},
};

/** Returns the candidate package names for this platform. */
function candidates() {
	return PLATFORM_PACKAGES[process.platform]?.[process.arch] ?? [];
}

/** Resolves a package's binary path, or null when it is not installed. */
function resolveBinary(packageName) {
	try {
		const packageJson = require.resolve(`${packageName}/package.json`);
		const binary = path.join(
			path.dirname(packageJson),
			"bin",
			process.platform === "win32" ? "monostyle.exe" : "monostyle",
		);

		return fs.existsSync(binary) ? binary : null;
	} catch {
		return null;
	}
}

/** Whether a failed spawn should fall through to the next candidate package. */
function shouldTryNext(result) {
	if (result.error) return true;

	// A libc mismatch surfaces as a missing-interpreter error rather than as an exit code, so the
	// stderr text is what distinguishes "wrong package" from "the tool failed".
	if (result.status !== 127) return false;

	return /not found|no such file or directory|exec format error/i.test(result.stderr ?? "");
}

/** Runs the first binary that works. */
function main() {
	const names = candidates();

	if (names.length === 0) {
		console.error(
			`monostyle does not publish npm binaries for ${process.platform}/${process.arch}. ` +
				"Install from GitHub releases or with `cargo install monostyle` instead.",
		);
		process.exit(1);
	}

	const failures = [];

	for (const name of names) {
		const binary = resolveBinary(name);
		if (!binary) continue;

		const result = spawnSync(binary, process.argv.slice(2), {
			stdio: ["inherit", "pipe", "pipe"],
			encoding: "utf8",
			windowsHide: false,
		});

		if (shouldTryNext(result)) {
			failures.push(`${name}: ${result.error?.message ?? result.stderr?.trim() ?? "failed to launch"}`);
			continue;
		}

		process.stdout.write(result.stdout ?? "");
		process.stderr.write(result.stderr ?? "");
		process.exit(result.status ?? 0);
	}

	console.error("Unable to find a compatible monostyle binary in the installed npm packages.");
	console.error(`Tried: ${names.join(", ")}`);

	if (failures.length > 0) {
		console.error(failures.join("\n"));
	}

	console.error(
		"Reinstall with `npm install -g @monostyle-rs/cli`, download a binary from GitHub releases, " +
			"or use `cargo install monostyle`.",
	);

	process.exit(1);
}

main();
