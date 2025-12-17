# Using the --from-merge Flag

## Overview

The `--from-merge` flag enables `pcu pr` to update the PRLOG after a PR has been merged to the main branch. This is useful for CI workflows that need to handle untrusted PRs from forked repositories with better security compliance.

## How It Works

When `--from-merge` is set:

1. The command can run on the `main` branch (normally it exits early)
2. It looks up the PR associated with the HEAD commit using GitHub's GraphQL API
3. It works with all merge strategies:
   - Merge commits
   - Squash and merge
   - Rebase and merge

The tool uses GitHub's `associatedPullRequests` API to find the PR that introduced the current HEAD commit, regardless of how it was merged.

## Usage

### Basic Usage

```bash
pcu -vv pr --from-merge --push
```

### CI Configuration Example

In CircleCI (`.circleci/config.yml`):

```yaml
jobs:
  update_prlog_after_merge:
    docker:
      - image: rust:latest
    steps:
      - checkout
      - run:
          name: Update PRLOG for merged PR
          command: |
            cargo install pcu
            pcu -vv pr --from-merge --push
    filters:
      branches:
        only: main
```

### Environment Variables

The command still requires these environment variables:
- `CIRCLE_PROJECT_USERNAME` (or set via `PCU_USERNAME`)
- `CIRCLE_PROJECT_REPONAME` (or set via `PCU_REPONAME`)
- `GITHUB_TOKEN` - GitHub personal access token or app token with repo access

### Comparison with Normal Mode

**Normal mode** (without `--from-merge`):
- Runs on PR branches
- Uses `CIRCLE_PULL_REQUEST` environment variable
- Exits early if on main branch

**From-merge mode** (with `--from-merge`):
- Can run on main branch
- Uses HEAD commit SHA to find associated PR via GraphQL
- Works after PR is already merged

## Error Handling

The command will fail with appropriate errors if:
- No PR is associated with the current commit
- GitHub API is unreachable
- Authentication fails
- The commit is not found in the repository

## Testing

To test locally (requires valid GitHub token):

```bash
# Set up environment
export CIRCLE_PROJECT_USERNAME=your-org
export CIRCLE_PROJECT_REPONAME=your-repo
export GITHUB_TOKEN=your-token

# Test on a commit that was part of a merged PR
git checkout main
pcu -vv pr --from-merge
```
