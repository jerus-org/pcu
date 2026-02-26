use std::{env, fs, io::Write};

use color_eyre::Result;

use crate::Error;

const BASH_ENV_VAR: &str = "BASH_ENV";
const CIRCLE_BRANCH_VAR: &str = "CIRCLE_BRANCH";

/// Write `export CIRCLE_BRANCH=<branch>` to the given file path (appending).
///
/// Separated from env-var lookup so callers can supply the path directly,
/// keeping tests free of global env-var mutation.
pub fn write_ci_branch_export(branch: &str, path: &str) -> Result<(), Error> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "export {CIRCLE_BRANCH_VAR}={branch}")?;
    log::info!("Exported {CIRCLE_BRANCH_VAR}={branch} to {path}");
    Ok(())
}

/// Write `export CIRCLE_BRANCH=<branch>` to the file named by `$BASH_ENV`.
///
/// Silently succeeds (with a warning log) when `BASH_ENV` is not set.
pub fn export_ci_branch(branch: &str) -> Result<(), Error> {
    match env::var(BASH_ENV_VAR) {
        Ok(path) => write_ci_branch_export(branch, &path),
        Err(_) => {
            log::warn!("{BASH_ENV_VAR} not set; {CIRCLE_BRANCH_VAR} not updated in environment");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek};
    use tempfile::NamedTempFile;

    // RED: written before the function is exported from the library.

    #[test]
    fn test_write_ci_branch_export_creates_export_line() {
        let mut tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        write_ci_branch_export("main", &path).unwrap();

        let mut contents = String::new();
        tmp.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "export CIRCLE_BRANCH=main\n");
    }

    #[test]
    fn test_write_ci_branch_export_appends_to_existing_content() {
        let mut tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        writeln!(tmp, "export SOME_VAR=value").unwrap();

        write_ci_branch_export("main", &path).unwrap();

        let mut contents = String::new();
        tmp.rewind().unwrap();
        tmp.read_to_string(&mut contents).unwrap();
        assert_eq!(
            contents,
            "export SOME_VAR=value\nexport CIRCLE_BRANCH=main\n"
        );
    }

    #[test]
    fn test_write_ci_branch_export_non_default_branch() {
        let mut tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        write_ci_branch_export("release/1.0", &path).unwrap();

        let mut contents = String::new();
        tmp.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "export CIRCLE_BRANCH=release/1.0\n");
    }

    #[test]
    fn test_export_ci_branch_no_bash_env_succeeds_silently() {
        // Temporarily ensure BASH_ENV is not set for this test.
        // We use a unique var name to detect it; if set, skip gracefully.
        let saved = env::var(BASH_ENV_VAR).ok();
        env::remove_var(BASH_ENV_VAR);

        let result = export_ci_branch("main");

        if let Some(v) = saved {
            env::set_var(BASH_ENV_VAR, v);
        }

        assert!(result.is_ok());
    }
}
