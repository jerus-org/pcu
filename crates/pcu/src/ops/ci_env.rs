use std::{env, fs, io::Write};

use color_eyre::Result;

use crate::Error;

const BASH_ENV_VAR: &str = "BASH_ENV";
const CIRCLE_BRANCH_VAR: &str = "CIRCLE_BRANCH";
const CIRCLE_SHA1_VAR: &str = "CIRCLE_SHA1";

/// Pull-request variables that describe the commit the pipeline started for.
/// After switching branch they no longer apply, so they are unset.
const PR_VARS: [&str; 6] = [
    "CIRCLE_PULL_REQUEST",
    "CIRCLE_PULL_REQUESTS",
    "CI_PULL_REQUEST",
    "CIRCLE_PR_NUMBER",
    "CIRCLE_PR_REPONAME",
    "CIRCLE_PR_USERNAME",
];

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
/// Returns `Err` when `BASH_ENV` is not set, allowing the caller to decide
/// how to handle the missing environment (e.g. warn and continue or fail).
pub fn export_ci_branch(branch: &str) -> Result<(), Error> {
    let path = env::var(BASH_ENV_VAR)
        .map_err(|_| Error::GitError(format!("{BASH_ENV_VAR} is not set")))?;
    write_ci_branch_export(branch, &path)
}

/// Append the environment describing a checked-out branch head to `path`:
/// `CIRCLE_BRANCH`, `CIRCLE_SHA1` and an `unset` of the pull-request variables.
pub fn write_ci_checkout_env(branch: &str, sha: &str, path: &str) -> Result<(), Error> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "export {CIRCLE_BRANCH_VAR}={branch}")?;
    writeln!(file, "export {CIRCLE_SHA1_VAR}={sha}")?;
    writeln!(file, "unset {}", PR_VARS.join(" "))?;
    log::info!("Exported {CIRCLE_BRANCH_VAR}={branch}, {CIRCLE_SHA1_VAR}={sha} and cleared pull request variables in {path}");
    Ok(())
}

/// Like [`write_ci_checkout_env`], writing to the file named by `$BASH_ENV`.
///
/// Returns `Err` when `BASH_ENV` is not set.
pub fn export_ci_checkout(branch: &str, sha: &str) -> Result<(), Error> {
    let path = env::var(BASH_ENV_VAR)
        .map_err(|_| Error::GitError(format!("{BASH_ENV_VAR} is not set")))?;
    write_ci_checkout_env(branch, sha, &path)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek};

    use tempfile::NamedTempFile;

    use super::*;

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
    fn test_export_ci_branch_no_bash_env_returns_err() {
        let saved = env::var(BASH_ENV_VAR).ok();
        env::remove_var(BASH_ENV_VAR);

        let result = export_ci_branch("main");

        if let Some(v) = saved {
            env::set_var(BASH_ENV_VAR, v);
        }

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("BASH_ENV is not set"));
    }

    #[test]
    fn test_write_ci_checkout_env_exports_sha_and_unsets_pr_vars() {
        let mut tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        write_ci_checkout_env("main", "abc123", &path).unwrap();

        let mut contents = String::new();
        tmp.read_to_string(&mut contents).unwrap();
        assert_eq!(
            contents,
            "export CIRCLE_BRANCH=main\nexport CIRCLE_SHA1=abc123\n\
             unset CIRCLE_PULL_REQUEST CIRCLE_PULL_REQUESTS CI_PULL_REQUEST \
             CIRCLE_PR_NUMBER CIRCLE_PR_REPONAME CIRCLE_PR_USERNAME\n"
        );
    }

    #[test]
    fn test_write_ci_checkout_env_appends() {
        let mut tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        writeln!(tmp, "export SOME_VAR=value").unwrap();

        write_ci_checkout_env("main", "abc123", &path).unwrap();

        let mut contents = String::new();
        tmp.rewind().unwrap();
        tmp.read_to_string(&mut contents).unwrap();
        assert!(contents.starts_with("export SOME_VAR=value\nexport CIRCLE_BRANCH=main\n"));
    }

    #[test]
    fn test_export_ci_checkout_no_bash_env_returns_err() {
        let saved = env::var(BASH_ENV_VAR).ok();
        env::remove_var(BASH_ENV_VAR);

        let result = export_ci_checkout("main", "abc123");

        if let Some(v) = saved {
            env::set_var(BASH_ENV_VAR, v);
        }

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("BASH_ENV is not set"));
    }
}
