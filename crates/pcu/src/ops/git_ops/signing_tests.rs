//! Signing behaviour of `commit_staged`, checked against real GnuPG signatures
//! and verified by git itself.

use std::{
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

use super::{GitOps, Sign, SignConfig};
use crate::{ops::gpg_testing::TestKeyring, Client};

struct Fixture {
    dir: TempDir,
    keyring: TestKeyring,
    client: Client,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("create repository directory");
        let repo = git2::Repository::init(dir.path()).expect("init repository");

        std::fs::write(dir.path().join("a.txt"), "a\n").expect("write a.txt");
        let mut index = repo.index().expect("index");
        index.add_path(Path::new("a.txt")).expect("stage a.txt");
        index.write().expect("write index");
        let tree = repo
            .find_tree(index.write_tree().expect("tree"))
            .expect("find tree");
        let author = git2::Signature::now("Seed", "seed@example.com").expect("signature");
        repo.commit(Some("HEAD"), &author, &author, "seed", &tree, &[])
            .expect("seed commit");
        drop(tree);

        let keyring = TestKeyring::new();
        let mut client = Client::new_local_at(dir.path()).expect("local client");
        client.gnupg_home = Some(keyring.home().to_path_buf());

        Self {
            dir,
            keyring,
            client,
        }
    }

    fn stage(&self, name: &str) {
        std::fs::write(self.dir.path().join(name), format!("{name}\n")).expect("write file");
        let mut index = self.client.git_repo.index().expect("index");
        index.add_path(Path::new(name)).expect("stage file");
        index.write().expect("write index");
    }

    fn set_config(&self, key: &str, value: &str) {
        self.client
            .git_repo
            .config()
            .expect("config")
            .set_str(key, value)
            .expect("set config");
    }

    /// Runs git in the repository with the test keyring, like a verifying CI job would.
    fn git(&self, args: &[&str]) -> Output {
        Command::new("git")
            .args(args)
            .current_dir(self.dir.path())
            .env("GNUPGHOME", self.keyring.home())
            .output()
            .expect("run git")
    }

    fn git_ok(&self, args: &[&str]) -> String {
        let out = self.git(args);
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn head(&self) -> String {
        self.git_ok(&["rev-parse", "HEAD"])
    }

    fn commit(&self, config: SignConfig, tag: Option<&str>) -> Result<(), crate::Error> {
        self.client.commit_staged(config, "feat: add b", "v", tag)
    }
}

fn identity(config: SignConfig) -> SignConfig {
    config.with_identity("Bot", "bot@example.com")
}

#[test]
fn signed_commit_verifies_and_is_signed_by_the_requested_key() {
    let fx = Fixture::new();
    let key = fx.keyring.generate_key("alpha");
    fx.stage("b.txt");

    fx.commit(
        identity(SignConfig::new(Sign::Gpg).with_signing_key(&key)),
        None,
    )
    .expect("signed commit");

    let verify = fx.git(&["verify-commit", "HEAD"]);
    assert!(
        verify.status.success(),
        "git verify-commit failed: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
    let signer = fx.git_ok(&["log", "-1", "--format=%GK"]);
    assert!(key.ends_with(&signer), "signed by {signer}, expected {key}");
}

#[test]
fn signed_commit_advances_the_branch_and_keeps_head_attached() {
    let fx = Fixture::new();
    let key = fx.keyring.generate_key("alpha");
    let before = fx.head();
    let branch = fx.git_ok(&["symbolic-ref", "--short", "HEAD"]);
    fx.stage("b.txt");

    fx.commit(
        identity(SignConfig::new(Sign::Gpg).with_signing_key(&key)),
        None,
    )
    .expect("signed commit");

    assert_eq!(fx.git_ok(&["symbolic-ref", "--short", "HEAD"]), branch);
    assert_eq!(fx.git_ok(&["rev-parse", &branch]), fx.head());
    assert_eq!(fx.git_ok(&["rev-parse", "HEAD^"]), before);
}

#[test]
fn signed_commit_records_the_identity_and_a_signoff() {
    let fx = Fixture::new();
    let key = fx.keyring.generate_key("alpha");
    fx.stage("b.txt");

    fx.commit(
        identity(SignConfig::new(Sign::Gpg).with_signing_key(&key)),
        None,
    )
    .expect("signed commit");

    assert_eq!(
        fx.git_ok(&["log", "-1", "--format=%an <%ae> | %cn <%ce>"]),
        "Bot <bot@example.com> | Bot <bot@example.com>"
    );
    let message = fx.git_ok(&["log", "-1", "--format=%B"]);
    assert!(message.starts_with("feat: add b"), "{message}");
    assert!(
        message.ends_with("Signed-off-by: Bot <bot@example.com>"),
        "{message}"
    );
}

#[test]
fn a_signed_commit_on_a_detached_head_moves_head() {
    let fx = Fixture::new();
    let key = fx.keyring.generate_key("alpha");
    fx.git_ok(&["checkout", "-q", "--detach"]);
    let before = fx.head();
    fx.stage("b.txt");

    fx.commit(
        identity(SignConfig::new(Sign::Gpg).with_signing_key(&key)),
        None,
    )
    .expect("signed commit");

    assert!(
        !fx.git(&["symbolic-ref", "-q", "HEAD"]).status.success(),
        "HEAD must stay detached"
    );
    assert_eq!(fx.git_ok(&["rev-parse", "HEAD^"]), before);
    assert!(fx.git(&["verify-commit", "HEAD"]).status.success());
}

#[test]
fn an_explicit_signing_key_wins_over_the_configured_one() {
    let fx = Fixture::new();
    let explicit = fx.keyring.generate_key("alpha");
    let configured = fx.keyring.generate_key("beta");
    fx.set_config("user.signingkey", &configured);
    fx.stage("b.txt");

    fx.commit(
        identity(SignConfig::new(Sign::Gpg).with_signing_key(&explicit)),
        None,
    )
    .expect("signed commit");

    let signer = fx.git_ok(&["log", "-1", "--format=%GK"]);
    assert!(explicit.ends_with(&signer), "signed by {signer}");
}

#[test]
fn without_an_explicit_key_the_configured_one_is_used() {
    let fx = Fixture::new();
    let _unused = fx.keyring.generate_key("alpha");
    let configured = fx.keyring.generate_key("beta");
    fx.set_config("user.signingkey", &configured);
    fx.stage("b.txt");

    fx.commit(identity(SignConfig::new(Sign::Gpg)), None)
        .expect("signed commit");

    let signer = fx.git_ok(&["log", "-1", "--format=%GK"]);
    assert!(configured.ends_with(&signer), "signed by {signer}");
}

#[test]
fn a_failed_signature_creates_no_commit() {
    let fx = Fixture::new();
    let _key = fx.keyring.generate_key("alpha");
    let before = fx.head();
    fx.stage("b.txt");

    let result = fx.commit(
        identity(
            SignConfig::new(Sign::Gpg).with_signing_key("0000000000000000000000000000000000000000"),
        ),
        None,
    );

    assert!(result.is_err(), "signing with an unknown key must fail");
    assert_eq!(fx.head(), before, "HEAD must not move when signing fails");
}

#[test]
fn an_unsigned_commit_carries_no_signature() {
    let fx = Fixture::new();
    fx.stage("b.txt");

    fx.commit(identity(SignConfig::new(Sign::None)), None)
        .expect("unsigned commit");

    assert_eq!(fx.git_ok(&["log", "-1", "--format=%G?"]), "N");
}

#[test]
fn a_signed_release_tag_verifies_and_points_at_the_new_commit() {
    let fx = Fixture::new();
    let key = fx.keyring.generate_key("alpha");
    fx.set_config("user.signingkey", &key);
    fx.stage("b.txt");

    fx.commit(
        identity(SignConfig::new(Sign::Gpg).with_signing_key(&key)),
        Some("1.0.0"),
    )
    .expect("signed commit and tag");

    assert_eq!(fx.git_ok(&["cat-file", "-t", "v1.0.0"]), "tag");
    assert!(fx
        .git_ok(&["cat-file", "tag", "v1.0.0"])
        .contains("-----BEGIN PGP SIGNATURE-----"));
    let verify = fx.git(&["tag", "-v", "v1.0.0"]);
    assert!(
        verify.status.success(),
        "git tag -v failed: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
    assert_eq!(fx.git_ok(&["rev-parse", "v1.0.0^{commit}"]), fx.head());
}

#[test]
fn an_unsigned_release_tag_is_annotated_but_unverifiable() {
    let fx = Fixture::new();
    fx.stage("b.txt");

    fx.commit(identity(SignConfig::new(Sign::None)), Some("1.0.0"))
        .expect("unsigned commit and tag");

    assert_eq!(fx.git_ok(&["cat-file", "-t", "v1.0.0"]), "tag");
    assert!(!fx
        .git_ok(&["cat-file", "tag", "v1.0.0"])
        .contains("BEGIN PGP SIGNATURE"));
    assert_eq!(fx.git_ok(&["rev-parse", "v1.0.0^{commit}"]), fx.head());
}
