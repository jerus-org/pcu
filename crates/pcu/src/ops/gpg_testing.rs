//! An ephemeral GnuPG keyring for tests that create real signatures.

use std::{
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

/// A throwaway `GNUPGHOME`. Tests hand its path to the code under test rather
/// than mutating the process environment, so they stay safe to run in parallel.
pub(crate) struct TestKeyring {
    dir: TempDir,
}

impl TestKeyring {
    pub(crate) fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("create keyring directory"),
        }
    }

    pub(crate) fn home(&self) -> &Path {
        self.dir.path()
    }

    /// Generates a passphrase-less ed25519 signing key and returns its fingerprint.
    pub(crate) fn generate_key(&self, name: &str) -> String {
        let uid = format!("{name} <{name}@example.com>");
        let out = self.gpg(&[
            "--batch",
            "--pinentry-mode",
            "loopback",
            "--passphrase",
            "",
            "--quick-generate-key",
            &uid,
            "ed25519",
            "sign",
            "never",
        ]);
        assert!(
            out.status.success(),
            "gpg key generation failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        self.fingerprint(&uid)
    }

    fn fingerprint(&self, uid: &str) -> String {
        let out = self.gpg(&["--batch", "--with-colons", "--list-secret-keys", uid]);
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|line| line.strip_prefix("fpr:::::::::"))
            .map(|fpr| fpr.trim_end_matches(':').to_string())
            .expect("fingerprint in gpg --with-colons output")
    }

    fn gpg(&self, args: &[&str]) -> Output {
        Command::new("gpg")
            .arg("--homedir")
            .arg(self.home())
            .args(args)
            .output()
            .expect("gpg must be installed to run the signing tests")
    }
}

impl Drop for TestKeyring {
    fn drop(&mut self) {
        // Stop the agent this keyring started so no process outlives the test.
        let _ = Command::new("gpgconf")
            .arg("--homedir")
            .arg(self.home())
            .args(["--kill", "all"])
            .output();
    }
}
