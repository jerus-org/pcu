use std::collections::HashMap;
use crate::Error;

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
    pub author_name: String,
    pub subject: String,
    pub signature_status: SignatureStatus,
    pub key_id: Option<String>,
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
    /// External contributor (unsigned allowed)
    ExternalUnsigned,
    /// External contributor (signed bonus)
    ExternalSigned,
    /// Trusted identity missing signature (FAIL)
    ImpersonationAttempt,
    /// Trusted identity signed with wrong key (FAIL)
    KeyMismatch { expected: Vec<String>, actual: Option<String> },
    /// Bad signature (FAIL)
    BadSignature,
}

impl VerificationReason {
    /// Get privacy-safe display message (no PII)
    pub fn display_message(&self) -> &'static str {
        match self {
            VerificationReason::TrustedVerified => "Trusted identity (signed, verified)",
            VerificationReason::ExternalUnsigned => "External contributor (unsigned, allowed)",
            VerificationReason::ExternalSigned => "External contributor (signed)",
            VerificationReason::ImpersonationAttempt => "Impersonation attempt: trusted identity unsigned",
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

/// Verify a single commit against the trust map
pub fn verify_commit(commit: &CommitInfo, trust_map: &TrustMap) -> VerificationResult {
    let is_trusted = trust_map.contains_key(&commit.author_email);
    
    if is_trusted {
        // TRUSTED IDENTITY: Must be signed with approved key
        match &commit.signature_status {
            SignatureStatus::Good | SignatureStatus::Unknown => {
                // Check if key is in allowlist
                if let Some(allowed_keys) = trust_map.get(&commit.author_email) {
                    if let Some(ref key_id) = commit.key_id {
                        // Check if this key is approved
                        let key_match = allowed_keys.iter().any(|k| key_id.ends_with(k) || k.ends_with(key_id));
                        
                        if key_match {
                            VerificationResult {
                                commit: commit.clone(),
                                passed: true,
                                reason: VerificationReason::TrustedVerified,
                            }
                        } else {
                            VerificationResult {
                                commit: commit.clone(),
                                passed: false,
                                reason: VerificationReason::KeyMismatch {
                                    expected: allowed_keys.clone(),
                                    actual: commit.key_id.clone(),
                                },
                            }
                        }
                    } else {
                        // Signed status but no key ID?
                        VerificationResult {
                            commit: commit.clone(),
                            passed: false,
                            reason: VerificationReason::ImpersonationAttempt,
                        }
                    }
                } else {
                    // Should not happen (is_trusted but no entry)
                    VerificationResult {
                        commit: commit.clone(),
                        passed: false,
                        reason: VerificationReason::ImpersonationAttempt,
                    }
                }
            }
            SignatureStatus::Bad | SignatureStatus::Expired | SignatureStatus::ExpiredKey | SignatureStatus::Revoked => {
                VerificationResult {
                    commit: commit.clone(),
                    passed: false,
                    reason: VerificationReason::BadSignature,
                }
            }
            SignatureStatus::None => {
                VerificationResult {
                    commit: commit.clone(),
                    passed: false,
                    reason: VerificationReason::ImpersonationAttempt,
                }
            }
        }
    } else {
        // EXTERNAL CONTRIBUTOR: Unsigned OK
        match &commit.signature_status {
            SignatureStatus::Good | SignatureStatus::Unknown => {
                VerificationResult {
                    commit: commit.clone(),
                    passed: true,
                    reason: VerificationReason::ExternalSigned,
                }
            }
            _ => {
                VerificationResult {
                    commit: commit.clone(),
                    passed: true,
                    reason: VerificationReason::ExternalUnsigned,
                }
            }
        }
    }
}

/// Verify a list of commits and return summary
pub fn verify_commits(commits: Vec<CommitInfo>, trust_map: &TrustMap) -> (Vec<VerificationResult>, VerificationSummary) {
    let mut results = Vec::new();
    let mut summary = VerificationSummary::default();
    
    for commit in commits {
        let result = verify_commit(&commit, trust_map);
        
        summary.commits_checked += 1;
        
        match &result.reason {
            VerificationReason::TrustedVerified => {
                summary.trusted_verified += 1;
            }
            VerificationReason::ExternalUnsigned | VerificationReason::ExternalSigned => {
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
        trust_map.insert("trusted@example.test".to_string(), vec!["TESTKEY12345".to_string()]);
        
        let result = verify_commit(&commit, &trust_map);
        
        assert!(!result.passed);
        assert!(matches!(result.reason, VerificationReason::ImpersonationAttempt));
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
        trust_map.insert("trusted@example.test".to_string(), vec!["TESTKEY12345".to_string()]);
        
        let result = verify_commit(&commit, &trust_map);
        
        assert!(result.passed);
        assert!(matches!(result.reason, VerificationReason::ExternalUnsigned));
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
        trust_map.insert("trusted@example.test".to_string(), vec!["TESTKEY12345".to_string()]);
        
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
        trust_map.insert("trusted@example.test".to_string(), vec!["TESTKEY12345".to_string()]);
        
        let result = verify_commit(&commit, &trust_map);
        
        assert!(!result.passed);
        assert!(matches!(result.reason, VerificationReason::KeyMismatch { .. }));
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
        trust_map.insert("trusted@example.test".to_string(), vec!["TESTKEY12345".to_string()]);
        
        let (results, summary) = verify_commits(commits, &trust_map);
        
        assert_eq!(results.len(), 2);
        assert_eq!(summary.commits_checked, 2);
        assert_eq!(summary.trusted_verified, 1);
        assert_eq!(summary.external_contributors, 1);
        assert_eq!(summary.failures, 0);
    }
}
