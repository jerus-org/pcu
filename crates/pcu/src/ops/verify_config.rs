//! Policy for `pcu verify-signatures`: an allow list of identities accepted
//! without being collaborators, and a hard stop for everyone else.
//!
//! The policy lives in `pcu.toml` under `[verify_signatures]`:
//!
//! ```toml
//! [verify_signatures]
//! fail_on_external = true
//!
//! [[verify_signatures.allow]]
//! email = "29139614+renovate[bot]@users.noreply.github.com"
//! keys = ["B5690EEEBB952194"]          # GitHub's web-flow key
//! pr_authors = ["renovate[bot]"]       # required with that key
//! description = "Renovate (committed and signed by GitHub)"
//! ```
//!
//! # The file is read from the base ref, never the branch under review
//!
//! A pull request can edit any file it contains. If the policy were read from
//! the working tree, a PR could add its own author to the allow list, or turn
//! the hard stop off, and pass its own check. The policy is therefore read
//! from the git object at `--base` (for example `origin/main`), so a change to
//! it only takes effect once it has been merged.
//!
//! # Entries must name signing keys
//!
//! An author email is self-declared: anyone can write `renovate[bot]@…` into a
//! commit. An email-only entry would allow-list whoever forges it, so every
//! entry must list the key(s) its commits are signed with, and a commit only
//! matches when its signature is good and made with one of them.
//!
//! # GitHub's web-flow key needs a second, authenticated check
//!
//! GitHub signs commits it creates through its web UI and API with the
//! web-flow key, for whatever author the caller chose. Its signature proves
//! "GitHub made this commit", not who the author is, so email plus web-flow
//! key can be reproduced by someone other than the bot. An entry naming that
//! key must therefore also list `pr_authors`: the GitHub logins that must have
//! opened the pull request. GitHub authenticates who opens a PR, so a forged
//! author on someone else's PR no longer matches. When the opener cannot be
//! determined the commit fails rather than passes.

use std::path::Path;

use git2::{ErrorCode, Repository};
use serde::Deserialize;

use super::git_signature_ops::resolve_reference;
use crate::Error;

/// Path of the policy file, relative to the repository root.
pub const POLICY_PATH: &str = "pcu.toml";

/// GitHub's web-flow signing key, which signs commits GitHub creates for any
/// author (see the module docs).
pub(crate) const WEB_FLOW_KEY_ID: &str = "B5690EEEBB952194";

/// Shortest key ID accepted. A 16-hex-digit long ID is what git reports for a
/// signature; shorter IDs collide too easily to be a safe match.
const MIN_KEY_ID_LEN: usize = 16;

/// The verification policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifyConfig {
    /// Fail verification for any commit from an author who is neither a
    /// trusted identity nor on the allow list.
    pub fail_on_external: bool,
    /// Identities accepted without being collaborators.
    pub allow: Vec<AllowEntry>,
    /// Not part of the file: the GitHub login that opened the pull request
    /// under review, set by the caller from the GitHub API. `None` when it is
    /// unknown (no PR, or the lookup failed).
    pub pr_author: Option<String>,
}

/// One allow-listed identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowEntry {
    /// Author email, lowercase.
    pub email: String,
    /// Key IDs (or fingerprints) its commits must be signed with, uppercase.
    pub keys: Vec<String>,
    /// GitHub logins, lowercase, one of which must have opened the pull
    /// request. Required when `keys` names the web-flow key.
    pub pr_authors: Vec<String>,
    /// GitHub login whose registered GPG keys are fetched so `keys` can be
    /// verified. Lowercase. Only supplies key material: trust is still decided
    /// by `keys`, so a login that holds other keys adds nothing.
    pub github_user: Option<String>,
    /// Why the identity is allowed: a note for whoever reads the policy.
    pub description: Option<String>,
}

impl VerifyConfig {
    /// The allow-list entry for an author email, ignoring case and
    /// surrounding whitespace.
    pub fn allow_entry(&self, author_email: &str) -> Option<&AllowEntry> {
        let wanted = author_email.trim().to_lowercase();
        self.allow.iter().find(|entry| entry.email == wanted)
    }

    /// Whether any allow-list entry needs the PR opener's login, so the caller
    /// only makes the GitHub API call when it matters.
    pub fn needs_pr_author(&self) -> bool {
        self.allow.iter().any(|entry| !entry.pr_authors.is_empty())
    }

    /// Parse the policy from the text of a `pcu.toml`.
    pub fn from_toml(text: &str) -> Result<Self, Error> {
        let file: RawFile =
            toml::from_str(text).map_err(|e| Error::VerifyConfig(format!("{POLICY_PATH}: {e}")))?;
        let raw = file.verify_signatures;

        let mut allow: Vec<AllowEntry> = Vec::new();
        for (index, entry) in raw.allow.into_iter().enumerate() {
            let entry = AllowEntry::from_raw(entry, index + 1)?;
            if allow.iter().any(|known| known.email == entry.email) {
                return Err(Error::VerifyConfig(format!(
                    "allow entry {} repeats an email already allow-listed; list all of an \
                     identity's keys in one entry",
                    index + 1
                )));
            }
            allow.push(entry);
        }

        Ok(Self {
            fail_on_external: raw.fail_on_external,
            allow,
            pr_author: None,
        })
    }

    /// Read the policy from `pcu.toml` at `base_ref`. A missing file is the
    /// default policy; a file that exists but is invalid is an error, so a
    /// typo can never silently weaken the gate.
    pub fn load_from_ref(repo: &Repository, base_ref: &str) -> Result<Self, Error> {
        let oid = resolve_reference(repo, base_ref)?;
        let tree = repo.find_commit(oid)?.tree()?;

        let entry = match tree.get_path(Path::new(POLICY_PATH)) {
            Ok(entry) => entry,
            Err(e) if e.code() == ErrorCode::NotFound => {
                log::debug!("No {POLICY_PATH} at {base_ref}; using the default policy");
                return Ok(Self::default());
            }
            Err(e) => return Err(e.into()),
        };

        let blob = repo.find_blob(entry.id())?;
        let text = std::str::from_utf8(blob.content())
            .map_err(|_| Error::VerifyConfig(format!("{POLICY_PATH} is not valid UTF-8")))?;
        Self::from_toml(text)
    }
}

impl AllowEntry {
    /// Whether `key_id` (from a commit's signature) is one of this entry's
    /// keys. A long key ID matches the tail of a fingerprint, either way
    /// round.
    pub fn approves(&self, key_id: &str) -> bool {
        let key_id = key_id.trim().to_uppercase();
        !key_id.is_empty()
            && self
                .keys
                .iter()
                .any(|k| key_id.ends_with(k.as_str()) || k.ends_with(key_id.as_str()))
    }

    /// Whether the pull request was opened by one of the entry's logins. An
    /// entry with no `pr_authors` does not care. With some, an unknown opener
    /// does not match: fail closed.
    pub fn opener_allowed(&self, pr_author: Option<&str>) -> bool {
        if self.pr_authors.is_empty() {
            return true;
        }
        pr_author
            .map(|login| login.trim().to_lowercase())
            .is_some_and(|login| self.pr_authors.contains(&login))
    }

    /// Whether the entry names GitHub's web-flow key, in either form.
    fn names_web_flow_key(keys: &[String]) -> bool {
        keys.iter()
            .any(|k| k.ends_with(WEB_FLOW_KEY_ID) || WEB_FLOW_KEY_ID.ends_with(k.as_str()))
    }

    fn from_raw(raw: RawAllow, position: usize) -> Result<Self, Error> {
        let email = raw.email.trim().to_lowercase();
        if email.is_empty() || !email.contains('@') {
            return Err(Error::VerifyConfig(format!(
                "allow entry {position}: `email` must be an email address"
            )));
        }

        if raw.keys.is_empty() {
            return Err(Error::VerifyConfig(format!(
                "allow entry {position}: `keys` is required. An email alone is self-declared \
                 and can be forged, so an entry must name the key(s) its commits are signed with"
            )));
        }
        let keys = raw
            .keys
            .iter()
            .map(|key| normalise_key_id(key, position))
            .collect::<Result<Vec<_>, _>>()?;

        let pr_authors: Vec<String> = raw
            .pr_authors
            .iter()
            .map(|login| login.trim().to_lowercase())
            .collect();
        if pr_authors
            .iter()
            .any(|login| login.is_empty() || login.contains(char::is_whitespace))
        {
            return Err(Error::VerifyConfig(format!(
                "allow entry {position}: `pr_authors` must be GitHub logins"
            )));
        }
        if Self::names_web_flow_key(&keys) && pr_authors.is_empty() {
            return Err(Error::VerifyConfig(format!(
                "allow entry {position}: `keys` names GitHub's web-flow key, which signs commits \
                 for any author, so the author email and signature cannot identify the bot. Add \
                 `pr_authors` (the GitHub login that opens its pull requests, for example \
                 `renovate[bot]`) so the check also requires who opened the PR"
            )));
        }

        let github_user = match raw.github_user.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(login) if login.contains(char::is_whitespace) || login.contains('/') => {
                return Err(Error::VerifyConfig(format!(
                    "allow entry {position}: `github_user` must be a GitHub login"
                )));
            }
            Some(login) => Some(login.to_lowercase()),
        };

        let description = raw
            .description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());

        Ok(Self {
            email,
            keys,
            pr_authors,
            github_user,
            description,
        })
    }
}

/// Uppercase, drop spaces and a leading `0x`, and require enough hex digits.
fn normalise_key_id(raw: &str, position: usize) -> Result<String, Error> {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let digits = compact
        .strip_prefix("0x")
        .or_else(|| compact.strip_prefix("0X"))
        .unwrap_or(&compact)
        .to_uppercase();

    if digits.len() < MIN_KEY_ID_LEN
        || digits.len() > 40
        || !digits.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(Error::VerifyConfig(format!(
            "allow entry {position}: key `{raw}` must be {MIN_KEY_ID_LEN} to 40 hex digits \
             (a long key ID or a fingerprint)"
        )));
    }
    Ok(digits)
}

/// The whole `pcu.toml`: only `[verify_signatures]` is ours; the other
/// settings in the file belong to other pcu commands and are ignored here.
#[derive(Debug, Deserialize)]
struct RawFile {
    #[serde(default)]
    verify_signatures: RawVerify,
}

/// Unknown keys are an error: a misspelt `fail_on_external` must not silently
/// leave the hard stop off.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVerify {
    #[serde(default)]
    fail_on_external: bool,
    #[serde(default)]
    allow: Vec<RawAllow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAllow {
    email: String,
    #[serde(default)]
    keys: Vec<String>,
    #[serde(default)]
    pr_authors: Vec<String>,
    github_user: Option<String>,
    description: Option<String>,
}

/// Helpers for tests that need a git repository holding a policy file.
#[cfg(test)]
pub(crate) mod test_support {
    use git2::{Oid, Repository, Signature};
    use tempfile::TempDir;

    use super::POLICY_PATH;

    pub(crate) fn temp_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    /// Commit to `refname` a tree holding `content` as the policy file, or an
    /// empty tree when `None`.
    pub(crate) fn commit_policy(
        repo: &Repository,
        refname: &str,
        content: Option<&str>,
        parent: Option<Oid>,
    ) -> Oid {
        let mut builder = repo.treebuilder(None).unwrap();
        if let Some(content) = content {
            let blob = repo.blob(content.as_bytes()).unwrap();
            builder.insert(POLICY_PATH, blob, 0o100644).unwrap();
        }
        let tree = repo.find_tree(builder.write().unwrap()).unwrap();
        let sig = Signature::now("t", "t@example.test").unwrap();
        let parents: Vec<_> = parent
            .map(|p| repo.find_commit(p).unwrap())
            .into_iter()
            .collect();
        let parents: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some(refname), &sig, &sig, "policy", &tree, &parents)
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use git2::Oid;

    use super::test_support::{commit_policy, temp_repo};
    use super::*;

    const RENOVATE: &str = "29139614+renovate[bot]@users.noreply.github.com";
    const WEB_FLOW: &str = WEB_FLOW_KEY_ID;
    /// A key that signs only for its owner, so it needs no `pr_authors`.
    const OTHER_KEY: &str = "1A2B3C4D5E6F7A8B";

    fn renovate_policy() -> String {
        format!(
            r#"
            prlog = "PRLOG.md"

            [verify_signatures]
            fail_on_external = true

            [[verify_signatures.allow]]
            email = "{RENOVATE}"
            keys = ["{WEB_FLOW}"]
            pr_authors = ["renovate[bot]"]
            description = "Renovate (committed and signed by GitHub)"
            "#
        )
    }

    fn err(text: &str) -> String {
        VerifyConfig::from_toml(text).unwrap_err().to_string()
    }

    #[test]
    fn default_policy_changes_nothing() {
        let config = VerifyConfig::default();

        assert!(!config.fail_on_external);
        assert!(config.allow.is_empty());
    }

    #[test]
    fn a_file_without_the_section_is_the_default_policy() {
        let config = VerifyConfig::from_toml("prlog = \"PRLOG.md\"\nbranch = \"main\"\n").unwrap();

        assert_eq!(config, VerifyConfig::default());
        assert_eq!(
            VerifyConfig::from_toml("").unwrap(),
            VerifyConfig::default()
        );
    }

    #[test]
    fn parses_the_renovate_example() {
        let config = VerifyConfig::from_toml(&renovate_policy()).unwrap();

        assert!(config.fail_on_external);
        assert_eq!(config.allow.len(), 1);
        assert_eq!(config.allow[0].email, RENOVATE.to_lowercase());
        assert_eq!(config.allow[0].keys, vec![WEB_FLOW.to_string()]);
        assert_eq!(
            config.allow[0].description.as_deref(),
            Some("Renovate (committed and signed by GitHub)")
        );
    }

    #[test]
    fn github_user_is_normalised_and_bad_values_refused() {
        let ok = VerifyConfig::from_toml(
            "[[verify_signatures.allow]]\nemail = \"a@b.test\"\nkeys = [\"1A2B3C4D5E6F7A8B\"]\ngithub_user = \" Some-User \"\n",
        )
        .unwrap();
        assert_eq!(ok.allow[0].github_user.as_deref(), Some("some-user"));

        for bad in ["a b", "owner/repo"] {
            let text = format!(
                "[[verify_signatures.allow]]\nemail = \"a@b.test\"\nkeys = [\"1A2B3C4D5E6F7A8B\"]\ngithub_user = \"{bad}\"\n"
            );
            assert!(VerifyConfig::from_toml(&text).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_policy_shipped_in_this_repository_is_valid() {
        // The real pcu.toml at the repository root: a typo there is an error
        // for every verify-signatures run, so catch it before merge.
        let policy = VerifyConfig::from_toml(include_str!("../../../../pcu.toml")).unwrap();

        let renovate = policy
            .allow_entry("29139614+renovate[bot]@users.noreply.github.com")
            .expect("Renovate is allow-listed");
        assert!(
            policy.fail_on_external,
            "the hard stop is on for this repository"
        );
        assert_eq!(renovate.keys, vec![WEB_FLOW_KEY_ID.to_string()]);
        assert_eq!(renovate.pr_authors, vec!["renovate[bot]".to_string()]);
        assert!(renovate.opener_allowed(Some("renovate[bot]")));
        assert!(!renovate.opener_allowed(Some("someone-else")));
        assert!(
            !renovate.opener_allowed(None),
            "unknown opener fails closed"
        );

        let web_ui = policy
            .allow_entry("47631109+jerusdp@users.noreply.github.com")
            .expect("the maintainer's web UI edits are allow-listed");
        assert_eq!(web_ui.keys, vec![WEB_FLOW_KEY_ID.to_string()]);
        assert!(web_ui.opener_allowed(Some("jerusdp")));
        assert!(web_ui.opener_allowed(Some("renovate[bot]")));
        assert!(!web_ui.opener_allowed(Some("someone-else")));
        assert!(!web_ui.opener_allowed(None), "unknown opener fails closed");
    }

    #[test]
    fn an_entry_without_keys_is_refused() {
        let message = err(r#"
            [[verify_signatures.allow]]
            email = "renovate[bot]@users.noreply.github.com"
            "#);

        assert!(message.contains("`keys` is required"), "got: {message}");
        assert!(message.contains("forged"), "got: {message}");
    }

    #[test]
    fn an_entry_with_an_empty_key_list_is_refused() {
        let message = err(r#"
            [[verify_signatures.allow]]
            email = "a@example.test"
            keys = []
            "#);

        assert!(message.contains("`keys` is required"), "got: {message}");
    }

    #[test]
    fn misspelt_settings_are_errors_not_silent_no_ops() {
        let message = err("[verify_signatures]\nfail_on_extrenal = true\n");
        assert!(message.contains("fail_on_extrenal"), "got: {message}");

        let message = err(r#"
            [[verify_signatures.allow]]
            email = "a@example.test"
            keys = ["B5690EEEBB952194"]
            kyes = ["B5690EEEBB952194"]
            "#);
        assert!(message.contains("kyes"), "got: {message}");
    }

    #[test]
    fn malformed_toml_is_an_error() {
        assert!(err("[verify_signatures\n").contains(POLICY_PATH));
    }

    #[test]
    fn bad_emails_are_refused() {
        for email in ["", "   ", "not-an-email"] {
            let message = err(&format!(
                "[[verify_signatures.allow]]\nemail = \"{email}\"\nkeys = [\"{WEB_FLOW}\"]\n"
            ));
            assert!(message.contains("`email`"), "{email:?}: {message}");
        }
    }

    #[test]
    fn short_or_non_hex_keys_are_refused() {
        for key in [
            "B5690EEE",
            "not-a-key-id-at-all",
            "B5690EEEBB95219G",
            &"A".repeat(41),
        ] {
            let message = err(&format!(
                "[[verify_signatures.allow]]\nemail = \"a@example.test\"\nkeys = [\"{key}\"]\n"
            ));
            assert!(message.contains("hex digits"), "{key:?}: {message}");
        }
    }

    #[test]
    fn keys_and_emails_are_normalised() {
        let config = VerifyConfig::from_toml(
            r#"
            [[verify_signatures.allow]]
            email = "  Renovate@Example.Test "
            keys = ["0x1a2b3c4d5e6f7a8b", "1a2b 3c4d 5e6f 7a8b"]
            "#,
        )
        .unwrap();

        assert_eq!(config.allow[0].email, "renovate@example.test");
        assert_eq!(
            config.allow[0].keys,
            vec![OTHER_KEY.to_string(), OTHER_KEY.to_string()]
        );
    }

    #[test]
    fn a_repeated_email_is_refused() {
        let message = err(&format!(
            r#"
            [[verify_signatures.allow]]
            email = "a@example.test"
            keys = ["{OTHER_KEY}"]
            [[verify_signatures.allow]]
            email = "A@Example.Test"
            keys = ["{OTHER_KEY}"]
            "#
        ));

        assert!(message.contains("repeats an email"), "got: {message}");
    }

    #[test]
    fn allow_entry_lookup_ignores_case_and_whitespace() {
        let config = VerifyConfig::from_toml(&renovate_policy()).unwrap();

        assert!(config.allow_entry(RENOVATE).is_some());
        assert!(config.allow_entry(&RENOVATE.to_uppercase()).is_some());
        assert!(config.allow_entry(&format!("  {RENOVATE}\n")).is_some());
        assert!(config.allow_entry("someone-else@example.test").is_none());
    }

    #[test]
    fn approves_matches_a_long_id_against_a_fingerprint_either_way_round() {
        let config = VerifyConfig::from_toml(&renovate_policy()).unwrap();
        let entry = &config.allow[0];
        let fingerprint = format!("{}{WEB_FLOW}", "A".repeat(24));

        assert!(entry.approves(WEB_FLOW));
        assert!(entry.approves(&WEB_FLOW.to_lowercase()));
        assert!(entry.approves(&fingerprint), "fingerprint ending in the ID");
        assert!(!entry.approves("0000000000000000"));
        assert!(!entry.approves(""));
        assert!(!entry.approves("   "));
    }

    #[test]
    fn approves_a_fingerprint_entry_from_a_long_id() {
        let fingerprint = format!("{}{WEB_FLOW}", "A".repeat(24));
        let config = VerifyConfig::from_toml(&format!(
            "[[verify_signatures.allow]]\nemail = \"a@example.test\"\nkeys = [\"{fingerprint}\"]\npr_authors = [\"renovate[bot]\"]\n"
        ))
        .unwrap();

        assert!(config.allow[0].approves(WEB_FLOW));
    }

    // --- reading the policy from a git ref ---------------------------------

    fn commit_file(repo: &Repository, refname: &str, content: &str, parent: Option<Oid>) -> Oid {
        commit_policy(repo, refname, Some(content), parent)
    }

    fn empty_commit(repo: &Repository, refname: &str) -> Oid {
        commit_policy(repo, refname, None, None)
    }

    #[test]
    fn loads_the_policy_from_the_base_ref() {
        let (_dir, repo) = temp_repo();
        commit_file(&repo, "refs/heads/main", &renovate_policy(), None);

        let config = VerifyConfig::load_from_ref(&repo, "main").unwrap();

        assert!(config.fail_on_external);
        assert!(config.allow_entry(RENOVATE).is_some());
    }

    /// The point of reading from the base: a branch cannot allow-list its own
    /// author or switch the hard stop off.
    #[test]
    fn a_branch_cannot_change_the_policy_that_judges_it() {
        let (_dir, repo) = temp_repo();
        let base = commit_file(&repo, "refs/heads/main", &renovate_policy(), None);
        let hostile = format!(
            r#"
            [verify_signatures]
            fail_on_external = false
            [[verify_signatures.allow]]
            email = "attacker@example.test"
            keys = ["{OTHER_KEY}"]
            "#
        );
        commit_file(&repo, "refs/heads/feature", &hostile, Some(base));

        let judged_by_base = VerifyConfig::load_from_ref(&repo, "main").unwrap();

        assert!(judged_by_base.fail_on_external, "hard stop stays on");
        assert!(judged_by_base
            .allow_entry("attacker@example.test")
            .is_none());
        // The branch's own file does say otherwise; that is what is ignored.
        let own = VerifyConfig::load_from_ref(&repo, "feature").unwrap();
        assert!(own.allow_entry("attacker@example.test").is_some());
    }

    #[test]
    fn a_base_without_the_file_is_the_default_policy() {
        let (_dir, repo) = temp_repo();
        empty_commit(&repo, "refs/heads/main");

        assert_eq!(
            VerifyConfig::load_from_ref(&repo, "main").unwrap(),
            VerifyConfig::default()
        );
    }

    #[test]
    fn an_invalid_file_at_the_base_is_an_error_not_the_default() {
        let (_dir, repo) = temp_repo();
        commit_file(
            &repo,
            "refs/heads/main",
            "[verify_signatures]\nfail_on_extrenal = true\n",
            None,
        );

        let message = VerifyConfig::load_from_ref(&repo, "main")
            .unwrap_err()
            .to_string();

        assert!(message.contains("fail_on_extrenal"), "got: {message}");
    }

    #[test]
    fn an_unknown_base_ref_is_an_error() {
        let (_dir, repo) = temp_repo();
        empty_commit(&repo, "refs/heads/main");

        assert!(VerifyConfig::load_from_ref(&repo, "no-such-ref").is_err());
    }

    // --- web-flow entries need an authenticated second check (#1132) -------

    /// GitHub's web-flow key signs for any author, so the email and signature
    /// cannot tell the bot from someone using its author email.
    #[test]
    fn an_entry_naming_the_web_flow_key_must_list_pr_authors() {
        let fingerprint = format!("{}{WEB_FLOW}", "A".repeat(24));
        for key in [WEB_FLOW.to_string(), WEB_FLOW.to_lowercase(), fingerprint] {
            let message = err(&format!(
                "[[verify_signatures.allow]]\nemail = \"{RENOVATE}\"\nkeys = [\"{key}\"]\n"
            ));

            assert!(message.contains("web-flow"), "{key}: {message}");
            assert!(message.contains("`pr_authors`"), "{key}: {message}");
        }
    }

    #[test]
    fn web_flow_with_pr_authors_is_accepted() {
        assert!(VerifyConfig::from_toml(&renovate_policy()).is_ok());
    }

    #[test]
    fn a_key_that_signs_only_for_its_owner_needs_no_pr_authors() {
        let config = VerifyConfig::from_toml(&format!(
            "[[verify_signatures.allow]]\nemail = \"bot@example.test\"\nkeys = [\"{OTHER_KEY}\"]\n"
        ))
        .unwrap();

        assert!(config.allow[0].pr_authors.is_empty());
        assert!(!config.needs_pr_author());
    }

    #[test]
    fn needs_pr_author_only_when_an_entry_lists_them() {
        assert!(VerifyConfig::from_toml(&renovate_policy())
            .unwrap()
            .needs_pr_author());
        assert!(!VerifyConfig::default().needs_pr_author());
    }

    #[test]
    fn pr_authors_are_normalised_and_blank_ones_refused() {
        let config = VerifyConfig::from_toml(&format!(
            "[[verify_signatures.allow]]\nemail = \"a@example.test\"\nkeys = [\"{OTHER_KEY}\"]\n\
             pr_authors = [\"  Renovate[bot] \"]\n"
        ))
        .unwrap();
        assert_eq!(
            config.allow[0].pr_authors,
            vec!["renovate[bot]".to_string()]
        );

        for bad in ["\"\"", "\"   \"", "\"two words\""] {
            let message = err(&format!(
                "[[verify_signatures.allow]]\nemail = \"a@example.test\"\nkeys = [\"{OTHER_KEY}\"]\n\
                 pr_authors = [{bad}]\n"
            ));
            assert!(message.contains("GitHub logins"), "{bad}: {message}");
        }
    }

    #[test]
    fn opener_allowed_matches_the_login_ignoring_case() {
        let config = VerifyConfig::from_toml(&renovate_policy()).unwrap();
        let entry = &config.allow[0];

        assert!(entry.opener_allowed(Some("renovate[bot]")));
        assert!(entry.opener_allowed(Some("Renovate[Bot]")));
        assert!(entry.opener_allowed(Some("  renovate[bot] ")));
        assert!(!entry.opener_allowed(Some("someone-else")));
        assert!(!entry.opener_allowed(Some("")));
    }

    /// When the opener is unknown the check cannot be confirmed, so it fails.
    #[test]
    fn opener_allowed_fails_closed_when_the_opener_is_unknown() {
        let config = VerifyConfig::from_toml(&renovate_policy()).unwrap();

        assert!(!config.allow[0].opener_allowed(None));
    }

    #[test]
    fn an_entry_without_pr_authors_ignores_the_opener() {
        let config = VerifyConfig::from_toml(&format!(
            "[[verify_signatures.allow]]\nemail = \"bot@example.test\"\nkeys = [\"{OTHER_KEY}\"]\n"
        ))
        .unwrap();

        assert!(config.allow[0].opener_allowed(None));
        assert!(config.allow[0].opener_allowed(Some("anyone")));
    }
}
