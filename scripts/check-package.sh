#!/usr/bin/env bash
# Packages the crate as it would be published, then runs the tests from inside
# the package, so the package is proved to hold everything the tests need.
set -euo pipefail

mkdir -p target
log=target/check-package.log

# A push checks the work tree, which can hold files that are not committed yet.
cargo package --locked --allow-dirty 2>&1 | tee "$log"

# `include` in Cargo.toml decides what ships, and a target whose source it
# leaves out is only a warning, so the warning has to be the failure.
if grep -q '^warning' "$log"; then
  echo "check-package: cargo package warned; the published crate would not hold what the repo does" >&2
  exit 1
fi

version="$(sed -n 's/^version *= *"\(.*\)"$/\1/p' Cargo.toml | head -n 1)"

# A target folder that outlives the package folder, which cargo package
# replaces each run, so a push does not compile every dependency again.
cargo test --locked --workspace \
  --manifest-path "target/package/super-rex-$version/Cargo.toml" \
  --target-dir target/package-test
