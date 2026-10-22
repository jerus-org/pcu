//! Detached GPG signatures for git objects.

use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::Error;

/// Signs bytes with a GPG key and returns a detached, ASCII-armoured signature.
///
/// Runs `gpg --status-fd 2 -bsau <key>`, the invocation git itself makes, so
/// `git verify-commit` and `git verify-tag` accept the result. The payload is
/// any git object text, which lets commit and tag buffers share this one step.
pub(crate) struct GpgSigner {
    key: String,
    /// `GNUPGHOME` for the subprocess; `None` inherits the environment.
    home: Option<PathBuf>,
}

impl GpgSigner {
    pub(crate) fn new(key: impl Into<String>, home: Option<PathBuf>) -> Self {
        Self {
            key: key.into(),
            home,
        }
    }

    pub(crate) fn sign(&self, payload: &[u8]) -> Result<String, Error> {
        let mut cmd = Command::new("gpg");
        cmd.args(["--status-fd", "2", "-bsau", self.key.as_str()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(home) = &self.home {
            cmd.env("GNUPGHOME", home);
        }

        let mut child = cmd.spawn()?;

        let mut stdin = child.stdin.take().ok_or(Error::Stdin)?;
        stdin.write_all(payload)?;
        // gpg reads to end of input before it signs.
        drop(stdin);

        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Err(Error::Stdout(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        // Exit status alone is not proof of a signature; git requires this status line too.
        let stderr = std::str::from_utf8(&output.stderr)?;
        if !stderr.contains("\n[GNUPG:] SIG_CREATED ") {
            return Err(Error::GpgError(
                "failed to sign data, program gpg failed, SIG_CREATED not seen in stderr"
                    .to_string(),
            ));
        }

        Ok(std::str::from_utf8(&output.stdout)?.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::GpgSigner;
    use crate::ops::gpg_testing::TestKeyring;

    fn signer(keyring: &TestKeyring, key: &str) -> GpgSigner {
        GpgSigner::new(key, Some(keyring.home().to_path_buf()))
    }

    #[test]
    fn the_signature_is_a_detached_armoured_pgp_signature_over_the_payload() {
        let keyring = TestKeyring::new();
        let key = keyring.generate_key("alpha");

        let signature = signer(&keyring, &key)
            .sign(b"tree 0\n\nmessage\n")
            .expect("sign");

        assert!(signature.starts_with("-----BEGIN PGP SIGNATURE-----"));
        assert!(signature
            .trim_end()
            .ends_with("-----END PGP SIGNATURE-----"));
        assert_eq!(
            keyring.signer_of(&signature, b"tree 0\n\nmessage\n"),
            Some(key)
        );
    }

    #[test]
    fn the_signature_does_not_cover_a_different_payload() {
        let keyring = TestKeyring::new();
        let key = keyring.generate_key("alpha");

        let signature = signer(&keyring, &key).sign(b"original").expect("sign");

        assert_eq!(keyring.signer_of(&signature, b"tampered"), None);
    }

    #[test]
    fn the_requested_key_is_the_one_that_signs() {
        let keyring = TestKeyring::new();
        let _other = keyring.generate_key("alpha");
        let wanted = keyring.generate_key("beta");

        let signature = signer(&keyring, &wanted).sign(b"payload").expect("sign");

        assert_eq!(keyring.signer_of(&signature, b"payload"), Some(wanted));
    }

    #[test]
    fn payload_bytes_need_not_be_utf8() {
        let keyring = TestKeyring::new();
        let key = keyring.generate_key("alpha");
        let payload = [0xff, 0xfe, 0x00, 0x80, b'\n'];

        let signature = signer(&keyring, &key).sign(&payload).expect("sign");

        assert_eq!(keyring.signer_of(&signature, &payload), Some(key));
    }

    #[test]
    fn an_unknown_key_is_an_error() {
        let keyring = TestKeyring::new();
        let _key = keyring.generate_key("alpha");

        let result = signer(&keyring, "0000000000000000000000000000000000000000").sign(b"payload");

        assert!(result.is_err(), "signing with an unknown key must fail");
    }
}
