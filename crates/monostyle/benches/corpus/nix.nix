# A development shell with the string forms Nix makes hard: indented strings whose `''` does not
# always close, `${}` interpolation inside them, and the escapes `''$` and `'''`.
{ pkgs ? import <nixpkgs> { } }:

let
  inherit (pkgs) lib stdenv;

  version = "0.3.27";

  # An indented string: the `''$` is an escaped dollar, not an interpolation, and the `'''` is a
  # literal pair of quotes.
  motd = ''
    Welcome to the ${version} shell.
    Your home is ''${HOME} and quoting works like '''this'''.
    Run `just test` to start.
  '';

  # A function that renders an environment table; the interpolation nests a quoted string.
  renderEnv = env:
    lib.concatStringsSep "\n" (lib.mapAttrsToList (name: value: "${name}=${toString value}") env);

  toolchain = with pkgs; [
    rustup
    cargo-nextest
    cargo-llvm-cov
    hyperfine
    nodejs_24
  ];

  isDarwin = stdenv.isDarwin;

  darwinInputs = lib.optionals isDarwin (with pkgs.darwin.apple_sdk.frameworks; [
    Security
    SystemConfiguration
  ]);

  environment = {
    RUST_BACKTRACE = 1;
    CARGO_TERM_COLOR = "always";
    MONOSTYLE_VERSION = version;
  };
in
pkgs.mkShell {
  name = "monostyle-${version}";

  packages = toolchain ++ darwinInputs;

  shellHook = ''
    echo "${motd}"
    export PATH="$PWD/target/release:$PATH"

    if [ -z "''${IN_NIX_SHELL:-}" ]; then
      echo "not inside a nix shell" >&2
    fi

    ${renderEnv environment}
  '';

  passthru = {
    inherit version toolchain;

    # Nested attribute sets with an `if` expression, which is a decision the metrics count.
    platform =
      if isDarwin then { kind = "darwin"; linker = "ld64"; }
      else if stdenv.isLinux then { kind = "linux"; linker = "mold"; }
      else { kind = "other"; linker = null; };
  };
}
