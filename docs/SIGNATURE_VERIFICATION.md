# Commit Signature Verification

This repository implements **anti-impersonation signature verification** to ensure the authenticity and integrity of all commits from trusted identities.

## Overview

The signature verification system prevents malicious actors from impersonating trusted contributors by forging Git author information. This is critical for open source projects that accept contributions from untrusted third parties.

## How It Works

### The Problem

Git allows anyone to set arbitrary author names and emails:

```bash
# Attacker can impersonate a maintainer
git config user.name "Jeremiah Russell"
git config user.email "jerry@jrussell.ie"
git commit -m "Add backdoor"
# Result: Commit appears to be from maintainer!
```

Without signature verification, there's no way to verify that a commit claiming to be from a trusted identity was actually created by that person.

### The Solution

Our signature verification system enforces:

1. **Trusted identities MUST sign commits** - Any commit claiming to be from a maintainer or bot must be cryptographically signed with an approved GPG key
2. **Untrusted contributors may be unsigned** - External contributors don't need GPG setup, keeping the contribution barrier low
3. **Allowlist enforcement** - Only pre-approved keys can sign for trusted identities

### Trust Model

```
┌─────────────────────────────────────────────────────────┐
│                 Commit Verification Flow                 │
└─────────────────────────────────────────────────────────┘

Is author email in trusted list?
        │
        ├─── YES ──> MUST be signed by approved key
        │            │
        │            ├─── Signature valid? ──> ✅ PASS
        │            └─── No signature?   ──> ❌ FAIL (Impersonation!)
        │
        └─── NO ───> Is it on the allow list (see below)?
                     │
                     ├─── YES ──> MUST be signed by one of the entry's keys
                     │            │
                     │            ├─── Good signature, listed key? ──> ✅ PASS
                     │            └─── Anything else?              ──> ❌ FAIL
                     │
                     └─── NO ───> External contributor
                                  │
                                  ├─── Hard stop off (default) ──> ✅ PASS, reported as Unsigned
                                  └─── Hard stop on            ──> ❌ FAIL
```

## Trusted Identities

The following identities are considered trusted and **must** sign all their commits:

| Email | Owner | Key Fingerprint(s) | Purpose |
|-------|-------|-------------------|---------|
| `jerry@jrussell.ie` | Jeremiah Russell | `E576B835ACE207E5` | Primary maintainer |
| `47631109+jerusdp@users.noreply.github.com` | Jeremiah Russell (GitHub) | `E576B835ACE207E5`, `B5690EEEBB952194` | GitHub web merges |
| `171541392+jerus-bot@users.noreply.github.com` | Jerus Bot | `EB85EDFF0BCB42F8` | CI/CD automation |

### GitHub Web-Flow Key

GitHub signs merge commits with its web-flow key `B5690EEEBB952194`. This key is trusted for merge commits performed via GitHub's web interface.

## Allow List and Hard Stop

By default an author who is not a trusted identity passes, whatever the commit's signature. That keeps the barrier low for outside contributors, but it means the check cannot act as a gate: a commit from an unexpected or compromised identity can pass, and with auto-merge be merged. Two settings change that. Both live in `pcu.toml` under `[verify_signatures]`.

```toml
[verify_signatures]
# Fail for any commit from an author who is neither a trusted identity
# nor on the allow list. Off unless you set it.
fail_on_external = true

# Identities accepted without being collaborators.
[[verify_signatures.allow]]
email = "29139614+renovate[bot]@users.noreply.github.com"
keys = ["B5690EEEBB952194"]   # GitHub's web-flow key
pr_authors = ["renovate[bot]"]  # required with that key, see below
description = "Renovate (committed and signed by GitHub)"
```

The hard stop can also be turned on with `pcu verify-signatures --fail-on-external`; either one switches it on.

### Allow-list entries

- **`email`** is the commit's author email, matched ignoring case.
- **`keys` is required.** An author email is self-declared: anyone can write `renovate[bot]@…` into a commit. An entry with no keys would allow-list whoever forges it, so pcu refuses to load one. A commit matches only when its signature is good and made with one of the listed keys (16 to 40 hex digits: a long key ID or a fingerprint).
- **`pr_authors`** is a list of GitHub logins, one of which must have opened the pull request. It is **required when `keys` names GitHub's web-flow key** and optional otherwise (see below).
- **`description`** is a note for whoever reads the file.
- An allow-listed identity with a missing, bad or unlisted-key signature **fails**, like a trusted identity's unsigned commit.
- A trusted identity is always judged as trusted. An allow-list entry can never weaken that rule.

Renovate authors its commits and GitHub commits and signs them, so its entry names the author email and GitHub's web-flow key.

- **`github_user`** (optional) is the GitHub account that has the key registered. pcu fetches that account's GPG keys so the signature can be verified. Only keys matching the entry's `keys` are imported, and the entry's `keys` still decide what passes, so naming an account grants no trust of its own. List the **signing subkey's ID** (what `git log --format=%GK` shows), not just the primary key, because git reports the subkey.

### Where allow-listed keys come from

`git` can only verify a signature whose public key is in the keyring. Keys of write collaborators and GitHub's web-flow key are imported automatically. For every other key an entry lists, pcu tries, in order:

1. the entry's `github_user`, via GitHub's `/users/{login}/gpg_keys`;
2. `keys.openpgp.org`, **only for a full 40-digit fingerprint** (a short ID can be forged by collision, a fingerprint cannot).

If a key still cannot be found, the run **fails with a message naming the entry** rather than reporting every commit as unverifiable. An entry that lists only the web-flow key needs neither.

### GitHub's web-flow key needs a second check

GitHub signs the commits it creates through its web UI and API with the web-flow key, **for whatever author the caller chose**. That signature proves "GitHub made this commit", not who the author is. A commit that claims `renovate[bot]` as its author can arrive with a good, listed web-flow signature even though someone else made it. So an entry that names the web-flow key must also list `pr_authors`, the GitHub logins that must have opened the pull request. GitHub authenticates who opens a PR, so a forged author on someone else's PR no longer matches.

- pcu looks the opener up with the GitHub API, using the PR number from `--pr-number` or, failing that, from CircleCI's `CIRCLE_PULL_REQUEST`.
- If the opener cannot be determined (no PR number, or the lookup fails), the entry **fails closed**: the commit fails rather than passes.
- pcu refuses to load a policy where an entry names the web-flow key without `pr_authors`.

### The policy is read from a trusted ref

pcu reads `pcu.toml` from the git object at `--policy-ref` (default `origin/main`), **not** from the branch under review. A pull request can edit any file it contains; if the policy were read from the working tree, a PR could allow-list its own author, or switch the hard stop off, and pass its own check. A change to the policy therefore takes effect only after it has been merged, and it should go through code-owner review like any other security-sensitive file.

`--policy-ref` is separate from `--base` on purpose. `--base` only chooses where the commit range starts; `--base HEAD~3` must not make the policy come from a commit that predates it. If your default branch is not `main`, set `--policy-ref` to it. It must be a ref the branch under review cannot change.

- A missing file means the default policy.
- A file that exists but is invalid is an **error**, never the default. Unknown keys are errors too, so a misspelt `fail_on_external` cannot silently leave the hard stop off.

### Failures always fail the run

Any failed commit makes `pcu verify-signatures` exit non-zero, and the PR comment is posted **before** it fails, so a failing run replaces an earlier "success" comment instead of leaving it on the PR. The old `--fail-on-unsigned` switch is deprecated: it could only ever be on, and nothing turns a failure into a pass.

### Making it a gate

1. Add the allow-list entries you need (for example Renovate) to `pcu.toml` and merge them.
2. Turn the hard stop on (`fail_on_external = true`, in a second change, so the first lands without it).
3. Make the verify job a **required status check** on the default branch, so a PR cannot merge without it. Without that, a PR can simply delete the job from its own CI configuration.

The verify job must run a pcu release that includes this feature. Until then the settings are ignored.

## Known Limitations and Planned Follow-ups

These are understood and not yet fixed. None weakens the gate: each fails closed or is cosmetic.

| Area | Limitation | Planned change |
|------|-----------|----------------|
| Tests | No end-to-end test of the `verify-signatures` command flow (policy load, API lookups, PR comment, exit code). Logic is unit-tested piece by piece. | Add a test that drives the command against a temporary repository and a mocked GitHub API. |
| Policy changes | A PR that edits `pcu.toml` is judged by the policy on the trusted ref, so it cannot approve itself, but the report does not point out that the policy file changed. | Flag changes to `pcu.toml` in the commit range in the report and PR comment so reviewers look at them. |
| Failure message | A policy failure (bad `pcu.toml`, unknown ref) shows the generic "possible impersonation" action text. | Give policy and configuration failures their own text. |
| `description` | Parsed and shown nowhere. | Show it in the allow-listed table, or remove the field. |
| Key identity | Keys are matched by ID suffix. A fingerprint is recommended but pcu does not compare the signer's fingerprint (`%GF`). | Compare the full fingerprint when an entry lists one, and pin the web-flow fingerprint. |
| Policy file | The policy shares `pcu.toml` with other commands, and there is no lint-only mode. | Add `--check-policy` and consider a dedicated policy file. |
| Where it is enforced | The hard stop is enforced only where pcu v0.6.41 or later runs. In this repository that is `test-verify-signatures` (pcu built from the branch). The toolkit's `verify_commit_signatures` jobs run the pcu in the `base_env` image, so they do not enforce it until the image carries pcu v0.6.41 or later and the toolkit is updated. | Update the image, release the toolkit, then update this repository's toolkit pin (each step by Renovate). |
| Fork pull requests | This project builds GitHub App pipelines, which CircleCI never builds for forked pull requests, so a fork PR gets no CircleCI run (including no signature check). Confirmed on test fork PRs: no check runs or statuses, and GitHub reports them as blocked because the required `validation` check never reports. To accept a contribution, a maintainer follows [ADOPTING_FORK_PRS.md](ADOPTING_FORK_PRS.md): the change is reviewed and brought onto a branch in this repository as one signed commit, crediting the contributor, so `test-verify-signatures` runs on it and the hard stop applies. | Automate the steps as `pcu adopt-pr` ([#1148](https://github.com/jerus-org/pcu/issues/1148)) once the manual process has been refined. If a check on fork PRs themselves is wanted, it has to come from outside CircleCI's pipelines for this project, for example a GitHub Actions `pull_request_target` workflow. |
| Test code | Duplicate helpers across the test modules. | Consolidate into `test_support`. |

## Configuration

Trusted identities and their keys are configured in `.circleci/verify-signatures.sh`:

```bash
TRUSTED_KEYS=(
  "jerry@jrussell.ie|E576B835ACE207E5"
  "47631109+jerusdp@users.noreply.github.com|E576B835ACE207E5,B5690EEEBB952194"
  "171541392+jerus-bot@users.noreply.github.com|EB85EDFF0BCB42F8"
)
```

### Adding a New Trusted Identity

To add a new maintainer or bot:

1. Get their GPG key fingerprint:
   ```bash
   gpg --list-keys their-email@example.com
   ```

2. Add entry to `TRUSTED_KEYS` in `.circleci/verify-signatures.sh`:
   ```bash
   "their-email@example.com|FINGERPRINT1,FINGERPRINT2"
   ```

3. Commit and push the change (signed by an existing maintainer)

### Key Rotation

When a maintainer rotates their GPG key:

1. Add new fingerprint to their entry (comma-separated):
   ```bash
   "jerry@jrussell.ie|OLD_KEY,NEW_KEY"
   ```

2. After transition period (e.g., 1 month), remove old key:
   ```bash
   "jerry@jrussell.ie|NEW_KEY"
   ```

## CI/CD Integration

The signature verification runs automatically on every pull request as part of the `validation` workflow:

```yaml
jobs:
  verify_commit_signatures:
    executor: base-env
    steps:
      - checkout
      - run: Import GitHub web-flow GPG key
      - run: bash .circleci/verify-signatures.sh
```

### Credentials

`verify-signatures` makes three kinds of GitHub call and keeps their credentials apart. The credentials come from the environment only, never from `pcu.toml`, so a file on the branch under review cannot choose them.

| Call | Credential | Notes |
|---|---|---|
| **Collaborator list** | `PCU_COLLAB_READ`, a fine-grained read-only token, when set; otherwise the reads credential below | The endpoint returns only what the caller may see: it needs a caller with write access who is an organisation member. A GitHub App is neither, and sees only direct collaborators. |
| **Other reads** (GPG keys, the pull request opener) | The GitHub App when `PCU_APP_ID` is set (with `PCU_PRIVATE_KEY`), otherwise `--github-token` or `GITHUB_TOKEN` | The App wins even when a token is supplied, and is refused if any of its permissions is more than read. Blank values count as unset. |
| **The pull request comment** (`--post-comment`) | `GITHUB_TOKEN` | Needs write access to pull requests. |

**`PCU_COLLAB_READ`** is a fine-grained personal access token with the organisation as resource owner, access to the repositories that run the check, and Metadata (read) only. Its owner must be an organisation member with write access to those repositories. Give it an expiry and a rotation owner. Hold it in a context loaded only by the verify job.

A job that runs code a branch controls, such as a pcu built from the branch, should hold only read-only credentials and no write token.

### Minimum trusted collaborators

The trust list is refused when it holds fewer collaborators with write access than the policy's `min_trusted`, because a credential that cannot see every collaborator would report the missing ones as external contributors. The default is `2`:

```toml
[verify_signatures]
min_trusted = 2   # 0 turns the check off, for a repository with a single maintainer
```

Like the rest of the policy it is read from the policy ref, so a branch cannot lower it.

### Workflow Requirements

The `trigger_success_pipeline` job requires `verify_commit_signatures` to pass, ensuring no PR can be merged without passing signature verification.

## For Contributors

### External Contributors (No GPG Required)

If you're contributing to this project and are not a maintainer:

- ✅ You do NOT need to set up GPG signing
- ✅ Your commits can be unsigned
- ✅ Standard PR review process applies

Just follow the normal contribution workflow!

### Trusted Maintainers (GPG Required)

If you are a maintainer or have commit access:

- ⚠️ You MUST sign all your commits
- ⚠️ Your signature must use an approved key
- ⚠️ Unsigned commits will fail CI

#### Setting Up GPG Signing

1. **Generate a GPG key** (if you don't have one):
   ```bash
   gpg --full-generate-key
   # Choose RSA, 4096 bits, no expiration
   ```

2. **Configure Git to use your key**:
   ```bash
   # Get your key ID
   gpg --list-secret-keys --keyid-format=long
   
   # Configure Git
   git config --global user.signingkey YOUR_KEY_ID
   git config --global commit.gpgsign true
   ```

3. **Add your public key to GitHub**:
   ```bash
   # Export public key
   gpg --armor --export YOUR_KEY_ID
   
   # Add to GitHub: Settings > SSH and GPG keys > New GPG key
   ```

4. **Get your key added to allowlist**:
   - Submit a PR adding your key fingerprint to `TRUSTED_KEYS`
   - Another maintainer must review and merge

#### Verifying Your Commits

Before pushing:

```bash
# Check if your commit is signed
git log --show-signature -1

# You should see:
# gpg: Signature made ...
# gpg: Good signature from "Your Name <your@email>"
```

## For Maintainers

### Reviewing PRs

When reviewing a PR, check the CI output for the `verify_commit_signatures` job:

#### ✅ Passing Example
```
=== Commit Signature Verification ===
✓ OK   abc12345 feat: add new feature
    External contributor: John Doe <john@example.com>
    Unsigned (allowed for external contributors)

✓ OK   def67890 ci: update workflow
    Verified: Jeremiah Russell <jerry@jrussell.ie>
    Signed by: Jeremiah Russell (garden) (key: E576B835ACE207E5)

✓ All signature checks passed!
```

#### ❌ Failing Example (Impersonation Attempt)
```
=== Commit Signature Verification ===
✗ FAIL abc12345 Add backdoor
    Security violation: Author jerry@jrussell.ie claims trusted identity
    but signature status is 'N' (expected G or U)
    This could be an impersonation attempt!

✗ Signature verification FAILED!
```

**Action**: If verification fails, investigate immediately. This could indicate:
1. Attempted impersonation attack
2. Maintainer forgot to sign
3. Maintainer using wrong key

### Merging Strategy

**Important**: Use **merge commits**, not rebase or squash:

- ✅ Merge commits preserve original signatures
- ❌ Rebase re-signs all commits, losing original verification
- ❌ Squash combines commits, losing individual attribution

#### GitHub Settings

Configure repository settings:
```
✅ Allow merge commits
❌ Allow squash merging (loses attribution)
❌ Allow rebase merging (incompatible with vigilant mode)
```

#### When Merging

1. Ensure all CI checks pass (including signature verification)
2. Click "Create a merge commit" in GitHub UI
3. GitHub will sign the merge commit with its web-flow key
4. Result: Original commits preserve their signatures, merge commit signed by GitHub

### Handling Verification Failures

If a commit from a trusted identity fails verification:

1. **Check if it's the right person**:
   - Ask them to confirm they created the commit
   - They can run: `git log --show-signature <commit-hash>`

2. **If legitimate but wrong key**:
   - They may have a new key not in allowlist
   - Add their new key fingerprint to `TRUSTED_KEYS`

3. **If impersonation**:
   - **Do not merge the PR**
   - Report to GitHub security
   - Review recent commits from that contributor
   - Consider blocking the user

## Security Considerations

### What This Protects Against

✅ **Identity impersonation** - Attacker can't claim to be a maintainer  
✅ **Supply chain attacks** - Can track which actual person/bot contributed code  
✅ **Attribution manipulation** - Clear audit trail of who wrote what  
✅ **Unauthorized commits** - Only approved keys can commit as trusted identities  

### What This Doesn't Protect Against

❌ **Compromised maintainer account** - If attacker steals GPG key  
❌ **Social engineering** - Attacker convinces maintainer to sign malicious code  
❌ **Code review bypasses** - Still need careful review of all changes  
❌ **Dependency attacks** - External dependencies not covered  

### Defense in Depth

This is one layer of security. Also important:

- **Code review** - All PRs require review
- **CI/CD tests** - Automated security scanning
- **Branch protection** - Prevent force pushes
- **Two-factor authentication** - On all maintainer accounts
- **Key management** - Secure GPG private keys

## Troubleshooting

### "Could not compute merge-base"

The verification script couldn't find common history between your branch and main.

**Fix**: Rebase your branch on latest main:
```bash
git fetch origin
git rebase origin/main
```

### "Author claims trusted identity but signature status is 'N'"

You are a maintainer but your commit is unsigned.

**Fix**: Re-commit with signature:
```bash
git commit --amend -S --no-edit
git push --force-with-lease
```

### "Key mismatch"

Your commit is signed but with a key not in the allowlist.

**Fix**: Either:
1. Re-sign with approved key:
   ```bash
   git config user.signingkey APPROVED_KEY_ID
   git commit --amend -S --no-edit
   git push --force-with-lease
   ```

2. Or add your new key to allowlist (via separate PR)

### "Signature verification error"

GPG can't verify your signature.

**Check**:
- Is your key expired? `gpg --list-keys`
- Is your key uploaded to GitHub?
- Is your Git config correct? `git config user.signingkey`

## References

- [GitHub: Signing commits](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits)
- [GitHub: Vigilant mode](https://github.blog/changelog/2021-05-17-vigilant-mode-for-commit-signature-verification/)
- [GPG documentation](https://gnupg.org/documentation/)
- [Git commit signature verification](https://git-scm.com/book/en/v2/Git-Tools-Signing-Your-Work)

## Questions?

If you have questions about signature verification:
- Open an issue
- Contact maintainers
- Review this documentation

Remember: Signature verification protects everyone in the project by ensuring code authenticity!
