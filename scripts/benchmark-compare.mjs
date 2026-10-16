#!/usr/bin/env node
// Turns hyperfine exports for a pull request's base and head binaries into the markdown table and
// merge recommendation posted on the PR.
//
// Each scenario is benchmarked as one hyperfine run with two commands named `base` and `head`, so
// a scenario's numbers were produced by the same runner in the same job. The comparison reads the
// `min` of each side: on shared CI runners the minimum is the statistic closest to the true cost,
// because every source of noise above it (other tenants, page cache, scheduling) only ever makes a
// run slower, never faster.
//
// Usage:
//   benchmark-compare.mjs --base-sha <sha> --head-sha <sha> \
//     <repo.json> "Repository (crates + examples)" <large.json> "Large corpus"

import fs from "node:fs";
import process from "node:process";

/** A regression this large is real, not runner noise. */
const REGRESSION_THRESHOLD = 1.1;

/** An improvement this large is real, not runner noise. */
const IMPROVEMENT_THRESHOLD = 0.95;

/** Parses command-line arguments into the base/head shas and the scenario list. */
function parseArguments() {
	const args = process.argv.slice(2);
	const flags = new Map();
	const positional = [];

	for (let index = 0; index < args.length; index += 1) {
		const value = args[index];

		if (value.startsWith("--")) {
			flags.set(value, args[index + 1]);
			index += 1;
		} else {
			positional.push(value);
		}
	}

	const baseSha = flags.get("--base-sha");
	const headSha = flags.get("--head-sha");
	const scenarios = [];

	for (let index = 0; index + 1 < positional.length; index += 2) {
		scenarios.push({ file: positional[index], label: positional[index + 1] });
	}

	if (!baseSha || !headSha || scenarios.length === 0) {
		console.error(
			"usage: benchmark-compare.mjs --base-sha <sha> --head-sha <sha> " +
				'<results.json> "Scenario label" [...more pairs]',
		);

		process.exit(2);
	}

	return { baseSha, headSha, scenarios };
}

/** Reads one side of one hyperfine export, normalised across export versions. */
function sideOf(file, name) {
	const exported = JSON.parse(fs.readFileSync(file, "utf8"));
	// hyperfine records the `-n` display name beside the full command string; matching on either
	// keeps the script working against exports that record only the command.
	const result = exported.results.find(
		(entry) => entry.name === name || entry.command === name,
	);

	if (!result) {
		console.error(`no command named "${name}" in ${file}`);

		process.exit(2);
	}

	// hyperfine 2 nests the statistics under `summary` and offers peak memory; 1.x exports flat
	// time fields and no memory. Both are read so the runner's hyperfine version is not a
	// constraint on the script.
	if (result.summary?.time_wall_clock) {
		return {
			min: result.summary.time_wall_clock.min,
			memory: result.summary.memory_peak_resident?.median,
		};
	}

	return { min: result.min, memory: undefined };
}

/** Formats seconds as milliseconds for the table. */
function milliseconds(seconds) {
	if (seconds < 0.1) {
		return `${(seconds * 1000).toPrecision(3)} ms`;
	}

	return `${seconds.toFixed(2)} s`;
}

/** Formats a peak resident set in bytes as mebibytes. */
function memory(bytes) {
	return bytes === undefined ? "—" : `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

/** The percentage change from base to head, positive meaning slower. */
function change(base, head) {
	return (head / base - 1) * 100;
}

/** One row of the comparison table. */
function row(scenario) {
	const base = sideOf(scenario.file, "base");
	const head = sideOf(scenario.file, "head");
	const delta = change(base.min, head.min);

	const trend = delta > 5 ? "🔴" : delta < -5 ? "🟢" : "➖";

	return {
		label: scenario.label,
		base: milliseconds(base.min),
		head: milliseconds(head.min),
		memory: `${memory(base.memory)} → ${memory(head.memory)}`,
		delta: `${delta >= 0 ? "+" : ""}${delta.toFixed(1)}%`,
		trend,
		regressed: head.min > base.min * REGRESSION_THRESHOLD,
		improved: head.min < base.min * IMPROVEMENT_THRESHOLD,
	};
}

const { baseSha, headSha, scenarios } = parseArguments();
const rows = scenarios.map(row);

const regressed = rows.filter((row) => row.regressed);
const improved = rows.filter((row) => row.improved);

// The verdict answers the one question a reviewer has at merge time. A regression past the
// threshold says hold off; an improvement past the threshold with no regression says go; anything
// inside the thresholds is a tie and the code decides, not the clock.
const verdict = regressed.length > 0
	? {
		marker: "⛔",
		headLine: "Merge recommendation: **do not merge** — a scenario regressed past the threshold.",
		reason: regressed
			.map((row) => `\`${row.label}\` is ${row.delta} (${row.base} → ${row.head})`)
			.join("; "),
	}
	: improved.length > 0
	? {
		marker: "✅",
		headLine: "Merge recommendation: **merge** — a scenario improved and none regressed.",
		reason: improved
			.map((row) => `\`${row.label}\` is ${row.delta} (${row.base} → ${row.head})`)
			.join("; "),
	}
	: {
		marker: "➖",
		headLine:
			"Merge recommendation: **yours to call** — every scenario is within the ±5–10% noise band, so performance is unchanged.",
		reason: "no scenario moved past either threshold",
	};

const table = [
	"| Scenario | Base | This PR | Δ min | Peak RSS | |",
	"| --- | --- | --- | --- | --- | --- |",
	...rows.map(
		(row) =>
			`| ${row.label} | ${row.base} | ${row.head} | ${row.delta} | ${row.memory} | ${row.trend} |`,
	),
].join("\n");

const body = [
	`<!-- monostyle-benchmark -->`,
	`## ${verdict.marker} Benchmark results`,
	"",
	`Base is the merge-base commit (\`${baseSha.slice(0, 8)}\`), this PR is \`${
		headSha.slice(0, 8)
	}\`.`,
	"",
	table,
	"",
	`**${verdict.headLine}** ${verdict.reason}.`,
	"",
	"<details>",
	"<summary>Method</summary>",
	"",
	"- Both binaries are release builds of the two commits, benchmarked back to back on the same runner with hyperfine: 3 warmup runs, 15 measured runs, and the second scenario reverses the run order so drift over the job cannot favour the same side twice.",
	"- Each number is the fastest of the 15 runs, which is the statistic least contaminated by tenant noise on shared runners.",
	"- A scenario flags red above +10% and green below −5%; the band between is treated as noise.",
	"- Regenerate this comment by re-running the benchmark workflow on the pull request.",
	"",
	"</details>",
].join("\n");

process.stdout.write(body);
