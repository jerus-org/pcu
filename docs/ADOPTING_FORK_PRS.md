# Adopting a fork pull request

Work instruction for a maintainer. It describes how to bring an approved pull
request from a fork onto a branch in this repository, so that it gets CI and
can be merged under the normal rules.

> **Status:** a manual procedure to be tried and refined before it is
> automated as `pcu adopt-pr` ([#1148](https://github.com/jerus-org/pcu/issues/1148)).
> Record what you find in the notes table at the end.

## Why this is needed

- **Fork pull requests get no CI run.** This project's CircleCI pipelines are
  GitHub App pipelines, which CircleCI never builds for forks. The required
  `validation - 9cc51dba` check therefore never reports, and GitHub shows the
  pull request as blocked. That is the intended default.
- **The hard stop judges the author of every commit.** With
  `fail_on_external = true`, a commit by someone who is not a trusted
  collaborator or on the allow list fails verification. Re-signing the
  contributor's commit does not change its author, so it still fails (see
  [SIGNATURE_VERIFICATION.md](SIGNATURE_VERIFICATION.md)).
- **So the maintainer takes ownership.** The change is brought in as one commit
  that the maintainer authors and signs, with a `Co-authored-by` trailer for
  each contributor so they are credited.

## Before you start

- The contribution is wanted: agreed in the pull request or an issue.
- You have push access to this repository and a working signing key
  (`git config user.signingkey` and `commit.gpgsign true`).
- Your clone's `origin` is this repository.
- Replace `N` below with the pull request number.

## Warning: reviewing is the control

Once the commits are on a branch here and a pull request is open, CI runs them.
CI builds pcu from the branch and runs it with credentials, and `build.rs`,
dependencies and CI configuration all run in jobs. Until that exposure is
reduced, **your review is the only barrier.**

- Read the whole diff before you build, test or run anything from the fork.
- Prefer a throwaway container or VM for any local build of the fork's code.
- Open the pull request here only after the review is complete.

## Steps

### 1. Fetch the fork's commits

GitHub exposes every pull request head under `refs/pull/N/head`, forks included:

```bash
git fetch origin pull/N/head:fork-pr-N
```

Do not check the branch out or build it yet.

### 2. Review the change

```bash
git diff --stat origin/main...fork-pr-N
```

**If this prints nothing, stop.** The pull request has no change against
`main` (for example only empty commits, or its content is already on `main`),
so there is nothing to adopt and no pull request could be opened from it. Roll
back as described under [Abandoning](#abandoning-and-rolling-back).

Flag paths that run in CI or change what runs. Any output here means those
files need line-by-line attention:

```bash
git diff --name-only origin/main...fork-pr-N | grep -E '^(\.circleci/|\.github/|\.claude/)|(^|/)build\.rs$|(^|/)Cargo\.(toml|lock)$|(^|/)pcu\.toml$|(^|/)scripts/|\.sh$'
```

For `Cargo.toml` and `Cargo.lock`, list every added or changed dependency and
check each one is expected. Check for binary files (a `-` in the first column):

```bash
git diff --numstat origin/main...fork-pr-N | awk '$1=="-"'
```

If the change touches `.circleci/**` or `.github/**`, review that part
separately and with extra care, or ask the contributor to split it out.

### 3. List the contributors

```bash
git log --no-merges --format='%an <%ae>' origin/main..fork-pr-N | sort -u
```

Each line becomes a `Co-authored-by` trailer. Check the identities look like
real contributors.

### 4. Make a branch and squash the change onto it

```bash
git switch -c contrib/pr-N origin/main
```

```bash
git merge --squash fork-pr-N
```

If there are conflicts, resolve them. That resolution is now your change, so
review it too. Then check the squashed change matches what you reviewed (same
files, same counts as the step 2 `--stat`):

```bash
git diff --cached --stat
```

If that prints nothing, nothing was staged: stop and roll back. Committing
would fail, or the next step would appear to succeed against an older commit.

### 5. Commit as yourself, signed, crediting the contributors

Use the repository's conventional commit format for the title.

```bash
git commit -S -F - <<'MSG'
<type>: <title of the pull request>

<short description of the change>

Supersedes #N.

Co-authored-by: Contributor Name <contributor@example.com>
MSG
```

Check the commit is yours and signed (expect `G`):

```bash
git log -1 --format='%G? %an | %s'
```

The line must show your name and the new title. If it shows an older commit
(for example the bot's PRLOG update), nothing was committed: stop and roll
back.

### 6. Push and open the pull request

```bash
git push -u origin contrib/pr-N
```

Open a pull request from `contrib/pr-N` into `main`. In the description say
that it supersedes #N, name the original contributor, say the diff was
reviewed in full, and list any flagged paths from step 2 (or "none").

### 7. Check the result

On the new pull request, `test-verify-signatures` should report the commit as
**Trusted identity (signed, verified)**, and `validation - 9cc51dba` should
report and pass. Merge under the normal rules (approval and code-owner review).
After the merge, GitHub shows the contributor from the trailer.

### 8. Close the original

Comment on #N with a link to the new pull request and a thank-you, then close
it. Delete `contrib/pr-N` and your local `fork-pr-N` branch.

## Do not

- **Do not push the fork's commits as they are.** The hard stop fails them, and
  unreviewed code would run in CI.
- **Do not `cherry-pick -S` them.** The signature is yours but the author stays
  the contributor, so the hard stop still fails.
- **Do not add the contributor to the allow list.** The allow list is for
  identities whose keys can be pinned, such as bots. People are credited with
  a trailer instead.
- **Do not merge by bypass** to avoid doing the steps above.

## Edge cases

| Case | What to do |
|---|---|
| The change is empty (no diff against `main`) | Stop at step 2 and roll back. There is nothing to adopt. |
| `main` moves while the new pull request waits | Bring the branch up to date (see below) and re-check the result before it is merged. |
| The change touches `.circleci/**`, `.github/**` or `build.rs` | Review those files first and separately, or ask for them to be split into another change. |
| Conflicts with `main` | Resolve in step 4, then re-check the diff against what you reviewed. |
| The contributor pushes again after you fetched | Re-run steps 1 and 2 and review what changed since, before step 4. |
| Several contributors | One trailer per distinct person from step 3. |
| A very large change | Ask for it to be split; a single squashed commit is harder to review. |

## Keeping the branch up to date

If `main` moves before the new pull request is merged, bring the branch up to
date with the GitHub "Update branch" button or locally:

```bash
git fetch origin main
```

```bash
git merge origin/main
```

- Do not rebase a branch you have pushed.
- Any push dismisses existing approvals, and the ruleset requires approval of
  the last push, so someone other than the person who pushed must approve
  again.
- `pcu verify-signatures` does not check merge commits, so it will not look at
  the merge result. If the merge needed conflict resolution, review the
  result yourself.

## Abandoning and rolling back

If you stop part-way (an empty change, an unsuitable change, or the
contributor withdraws), undo the local work and anything pushed:

```bash
git switch --detach origin/main
```

```bash
git branch -D contrib/pr-N fork-pr-N
```

If you already pushed, delete the remote branch (`git push origin --delete
contrib/pr-N`) and close any pull request opened from it. Then comment on the
original pull request to say what happened.

## Notes to record while you try this

Fill this in as you go. It decides what `pcu adopt-pr` has to do.

| Step | What happened | Friction or surprise | Idea |
|---|---|---|---|
| | | | |
