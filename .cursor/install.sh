#!/usr/bin/env bash
# Installs Nix and devenv, then warms this repository's development shell.
#
# Idempotent: a second run reuses the Nix store, the devenv profile, and the
# evaluated shell. Nix and devenv are linked into /usr/local/bin because login
# shells do not source ~/.bashrc, so the installer's profile snippet would be
# invisible to later commands. nix.conf carries flakes, nix-command, and the
# devenv binary cache for every shell, login or not.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

nix_conf_dir="${XDG_CONFIG_HOME:-${HOME}/.config}/nix"
nix_conf="${nix_conf_dir}/nix.conf"

# devenv.cachix.org mirrors cache.nixos.org and holds the devenv shell closure.
# Without it, `devenv shell` builds that closure from source.
write_nix_conf() {
	mkdir -p "$nix_conf_dir"
	cat >"$nix_conf" <<'EOF'
experimental-features = nix-command flakes
accept-flake-config = true
extra-substituters = https://devenv.cachix.org
extra-trusted-public-keys = devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=
ssl-cert-file = /etc/ssl/certs/ca-certificates.crt
EOF
}

# ~/.nix-profile is a symlink to the current generation, so these links keep
# working when the profile is upgraded.
link_profile_bins() {
	local bindir="${HOME}/.nix-profile/bin"
	local cmd

	if [ ! -d "$bindir" ]; then
		printf 'nix profile bin directory is missing: %s\n' "$bindir" >&2
		exit 1
	fi

	for cmd in nix nix-env nix-store nix-channel nix-build nix-instantiate nix-shell nix-hash nix-collect-garbage devenv; do
		if [ -e "${bindir}/${cmd}" ]; then
			sudo ln -sfn "${bindir}/${cmd}" "/usr/local/bin/${cmd}"
		fi
	done
}

# Login shells read /etc/profile.d. Non-login shells use the /usr/local/bin links.
install_profile_snippet() {
	sudo tee /etc/profile.d/nix-devenv.sh >/dev/null <<'EOF'
if [ -d "${HOME}/.nix-profile/bin" ]; then
	export PATH="${HOME}/.nix-profile/bin:${PATH}"
fi
EOF
}

devenv_meets_requirement() {
	local version major minor

	version="$(devenv version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+(\.[0-9]+)?' | head -1 || true)"
	[ -n "$version" ] || return 1

	major="${version%%.*}"
	minor="${version#*.}"
	minor="${minor%%.*}"
	# devenv.yaml requires >= 2.1.
	[ "$major" -gt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -ge 1 ]; }
}

write_nix_conf

if [ ! -x "${HOME}/.nix-profile/bin/nix" ]; then
	# Single-user: no daemon to start on boot, and the user nix.conf's
	# substituters are honored. --no-modify-profile avoids a bashrc edit that
	# non-interactive login shells would skip.
	curl -fsSL https://nixos.org/nix/install | sh -s -- --no-daemon --yes --no-modify-profile --no-channel-add --nix-extra-conf-file "$nix_conf"
fi

export PATH="${HOME}/.nix-profile/bin:${PATH}"
link_profile_bins
install_profile_snippet

if ! devenv_meets_requirement; then
	nix profile install nixpkgs#devenv
	link_profile_bins
fi

if ! devenv_meets_requirement; then
	printf 'devenv from nixpkgs is older than 2.1:\n%s\n' "$(devenv version 2>&1 || true)" >&2
	exit 1
fi

# Download and realize the pinned shell so the next agent does not pay for it.
devenv shell -- true
