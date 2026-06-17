#!/usr/bin/env -S just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

default:
    {{ just_executable() }} --list

alias t := test
alias c := check

# run all tests, clippy, including journey tests, try building docs
test: clippy check doc unit-tests

clear-target:
    cargo clean

# Run cargo clippy on all crates
clippy *clippy-args:
    cargo clippy

# Build all code in suitable configurations
check:
    cargo check --all

# Run cargo doc on all crates
doc $RUSTDOCFLAGS="-D warnings":
    cargo doc --all --no-deps

# run all unit tests
unit-tests:
    cargo nextest run --all

# run various auditing tools to assure we are legal and safe
audit:
    cargo deny check advisories bans licenses sources

# verify the documented MSRV (rust-version) still builds with the locked deps.
# CI's "minimum" build uses the earliest rolling toolchain, which is typically
# newer than the documented MSRV, so it does NOT validate the true MSRV — run
# this locally on every change (especially when Cargo.lock changes).
msrv:
    cargo msrv verify --manifest-path crates/pcu/Cargo.toml

# discover the true minimum supported rust-version (bisects; run when a dep bump
# breaks `just msrv`, then bump rust-version + CI min_rust_version + README badge).
msrv-find:
    cargo msrv find --manifest-path crates/pcu/Cargo.toml

# run nightly rustfmt for its extra features, but check that it won't upset stable rustfmt
fmt:
    cargo +nightly fmt --all -- --config-path rustfmt-nightly.toml
    cargo +stable fmt --all -- --check
    just --fmt --unstable

# Generate coverage reported
cov:
    cargo tarpaulin --output-dir coverage --out lcov

# Smart release dry run
sr-dry:
    cargo smart-release -u --dry-run-cargo-publish --allow-fully-generated-changelogs --changelog-without commit-details

# Execute smart release
sr:
    cargo smart-release -u --allow-fully-generated-changelogs --changelog-without commit-details --execute
