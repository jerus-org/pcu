# RFC 0001: `verify-signatures` Subcommand

## Summary

Add a `pcu verify-signatures` subcommand that implements anti-impersonation GPG signature verification for commits. This will replace the bash script prototype with a production-quality Rust implementation that can be distributed as a single binary and reused across all jerus-org repositories.

## Motivation

### The Problem

Git allows anyone to set arbitrary author names and emails, making it trivial for attackers to impersonate trusted maintainers:

```bash
# Attacker can impersonate a maintainer
git config user.name "Alice Maintainer"
git config user.email "alice@example.org"
git commit -m "Add backdoor"
# Result: Commit appears to be from maintainer!
```

Without signature verification, there's no cryptographic proof that a commit claiming to be from a trusted identity was actually created by that person. This is a critical supply chain security risk for open source projects.

### Current State

A bash script prototype was developed and tested in the `named-colour` repository:
- Location: `.circleci/verify-signatures.sh` (277 lines)
- Documentation: `docs/SIGNATURE_VERIFICATION.md`
- Status: Working prototype, successfully deployed

The bash script works but has limitations:
- **Security risk**: Script must be copied to every repository, creating N attack surfaces
- **Maintenance burden**: Changes must be manually propagated across repos
- **No testing**: Bash scripts are difficult to unit test
- **No type safety**: Prone to subtle bugs
- **Distribution**: Requires bash, curl, jq, gpg, git on CI image

### Proposed Solution

Migrate the bash script functionality into `pcu` as a `verify-signatures` subcommand:

```bash
pcu verify-signatures [options]
```

**Benefits**:
- ✅ Single source of truth (one codebase to audit)
- ✅ Version controlled (repos pin specific pcu versions)
- ✅ Comprehensive testing (unit + integration tests)
- ✅ Type safety (Rust's memory safety + strong typing)
- ✅ Better error handling (Result types, structured errors)
- ✅ Distribution (statically-linked binary, no runtime deps)
- ✅ Reusable (circleci-toolkit orb can call `pcu verify-signatures`)
- ✅ Privacy-preserving (no personal data logged)

## Design

### CLI Interface

```bash
pcu verify-signatures [OPTIONS]

OPTIONS:
    --base <REF>           Base ref for commit range [default: origin/main]
    --head <REF>           Head ref for commit range [default: HEAD]
    --repo-owner <OWNER>   Repository owner (auto-detected from git remote or $CIRCLE_PROJECT_USERNAME)
    --repo-name <NAME>     Repository name (auto-detected from git remote or $CIRCLE_PROJECT_REPONAME)
    --fetch-depth <N>      Git fetch depth [default: 200]
    --github-token <TOKEN> GitHub PAT for API access [env: GITHUB_TOKEN]
    --fail-on-unsigned     Fail if trusted identities have unsigned commits [default: true]
    -v, --verbose          Increase logging verbosity
    -q, --quiet            Decrease logging verbosity
    -h, --help             Print help
```

### Verification Algorithm

```rust
for each commit in range(base..head) {
    let author_email = commit.author_email();
    let is_trusted = trusted_identities.contains(author_email);
    
    if is_trusted {
        // TRUSTED IDENTITY: Must be signed with approved key
        match commit.signature_status() {
            Good | Untrusted => {
                // Verify key is in allowlist
                if !approved_keys_for_email(author_email).contains(commit.key_id()) {
                    return Error::KeyMismatch;
                }
            }
            None | Bad | Unknown | Expired | Revoked => {
                return Error::ImpersonationAttempt;
            }
        }
    } else {
        // EXTERNAL CONTRIBUTOR: Unsigned OK (low barrier)
        // Signed is bonus (we verify if present)
    }
}
```

### Trust List Management

Trusted identities will be **dynamically fetched** from GitHub API (no hardcoded lists):

```rust
// GET /repos/{owner}/{repo}/collaborators
// Filter: .permissions.push == true OR .permissions.admin == true
let collaborators = github_client
    .repos
    .list_collaborators(owner, repo)
    .send()
    .await?;

let trusted_users = collaborators
    .into_iter()
    .filter(|c| c.permissions.push || c.permissions.admin)
    .collect();

// For each trusted user, fetch their GPG keys
// GET /users/{username}/gpg_keys
for user in trusted_users {
    let keys = github_client
        .users
        .list_gpg_keys_for_user(&user.login)
        .send()
        .await?;
    
    for key in keys {
        // Import to local GPG keyring
        gpg_import(&key.raw_key)?;
        
        // Map emails to key IDs
        for email in &key.emails {
            trust_map.insert(email.email.clone(), key.key_id.clone());
        }
    }
}
```

### Privacy-Preserving Logging

**Critical**: Logging MUST NOT expose personal identifiable information (PII) of contributors.

```rust
// ❌ BAD: Exposes personal data
log::info!("Trusting: alice@example.org → ABCD1234EF567890");
log::info!("Processing: alicemaintainer");

// ✅ GOOD: Privacy-preserving
log::debug!("Imported {} GPG key(s) from GitHub", key_count);
log::info!("Found {} trusted collaborator(s) with write access", trusted_count);
log::trace!("Trust map contains {} email→key mappings", trust_map.len());
```

**Output format** (privacy-safe):

```
=== Commit Signature Verification ===
Fetching trusted identities from GitHub...
Repository: org/repo

ℹ  Found 3 collaborator(s) with write access
ℹ  Imported 5 GPG key(s)
ℹ  Built trust map with 8 email→key mappings

ℹ  Checking commit range: abc12345..def67890
ℹ  Found 3 commit(s) to verify

✓ OK   abc12345 feat: add new feature
    External contributor (unsigned, allowed)

✓ OK   def67890 ci: update workflow
    Trusted identity (signed, verified)

✓ OK   123abcde Merge pull request #42
    Trusted identity (signed, verified)

=== Verification Summary ===
Commits checked:         3
Trusted verified:        2
External contributors:   1

✓ All signature checks passed!
```

**Verbose mode** (`-v`) may include commit SHA and subject, but still no PII:

```
✓ OK   def67890 ci: update workflow
    Trusted identity verified
    Key ID: ****1234 (last 4 digits only)
```

**Error messages** must also be privacy-preserving:

```rust
// ❌ BAD: Exposes email
return Err(VerificationError::ImpersonationAttempt {
    email: "alice@example.org",
    sha: "abc123",
});

// ✅ GOOD: Privacy-preserving
return Err(VerificationError::ImpersonationAttempt {
    sha: "abc123",
    message: "Commit claims trusted identity but is unsigned",
});
```

### Module Structure

```
crates/pcu/src/
├── cli/
│   ├── mod.rs
│   └── verify_signatures/
│       ├── mod.rs              # CLI command definition
│       ├── commands/
│       │   └── cmd_verify.rs   # Main command implementation
│       └── tests.rs            # CLI integration tests
├── ops/
│   └── signature_ops.rs        # Core verification logic
└── client/
    ├── gpg.rs                  # GPG operations (import, verify)
    └── trust.rs                # Trust list fetching from GitHub
```

### Dependencies

New crates to add:

```toml
[dependencies]
# GPG signature verification
sequoia-openpgp = "1.21"  # Pure Rust OpenPGP (no system deps)

# Already have:
git2 = "0.20.2"            # Git operations
octocrate = "2.2.0"        # GitHub REST API client
gql_client = "1.0.8"       # GitHub GraphQL client
```

**Recommendation**: Use `sequoia-openpgp` for:
- Pure Rust (no C dependencies, easier cross-compile)
- Memory safety
- Better error handling
- Actively maintained by security experts

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("Impersonation attempt detected: Commit {sha} claims trusted identity but is unsigned")]
    ImpersonationAttempt { sha: String },
    
    #[error("Key mismatch: Commit {sha} signed with unapproved key")]
    KeyMismatch { sha: String },
    
    #[error("Failed to fetch collaborators from GitHub: {0}")]
    GitHubAPIError(#[from] octocrate::Error),
    
    #[error("Git operation failed: {0}")]
    GitError(#[from] git2::Error),
    
    #[error("GPG operation failed: {0}")]
    GpgError(String),
    
    #[error("Could not determine repository from git remote or environment")]
    RepositoryNotFound,
}
```

### Integration with Existing pcu Architecture

`pcu` already has excellent patterns for GitHub authentication:

```rust
// From crates/pcu/src/client.rs:165-220
async fn get_github_apis(settings: &Config) -> Result<(GitHubAPI, gql_client::Client), Error> {
    // Supports both GitHub App and PAT authentication
    let (config, token) = match settings.get::<String>("app_id") {
        Ok(app_id) => {
            // GitHub App with installation token
            let app_authorization = AppAuthorization::new(app_id, private_key);
            let config = APIConfig::with_token(app_authorization).shared();
            // ... get installation token ...
        }
        Err(_) => {
            // Fallback to Personal Access Token
            let pat = settings.get::<String>("pat")?;
            let personal_access_token = PersonalAccessToken::new(&pat);
            (APIConfig::with_token(personal_access_token).shared(), pat)
        }
    };
    
    let github_rest = GitHubAPI::new(&config);
    let github_graphql = gql_client::Client::new_with_headers(END_POINT, headers);
    
    Ok((github_rest, github_graphql))
}
```

**Reuse strategy**: The `verify-signatures` command will use the same `Commands::get_client()` pattern to get authenticated GitHub clients, ensuring consistent auth across all pcu subcommands.

### Configuration

Settings can be provided via:
1. Environment variables (prefixed with `PCU_`)
2. `pcu.toml` config file
3. Command-line arguments

Example `pcu.toml` for signature verification:

```toml
# Standard pcu config (already exists)
prlog = "PRLOG.md"
branch = "CIRCLE_BRANCH"
username = "CIRCLE_PROJECT_USERNAME"
reponame = "CIRCLE_PROJECT_REPONAME"
dev_platform = "https://github.com/"

# Signature verification specific
[verify_signatures]
fail_on_unsigned = true
fetch_depth = 200
```

### Output Format

Human-readable output with color (privacy-preserving):

```
=== Commit Signature Verification (Dynamic) ===
Fetching trusted identities from GitHub...

Repository: example-org/example-repo

ℹ  Found 2 collaborator(s) with write access
ℹ  Imported 3 GPG key(s)
ℹ  Built trust map with 5 email→key mappings

Importing GitHub web-flow key...
✓ GitHub web-flow key imported

ℹ  Checking commit range: abc12345..def67890
ℹ  Found 3 commit(s) to verify

✓ OK   abc12345 feat: add new feature
    External contributor (unsigned, allowed)

✓ OK   def67890 ci: update workflow
    Trusted identity (signed, verified)

✓ OK   123abcde Merge pull request #42
    Trusted identity (signed, verified)

=== Verification Summary ===
Commits checked:         3
Trusted verified:        2
External contributors:   1

✓ All signature checks passed!

No impersonation attempts detected.
```

Machine-readable output (JSON format for CI, also privacy-preserving):

```bash
pcu verify-signatures --json
```

```json
{
  "status": "passed",
  "summary": {
    "commits_checked": 3,
    "trusted_verified": 2,
    "external_contributors": 1,
    "failures": 0
  },
  "commits": [
    {
      "sha": "abc12345",
      "subject": "feat: add new feature",
      "signature_status": "unsigned",
      "identity_type": "external",
      "result": "pass"
    },
    {
      "sha": "def67890",
      "subject": "ci: update workflow",
      "signature_status": "good",
      "identity_type": "trusted",
      "result": "pass"
    }
  ]
}
```

Note: JSON output omits author names/emails for privacy.

### Testing Strategy

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trusted_identity_unsigned_commit_fails() {
        let commit = MockCommit {
            author_email: "trusted@example.test",
            signature_status: SignatureStatus::None,
        };
        let trust_map = TrustMap::from([
            ("trusted@example.test", vec!["TESTKEY12345"])
        ]);
        
        let result = verify_commit(&commit, &trust_map);
        
        assert!(matches!(result, Err(VerificationError::ImpersonationAttempt { .. })));
    }
    
    #[test]
    fn test_external_contributor_unsigned_commit_passes() {
        let commit = MockCommit {
            author_email: "external@example.test",
            signature_status: SignatureStatus::None,
        };
        let trust_map = TrustMap::from([
            ("trusted@example.test", vec!["TESTKEY12345"])
        ]);
        
        let result = verify_commit(&commit, &trust_map);
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_key_mismatch_fails() {
        let commit = MockCommit {
            author_email: "trusted@example.test",
            signature_status: SignatureStatus::Good,
            key_id: "WRONG_KEY_ID",
        };
        let trust_map = TrustMap::from([
            ("trusted@example.test", vec!["TESTKEY12345"])
        ]);
        
        let result = verify_commit(&commit, &trust_map);
        
        assert!(matches!(result, Err(VerificationError::KeyMismatch { .. })));
    }
}
```

#### Integration Tests

Create test fixtures with real Git repos and GPG keys (using test keys only):

```rust
#[tokio::test]
async fn test_verify_signatures_real_repo() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_repo(&temp_dir);
    
    let result = run_verification(&temp_dir, "main", "HEAD").await;
    
    assert!(result.is_ok());
}
```

#### CI Testing

Test in multiple scenarios:
- All commits signed (should pass)
- External contributor unsigned (should pass)
- Trusted identity unsigned (should fail)
- Key mismatch (should fail)
- GitHub web-flow merge commits (should pass)

## Rollout Plan

### Phase 1: Prototype (✅ Complete)
- [x] Bash script implementation in named-colour
- [x] Documentation
- [x] CI integration
- [x] Testing and validation

### Phase 2: Rust Implementation (Current)
- [ ] Move bash script to pcu repo as reference
- [ ] Create RFC/issue
- [ ] Design document
- [ ] Implement `pcu verify-signatures` subcommand
- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation

### Phase 3: Toolkit Integration
- [ ] Add `pcu verify-signatures` to circleci-toolkit orb
- [ ] Create reusable job: `circleci-toolkit/verify-commit-signatures`
- [ ] Update orb documentation

### Phase 4: Repository Rollout
- [ ] Deploy to named-colour (replace bash script)
- [ ] Deploy to pcu
- [ ] Deploy to circleci-toolkit
- [ ] Deploy to other jerus-org repos
- [ ] Deprecate bash script

## Alternatives Considered

### Alternative 1: Keep bash script, copy to all repos
**Rejected**: Creates N attack surfaces, maintenance burden, no testing

### Alternative 2: GitHub Action instead of CI job
**Rejected**: Locks us into GitHub Actions, we use CircleCI

### Alternative 3: Separate binary (`verify-signatures-cli`)
**Rejected**: pcu is already distributed to CI environments, adding another binary is overhead

### Alternative 4: Python script
**Rejected**: Requires Python runtime, dependency management, less type safety than Rust

## Security Considerations

### Attack Surfaces

1. **GPG key import**: Only import from GitHub's official API (HTTPS enforced)
2. **GitHub API**: Validate responses, handle rate limits
3. **Git operations**: Use libgit2 (memory-safe) via git2-rs
4. **Network requests**: HTTPS-only, TLS 1.2+

### Threat Model

**In scope**:
- ✅ Identity impersonation attacks
- ✅ Unsigned commits from trusted identities
- ✅ Key mismatch/rotation attacks

**Out of scope** (requires additional layers):
- ❌ Compromised maintainer GPG keys
- ❌ Social engineering of maintainers
- ❌ Code review bypasses

### Privacy

Dynamic trust list fetching means:
- ✅ No hardcoded list of trusted identities in code
- ✅ Keys fetched from public GitHub profiles (already public)
- ✅ No secrets stored in repository
- ✅ **No PII logged or exposed in output**
- ✅ Aggregate statistics only (counts, not identities)

**Privacy principles**:
1. **Minimize PII collection**: Only fetch what's needed for verification
2. **No PII persistence**: Trust map lives in memory only
3. **No PII in logs**: Use aggregates and anonymized data
4. **No PII in errors**: Error messages reference commits, not people
5. **Public data only**: GitHub profiles are already public

## Open Questions

1. **GPG library choice**: `sequoia-openpgp` (pure Rust) vs `gpgme` (C bindings)?
   - **Recommendation**: `sequoia-openpgp` for memory safety and no system deps

2. **Caching**: Should we cache collaborator/key data to reduce GitHub API calls?
   - **Recommendation**: No caching initially, add if rate limits become an issue

3. **Offline mode**: Support verification without GitHub API access?
   - **Recommendation**: Add `--trust-file trust.json` option for offline use

4. **Key rotation**: How to handle gradual key rotation?
   - **Recommendation**: Support multiple keys per email (API provides this)

5. **Verbose logging**: How much detail in verbose mode without exposing PII?
   - **Recommendation**: SHA prefixes (8 chars), commit subjects, key suffixes (4 digits)

## References

- Bash script prototype: `docs/reference-implementation-verify-signatures.sh`
- Documentation: `docs/SIGNATURE_VERIFICATION.md`
- GitHub GPG API: https://docs.github.com/en/rest/users/gpg-keys
- GitHub Collaborators API: https://docs.github.com/en/rest/collaborators/collaborators
- Sequoia-PGP: https://sequoia-pgp.org/
- Git commit signing: https://git-scm.com/book/en/v2/Git-Tools-Signing-Your-Work

## Implementation Checklist

- [ ] Add `sequoia-openpgp` dependency
- [ ] Create `crates/pcu/src/cli/verify_signatures/` module
- [ ] Create `crates/pcu/src/ops/signature_ops.rs` module
- [ ] Implement trust list fetching from GitHub API
- [ ] Implement GPG key import and verification
- [ ] Implement commit signature verification logic
- [ ] Add privacy-preserving logging
- [ ] Add CLI command with clap
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Update pcu documentation
- [ ] Create example CI configuration
- [ ] Test in named-colour repository
- [ ] Update circleci-toolkit orb
