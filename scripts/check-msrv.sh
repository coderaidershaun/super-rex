#!/usr/bin/env bash
# Checks that the crate builds with the oldest Rust that Cargo.toml claims, and
# installs that Rust first when it is missing.
set -euo pipefail

# Cargo.toml is the only place the version is written, so this check cannot
# drift from what the crate promises.
msrv="$(sed -n 's/^rust-version *= *"\(.*\)"$/\1/p' Cargo.toml)"
if [ -z "$msrv" ]; then
  echo "check-msrv: Cargo.toml has no rust-version" >&2
  exit 1
fi

if ! rustup run "$msrv" rustc --version >/dev/null 2>&1; then
  rustup toolchain install --no-self-update --profile minimal "$msrv"
fi

cargo +"$msrv" check --locked --workspace --all-targets
