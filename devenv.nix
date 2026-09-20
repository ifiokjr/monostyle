{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

let
  currentDir = builtins.dirOf __curPos.file;
  custom = inputs.ifiokjr-nixpkgs.packages.${pkgs.stdenv.system};
in
{
  packages =
    with pkgs;
    [
      cacert
      cargo-binstall
      cargo-nextest
      cargo-run-bin
      custom.mdt
      dprint
      gh
      git
      gitleaks
      hyperfine
      jq
      mdbook
      nixfmt
      python3
      rustup
      shfmt
      taplo
      unzip
      zip
      zizmor
    ]
    ++ lib.optionals stdenv.isDarwin [
      coreutils
    ];

  enterShell = ''
    set -euo pipefail
    export PATH="$DEVENV_PROFILE/bin:$PATH"
  '';

  # disable dotenv since it interferes with variable interpolation in the shell
  dotenv.disableHint = true;

  git-hooks = {
    hooks = {
      "secrets:commit" = {
        enable = true;
        verbose = true;
        pass_filenames = true;
        name = "secrets";
        description = "Scan staged changes for leaked secrets with gitleaks.";
        entry = "${pkgs.gitleaks}/bin/gitleaks protect --staged --verbose --redact";
        stages = [ "pre-commit" ];
      };
      "lint:format" = {
        enable = true;
        verbose = true;
        pass_filenames = true;
        name = "lint:format";
        description = "Run workspace autofixes before commit and restage the results.";
        entry = "${config.env.DEVENV_PROFILE}/bin/lint:format";
        stages = [ "pre-commit" ];
      };
      "lint:push" = {
        enable = true;
        verbose = true;
        pass_filenames = false;
        name = "lint:push";
        description = "Run the local CI lint rules and test suite before push.";
        entry = "${config.env.DEVENV_PROFILE}/bin/lint:push";
        stages = [ "pre-push" ];
      };
    };
  };

  scripts = {
    "monostyle:dev" = {
      exec = ''
        set -euo pipefail
        cargo run --quiet --package monostyle --bin monostyle -- "$@"
      '';
      description = "The dev build of the `monostyle` executable";
      binary = "bash";
    };
    "monostyle" = {
      exec = ''
        set -euo pipefail
        cargo run --quiet --release --package monostyle --bin monostyle -- "$@"
      '';
      description = "The release build of the `monostyle` executable";
      binary = "bash";
    };

    # The score command is deliberately part of the default task list. A tool that measures
    # readability should be run on its own source, or its standards are aspirational rather
    # than enforced.
    "score" = {
      exec = ''
        set -euo pipefail
        cargo run --quiet --release --package monostyle --bin monostyle -- \
          check crates examples --units "$@"
      '';
      description = "Score this repository with monostyle.";
      binary = "bash";
    };
    "score:strict" = {
      exec = ''
        set -euo pipefail
        cargo run --quiet --release --package monostyle --bin monostyle -- \
          check crates --strict --fail-under 95 "$@"
      '';
      description = "Score this repository strictly and fail below 95.";
      binary = "bash";
    };
    "score:json" = {
      exec = ''
        set -euo pipefail
        cargo run --quiet --release --package monostyle --bin monostyle -- \
          check crates examples --format json "$@"
      '';
      description = "Emit this repository's scores as JSON.";
      binary = "bash";
    };

    "build:all" = {
      exec = ''
        set -euo pipefail
        if [ -z "''${CI:-}" ]; then
          echo "Building project locally"
          cargo build --workspace --all-features
        else
          echo "Building in CI"
          cargo build --workspace --all-features --locked
        fi
      '';
      description = "Build all crates with all features activated.";
      binary = "bash";
    };
    "build:dist" = {
      exec = ''
        set -euo pipefail
        echo "Building with dist profile (LTO, codegen-units=1, strip)"
        cargo build --workspace --all-features --locked --profile dist
      '';
      description = "Build the release binary with the dist profile.";
      binary = "bash";
    };
    "build:book" = {
      exec = ''
        set -euo pipefail
        mdbook build docs
      '';
      description = "Build the mdbook documentation.";
      binary = "bash";
    };

    "test:all" = {
      exec = ''
        set -euo pipefail
        test:cargo
        test:docs
      '';
      description = "Run all tests.";
      binary = "bash";
    };
    "test:cargo" = {
      exec = ''
        set -euo pipefail
        cargo nextest run --workspace --all-features
      '';
      description = "Run cargo tests with nextest.";
      binary = "bash";
    };
    "test:docs" = {
      exec = ''
        set -euo pipefail
        cargo test --doc --workspace --all-features
      '';
      description = "Run documentation tests.";
      binary = "bash";
    };
    "coverage:all" = {
      exec = ''
        set -euo pipefail
        mkdir -p target/coverage
        cargo llvm-cov clean --workspace
        cargo llvm-cov test --workspace --all-features --lib --tests --no-report
        cargo llvm-cov report --summary-only --fail-under-lines 70
        cargo llvm-cov report --lcov --output-path target/coverage/lcov.info
      '';
      description = "Run workspace coverage and enforce a 70% line-coverage floor.";
      binary = "bash";
    };

    "fix:all" = {
      exec = ''
        set -euo pipefail
        fix:clippy
        fix:format
      '';
      description = "Fix all autofixable problems.";
      binary = "bash";
    };
    "fix:format" = {
      exec = ''
        set -euo pipefail
        repo_root="$(git rev-parse --show-toplevel)"
        dprint fmt --config "$repo_root/dprint.json"
      '';
      description = "Format files with dprint.";
      binary = "bash";
    };
    "fix:clippy" = {
      exec = ''
        set -euo pipefail
        cargo clippy --workspace --fix --allow-dirty --allow-staged --all-features --all-targets
      '';
      description = "Fix clippy lints for rust.";
      binary = "bash";
    };

    "lint:all" = {
      exec = ''
        set -euo pipefail

        run_step() {
          local name="$1"
          shift
          echo "Currently running: $name"
          "$@"
        }

        run_step "lint:clippy" lint:clippy
        run_step "lint:format" lint:format
        run_step "lint:workflows" lint:workflows
        run_step "docs:check" docs:check
      '';
      description = "Run all checks.";
      binary = "bash";
    };
    "lint:push" = {
      exec = ''
        set -euo pipefail

        run_step() {
          local name="$1"
          shift
          echo "Currently running: $name"
          "$@"
        }

        run_step "gitleaks detect" ${pkgs.gitleaks}/bin/gitleaks detect --verbose --redact
        run_step "lint:clippy" ${currentDir}/.devenv/profile/bin/lint:clippy
        run_step "lint:format" ${currentDir}/.devenv/profile/bin/lint:format
        run_step "test:cargo" ${currentDir}/.devenv/profile/bin/test:cargo
        run_step "lint:workflows" ${currentDir}/.devenv/profile/bin/lint:workflows
        run_step "docs:check" ${currentDir}/.devenv/profile/bin/docs:check
      '';
      description = "Used for the pre push checks";
      binary = "bash";
    };
    "lint:clippy" = {
      exec = ''
        set -euo pipefail
        # Treat all compiler and clippy warnings as errors so warning-only regressions never
        # make it into CI or a pushed branch.
        cargo clippy --workspace --all-features --all-targets -- -D warnings
      '';
      description = "Check that all rust lints are passing with warnings denied.";
      binary = "bash";
    };
    "lint:format" = {
      exec = ''
        set -euo pipefail
        ${pkgs.dprint}/bin/dprint check
      '';
      description = "Check that all files are formatted.";
      binary = "bash";
    };
    "lint:workflows" = {
      exec = ''
        set -euo pipefail
        zizmor .github/workflows/ .github/actions/ 2>/dev/null || \
          zizmor .github/workflows/
      '';
      description = "Scan GitHub Actions workflows for security vulnerabilities with zizmor.";
      binary = "bash";
    };
    "docs:check" = {
      exec = ''
        set -euo pipefail
        mdt check
      '';
      description = "Check that shared documentation blocks are synchronized.";
      binary = "bash";
    };
    "docs:update" = {
      exec = ''
        set -euo pipefail
        mdt update
      '';
      description = "Update shared documentation blocks.";
      binary = "bash";
    };
  };
}
