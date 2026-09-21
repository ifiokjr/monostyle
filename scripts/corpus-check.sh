#!/usr/bin/env bash
#
# Runs two monostyle binaries over a repository at a pull request's head and reports the difference.
#
# The point is to see what a rule change does to real code rather than to a fixture. The released
# binary is the baseline: it is what a project's CI is running today, so any finding it reports and the
# new build does not is a false positive that has been removed. The comparison is what makes a claim
# like "prose is no longer scored as code" checkable instead of asserted.
#
# Usage:
#   scripts/corpus-check.sh <owner/repo> <pr-number> [work-dir]
#
# Output, in <work-dir> (default /tmp/monostyle-corpus/<owner>-<repo>-<pr>):
#   before.txt / before.json   the released binary's report
#   after.txt  / after.json    the working-tree binary's report
#   summary.md                 the score table and the per-rule finding counts
#
# Requires: gh, jq, git, and a release binary for the baseline. Set MONOSTYLE_BASELINE to point at one
# you already have, or leave it unset to download the pinned tag's asset for this platform.

set -euo pipefail

BASELINE_TAG="${MONOSTYLE_BASELINE_TAG:-v0.1.0}"

if [[ $# -lt 2 ]]; then
	printf 'usage: %s <owner/repo> <pr-number> [work-dir]\n' "$0" >&2
	exit 2
fi

repo="$1"
pr="$2"
slug="${repo//\//-}"
work_dir="${3:-/tmp/monostyle-corpus/${slug}-${pr}}"
repo_dir="${work_dir}/repo"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

for tool in gh jq git; do
	if ! command -v "$tool" >/dev/null; then
		printf 'missing dependency: %s\n' "$tool" >&2
		exit 1
	fi
done

mkdir -p "$work_dir"

# --- the two binaries -------------------------------------------------------

# The working tree's build is the candidate. Building it here rather than taking a path keeps the two
# sides of the comparison honest: whatever is on disk right now is what gets measured.
printf 'building the working-tree binary...\n'
cargo build --quiet --release --manifest-path "${repo_root}/Cargo.toml" -p monostyle
candidate="${repo_root}/target/release/monostyle"

# The baseline is the released binary, which is what a downstream project's CI is running today.
baseline="${MONOSTYLE_BASELINE:-}"

if [[ -z "$baseline" ]]; then
	baseline_dir="${work_dir}/baseline"
	mkdir -p "$baseline_dir"

	case "$(uname -s)-$(uname -m)" in
	Darwin-arm64) asset="monostyle-aarch64-apple-darwin" ;;
	Darwin-x86_64) asset="monostyle-x86_64-apple-darwin" ;;
	Linux-aarch64) asset="monostyle-aarch64-unknown-linux-gnu" ;;
	Linux-x86_64) asset="monostyle-x86_64-unknown-linux-gnu" ;;
	*)
		printf 'no prebuilt baseline for this platform; set MONOSTYLE_BASELINE\n' >&2
		exit 1
		;;
	esac

	baseline="${baseline_dir}/monostyle"

	if [[ ! -x "$baseline" ]]; then
		printf 'downloading the %s baseline (%s)...\n' "$BASELINE_TAG" "$asset"

		# The release assets are bare binaries named for their target triple, so the download is the
		# binary itself and needs only to be made executable.
		gh release download "$BASELINE_TAG" \
			--repo ifiokjr/monostyle \
			--pattern "$asset" \
			--dir "$baseline_dir" \
			--clobber

		mv "${baseline_dir}/${asset}" "$baseline"
		chmod +x "$baseline"
	fi
fi

printf 'baseline: %s\n' "$("$baseline" --version 2>/dev/null || echo unknown)"
printf 'candidate: %s\n' "$("$candidate" --version 2>/dev/null || echo unknown)"

# --- the source under test --------------------------------------------------

if [[ ! -d "$repo_dir/.git" ]]; then
	printf 'cloning %s...\n' "$repo"
	gh repo clone "$repo" "$repo_dir" -- --quiet
fi

printf 'checking out %s#%s...\n' "$repo" "$pr"
git -C "$repo_dir" fetch --quiet origin "pull/${pr}/head"
git -C "$repo_dir" checkout --quiet --detach FETCH_HEAD

# --- the comparison ---------------------------------------------------------

# `--no-color` keeps the text reports diffable, and stderr is captured separately: the floor-failure
# messages go to stderr, so merging the streams would put prose after the closing brace of the JSON
# report and make it unparseable. The JSON reports are what the summary is built from, because a score
# is easier to compare as a number than as a rendered table.
printf 'analyzing with the baseline...\n'
"$baseline" check "$repo_dir" --no-color >"${work_dir}/before.txt" 2>"${work_dir}/before.err" || true
"$baseline" check "$repo_dir" --format json >"${work_dir}/before.json" 2>>"${work_dir}/before.err" || true

printf 'analyzing with the candidate...\n'
"$candidate" check "$repo_dir" --no-color >"${work_dir}/after.txt" 2>"${work_dir}/after.err" || true
"$candidate" check "$repo_dir" --format json >"${work_dir}/after.json" 2>>"${work_dir}/after.err" || true

# A report that is not JSON means the binary failed rather than that it found nothing. Saying so here
# is better than a summary full of zeros.
for side in before after; do
	if ! jq -e '.readability' "${work_dir}/${side}.json" >/dev/null 2>&1; then
		printf 'the %s report is not valid JSON; see %s.json\n' "$side" "$side" >&2
		exit 1
	fi
done

{
	printf '# %s#%s\n\n' "$repo" "$pr"
	printf '| Metric | Baseline | Candidate |\n'
	printf '| --- | --- | --- |\n'

	for key in readability complexity; do
		before="$(jq -r ".${key}.value | . * 10 | round / 10" "${work_dir}/before.json")"
		after="$(jq -r ".${key}.value | . * 10 | round / 10" "${work_dir}/after.json")"
		printf '| %s | %s | %s |\n' "$key" "$before" "$after"
	done

	printf '\n## Findings per rule\n\n'
	printf '| Rule | Baseline | Candidate |\n'
	printf '| --- | --- | --- |\n'

	# Every rule either side reported, so a rule that only the candidate fires on is visible.
	jq -r '[.files[].findings[].rule] | group_by(.) | map({key: .[0], value: length}) | from_entries | to_entries[] | "\(.key) \(.value)"' \
		"${work_dir}/before.json" | sort >"${work_dir}/before.rules"

	jq -r '[.files[].findings[].rule] | group_by(.) | map({key: .[0], value: length}) | from_entries | to_entries[] | "\(.key) \(.value)"' \
		"${work_dir}/after.json" | sort >"${work_dir}/after.rules"

	awk '{count[$1]=$2} END {for (rule in count) print rule}' \
		"${work_dir}/before.rules" "${work_dir}/after.rules" | sort | while read -r rule; do
		b="$(awk -v r="$rule" '$1==r {print $2}' "${work_dir}/before.rules")"
		a="$(awk -v r="$rule" '$1==r {print $2}' "${work_dir}/after.rules")"

		printf '| `%s` | %s | %s |\n' "$rule" "${b:-0}" "${a:-0}"
	done

	printf '\n## Findings on Markdown files\n\n'

	# The specific claim the rule fixes make: prose is not code, so no code rule may fire on a `.md`
	# file. Anything listed here is a false positive that survived.
	for side in before after; do
		printf '### %s\n\n' "$side"

		jq -r '.files[] | select(.path | endswith(".md")) | .findings[] | .rule' \
			"${work_dir}/${side}.json" | sort | uniq -c | sed 's/^/    /' || true

		printf '\n'
	done
} >"${work_dir}/summary.md"

printf '\nwrote %s\n\n' "${work_dir}/summary.md"
cat "${work_dir}/summary.md"
