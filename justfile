set shell := ["bash", "-euo", "pipefail", "-c"]

cargo_deny_version := "0.20.2"
jscpd_version := "5.4.1"

default:
    @just --list

check: fmt-check lint qlty deny dup test

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --all-targets --all-features --locked -- -D warnings

lint-fix:
    cargo clippy --fix --allow-dirty --allow-no-vcs --all-targets --all-features --locked
    cargo fmt --all

test:
    cargo test --locked

# Advisories, licenses, bans, sources
deny:
    cargo deny check

# Quality analytics and smell detector:
qlty:
    #!/usr/bin/env bash
    if ! command -v qlty >/dev/null; then
        echo "qlty not installed, skipping (just setup)" >&2
        exit 0
    fi
    qlty smells --all --quiet --include-tests
    qlty check --all --no-progress --no-upgrade-check

# Copy-paste detection, fails above 5%
dup:
    jscpd -t 5 --silent src

setup:
    cargo install cargo-deny --locked --version {{cargo_deny_version}}
    npm install --global jscpd@{{jscpd_version}}
    command -v qlty >/dev/null || curl -fsSL https://qlty.sh | bash
