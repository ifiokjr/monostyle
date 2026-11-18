#!/usr/bin/env bash
# Builds and publishes release artifacts. Heredocs in every flavour are the point: a quoted
# delimiter disables interpolation, a dash strips leading tabs, and the body is data until the
# terminator line.

set -euo pipefail

readonly VERSION="${1:-$(git describe --tags --abbrev=0)}"
readonly TARGETS=(x86_64-unknown-linux-musl aarch64-apple-darwin x86_64-pc-windows-msvc)
readonly OUT="${OUT:-dist}"

log() {
	printf '[release] %s\n' "$*" >&2
}

usage() {
	cat <<'EOF'
usage: release.sh [version]

Builds every target, writes checksums, and prints the release notes. $VERSION is not
expanded inside this block because the delimiter is quoted.
EOF
}

notes() {
	local version="$1"

	cat <<EOF
# ${version}

Built on $(date -u +%Y-%m-%d) for ${#TARGETS[@]} targets:
$(printf -- '- %s\n' "${TARGETS[@]}")
EOF
}

checksums() {
	# The dash form lets the body stay indented with the function.
	cat <<-SHA
		$(cd "$OUT" && shasum -a 256 -- *)
	SHA
}

build() {
	local target="$1"
	local binary="monostyle"

	case "$target" in
		*windows*) binary="monostyle.exe" ;;
		*linux*) export CC=musl-gcc ;;
		*) ;;
	esac

	if [[ -n "${SKIP_BUILD:-}" ]]; then
		log "skipping $target"

		return 0
	fi

	cargo build --release --target "$target"
	cp "target/$target/release/$binary" "$OUT/monostyle-$target${binary#monostyle}"
}

main() {
	if [[ "${1:-}" == "--help" ]]; then
		usage

		exit 0
	fi

	mkdir -p "$OUT"

	for target in "${TARGETS[@]}"; do
		build "$target" || { log "failed: $target"; exit 1; }
	done

	notes "$VERSION" > "$OUT/notes.md"
	checksums > "$OUT/SHA256SUMS"

	while IFS= read -r line; do
		log "$line"
	done < "$OUT/SHA256SUMS"
}

main "$@"
