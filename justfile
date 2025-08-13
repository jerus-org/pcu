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
    cargo test --all

# run various auditing tools to assure we are legal and safe
audit:
    cargo deny check advisories bans licenses sources

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
