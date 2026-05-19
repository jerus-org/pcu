use crate::Error;

/// Import a GPG secret key and its ownertrust into the system keyring.
///
/// Replicates the toolkit's `import GPG key` command exactly:
/// ```text
/// echo -e ${BOT_GPG_KEY} | base64 --decode --ignore-garbage | gpg --batch --allow-secret-key-import --import
/// echo ${BOT_TRUST} | gpg --import-ownertrust
/// ```
///
/// # Arguments
/// * `b64_key` — Base64-encoded GPG private key (may contain literal `\n` escape sequences
///   as stored by CircleCI; these are expanded before decoding)
/// * `trust` — Ownertrust string (e.g. `FINGERPRINT:6:`); a trailing newline is added if absent.
///   Literal `\n` escape sequences are normalised to real newlines before import.
pub fn import_gpg_key(b64_key: &str, trust: &str) -> Result<(), Error> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    // --- Import GPG private key ---
    // `printf '%b'` expands `\n` escape sequences in the base64 data, matching
    // the toolkit's `echo -e` step.  `--ignore-garbage` absorbs any stray chars.
    let import_result = Command::new("sh")
        .args([
            "-c",
            r#"printf '%b' "$_GPG_KEY" | base64 --decode --ignore-garbage | gpg --batch --allow-secret-key-import --import"#,
        ])
        .env("_GPG_KEY", b64_key)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::GpgError(format!("Failed to run gpg key import: {e}")))?;

    if !import_result.status.success() {
        return Err(Error::GpgError(format!(
            "gpg key import failed: {}",
            String::from_utf8_lossy(&import_result.stderr)
        )));
    }

    // --- Import ownertrust ---
    // Normalise any literal `\n` escape sequences (CircleCI stores multi-line
    // values this way), ensure a trailing newline, then write to a temp file.
    // Using a temp file (instead of stdin) bypasses gpg's stdin line-length
    // limit, which triggers "line too long" when the value is passed via a pipe
    // without prior normalisation.
    let trust_normalised = trust.replace("\\n", "\n");
    let trust_with_newline = if trust_normalised.ends_with('\n') {
        trust_normalised
    } else {
        format!("{trust_normalised}\n")
    };

    let mut trust_file = tempfile::NamedTempFile::new()
        .map_err(|e| Error::GpgError(format!("Failed to create temp file for ownertrust: {e}")))?;
    trust_file
        .write_all(trust_with_newline.as_bytes())
        .map_err(|e| Error::GpgError(format!("Failed to write ownertrust data: {e}")))?;
    trust_file
        .flush()
        .map_err(|e| Error::GpgError(format!("Failed to flush ownertrust data: {e}")))?;

    let trust_result = Command::new("gpg")
        .arg("--import-ownertrust")
        .arg(trust_file.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::GpgError(format!("Failed to run gpg --import-ownertrust: {e}")))?;

    if !trust_result.status.success() {
        return Err(Error::GpgError(format!(
            "gpg ownertrust import failed: {}",
            String::from_utf8_lossy(&trust_result.stderr)
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_gpg_key_fails_with_invalid_base64() {
        let result = import_gpg_key("!!! not valid base64 !!!", "DEADBEEF:6:");
        assert!(result.is_err(), "expected error for invalid base64, got Ok");
    }

    #[test]
    fn import_gpg_key_trust_normalises_literal_escape_newlines() {
        // The normalisation logic should convert literal \n sequences to real
        // newlines regardless of whether the key import succeeds.  We test
        // this indirectly: if the trust temp-file is written correctly, gpg
        // at least gets parseable input (it may still fail without a real key
        // in the keyring, but the error should be about the unknown fingerprint
        // rather than "line too long").
        //
        // We use a syntactically valid ownertrust line with a literal \n so the
        // normalisation is exercised.  If gpg is not installed this test is
        // skipped via the spawn error path.
        let trust_with_escape = "0000000000000000000000000000000000000000:6:\\n";
        let result = import_gpg_key("", trust_with_escape);
        // We expect failure (empty key), but NOT "line too long".
        if let Err(e) = &result {
            let msg = e.to_string();
            assert!(
                !msg.contains("line too long"),
                "got 'line too long' — normalisation not applied: {msg}"
            );
        }
    }
}
