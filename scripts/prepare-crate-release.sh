#!/bin/bash
set -eo pipefail

# Usage: ./scripts/prepare-crate-release.sh <package> <tag_prefix> <subdir> [--dry-run]
# Example: ./scripts/prepare-crate-release.sh pcu "pcu-v" "crates/pcu"

PACKAGE=$1
TAG_PREFIX=$2
SUBDIR=$3
DRY_RUN=${4:-""}

if [ -z "$PACKAGE" ] || [ -z "$TAG_PREFIX" ] || [ -z "$SUBDIR" ]; then
    echo "Usage: $0 <package> <tag_prefix> <subdir> [--dry-run]"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Step 1: Detect if release needed
echo "=== Detecting release requirement for $PACKAGE ==="
DETECT_OUTPUT=$("$SCRIPT_DIR/detect-crate-release.sh" "$TAG_PREFIX" "$SUBDIR")
echo "$DETECT_OUTPUT"

if echo "$DETECT_OUTPUT" | grep -q "RELEASE_NEEDED=false"; then
    echo "No release needed for $PACKAGE"
    exit 0
fi

# Extract version
SEMVER=$(echo "$DETECT_OUTPUT" | grep "SEMVER=" | cut -d= -f2)
echo "Detected version: $SEMVER"

# Step 2: Check if already published to crates.io
echo "=== Checking crates.io for $PACKAGE@$SEMVER ==="
if "$SCRIPT_DIR/check-crates-io-version.sh" "$PACKAGE" "$SEMVER"; then
    PUBLISH_FLAG="--no-publish"
    echo "Version exists on crates.io, will use --no-publish"
else
    PUBLISH_FLAG=""
    echo "Version not on crates.io, will publish"
fi

# Step 3: Execute cargo release (no push)
echo "=== Running cargo release for $PACKAGE ==="
if [ "$DRY_RUN" = "--dry-run" ]; then
    echo "[DRY-RUN] Would run: cargo release -p $PACKAGE $PUBLISH_FLAG --no-push --execute $SEMVER"
else
    cargo release -p "$PACKAGE" $PUBLISH_FLAG --no-push --execute "$SEMVER"
fi

echo "=== $PACKAGE prepared for release ==="
echo "TAG=${TAG_PREFIX}${SEMVER}"
