#!/bin/bash
set -eo pipefail

# Usage: ./scripts/prepare-crate-release.sh <package> <tag_prefix> <subdir> [--dry-run]
# Example: ./scripts/prepare-crate-release.sh pcu "pcu-v" "crates/pcu"

PACKAGE="${1}"
TAG_PREFIX="${2}"
SUBDIR="${3}"
DRY_RUN="${4:-}"

if [[ -z "${PACKAGE}" ]] || [[ -z "${TAG_PREFIX}" ]] || [[ -z "${SUBDIR}" ]]; then
    echo "Usage: $0 <package> <tag_prefix> <subdir> [--dry-run]"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Step 1: Detect if release needed
echo "=== Detecting release requirement for ${PACKAGE} ==="
DETECT_OUTPUT=$("${SCRIPT_DIR}/detect-crate-release.sh" "${TAG_PREFIX}" "${SUBDIR}")
echo "${DETECT_OUTPUT}"

if echo "${DETECT_OUTPUT}" | grep -q "RELEASE_NEEDED=false"; then
    echo "No release needed for ${PACKAGE}"
    exit 0
fi

# Extract version
SEMVER=$(echo "${DETECT_OUTPUT}" | grep "SEMVER=" | cut -d= -f2)
echo "Detected version: ${SEMVER}"

# Step 2: Check if already published to crates.io
echo "=== Checking crates.io for ${PACKAGE}@${SEMVER} ==="
if "${SCRIPT_DIR}/check-crates-io-version.sh" "${PACKAGE}" "${SEMVER}"; then
    PUBLISH_FLAG="--no-publish"
    echo "Version exists on crates.io, will use --no-publish"
else
    PUBLISH_FLAG=""
    echo "Version not on crates.io, will publish"
fi

# Step 3: Commit any pending Cargo.lock changes from prior releases
# This handles the case where a dependency crate was just released
if git diff --quiet Cargo.lock 2>/dev/null; then
    echo "Cargo.lock is clean"
else
    echo "=== Committing Cargo.lock changes from dependency updates ==="
    git add Cargo.lock
    git commit -S -s -m "chore: update Cargo.lock for ${PACKAGE} release"
fi

# Step 4: Execute cargo release (no push)
echo "=== Running cargo release for ${PACKAGE} ==="
if [[ "${DRY_RUN}" = "--dry-run" ]]; then
    echo "[DRY-RUN] Would run: cargo release -p ${PACKAGE} ${PUBLISH_FLAG} --no-push --execute ${SEMVER}"
else
    # shellcheck disable=SC2086
    cargo release -p "${PACKAGE}" ${PUBLISH_FLAG} --no-push --execute "${SEMVER}"
fi

echo "=== ${PACKAGE} prepared for release ==="
echo "TAG=${TAG_PREFIX}${SEMVER}"
