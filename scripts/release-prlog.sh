#!/bin/bash
set -exo pipefail

# Usage: ./scripts/release-prlog.sh <version>
# Example: ./scripts/release-prlog.sh 0.7.0
#
# PRLOG uses standard v prefix tags (v0.7.0) to maintain compatibility
# with existing version links. Crate releases use crate-specific prefixes.

VERSION="${1}"
DATE=$(date +%Y-%m-%d)
TAG="v${VERSION}"

if [[ -z "${VERSION}" ]]; then
    echo "Usage: $0 <version>" >&2
    exit 1
fi

# Update PRLOG.md
sed -i "s/## \[Unreleased\]/## [${VERSION}] - ${DATE}/" PRLOG.md
sed -i "s/\[Unreleased\]:/[${VERSION}]:/" PRLOG.md
sed -i "s/\.\.\.HEAD/...${TAG}/" PRLOG.md

# Add new Unreleased section
sed -i "/## \[${VERSION}\]/i ## [Unreleased]\n\n### Added\n\n### Changed\n\n### Fixed\n" PRLOG.md

# Commit and tag
git add PRLOG.md
git commit -S -s -m "chore: Release PRLOG v${VERSION}"
git tag -s -m "${TAG}" "${TAG}"

echo "PRLOG released as ${TAG}"
