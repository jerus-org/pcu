# Plan: Per-Crate Release Workflow

## Overview

Update the release workflow to support **independent versioning** for:
1. **PRLOG.md** - workspace-level PR log with standard `v` prefix tags
2. **Each crate** - individual CHANGELOGs generated via `gen-changelog`

## Design Decisions

### Independent Versioning
- PRLOG tracks all workspace changes (code, CI, docs, etc.)
- Crate CHANGELOGs track only changes affecting that crate
- PRLOG uses standard `v` prefix tags to maintain compatibility with existing links
- Each crate has its own tag prefix: `pcu-v`, `gen-bsky-v`, `gen-linkedin-v`

### PRLOG Ordering: After Crate Releases
PRLOG releases **after** all crate releases complete:
- All crate updates must succeed before PRLOG is updated
- Failed crate releases don't leave PRLOG in inconsistent state

### Push Strategy: Single Aggregated Push
- Individual crate jobs do **not** push
- `aggregate_and_push` job collects all and pushes atomically
- Avoids race conditions from concurrent pushes

### crates.io Recovery: Auto-Detection
Check crates.io API before publishing to handle recovery from partial failures:
- First run: Publishes to crates.io normally
- Recovery run: Skips publish (already done), just creates commits/tags/changelog

## Version Detection

Use `nextsv` to automatically detect which components need releases:

```bash
# PRLOG version (workspace-wide, standard v prefix)
nextsv calculate --prefix "v"

# Per-crate versions (filtered by subdir, crate-specific prefixes)
nextsv calculate --prefix "pcu-v" --subdir "crates/pcu"
nextsv calculate --prefix "gen-bsky-v" --subdir "crates/gen-bsky"
nextsv calculate --prefix "gen-linkedin-v" --subdir "crates/gen-linkedin"
```

**Logic:**
- If `nextsv` returns a bump level (patch/minor/major) → release needed
- If `nextsv` returns "none" → skip that component

## Release Scenarios

| Scenario | PRLOG Action | Crate Action |
|----------|--------------|--------------|
| All changes impact pcu only | Update PRLOG version | Release pcu with CHANGELOG |
| Changes impact all crates | Update PRLOG version | Release all crates with CHANGELOGs |
| Only CI/workspace changes | Update PRLOG version | No crate releases |
| Changes impact gen-bsky only | Update PRLOG version | Release gen-bsky with CHANGELOG |

## Workflow Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           RELEASE WORKFLOW                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐                                                           │
│  │    tools     │                                                           │
│  └──────┬───────┘                                                           │
│         │                                                                   │
│         ▼                                                                   │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    PARALLEL DETECT + PREPARE (NO PUSH)               │  │
│  │                                                                       │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐       │  │
│  │  │  prepare-pcu    │  │ prepare-gen-bsky│  │prepare-gen-linkedin│     │  │
│  │  │  nextsv detect  │  │ nextsv detect   │  │ nextsv detect    │       │  │
│  │  │  check crates.io│  │ check crates.io │  │ check crates.io  │       │  │
│  │  │  cargo release  │  │ cargo release   │  │ cargo release    │       │  │
│  │  │  gen-changelog  │  │ gen-changelog   │  │ gen-changelog    │       │  │
│  │  │  persist to     │  │ persist to      │  │ persist to       │       │  │
│  │  │  workspace      │  │ workspace       │  │ workspace        │       │  │
│  │  └────────┬────────┘  └────────┬────────┘  └────────┬─────────┘       │  │
│  │           │                    │                    │                 │  │
│  └───────────┼────────────────────┼────────────────────┼─────────────────┘  │
│              │                    │                    │                    │
│              └────────────────────┼────────────────────┘                    │
│                                   ▼                                         │
│                        ┌─────────────────────┐                              │
│                        │   aggregate-push    │                              │
│                        │   Push all tags     │                              │
│                        │   Push to main      │                              │
│                        └──────────┬──────────┘                              │
│                                   │                                         │
│                                   ▼                                         │
│                        ┌─────────────────────┐                              │
│                        │   release-prlog     │                              │
│                        │   nextsv detect     │                              │
│                        │   Update PRLOG.md   │                              │
│                        │   Commit + tag      │                              │
│                        │   Push              │                              │
│                        └─────────────────────┘                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Failure Scenarios

### Pre-Publish Failure
Failure before `cargo publish` (e.g., test failure):
- Nothing pushed or published
- Clean slate for retry after fix

### Post-Publish Failure
Failure after `cargo publish` but before aggregate push:
- Crates are on crates.io but repo doesn't reflect it
- Recovery: Re-run workflow with auto-detection (checks crates.io, uses `--no-publish`)

## Implementation Phases

### Phase 1: PCU Scripts and Local Testing

Build and validate all scripts locally before CI integration.

#### 1.1 Create `scripts/detect-crate-release.sh`
```bash
#!/bin/bash
set -eo pipefail

# Usage: ./scripts/detect-crate-release.sh <tag_prefix> <subdir>
# Example: ./scripts/detect-crate-release.sh "pcu-v" "crates/pcu"
# Outputs: Sets RELEASE_NEEDED, RELEASE_BUMP, SEMVER environment variables

TAG_PREFIX=$1
SUBDIR=$2

if [ -z "$TAG_PREFIX" ] || [ -z "$SUBDIR" ]; then
    echo "Usage: $0 <tag_prefix> <subdir>"
    exit 1
fi

BUMP=$(nextsv calculate --prefix "$TAG_PREFIX" --subdir "$SUBDIR" 2>/dev/null || echo "none")

if [ "$BUMP" = "none" ]; then
    echo "No release needed for $SUBDIR"
    echo "RELEASE_NEEDED=false"
    exit 0
fi

VERSION=$(nextsv --number calculate --prefix "$TAG_PREFIX" --subdir "$SUBDIR")

echo "Release needed: $BUMP → $VERSION"
echo "RELEASE_NEEDED=true"
echo "RELEASE_BUMP=$BUMP"
echo "SEMVER=$VERSION"
```

#### 1.2 Create `scripts/check-crates-io-version.sh`
```bash
#!/bin/bash
set -eo pipefail

# Usage: ./scripts/check-crates-io-version.sh <crate_name> <version>
# Returns: 0 if version exists, 1 if not

CRATE_NAME=$1
TARGET_VERSION=$2

if [ -z "$CRATE_NAME" ] || [ -z "$TARGET_VERSION" ]; then
    echo "Usage: $0 <crate_name> <version>"
    exit 1
fi

if curl -s "https://crates.io/api/v1/crates/${CRATE_NAME}/versions" | \
   jq -e ".versions[] | select(.num == \"${TARGET_VERSION}\")" > /dev/null 2>&1; then
    echo "Version ${TARGET_VERSION} exists on crates.io"
    exit 0
else
    echo "Version ${TARGET_VERSION} not found on crates.io"
    exit 1
fi
```

#### 1.3 Create `scripts/release-prlog.sh`
```bash
#!/bin/bash
set -exo pipefail

# Usage: ./scripts/release-prlog.sh <version>
# Example: ./scripts/release-prlog.sh 0.7.0

VERSION=$1
DATE=$(date +%Y-%m-%d)
TAG="prlog-v${VERSION}"

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
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
```

#### 1.4 Create `scripts/prepare-crate-release.sh`
```bash
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
```

#### 1.5 Create Release Hooks for Each Crate

**`crates/pcu/release-hook.sh`:**
```bash
#!/bin/bash
set -exo pipefail

NAME="CHANGELOG.md"
PACKAGE=pcu
REPO_DIR="../.."

gen-changelog generate \
    --display-summaries \
    --name ${NAME} \
    --package ${PACKAGE} \
    --repository-dir ${REPO_DIR} \
    --next-version "$SEMVER"
```

**`crates/gen-bsky/release-hook.sh`:**
```bash
#!/bin/bash
set -exo pipefail

NAME="CHANGELOG.md"
PACKAGE=gen-bsky
REPO_DIR="../.."

gen-changelog generate \
    --display-summaries \
    --name ${NAME} \
    --package ${PACKAGE} \
    --repository-dir ${REPO_DIR} \
    --next-version "$SEMVER"
```

**`crates/gen-linkedin/release-hook.sh`:**
```bash
#!/bin/bash
set -exo pipefail

NAME="CHANGELOG.md"
PACKAGE=gen-linkedin
REPO_DIR="../.."

gen-changelog generate \
    --display-summaries \
    --name ${NAME} \
    --package ${PACKAGE} \
    --repository-dir ${REPO_DIR} \
    --next-version "$SEMVER"
```

#### 1.6 Update Per-Crate release.toml Files

**`crates/pcu/release.toml`:**
```toml
pre-release-commit-message = "chore: Release pcu"
tag-message = "{{tag_name}}"
tag-name = "pcu-v{{version}}"
pre-release-replacements = []
pre-release-hook = ["./release-hook.sh"]
```

**`crates/gen-bsky/release.toml`:**
```toml
pre-release-commit-message = "chore: Release gen-bsky"
tag-message = "{{tag_name}}"
tag-name = "gen-bsky-v{{version}}"
pre-release-replacements = []
pre-release-hook = ["./release-hook.sh"]
```

**`crates/gen-linkedin/release.toml`** (new):
```toml
pre-release-commit-message = "chore: Release gen-linkedin"
tag-message = "{{tag_name}}"
tag-name = "gen-linkedin-v{{version}}"
pre-release-replacements = []
pre-release-hook = ["./release-hook.sh"]
```

#### 1.7 Update Root release.toml

Remove PRLOG replacements (handled by `scripts/release-prlog.sh`):
```toml
sign-tag = true
sign-commit = true
consolidate-commits = true
allow-branch = ["main"]
# PRLOG updates handled by scripts/release-prlog.sh
```

#### 1.8 Create `crates/gen-linkedin/CHANGELOG.md`
```markdown
# Changelog

All notable changes to this project will be documented in this file.
```

#### 1.9 Bootstrap: Create Initial prlog-v Tag
```bash
git tag -s -m "prlog-v0.7.0 - baseline for independent PRLOG versioning" prlog-v0.7.0 v0.6.3
git push origin prlog-v0.7.0
```

### Phase 2: CI Integration with PCU Scripts

Update `.circleci/release.yml` to use the local PCU scripts.

```yaml
version: 2.1

parameters:
  min_rust_version:
    type: string
    default: "1.87"
  fingerprint:
    type: string
    default: SHA256:OkxsH8Z6Iim6WDJBaII9eTT9aaO1f3eDc6IpsgYYPVg

orbs:
  toolkit: jerus-org/circleci-toolkit@4.0.1

jobs:
  tools:
    executor:
      name: toolkit/rust_env_rolling
    steps:
      - run:
          name: Verify tools
          command: |
            set -ex
            nextsv --version
            pcu --version
            cargo release --version
            gen-changelog --version
            jq --version

  prepare-crate:
    parameters:
      package:
        type: string
      tag_prefix:
        type: string
      subdir:
        type: string
    executor:
      name: toolkit/rust_env_rolling
    steps:
      - checkout
      - add_ssh_keys:
          fingerprints:
            - << pipeline.parameters.fingerprint >>
      - toolkit/gpg_key
      - toolkit/git_config
      - run:
          name: Prepare << parameters.package >> release
          command: |
            chmod +x scripts/*.sh
            chmod +x << parameters.subdir >>/release-hook.sh
            ./scripts/prepare-crate-release.sh "<< parameters.package >>" "<< parameters.tag_prefix >>" "<< parameters.subdir >>"
      - persist_to_workspace:
          root: .
          paths:
            - .git
            - << parameters.subdir >>

  aggregate-push:
    executor:
      name: toolkit/rust_env_rolling
    steps:
      - attach_workspace:
          at: .
      - add_ssh_keys:
          fingerprints:
            - << pipeline.parameters.fingerprint >>
      - toolkit/gpg_key
      - toolkit/git_config
      - run:
          name: Push all crate releases
          command: |
            git push origin main --tags

  release-prlog:
    executor:
      name: toolkit/rust_env_rolling
    steps:
      - checkout
      - add_ssh_keys:
          fingerprints:
            - << pipeline.parameters.fingerprint >>
      - toolkit/gpg_key
      - toolkit/git_config
      - run:
          name: Detect and release PRLOG
          command: |
            set -exo pipefail
            chmod +x scripts/*.sh

            BUMP=$(nextsv calculate --prefix "prlog-v" 2>/dev/null || echo "first")

            if [ "$BUMP" = "none" ]; then
              echo "No PRLOG release needed"
              exit 0
            fi

            if [ "$BUMP" = "first" ]; then
              CURRENT=$(git describe --tags --abbrev=0 --match "v*" 2>/dev/null || echo "v0.6.3")
              VERSION=$(echo $CURRENT | sed 's/v//' | awk -F. '{print $1"."$2+1".0"}')
            else
              VERSION=$(nextsv --number calculate --prefix "prlog-v")
            fi

            ./scripts/release-prlog.sh "$VERSION"
            git push origin main --tags

workflows:
  release:
    jobs:
      - tools

      # Parallel: Prepare all crate releases (NO PUSH)
      - prepare-crate:
          name: prepare-pcu
          requires: [tools]
          package: pcu
          tag_prefix: "pcu-v"
          subdir: "crates/pcu"
          context:
            - release
            - bot-check

      - prepare-crate:
          name: prepare-gen-bsky
          requires: [tools]
          package: gen-bsky
          tag_prefix: "gen-bsky-v"
          subdir: "crates/gen-bsky"
          context:
            - release
            - bot-check

      - prepare-crate:
          name: prepare-gen-linkedin
          requires: [tools]
          package: gen-linkedin
          tag_prefix: "gen-linkedin-v"
          subdir: "crates/gen-linkedin"
          context:
            - release
            - bot-check

      # Sequential: Push all crate releases atomically
      - aggregate-push:
          requires:
            - prepare-pcu
            - prepare-gen-bsky
            - prepare-gen-linkedin
          context:
            - release
            - bot-check

      # Sequential: PRLOG release (after all crates pushed)
      - release-prlog:
          requires:
            - aggregate-push
          context:
            - release
            - bot-check
```

### Phase 3: Toolkit Migration

After validating the workflow in PCU, migrate reusable components to circleci-toolkit.

#### Toolkit Components to Create

| Component | Type | Description |
|-----------|------|-------------|
| `detect_crate_release` | Command | Detect if crate needs release using nextsv |
| `check_crates_io_version` | Command | Check if version already exists on crates.io |
| `detect_and_prepare_crate` | Job | Detect, check crates.io, build release locally, persist artifacts (NO PUSH) |
| `aggregate_and_push` | Job | Collect all prepared releases, push atomically |
| `detect_and_release_prlog` | Job | Detect and release PRLOG (runs last) |

#### Migration Steps

1. Extract `scripts/detect-crate-release.sh` → `toolkit/detect_crate_release` command
2. Extract `scripts/check-crates-io-version.sh` → `toolkit/check_crates_io_version` command
3. Create `toolkit/detect_and_prepare_crate` job using the commands
4. Create `toolkit/aggregate_and_push` job
5. Create `toolkit/detect_and_release_prlog` job
6. Release new toolkit version
7. Update PCU's `.circleci/release.yml` to use toolkit jobs

#### Final Toolkit-Based Workflow

```yaml
workflows:
  release:
    jobs:
      - tools

      - toolkit/detect_and_prepare_crate:
          name: prepare-pcu
          requires: [tools]
          package: pcu
          tag_prefix: "pcu-v"
          subdir: "crates/pcu"
          ssh_fingerprint: << pipeline.parameters.fingerprint >>
          context: [release, bot-check]

      - toolkit/detect_and_prepare_crate:
          name: prepare-gen-bsky
          requires: [tools]
          package: gen-bsky
          tag_prefix: "gen-bsky-v"
          subdir: "crates/gen-bsky"
          ssh_fingerprint: << pipeline.parameters.fingerprint >>
          context: [release, bot-check]

      - toolkit/detect_and_prepare_crate:
          name: prepare-gen-linkedin
          requires: [tools]
          package: gen-linkedin
          tag_prefix: "gen-linkedin-v"
          subdir: "crates/gen-linkedin"
          ssh_fingerprint: << pipeline.parameters.fingerprint >>
          context: [release, bot-check]

      - toolkit/aggregate_and_push:
          requires: [prepare-pcu, prepare-gen-bsky, prepare-gen-linkedin]
          ssh_fingerprint: << pipeline.parameters.fingerprint >>
          context: [release, bot-check]

      - toolkit/detect_and_release_prlog:
          requires: [toolkit/aggregate_and_push]
          ssh_fingerprint: << pipeline.parameters.fingerprint >>
          release_script: "./scripts/release-prlog.sh"
          context: [release, bot-check]
```

## Summary of Changes

| File | Action | Phase |
|------|--------|-------|
| `scripts/detect-crate-release.sh` | Create | 1 |
| `scripts/check-crates-io-version.sh` | Create | 1 |
| `scripts/release-prlog.sh` | Create | 1 |
| `scripts/prepare-crate-release.sh` | Create | 1 |
| `crates/pcu/release-hook.sh` | Create | 1 |
| `crates/gen-bsky/release-hook.sh` | Create | 1 |
| `crates/gen-linkedin/release-hook.sh` | Create | 1 |
| `crates/pcu/release.toml` | Update | 1 |
| `crates/gen-bsky/release.toml` | Update | 1 |
| `crates/gen-linkedin/release.toml` | Create | 1 |
| `crates/gen-linkedin/CHANGELOG.md` | Create | 1 |
| `release.toml` (root) | Update | 1 |
| `prlog-v0.7.0` tag | Create | 1 |
| `.circleci/release.yml` | Update | 2 |
| `toolkit/detect_crate_release` | Create | 3 |
| `toolkit/check_crates_io_version` | Create | 3 |
| `toolkit/detect_and_prepare_crate` | Create | 3 |
| `toolkit/aggregate_and_push` | Create | 3 |
| `toolkit/detect_and_release_prlog` | Create | 3 |
| `.circleci/release.yml` | Update to toolkit | 3 |
