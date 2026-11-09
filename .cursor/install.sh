#!/usr/bin/env bash
# Installs the toolchain and dependency cache a cloud agent needs to build, lint, and test.
#
# Idempotent: a second run reuses the pinned toolchain, the nextest binary, and the cargo
# registry cache. Binaries are placed on the default PATH. Login shells do not source
# ~/.bashrc, so a version manager that only edits rc files would be invisible to install.

set -euo pipefail

# The version CI resolves through taiki-e/install-action as of this setup. Pinned so a later
# nextest release cannot change test behavior under agents without a review.
NEXTEST_VERSION="0.9.151"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v rustup >/dev/null 2>&1; then
	curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
		| sh -s -- -y --default-toolchain none --profile default --no-modify-path
	sudo ln -sfn "${HOME}/.cargo/bin/rustup" /usr/local/bin/rustup
	sudo ln -sfn "${HOME}/.cargo/bin/cargo" /usr/local/bin/cargo
	sudo ln -sfn "${HOME}/.cargo/bin/rustc" /usr/local/bin/rustc
	sudo ln -sfn "${HOME}/.cargo/bin/rustdoc" /usr/local/bin/rustdoc
	sudo ln -sfn "${HOME}/.cargo/bin/rustfmt" /usr/local/bin/rustfmt
	sudo ln -sfn "${HOME}/.cargo/bin/cargo-fmt" /usr/local/bin/cargo-fmt
	sudo ln -sfn "${HOME}/.cargo/bin/cargo-clippy" /usr/local/bin/cargo-clippy
fi

# rust-toolchain.toml names the channel and the rustfmt, clippy, and rust-analyzer components.
# Installing the active toolchain from the repository root picks that file up. A toolchain that
# is already present is left in place.
rustup toolchain install --profile default --no-self-update
rustup component add rustfmt clippy rust-analyzer

case "$(uname -m)" in
	x86_64) nextest_target="x86_64-unknown-linux-gnu" ;;
	aarch64 | arm64) nextest_target="aarch64-unknown-linux-gnu" ;;
	*)
		printf 'unsupported architecture for cargo-nextest: %s\n' "$(uname -m)" >&2
		exit 1
		;;
esac

cargo_bin_dir="${CARGO_HOME:-${HOME}/.cargo}/bin"
nextest_dest="${cargo_bin_dir}/cargo-nextest"

if ! "${nextest_dest}" --version 2>/dev/null | grep -F "cargo-nextest ${NEXTEST_VERSION}" >/dev/null; then
	archive="cargo-nextest-${NEXTEST_VERSION}-${nextest_target}.tar.gz"
	url="https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${NEXTEST_VERSION}/${archive}"
	tmp="$(mktemp -d)"
	curl -fsSL "$url" -o "${tmp}/${archive}"
	curl -fsSL "${url%.tar.gz}.sha256" -o "${tmp}/${archive%.tar.gz}.sha256"
	(
		cd "$tmp"
		sha256sum -c "${archive%.tar.gz}.sha256"
	)
	tar -xzf "${tmp}/${archive}" -C "$tmp"
	mkdir -p "$cargo_bin_dir"
	mv "${tmp}/cargo-nextest" "$nextest_dest"
	chmod +x "$nextest_dest"
	rm -rf "$tmp"
fi

if ! command -v cargo-nextest >/dev/null 2>&1; then
	sudo ln -sfn "$nextest_dest" /usr/local/bin/cargo-nextest
fi

cargo fetch --locked
