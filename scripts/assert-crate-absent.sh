#!/usr/bin/env bash
# Usage: scripts/assert-crate-absent.sh <tree-dump-file> <cargo-tree-flags> <crate>...
#
# Fails if any <crate> appears in a pre-captured dependency-tree dump. The
# caller captures the dump ONCE per feature combination via:
#   cargo tree <cargo-tree-flags> --package pcu > <tree-dump-file>
# so checking N crates costs one graph walk, not N. On failure, re-runs
# `cargo tree --invert` for just the failing crate to print its full
# reverse-dependency chain.
#
# <cargo-tree-flags> MUST include `--target all`: some gated deps (e.g. `lru`,
# pulled in only for a non-host/wasm target via atrium-common) are invisible
# in the default host-only tree, gated or not — that's what makes this
# assertion actually load-bearing. This is the single source of truth for the
# check; both `just git-only` and the `exp-git-only-build` CI job call it, so
# there is nowhere else for the logic to drift.
set -euo pipefail

tree_file="$1"
tree_flags="$2"
shift 2

for crate in "$@"; do
    if grep -qE "(^|[[:space:]])${crate} v" "$tree_file"; then
        echo "FAIL: $crate is present" >&2
        # shellcheck disable=SC2086
        cargo tree $tree_flags --package pcu --invert "$crate" >&2
        exit 1
    fi
    echo "OK: no $crate"
done
