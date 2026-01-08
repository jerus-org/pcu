#!/bin/bash
set -eo pipefail

# Usage: ./scripts/detect-crate-release.sh <tag_prefix> <subdir>
# Example: ./scripts/detect-crate-release.sh "pcu-v" "crates/pcu"
# Outputs: Sets RELEASE_NEEDED, RELEASE_BUMP, SEMVER environment variables

TAG_PREFIX="${1}"
SUBDIR="${2}"

if [[ -z "${TAG_PREFIX}" ]] || [[ -z "${SUBDIR}" ]]; then
    echo "Usage: $0 <tag_prefix> <subdir>"
    exit 1
fi

BUMP=$(nextsv calculate --prefix "${TAG_PREFIX}" --subdir "${SUBDIR}" 2>/dev/null || echo "none")

if [[ "${BUMP}" = "none" ]]; then
    echo "No release needed for ${SUBDIR}"
    echo "RELEASE_NEEDED=false"
    exit 0
fi

VERSION=$(nextsv --number calculate --prefix "${TAG_PREFIX}" --subdir "${SUBDIR}" | tail -1)

echo "Release needed: ${BUMP} -> ${VERSION}"
echo "RELEASE_NEEDED=true"
echo "RELEASE_BUMP=${BUMP}"
echo "SEMVER=${VERSION}"
