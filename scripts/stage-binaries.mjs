#!/usr/bin/env node
// Copies a built binary into the platform package that should ship it.
//
// The release workflow builds one target per runner and calls this with the target triple, so the
// mapping from Rust target to npm package lives in exactly one place. Staging into the wrong
// package is the failure mode this prevents, and it is a silent one: the published package would
// install and then refuse to launch.
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

/** Maps a Rust target triple to its npm package suffix and binary name. */
const TARGETS = {
	"aarch64-apple-darwin": { package: "darwin-arm64", binary: "monostyle" },
	"x86_64-apple-darwin": { package: "darwin-x64", binary: "monostyle" },
	"aarch64-unknown-linux-gnu": {
		package: "linux-arm64-gnu",
		binary: "monostyle",
	},
	"aarch64-unknown-linux-musl": {
		package: "linux-arm64-musl",
		binary: "monostyle",
	},
	"x86_64-unknown-linux-gnu": { package: "linux-x64-gnu", binary: "monostyle" },
	"x86_64-unknown-linux-musl": {
		package: "linux-x64-musl",
		binary: "monostyle",
	},
	"aarch64-pc-windows-msvc": {
		package: "win32-arm64-msvc",
		binary: "monostyle.exe",
	},
	"x86_64-pc-windows-msvc": {
		package: "win32-x64-msvc",
		binary: "monostyle.exe",
	},
};

const target = process.argv[2];

if (!target) {
	console.error(
		"usage: stage-binaries.mjs <rust-target-triple> [path-to-binary]",
	);
	process.exit(1);
}

const mapping = TARGETS[target];

if (!mapping) {
	console.error(`unknown target triple: ${target}`);
	console.error(`known targets: ${Object.keys(TARGETS).join(", ")}`);
	process.exit(1);
}

// The default source is cargo's output for this target. The binary name comes from the mapping
// rather than a literal, because Windows appends `.exe` and looking for `monostyle` there found
// nothing: the build succeeded and the release failed at staging.
const source = process.argv[3] ?? `target/${target}/release/${mapping.binary}`;

if (!fs.existsSync(source)) {
	console.error(`built binary not found at ${source}`);
	process.exit(1);
}

const destinationDirectory = path.join(
	"packages",
	`monostyle__cli-${mapping.package}`,
	"bin",
);
fs.mkdirSync(destinationDirectory, { recursive: true });

const destination = path.join(destinationDirectory, mapping.binary);
fs.copyFileSync(source, destination);
fs.chmodSync(destination, 0o755);

console.log(`staged ${source} -> ${destination}`);
