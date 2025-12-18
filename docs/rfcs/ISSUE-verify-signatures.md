# Add `verify-signatures` Subcommand for Anti-Impersonation

## Summary

Implement `pcu verify-signatures` to prevent identity impersonation attacks by verifying GPG signatures on commits from trusted contributors.

## Problem

Git allows anyone to forge author names/emails, making it trivial to impersonate maintainers:

```bash
git config user.email "trusted-maintainer@example.org"
git commit -m "Add backdoor"  # Appears to be from maintainer!
```

Without signature verification, there's no cryptographic proof of authorship. This is a supply chain security risk.

## Proposed Solution

Migrate working bash script prototype (from named-colour repo) into `pcu` as a Rust subcommand:

```bash
pcu verify-signatures --base origin/main --head HEAD
```

**Key features**:
- ✅ Dynamic trust list (fetched from GitHub collaborators API)
- ✅ Enforce: Trusted identities MUST sign with approved keys
- ✅ Allow: External contributors can submit unsigned commits (low barrier)
- ✅ Privacy-preserving: No PII logged
- ✅ Type-safe: Rust implementation with comprehensive tests

## Design

See [RFC 0001](./0001-verify-signatures-subcommand.md) for detailed design document.

### Verification Logic

```
Is commit author in trusted list?
  ├─ YES → MUST be signed by approved key
  │        ├─ Valid signature? → ✅ PASS
  │        └─ No signature?    → ❌ FAIL (Impersonation!)
  └─ NO  → External contributor
           ├─ Signed?   → ✅ PASS (bonus)
           └─ Unsigned? → ✅ PASS (allowed)
```

### Privacy-Preserving Output

```
=== Commit Signature Verification ===
Repository: example-org/repo

ℹ  Found 3 collaborator(s) with write access
ℹ  Imported 5 GPG key(s)
ℹ  Found 3 commit(s) to verify

✓ OK   abc12345 feat: add new feature
    External contributor (unsigned, allowed)

✓ OK   def67890 ci: update workflow
    Trusted identity (signed, verified)

=== Summary ===
Commits checked:         3
Trusted verified:        2
External contributors:   1

✓ All checks passed!
```

Note: No names or emails logged to protect privacy.

## Dependencies

- `sequoia-openpgp`: Pure Rust OpenPGP library (no C deps)
- Existing: `git2`, `octocrate`, `gql_client`

## Implementation Plan

### Phase 1: Core Implementation
- [ ] Add `sequoia-openpgp` dependency
- [ ] Create module: `crates/pcu/src/cli/verify_signatures/`
- [ ] Create module: `crates/pcu/src/ops/signature_ops.rs`
- [ ] Implement GitHub API trust list fetching
- [ ] Implement GPG key import/verification
- [ ] Implement commit verification logic
- [ ] Add privacy-preserving logging
- [ ] Add CLI with clap

### Phase 2: Testing
- [ ] Unit tests for verification logic
- [ ] Integration tests with test repos
- [ ] CI test scenarios (signed/unsigned/impersonation)
- [ ] Documentation and examples

### Phase 3: Deployment
- [ ] Deploy to named-colour (replace bash script)
- [ ] Add to circleci-toolkit orb
- [ ] Rollout to other jerus-org repos

## References

- RFC: `docs/rfcs/0001-verify-signatures-subcommand.md`
- Bash prototype: `docs/reference-implementation-verify-signatures.sh`
- Documentation: `docs/SIGNATURE_VERIFICATION.md`

## Security Benefits

- ✅ Prevents identity impersonation attacks
- ✅ Protects supply chain integrity
- ✅ Maintains clear audit trail
- ✅ Low barrier for external contributors

## Acceptance Criteria

- [ ] `pcu verify-signatures` command implemented
- [ ] Dynamically fetches trust list from GitHub
- [ ] Correctly identifies impersonation attempts
- [ ] Allows unsigned commits from external contributors
- [ ] No PII exposed in logs or output
- [ ] Comprehensive test coverage (>80%)
- [ ] Documentation complete
- [ ] Successfully replaces bash script in named-colour

---

**Labels**: enhancement, security, privacy
**Priority**: High
**Related**: named-colour PR #136 (bash prototype)
