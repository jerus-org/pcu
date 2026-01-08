#!/bin/bash
set -eo pipefail

# Usage: ./scripts/check-crates-io-version.sh <crate_name> <version>
# Returns: 0 if version exists, 1 if not

CRATE_NAME="${1}"
TARGET_VERSION="${2}"

if [[ -z "${CRATE_NAME}" ]] || [[ -z "${TARGET_VERSION}" ]]; then
    echo "Usage: $0 <crate_name> <version>"
    exit 1
fi

# crates.io API requires a user agent per their data access policy
USER_AGENT="pcu-release-script/1.0 (https://github.com/jerus-org/pcu)"

if curl -s -H "User-Agent: ${USER_AGENT}" "https://crates.io/api/v1/crates/${CRATE_NAME}/versions" | \
   jq -e ".versions[] | select(.num == \"${TARGET_VERSION}\")" > /dev/null 2>&1; then
    echo "Version ${TARGET_VERSION} exists on crates.io"
    exit 0
else
    echo "Version ${TARGET_VERSION} not found on crates.io"
    exit 1
fi
