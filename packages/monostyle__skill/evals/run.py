#!/usr/bin/env python3
"""Evals for the monostyle skill's paragraph-grouping guidance.

Each case directory under `cases/` holds a `task.md` (what the agent is asked
to do), an `input.*` (code whose blank lines are wrong in a specific way), and
an `expected.*` (the reference answer). A candidate answer is graded by
content-anchored checks plus the real `monostyle` binary — never by diffing,
because many layouts can be correct and the checks encode which ones.

Modes:
  ./run.py                     self mode — grade the shipped expected answers
  ./run.py --answers DIR       grade DIR/<case>.<ext> as an agent's answers

A case passes only when every check passes. Exit status is non-zero if any
case fails, so this runs as a CI step in self mode.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
CASES = HERE / "cases"
REPO_ROOT = HERE.parents[2]


def monostyle_binary(override: str | None) -> str:
	if override:
		return override
	candidate = REPO_ROOT / "target" / "release" / "monostyle"
	if candidate.exists():
		return str(candidate)
	return "monostyle"


def check(path: Path, must_contain: list[str], must_not_contain: list[str]) -> list[str]:
	text = path.read_text()
	problems = []
	for pattern in must_contain:
		if not re.search(pattern, text):
			problems.append(f"missing: /{pattern}/")
	for pattern in must_not_contain:
		if re.search(pattern, text):
			problems.append(f"present but must not be: /{pattern}/")
	return problems


def monostyle_findings(binary: str, path: Path, rule: str) -> list[str]:
	result = subprocess.run(
		[binary, "check", str(path), "-f", "json", "--quiet"],
		capture_output=True,
		text=True,
	)
	if result.returncode not in (0, 1):
		return [f"monostyle failed to run: {result.stderr.strip()[:120]}"]
	report = json.loads(result.stdout)
	return [
		f"{path.name}:{finding['span']['start_line']} {finding['message'][:60]}"
		for file in report.get("files", [])
		for finding in file.get("findings", [])
		if finding["rule"] == rule and finding.get("weight", 0) > 0
	]


def stripped(text: str) -> str:
	return [line for line in text.splitlines() if line.strip()]


# --- case checks ------------------------------------------------------------
# Each entry: expected extension, and the checks as (regex-in, regex-out)
# pairs. `rule` names the monostyle rule the answer must not trip. The
# `same_statements` guard compares input and answer with blanks removed, so a
# candidate cannot pass by deleting the code it was asked to regroup.

CASE_CHECKS: dict[str, dict] = {
	"mid-concept-break": {
		"ext": "rs",
		"rule": "readability/group-separation",
		"must_contain": [
			r"copy_from_slice\(chunk\);\n\n\tlet mut revision",
		],
		"must_not_contain": [
			r"let mut revision = \[0u8; 4\];\n\n",
		],
	},
	"sub-concepts-in-a-long-run": {
		"ext": "ts",
		"rule": "readability/group-separation",
		"must_contain": [
			r"idle_ms\", 60_000\);\n\n\tconst attempts",
			r"backoff_ms\", 500\);\n\n\tconst sinkUrl",
		],
		"must_not_contain": [
			r"connect_ms\", 3_000\);\n\n\tconst readMs",
			r"telemetry_url\"[^;]*;\n\n\tconst batchSize",
		],
	},
	"build-then-populate": {
		"ext": "ts",
		"rule": "readability/group-separation",
		"must_contain": [
			r"new JsObject\(\);\n\tset\(output",
		],
		"must_not_contain": [
			r"new JsObject\(\);\n\n\tset\(output",
		],
	},
	"arrange-act-assert": {
		"ext": "rs",
		"rule": "readability/group-separation",
		"must_contain": [
			r"load_context\(server\.path\(\)\);\n\n\tlet updates",
			r"unwrap_err\(\);\n\tassert!",
		],
		"must_not_contain": [
			r'write_settings\(r#"\n\t\t\[settings\]\n\t\turl = "https://example\.invalid"\n\t"#\);\n\n\tlet ctx',
		],
	},
}


def grade_case(name: str, answer: Path, binary: str) -> list[str]:
	problems: list[str] = []

	if name in CASE_CHECKS:
		spec = CASE_CHECKS[name]
		input_path = CASES / name / f"input.{spec['ext']}"
		if stripped(input_path.read_text()) != stripped(answer.read_text()):
			problems.append("statements changed: only blank-line placement may differ")
		problems += check(answer, spec["must_contain"], spec["must_not_contain"])
		problems += [
			f"group-separation: {finding}"
			for finding in monostyle_findings(binary, answer, spec["rule"])
		]
		return problems

	if name == "skill-examples":
		skill = (REPO_ROOT / "packages" / "monostyle__skill" / "SKILL.md").read_text()
		before = re.search(r"Before —.*?```rust\n(.*?)```", skill, re.S)
		after = re.search(r"After —.*?```rust\n(.*?)```", skill, re.S)
		if not before or not after:
			return ["SKILL.md lost its Before/After example pair"]
		if not re.search(r"let mut revision = \[0u8; 4\];\n\n", before.group(1)):
			problems.append("the Before example no longer shows the mid-concept break")
		if not re.search(r"copy_from_slice\(chunk\);\n\nlet mut revision", after.group(1)):
			problems.append("the After example no longer breaks between the two concepts")
		# The After example must itself be clean when it stands alone.
		with tempfile.NamedTemporaryFile("w", suffix=".rs", delete=False) as handle:
			handle.write("struct Fields { sequence: u64, revision: u32 }\n")
			handle.write("fn decode_fields(plan: &mut Plan<'_>) -> Fields {\n")
			handle.write(after.group(1).replace("\n    ", "\n\t"))
			handle.write("\n\tunreachable!()\n}\n")
			standalone = Path(handle.name)
		problems += [
			f"the After example trips the rule itself: {finding}"
			for finding in monostyle_findings(binary, standalone, "readability/group-separation")
		]
		return problems

	return [f"unknown case: {name}"]


def main() -> int:
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument("--answers", type=Path, help="directory of agent answers as <case>.<ext>")
	parser.add_argument("--monostyle", help="path to the monostyle binary")
	args = parser.parse_args()

	binary = monostyle_binary(args.monostyle)
	failures = 0

	for case_dir in sorted(CASES.iterdir()):
		if not case_dir.is_dir():
			continue
		name = case_dir.name
		if name == "skill-examples":
			answer = case_dir  # grades SKILL.md itself; no input file
		elif args.answers:
			ext = CASE_CHECKS[name]["ext"]
			answer = args.answers / f"{name}.{ext}"
			if not answer.exists():
				print(f"FAIL {name}: no answer at {answer}")
				failures += 1
				continue
		else:
			ext = CASE_CHECKS[name]["ext"]
			answer = case_dir / f"expected.{ext}"

		problems = grade_case(name, answer, binary)
		if problems:
			failures += 1
			print(f"FAIL {name}")
			for problem in problems:
				print(f"      {problem}")
		else:
			print(f"PASS {name}")

	print(f"\n{'all cases pass' if failures == 0 else f'{failures} case(s) failed'}")
	return 1 if failures else 0


if __name__ == "__main__":
	sys.exit(main())
