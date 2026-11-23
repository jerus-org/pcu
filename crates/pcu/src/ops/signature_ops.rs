use std::collections::HashMap;

use super::verify_config::{AllowEntry, VerifyConfig};

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
    /// Not a trusted identity, but on the policy's allow list, and the commit
    /// carries a good signature made with one of the entry's keys.
    AllowListed,
    /// On the allow list, but the commit is unsigned, badly signed or signed
    /// with a key the entry does not name (FAIL). Anyone can forge an author
    /// email, so this is treated like an impersonation attempt.
    AllowListedBadSignature,
    /// On the allow list with a good signature from a listed key, but the pull
    /// request was not opened by one of the entry's `pr_authors` (FAIL), or
    /// the opener could not be determined. GitHub's web-flow key signs
    /// commits for any author, so the author email and signature alone do not
    /// identify the bot.
    AllowListedWrongOpener,
    /// Not a trusted identity and not on the allow list, with the hard stop
    /// on (FAIL). `invalid_signature` is set when the commit also carries a
    /// bad, expired or revoked signature.
    ExternalNotAllowed { invalid_signature: bool },
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
            VerificationReason::ExternalUnsigned => {
                "External contributor (not a trusted identity, allowed)"
            }
            VerificationReason::ExternalInvalidSignature => {
                "External contributor (INVALID signature, allowed)"
            }
            VerificationReason::AllowListed => "Allow-listed identity (signed, verified)",
            VerificationReason::AllowListedBadSignature => {
                "Allow-listed identity: signature missing, invalid or from an unapproved key"
            }
            VerificationReason::AllowListedWrongOpener => {
                "Allow-listed identity: pull request not opened by one of its accounts"
            }
            VerificationReason::ExternalNotAllowed {
                invalid_signature: false,
            } => "External contributor not on the allow list",
            VerificationReason::ExternalNotAllowed {
                invalid_signature: true,
            } => "External contributor not on the allow list (INVALID signature)",
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
    /// Commits from allow-listed identities with a verified signature.
    pub allow_listed: usize,
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

/// Allow-list entries whose email is also a trusted identity. Such an entry is
/// never read, because a trusted identity is judged by its own keys, so the
/// policy file says something the verifier does not do.
pub fn shadowed_allow_entries<'a>(
    config: &'a VerifyConfig,
    trust_map: &TrustMap,
) -> Vec<&'a AllowEntry> {
    config
        .allow
        .iter()
        .filter(|entry| trusted_keys(trust_map, &entry.email).is_some())
        .collect()
}

/// Verify a single commit against the trust map, with the default policy (no
/// allow list, no hard stop).
#[cfg(test)]
pub fn verify_commit(commit: &CommitInfo, trust_map: &TrustMap) -> VerificationResult {
    verify_commit_with(commit, trust_map, &VerifyConfig::default())
}

/// Verify a single commit against the trust map and the verification policy.
///
/// A trusted identity is always judged as one, whatever the policy says: an
/// allow-list entry can never weaken the rule that a trusted identity's
/// signature must be good and made with an approved key.
pub fn verify_commit_with(
    commit: &CommitInfo,
    trust_map: &TrustMap,
    config: &VerifyConfig,
) -> VerificationResult {
    if trusted_keys(trust_map, &commit.author_email).is_some() {
        verify_trusted_commit(commit, trust_map)
    } else if let Some(entry) = config.allow_entry(&commit.author_email) {
        verify_allow_listed_commit(commit, entry, config.pr_author.as_deref())
    } else {
        verify_external_commit(commit, config)
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

/// Verify a commit from an allow-listed identity.
///
/// Two checks, both required. The signature must be good and made with one of
/// the entry's keys (the email is self-declared, so without this anyone could
/// forge it). And when the entry lists `pr_authors`, the pull request must have
/// been opened by one of those GitHub logins, which GitHub authenticates:
/// GitHub's web-flow key signs commits for any author, so the signature alone
/// does not tell the bot from someone using the same author email.
fn verify_allow_listed_commit(
    commit: &CommitInfo,
    entry: &AllowEntry,
    pr_author: Option<&str>,
) -> VerificationResult {
    let signed_by_an_approved_key = matches!(
        commit.signature_status,
        SignatureStatus::Good | SignatureStatus::Unknown
    ) && commit
        .key_id
        .as_deref()
        .is_some_and(|key_id| entry.approves(key_id));

    if !signed_by_an_approved_key {
        create_result(commit, false, VerificationReason::AllowListedBadSignature)
    } else if !entry.opener_allowed(pr_author) {
        create_result(commit, false, VerificationReason::AllowListedWrongOpener)
    } else {
        create_result(commit, true, VerificationReason::AllowListed)
    }
}

/// Verify a commit from an external contributor: neither a trusted identity
/// nor on the allow list. Allowed unless the policy's hard stop is on.
fn verify_external_commit(commit: &CommitInfo, config: &VerifyConfig) -> VerificationResult {
    // The author is not in the trust map, so nothing vouches for the commit
    // whatever its signature. A signature that is actively invalid is still
    // told apart from an honest absence of one.
    let invalid_signature = matches!(
        commit.signature_status,
        SignatureStatus::Bad
            | SignatureStatus::Expired
            | SignatureStatus::ExpiredKey
            | SignatureStatus::Revoked
    );

    if config.fail_on_external {
        create_result(
            commit,
            false,
            VerificationReason::ExternalNotAllowed { invalid_signature },
        )
    } else if invalid_signature {
        create_result(commit, true, VerificationReason::ExternalInvalidSignature)
    } else {
        create_result(commit, true, VerificationReason::ExternalUnsigned)
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

/// Verify a list of commits and return summary, with the default policy.
#[cfg(test)]
pub fn verify_commits(
    commits: Vec<CommitInfo>,
    trust_map: &TrustMap,
) -> (Vec<VerificationResult>, VerificationSummary) {
    verify_commits_with(commits, trust_map, &VerifyConfig::default())
}

/// Verify a list of commits against the trust map and the policy, and return
/// the summary.
pub fn verify_commits_with(
    commits: Vec<CommitInfo>,
    trust_map: &TrustMap,
    config: &VerifyConfig,
) -> (Vec<VerificationResult>, VerificationSummary) {
    let mut results = Vec::new();
    let mut summary = VerificationSummary::default();

    for commit in commits {
        let result = verify_commit_with(&commit, trust_map, config);

        summary.commits_checked += 1;

        match &result.reason {
            VerificationReason::TrustedVerified => {
                summary.trusted_verified += 1;
            }
            VerificationReason::AllowListed => {
                summary.allow_listed += 1;
            }
            VerificationReason::ExternalUnsigned
            | VerificationReason::ExternalInvalidSignature
            | VerificationReason::ExternalNotAllowed { .. } => {
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

    /// The only way left for an allow entry to name a trusted identity: a bot
    /// address that also appears on a collaborator's key.
    fn trusted_bot_map() -> TrustMap {
        let mut trust_map = trusted_map();
        trust_map.insert(
            "trusted[bot]@users.noreply.github.com".to_string(),
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

    // --- allow list and hard stop (#1132) -----------------------------------

    const RENOVATE: &str = "29139614+renovate[bot]@users.noreply.github.com";
    const WEB_FLOW: &str = "B5690EEEBB952194";

    fn policy(fail_on_external: bool) -> VerifyConfig {
        let mut config = VerifyConfig::from_toml(&format!(
            r#"
            [verify_signatures]
            fail_on_external = {fail_on_external}

            [[verify_signatures.allow]]
            email = "{RENOVATE}"
            keys = ["{WEB_FLOW}"]
            pr_authors = ["renovate[bot]"]
            description = "Renovate (committed and signed by GitHub)"
            "#
        ))
        .unwrap();
        // The legitimate case: Renovate opened the pull request.
        config.pr_author = Some("renovate[bot]".to_string());
        config
    }

    /// The case from the issue: Renovate authors the commit, GitHub's web-flow
    /// key signs it.
    #[test]
    fn test_renovate_signed_by_web_flow_is_allow_listed() {
        let commit = commit_by(RENOVATE, SignatureStatus::Good, Some(WEB_FLOW));

        let (results, summary) = verify_commits_with(vec![commit], &trusted_map(), &policy(true));

        assert!(results[0].passed);
        assert!(matches!(results[0].reason, VerificationReason::AllowListed));
        assert_eq!(summary.allow_listed, 1);
        assert_eq!(summary.external_contributors, 0);
        assert_eq!(summary.failures, 0);
    }

    #[test]
    fn test_allow_listed_with_unknown_validity_and_approved_key_passes() {
        let commit = commit_by(RENOVATE, SignatureStatus::Unknown, Some(WEB_FLOW));

        let result = verify_commit_with(&commit, &trusted_map(), &policy(false));

        assert!(matches!(result.reason, VerificationReason::AllowListed));
    }

    #[test]
    fn test_allow_listed_email_matches_in_any_case() {
        let commit = commit_by(
            &RENOVATE.to_uppercase(),
            SignatureStatus::Good,
            Some(WEB_FLOW),
        );

        let result = verify_commit_with(&commit, &trusted_map(), &policy(true));

        assert!(matches!(result.reason, VerificationReason::AllowListed));
    }

    /// A forged email must not get through: the signature has to match.
    #[test]
    fn test_allow_listed_email_without_an_approved_signature_fails() {
        let cases = [
            ("unsigned", SignatureStatus::None, None),
            ("bad signature", SignatureStatus::Bad, Some(WEB_FLOW)),
            ("revoked key", SignatureStatus::Revoked, Some(WEB_FLOW)),
            (
                "good, other key",
                SignatureStatus::Good,
                Some("1111111111111111"),
            ),
            ("good, no key id", SignatureStatus::Good, None),
        ];
        for (name, status, key) in cases {
            for config in [policy(false), policy(true)] {
                let commit = commit_by(RENOVATE, status.clone(), key);

                let result = verify_commit_with(&commit, &trusted_map(), &config);

                assert!(!result.passed, "{name} must fail");
                assert!(
                    matches!(result.reason, VerificationReason::AllowListedBadSignature),
                    "{name}: {:?}",
                    result.reason
                );
            }
        }
    }

    /// An allow-list entry can never weaken the trusted-identity rule.
    #[test]
    fn test_trusted_identity_is_judged_as_trusted_even_if_also_allow_listed() {
        let config = VerifyConfig::from_toml(&format!(
            "[[verify_signatures.allow]]\nemail = \"trusted[bot]@users.noreply.github.com\"\nkeys = [\"{WEB_FLOW}\"]\npr_authors = [\"renovate[bot]\"]\n"
        ))
        .unwrap();
        let commit = commit_by(
            "trusted[bot]@users.noreply.github.com",
            SignatureStatus::None,
            None,
        );

        let result = verify_commit_with(&commit, &trusted_bot_map(), &config);

        assert!(!result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::ImpersonationAttempt
        ));
    }

    #[test]
    fn a_shadowed_allow_entry_is_found_ignoring_case() {
        let config = VerifyConfig::from_toml(&format!(
            "[[verify_signatures.allow]]\nemail = \"Trusted[Bot]@Users.Noreply.GitHub.Com\"\nkeys = [\"{WEB_FLOW}\"]\npr_authors = [\"renovate[bot]\"]\n\n\
             [[verify_signatures.allow]]\nemail = \"bot[bot]@users.noreply.github.com\"\nkeys = [\"{WEB_FLOW}\"]\npr_authors = [\"renovate[bot]\"]\n"
        ))
        .unwrap();

        let shadowed = shadowed_allow_entries(&config, &trusted_bot_map());

        assert_eq!(shadowed.len(), 1);
        assert_eq!(shadowed[0].email, "trusted[bot]@users.noreply.github.com");
    }

    #[test]
    fn no_allow_entry_is_shadowed_when_none_names_a_trusted_identity() {
        let config = VerifyConfig::from_toml(&format!(
            "[[verify_signatures.allow]]\nemail = \"bot[bot]@users.noreply.github.com\"\nkeys = [\"{WEB_FLOW}\"]\npr_authors = [\"renovate[bot]\"]\n"
        ))
        .unwrap();

        assert!(shadowed_allow_entries(&config, &trusted_map()).is_empty());
        assert!(shadowed_allow_entries(&VerifyConfig::default(), &trusted_map()).is_empty());
    }

    #[test]
    fn test_hard_stop_fails_an_external_contributor_not_on_the_list() {
        let commit = commit_by("stranger@example.test", SignatureStatus::None, None);

        let (results, summary) = verify_commits_with(vec![commit], &trusted_map(), &policy(true));

        assert!(!results[0].passed);
        assert!(matches!(
            results[0].reason,
            VerificationReason::ExternalNotAllowed {
                invalid_signature: false
            }
        ));
        assert_eq!(summary.failures, 1);
        assert_eq!(summary.external_contributors, 1);
    }

    #[test]
    fn test_hard_stop_marks_an_external_with_an_invalid_signature() {
        let commit = commit_by("stranger@example.test", SignatureStatus::Bad, Some("ABCD"));

        let result = verify_commit_with(&commit, &trusted_map(), &policy(true));

        assert!(!result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::ExternalNotAllowed {
                invalid_signature: true
            }
        ));
    }

    #[test]
    fn test_hard_stop_still_passes_trusted_and_allow_listed_commits() {
        let commits = vec![
            commit_by(
                "trusted@example.test",
                SignatureStatus::Good,
                Some("TESTKEY12345"),
            ),
            commit_by(RENOVATE, SignatureStatus::Good, Some(WEB_FLOW)),
        ];

        let (results, summary) = verify_commits_with(commits, &trusted_map(), &policy(true));

        assert!(results.iter().all(|r| r.passed));
        assert_eq!(summary.trusted_verified, 1);
        assert_eq!(summary.allow_listed, 1);
        assert_eq!(summary.failures, 0);
    }

    /// Without the hard stop an external author still passes, as before.
    #[test]
    fn test_without_the_hard_stop_external_contributors_still_pass() {
        let commit = commit_by("stranger@example.test", SignatureStatus::None, None);

        let (results, summary) = verify_commits_with(vec![commit], &trusted_map(), &policy(false));

        assert!(results[0].passed);
        assert!(matches!(
            results[0].reason,
            VerificationReason::ExternalUnsigned
        ));
        assert_eq!(summary.failures, 0);
    }

    /// The default policy (no config file) is exactly the behaviour before.
    #[test]
    fn test_default_policy_matches_the_plain_functions() {
        let commit = commit_by(RENOVATE, SignatureStatus::Good, Some(WEB_FLOW));

        let plain = verify_commit(&commit, &trusted_map());
        let with = verify_commit_with(&commit, &trusted_map(), &VerifyConfig::default());

        assert_eq!(plain.passed, with.passed);
        assert!(matches!(plain.reason, VerificationReason::ExternalUnsigned));
        assert!(matches!(with.reason, VerificationReason::ExternalUnsigned));
    }

    // --- the PR opener check for web-flow entries (#1132) -------------------

    fn renovate_commit() -> CommitInfo {
        commit_by(RENOVATE, SignatureStatus::Good, Some(WEB_FLOW))
    }

    fn with_pr_author(mut config: VerifyConfig, login: Option<&str>) -> VerifyConfig {
        config.pr_author = login.map(str::to_string);
        config
    }

    /// The forgery the check exists for: GitHub signs a commit with the
    /// web-flow key for whatever author the caller chose, so a commit that
    /// claims Renovate as author arrives with a good, listed signature. It is
    /// on someone else's pull request, so it must not be allow-listed.
    #[test]
    fn test_web_flow_signed_commit_on_someone_elses_pr_is_not_allow_listed() {
        for config in [policy(false), policy(true)] {
            let config = with_pr_author(config, Some("an-attacker"));

            let result = verify_commit_with(&renovate_commit(), &trusted_map(), &config);

            assert!(!result.passed);
            assert!(matches!(
                result.reason,
                VerificationReason::AllowListedWrongOpener
            ));
        }
    }

    /// If GitHub cannot say who opened the PR, the entry cannot be confirmed.
    #[test]
    fn test_unknown_pr_opener_fails_closed() {
        let config = with_pr_author(policy(true), None);

        let result = verify_commit_with(&renovate_commit(), &trusted_map(), &config);

        assert!(!result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::AllowListedWrongOpener
        ));
    }

    #[test]
    fn test_renovate_on_its_own_pr_is_allow_listed() {
        let config = with_pr_author(policy(true), Some("Renovate[bot]"));

        let result = verify_commit_with(&renovate_commit(), &trusted_map(), &config);

        assert!(result.passed);
        assert!(matches!(result.reason, VerificationReason::AllowListed));
    }

    /// Both checks are needed: the right opener does not rescue a bad signature.
    #[test]
    fn test_the_right_opener_does_not_rescue_a_bad_signature() {
        let unsigned = commit_by(RENOVATE, SignatureStatus::None, None);

        let result = verify_commit_with(&unsigned, &trusted_map(), &policy(true));

        assert!(!result.passed);
        assert!(matches!(
            result.reason,
            VerificationReason::AllowListedBadSignature
        ));
    }

    /// An entry for a key that signs only for its owner does not need the
    /// opener, so an unknown one is fine.
    #[test]
    fn test_entry_without_pr_authors_does_not_need_the_opener() {
        let config = VerifyConfig::from_toml(
            "[[verify_signatures.allow]]\nemail = \"bot[bot]@users.noreply.github.com\"\nkeys = [\"1A2B3C4D5E6F7A8B\"]\n",
        )
        .unwrap();
        let commit = commit_by(
            "bot[bot]@users.noreply.github.com",
            SignatureStatus::Good,
            Some("1A2B3C4D5E6F7A8B"),
        );

        let result = verify_commit_with(&commit, &trusted_map(), &config);

        assert!(result.passed);
        assert!(matches!(result.reason, VerificationReason::AllowListed));
    }

    #[test]
    fn test_wrong_opener_is_a_failure_in_the_summary_and_not_an_external() {
        let config = with_pr_author(policy(false), Some("an-attacker"));

        let (_, summary) = verify_commits_with(vec![renovate_commit()], &trusted_map(), &config);

        assert_eq!(summary.failures, 1);
        assert_eq!(summary.allow_listed, 0);
        assert_eq!(summary.external_contributors, 0);
    }
}
