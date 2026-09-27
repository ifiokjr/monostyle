#!/usr/bin/env python3
"""Generates expected snapshots for the auto-fix fixture corpus.

For every ``*.input.*`` fixture, runs the freshly built binary's real ``fix``
pipeline over a scratch copy, writes the result to the matching
``*.expected.*`` file, and prints a unified diff of everything the fixer
changed. The diffs are what a reviewer reads: every changed line must be a
blank-line move outside of string and comment content, or the fixture exposed
a bug rather than a snapshot.
"""

from __future__ import annotations

import difflib
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CORPUS = ROOT / "crates/monostyle/tests/fixtures/fix"
BINARY = ROOT / "target/debug/monostyle"
SCRATCH = pathlib.Path("/tmp/monostyle-fixture-snapshots")


def main() -> int:
    if not BINARY.exists():
        print(f"binary missing: {BINARY}", file=sys.stderr)
        return 1

    SCRATCH.mkdir(exist_ok=True)
    inputs = sorted(CORPUS.rglob("*.input.*"))
    generated = 0

    for index, source in enumerate(inputs):
        # Byte mode everywhere: universal-newline translation would silently
        # rewrite CRLF fixtures to LF and corrupt the snapshots they produce.
        text_bytes = source.read_bytes()
        text = text_bytes.decode('utf-8', errors='replace')
        suffix = source.suffixes[-2].lstrip(".")
        language_suffix = source.name.split(".input.")[-1]
        work = SCRATCH / f"{index:03d}-{source.stem}.{language_suffix}"
        work.write_bytes(text_bytes)

        result = subprocess.run(
            [str(BINARY), "fix", str(work), "--no-color"],
            capture_output=True,
            text=True,
        )
        fixed_bytes = work.read_bytes()
        fixed = fixed_bytes.decode('utf-8', errors='replace')

        expected = source.with_name(source.name.replace(".input.", ".expected."))
        expected.write_bytes(fixed_bytes)

        if fixed != text:
            generated += 1
            diff = difflib.unified_diff(
                text.splitlines(),
                fixed.splitlines(),
                fromfile=str(expected.relative_to(ROOT)),
                tofile=f"{expected.relative_to(ROOT)} (fixed)",
                lineterm="",
            )
            print("\n".join(diff))

        if "unterminated construct" in result.stderr:
            print(f"!! {source.relative_to(ROOT)}: scan not clean")

    total = len(inputs)
    print(f"\n{total} fixtures, {generated} with changes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
