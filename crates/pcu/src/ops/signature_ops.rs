use std::collections::HashMap;

/// Trust map: email -> list of approved GPG key IDs
pub type TrustMap = HashMap<String, Vec<String>>;

/// Signature verification status from git
#[derive(Debug, Clone, PartialEq)]
pub enum SignatureStatus {
    /// Good signature (G)
    Good,
    /// Bad signature (B)
    Bad,
    /// Good signature, unknown validity (U)
    Unknown,
    /// Good signature, expired (X)
    Expired,
    /// Good signature, made by expired key (Y)
    ExpiredKey,
    /// Good signature, made by revoked key (R)
    Revoked,
    /// No signature (N)
    None,
}

impl SignatureStatus {
    pub fn from_git_format(status: &str) -> Self {
        match status {
            "G" => SignatureStatus::Good,
            "B" => SignatureStatus::Bad,
            "U" => SignatureStatus::Unknown,
            "X" => SignatureStatus::Expired,
            "Y" => SignatureStatus::ExpiredKey,
            "R" => SignatureStatus::Revoked,
            "N" | "" => SignatureStatus::None,
            _ => SignatureStatus::None,
        }
    }
}

/// Information about a commit to be verified
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub author_email: String,
    #[allow(dead_code)]
    pub author_name: String,
    pub subject: String,
    pub signature_status: SignatureStatus,
    pub key_id: Option<String>,
    #[allow(dead_code)]
    pub signer: Option<String>,
}

/// Result of verifying a single commit
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub commit: CommitInfo,
    pub passed: bool,
    pub reason: VerificationReason,
}

/// Reason for verification pass/fail
#[derive(Debug, Clone)]
pub enum VerificationReason {
    /// Trusted identity with valid signature
    TrustedVerified,
    /// Author is not a trusted identity. Reported as unsigned whatever the
    /// commit's signature: only trusted identities are verified, so a
    /// signature from an unknown author (a bot, or a potentially malicious
    /// external contributor) is never presented as a good one.
    ExternalUnsigned,
    /// Author is not a trusted identity AND the commit carries a signature
    /// that is bad, expired or made with an expired or revoked key. Unlike a
    /// plain unsigned commit this is an active warning sign (tampering, a
    /// forged signature, a compromised key). Still allowed today; the
    /// hard-stop mode (#1132) is where it can be made to fail.
    ExternalInvalidSignature,
    /// Trusted identity missing signature (FAIL)
    ImpersonationAttempt,
    /// Trusted identity signed with wrong key (FAIL)
    KeyMismatch {
        #[allow(dead_code)]
        expected: Vec<String>,
        #[allow(dead_code)]
        actual: Option<String>,
    },
    /// Bad signature (FAIL)
    BadSignature,
}

impl VerificationReason {
    /// Get privacy-safe display message (no PII)
    pub fn display_message(&self) -> &'static str {
        match self {
            VerificationReason::TrustedVerified => "Trusted identity (signed, verified)",
            VerificationReason::ExternalUnsigned => "External contributor (unsigned, allowed)",
            VerificationReason::ExternalInvalidSignature => {
                "External contributor (INVALID signature, allowed)"
            }
            VerificationReason::ImpersonationAttempt => {
                "Impersonation attempt: trusted identity unsigned"
            }
            VerificationReason::KeyMismatch { .. } => "Key mismatch: signed with unapproved key",
            VerificationReason::BadSignature => "Bad signature",
        }
    }
}

/// Summary of verification run
#[derive(Debug, Default)]
pub struct VerificationSummary {
    pub commits_checked: usize,
    pub trusted_verified: usize,
    pub external_contributors: usize,
    pub failures: usize,
}

/// Look up a commit author's approved keys, ignoring the case of the email.
///
/// GitHub attributes a commit to an account whatever the case of the author
/// email, so `Jerry@Example.test` shows as the maintainer's own commit. An
/// exact-case lookup would classify it as an unknown external author, and an
/// unsigned impersonation would pass instead of failing. Trust-map keys are
/// stored lowercase; the fallback scan also covers a map built another way.
fn trusted_keys<'a>(trust_map: &'a TrustMap, email: &str) -> Option<&'a Vec<String>> {
    let email = email.trim();
    trust_map.get(email).or_else(|| {
        let wanted = email.to_lowercase();
        trust_map
            .iter()
            .find(|(known, _)| known.trim().to_lowercase() == wanted)
            .map(|(_, keys)| keys)
    })
}

/// Verify a single commit against the trust map
pub fn verify_commit(commit: &CommitInfo, trust_map: &TrustMap) -> VerificationResult {
    let is_trusted = trusted_keys(trust_map, &commit.author_email).is_some();

    if is_trusted {
        verify_trusted_commit(commit, trust_map)
    } else {
        verify_external_commit(commit)
    }
}

/// Verify a commit from a trusted identity
fn verify_trusted_commit(commit: &CommitInfo, trust_map: &TrustMap) -> VerificationResult {
    match &commit.signature_status {
        SignatureStatus::Good | SignatureStatus::Unknown => {
            verify_trusted_signature(commit, trust_map)
        }
        SignatureStatus::Bad
        | SignatureStatus::Expired
        | SignatureStatus::ExpiredKey
        | SignatureStatus::Revoked => {
            create_result(commit, false, VerificationReason::BadSignature)
        }
        SignatureStatus::None => {
            create_result(commit, false, VerificationReason::ImpersonationAttempt)
        }
    }
}

/// Verify the signature of a trusted commit
fn verify_trusted_signature(commit: &CommitInfo, trust_map: &TrustMap) -> VerificationResult {
    let Some(allowed_keys) = trusted_keys(trust_map, &commit.author_email) else {
        return create_result(commit, false, VerificationReason::ImpersonationAttempt);
    };

    let Some(ref key_id) = commit.key_id else {
        return create_result(commit, false, VerificationReason::ImpersonationAttempt);
    };

    if is_key_approved(key_id, allowed_keys) {
        create_result(commit, true, VerificationReason::TrustedVerified)
    } else {
        create_result(
            commit,
            false,
            VerificationReason::KeyMismatch {
                expected: allowed_keys.clone(),
                actual: commit.key_id.clone(),
            },
        )
    }
}

/// Check if a key ID is approved in the allowed keys list
fn is_key_approved(key_id: &str, allowed_keys: &[String]) -> bool {
    allowed_keys
        .iter()
        .any(|k| key_id.ends_with(k) || k.ends_with(key_id))
}

/// Verify a commit from an external contributor
fn verify_external_commit(commit: &CommitInfo) -> VerificationResult {
    // The author is not in the trust map, so nothing vouches for the commit
    // whatever its signature. A signature that is actively invalid is still
    // told apart from an honest absence of one.
    match &commit.signature_status {
        SignatureStatus::Bad
        | SignatureStatus::Expired
        | SignatureStatus::ExpiredKey
        | SignatureStatus::Revoked => {
            create_result(commit, true, VerificationReason::ExternalInvalidSignature)
        }
        _ => create_result(commit, true, VerificationReason::ExternalUnsigned),
    }
}

/// Helper to create a VerificationResult
fn create_result(
    commit: &CommitInfo,
    passed: bool,
    reason: VerificationReason,
) -> VerificationResult {
    VerificationResult {
        commit: commit.clone(),
        passed,
        reason,
    }
}

/// Verify a list of commits and return summary
pub fn verify_commits(
    commits: Vec<CommitInfo>,
    trust_map: &TrustMap,
) -> (Vec<VerificationResult>, VerificationSummary) {
    let mut results = Vec::new();
    let mut summary = VerificationSummary::default();

    for commit in commits {
        let result = verify_commit(&commit, trust_map);

        summary.commits_checked += 1;

        match &result.reason {
            VerificationReason::TrustedVerified => {
                summary.trusted_verified += 1;
            }
            VerificationReason::ExternalUnsigned | VerificationReason::ExternalInvalidSignature => {
                summary.external_contributors += 1;
            }
            _ => {}
        }

        if !result.passed {
            summary.failures += 1;
        }

        results.push(result);
    }

    (results, summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trusted_identity_unsigned_fails() {
        let commit = CommitInfo {
            sha: "abc123".to_string(),
            author_email: "trusted@example.test".to_string(),
            author_name: "Trusted User".to_string(),
            subject: "test commit".to_string(),
            signature_status: SignatureStatus::None,
            key_id: None,
            signer: None,
        };

        let mut trust_map = TrustMap::new();
        trust_map.insert(
            "trusted@example.test".to_string(),
            vec!["TESTKEY12345".to_string()],
        );

        let result = verify_commit(&commit, &trust_map);

        assert!(!result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::ImpersonationAttempt
        ));
    }

    #[test]
    fn test_external_contributor_unsigned_passes() {
        let commit = CommitInfo {
            sha: "abc123".to_string(),
            author_email: "external@example.test".to_string(),
            author_name: "External User".to_string(),
            subject: "test commit".to_string(),
            signature_status: SignatureStatus::None,
            key_id: None,
            signer: None,
        };

        let mut trust_map = TrustMap::new();
        trust_map.insert(
            "trusted@example.test".to_string(),
            vec!["TESTKEY12345".to_string()],
        );

        let result = verify_commit(&commit, &trust_map);

        assert!(result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::ExternalUnsigned
        ));
    }

    #[test]
    fn test_trusted_identity_correct_key_passes() {
        let commit = CommitInfo {
            sha: "abc123".to_string(),
            author_email: "trusted@example.test".to_string(),
            author_name: "Trusted User".to_string(),
            subject: "test commit".to_string(),
            signature_status: SignatureStatus::Good,
            key_id: Some("TESTKEY12345".to_string()),
            signer: Some("Trusted User".to_string()),
        };

        let mut trust_map = TrustMap::new();
        trust_map.insert(
            "trusted@example.test".to_string(),
            vec!["TESTKEY12345".to_string()],
        );

        let result = verify_commit(&commit, &trust_map);

        assert!(result.passed);
        assert!(matches!(result.reason, VerificationReason::TrustedVerified));
    }

    #[test]
    fn test_trusted_identity_wrong_key_fails() {
        let commit = CommitInfo {
            sha: "abc123".to_string(),
            author_email: "trusted@example.test".to_string(),
            author_name: "Trusted User".to_string(),
            subject: "test commit".to_string(),
            signature_status: SignatureStatus::Good,
            key_id: Some("WRONGKEY99999".to_string()),
            signer: Some("Someone Else".to_string()),
        };

        let mut trust_map = TrustMap::new();
        trust_map.insert(
            "trusted@example.test".to_string(),
            vec!["TESTKEY12345".to_string()],
        );

        let result = verify_commit(&commit, &trust_map);

        assert!(!result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::KeyMismatch { .. }
        ));
    }

    #[test]
    fn test_verify_commits_summary() {
        let commits = vec![
            CommitInfo {
                sha: "abc123".to_string(),
                author_email: "external@example.test".to_string(),
                author_name: "External".to_string(),
                subject: "external commit".to_string(),
                signature_status: SignatureStatus::None,
                key_id: None,
                signer: None,
            },
            CommitInfo {
                sha: "def456".to_string(),
                author_email: "trusted@example.test".to_string(),
                author_name: "Trusted".to_string(),
                subject: "trusted commit".to_string(),
                signature_status: SignatureStatus::Good,
                key_id: Some("TESTKEY12345".to_string()),
                signer: Some("Trusted".to_string()),
            },
        ];

        let mut trust_map = TrustMap::new();
        trust_map.insert(
            "trusted@example.test".to_string(),
            vec!["TESTKEY12345".to_string()],
        );

        let (results, summary) = verify_commits(commits, &trust_map);

        assert_eq!(results.len(), 2);
        assert_eq!(summary.commits_checked, 2);
        assert_eq!(summary.trusted_verified, 1);
        assert_eq!(summary.external_contributors, 1);
        assert_eq!(summary.failures, 0);
    }

    /// A signature from an author outside the trust map (renovate, or a
    /// potentially malicious contributor) must not be reported as signed.
    #[test]
    fn test_external_contributor_with_good_signature_is_still_unsigned() {
        let commit = CommitInfo {
            sha: "abc123".to_string(),
            author_email: "external@example.test".to_string(),
            author_name: "External User".to_string(),
            subject: "test commit".to_string(),
            signature_status: SignatureStatus::Good,
            key_id: Some("SOMEKEY".to_string()),
            signer: None,
        };

        let result = verify_commit(&commit, &TrustMap::new());

        assert!(result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::ExternalUnsigned
        ));
    }

    /// A bad, expired or revoked signature from an untrusted author is not the
    /// same as no signature: it must be reported distinctly (still allowed).
    #[test]
    fn test_external_contributor_with_invalid_signature_is_flagged() {
        for status in [
            SignatureStatus::Bad,
            SignatureStatus::Expired,
            SignatureStatus::ExpiredKey,
            SignatureStatus::Revoked,
        ] {
            let commit = CommitInfo {
                sha: "abc123".to_string(),
                author_email: "external@example.test".to_string(),
                author_name: "External User".to_string(),
                subject: "test commit".to_string(),
                signature_status: status.clone(),
                key_id: None,
                signer: None,
            };

            let (results, summary) = verify_commits(vec![commit], &TrustMap::new());

            assert!(
                matches!(
                    results[0].reason,
                    VerificationReason::ExternalInvalidSignature
                ),
                "{status:?} should be flagged"
            );
            assert!(results[0].passed, "still allowed until the hard stop");
            assert_eq!(summary.external_contributors, 1);
            assert_eq!(summary.failures, 0);
        }
    }

    fn trusted_map() -> TrustMap {
        let mut trust_map = TrustMap::new();
        trust_map.insert(
            "trusted@example.test".to_string(),
            vec!["TESTKEY12345".to_string()],
        );
        trust_map
    }

    fn commit_by(email: &str, status: SignatureStatus, key: Option<&str>) -> CommitInfo {
        CommitInfo {
            sha: "abc123".to_string(),
            author_email: email.to_string(),
            author_name: "Someone".to_string(),
            subject: "test commit".to_string(),
            signature_status: status,
            key_id: key.map(str::to_string),
            signer: None,
        }
    }

    /// An unsigned commit authored with a differently-cased trusted email is an
    /// impersonation attempt, not an unknown external author who passes.
    #[test]
    fn test_trusted_email_in_other_case_unsigned_is_impersonation() {
        for email in [
            "Trusted@Example.Test",
            "TRUSTED@EXAMPLE.TEST",
            " trusted@example.test ",
        ] {
            let commit = commit_by(email, SignatureStatus::None, None);

            let result = verify_commit(&commit, &trusted_map());

            assert!(!result.passed, "{email:?} must not pass as external");
            assert!(matches!(
                result.reason,
                VerificationReason::ImpersonationAttempt
            ));
        }
    }

    #[test]
    fn test_trusted_email_in_other_case_with_correct_key_passes() {
        let commit = commit_by(
            "Trusted@Example.Test",
            SignatureStatus::Good,
            Some("TESTKEY12345"),
        );

        let result = verify_commit(&commit, &trusted_map());

        assert!(result.passed);
        assert!(matches!(result.reason, VerificationReason::TrustedVerified));
    }

    /// A trust map built with a mixed-case key still matches a lowercase commit.
    #[test]
    fn test_mixed_case_trust_map_key_matches_lowercase_commit() {
        let mut trust_map = TrustMap::new();
        trust_map.insert(
            "Trusted@Example.Test".to_string(),
            vec!["TESTKEY12345".to_string()],
        );
        let commit = commit_by("trusted@example.test", SignatureStatus::None, None);

        let result = verify_commit(&commit, &trust_map);

        assert!(!result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::ImpersonationAttempt
        ));
    }

    #[test]
    fn test_different_email_is_still_external() {
        let commit = commit_by("other@example.test", SignatureStatus::None, None);

        let result = verify_commit(&commit, &trusted_map());

        assert!(result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::ExternalUnsigned
        ));
    }
}
