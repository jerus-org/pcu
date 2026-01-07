#!/bin/bash
set -exo pipefail

# Release hook for gen-bsky crate - generates CHANGELOG.md
# Called by cargo release via pre-release-hook

NAME="CHANGELOG.md"
PACKAGE=gen-bsky
REPO_DIR="../.."

# CRATE_VERSION is set by cargo release
VERSION="${CRATE_VERSION:-$1}"

if [ -z "$VERSION" ]; then
    echo "Error: No version specified (set CRATE_VERSION or pass as argument)"
    exit 1
fi

gen-changelog generate \
    --display-summaries \
    --name "${NAME}" \
    --package "${PACKAGE}" \
    --repository-dir "${REPO_DIR}" \
    --next-version "${VERSION}"

echo "Generated ${NAME} for ${PACKAGE}@${VERSION}"
