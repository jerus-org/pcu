#!/usr/bin/env -S just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

default:
    {{ just_executable() }} --list

alias t := test
alias c := check

# run all tests, clippy, including journey tests, try building docs
test: clippy check doc unit-tests git-only

# verify the git-only surface really is free of the attestation, bluesky and
# linkedin dependencies.
#
# A feature gate rots silently: a `use sigstore::…` (or `gen_bsky::…`,
# `gen_linkedin::…`) added outside the gate compiles perfectly under default
# features and only breaks for the consumers who opted out. Consumers depend
# on pcu with `default-features = false` precisely to keep `rsa`
# (RUSTSEC-2023-0071) and `lru` (RUSTSEC-2026-0253, via
# bsky-sdk -> atrium-api -> atrium-common) — both with no fixed version — out
# of their graph, so the absence of these crates is the property worth
# asserting, not merely that the build succeeds. The bsky/linkedin-only checks
# confirm each feature can be taken independently of the other.
git-only:
    #!/usr/bin/env bash
    set -euo pipefail
    assert_absent() {
        local crate="$1"
        shift
        # --target all: lru is only pulled in for a non-host (wasm) target via
        # atrium-common, so the default host-only tree never shows it, gated
        # or not — this flag is what makes the assertion actually load-bearing.
        if cargo tree --target all "$@" --package pcu --invert "$crate" 2>/dev/null | grep -q "^$crate"; then
            echo "FAIL: $crate is present ($*)" >&2
            cargo tree --target all "$@" --package pcu --invert "$crate" >&2
            exit 1
        fi
        echo "OK: no $crate ($*)"
    }

    cargo check --no-default-features --package pcu
    assert_absent rsa --no-default-features
    assert_absent lru --no-default-features
    assert_absent bsky-sdk --no-default-features
    assert_absent gen-linkedin --no-default-features

    cargo check --no-default-features --features bsky --package pcu
    assert_absent gen-linkedin --no-default-features --features bsky

    cargo check --no-default-features --features linkedin --package pcu
    assert_absent lru --no-default-features --features linkedin
    assert_absent bsky-sdk --no-default-features --features linkedin

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
