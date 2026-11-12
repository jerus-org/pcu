use std::path::Path;

use clap::Parser;
use color_eyre::Result;

use super::{CIExit, GitOps};
use crate::{write_ci_branch_export, write_ci_checkout_env, Client, Error};

/// Fetch and check out the head of a branch (local git only, no GitHub
/// credentials needed).
///
/// When `$BASH_ENV` is set, also exports `CIRCLE_BRANCH` and `CIRCLE_SHA1` for
/// the new head and unsets the pull request variables (`CIRCLE_PULL_REQUEST`,
/// `CIRCLE_PULL_REQUESTS`, `CI_PULL_REQUEST`, `CIRCLE_PR_NUMBER`,
/// `CIRCLE_PR_REPONAME`, `CIRCLE_PR_USERNAME`) so later steps describe the
/// branch head rather than the commit the pipeline started for.
#[derive(Debug, Parser, Clone)]
pub struct Checkout {
    /// Target branch to switch to
    #[clap(short, long)]
    pub branch: String,
}

impl Checkout {
    pub fn run(&self) -> Result<CIExit, Error> {
        self.run_at(Path::new("."), std::env::var("BASH_ENV").ok().as_deref())
    }

    /// Switch branch in the repository at `repo_path`, writing the CI
    /// environment to `bash_env` when given. Only local git operations are
    /// needed, so no GitHub credentials are required.
    fn run_at(&self, repo_path: &Path, bash_env: Option<&str>) -> Result<CIExit, Error> {
        let client = Client::new_local_at(repo_path)?;

        client.fetch_branch(&self.branch)?;
        client.checkout_branch(&self.branch)?;

        // Exporting is best effort: the branch is already switched, so a
        // failure here must not turn the command into an error.
        match bash_env {
            Some(path) => {
                let result = match client.git_repo.head().and_then(|h| h.peel_to_commit()) {
                    Ok(commit) => {
                        write_ci_checkout_env(&self.branch, &commit.id().to_string(), path)
                    }
                    Err(e) => {
                        log::warn!("Could not resolve HEAD, exporting branch only: {e}");
                        write_ci_branch_export(&self.branch, path)
                    }
                };
                if let Err(e) = result {
                    log::warn!("{e}");
                }
            }
            None => log::warn!("BASH_ENV is not set"),
        }

        Ok(CIExit::SwitchedBranch(self.branch.clone()))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use git2::{Repository, Signature};

    use super::Checkout;
    use crate::{CIExit, Cli};

    /// Checkout works with no GitHub credentials or CIRCLE_* variables: it
    /// only needs a git repository whose `origin` has the branch.
    #[test]
    fn test_checkout_runs_without_credentials_and_exports_head_sha() {
        let origin_dir = tempfile::tempdir().unwrap();
        let origin = Repository::init(origin_dir.path()).unwrap();
        let sig = Signature::now("t", "t@example.com").unwrap();
        let tree = origin
            .find_tree(origin.index().unwrap().write_tree().unwrap())
            .unwrap();
        let oid = origin
            .commit(Some("refs/heads/feature"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let work_dir = tempfile::tempdir().unwrap();
        let work = Repository::init(work_dir.path()).unwrap();
        work.remote("origin", origin_dir.path().to_str().unwrap())
            .unwrap();

        let env_file = work_dir.path().join("bash_env");
        let exit = Checkout {
            branch: "feature".to_string(),
        }
        .run_at(work_dir.path(), env_file.to_str())
        .unwrap();

        assert!(matches!(exit, CIExit::SwitchedBranch(b) if b == "feature"));
        let contents = std::fs::read_to_string(&env_file).unwrap();
        assert!(contents.contains("export CIRCLE_BRANCH=feature\n"));
        assert!(contents.contains(&format!("export CIRCLE_SHA1={oid}\n")));
        assert!(contents.contains("unset CIRCLE_PULL_REQUEST "));
    }

    #[test]
    fn test_checkout_parses_branch_long_flag() {
        let args = Cli::try_parse_from(["pcu", "checkout", "--branch", "main"]).unwrap();
        match args.command {
            crate::Commands::Checkout(c) => assert_eq!(c.branch, "main"),
            _ => panic!("expected Checkout command"),
        }
    }

    #[test]
    fn test_checkout_parses_branch_short_flag() {
        let args = Cli::try_parse_from(["pcu", "checkout", "-b", "main"]).unwrap();
        match args.command {
            crate::Commands::Checkout(c) => assert_eq!(c.branch, "main"),
            _ => panic!("expected Checkout command"),
        }
    }

    #[test]
    fn test_checkout_fast_forwards_existing_branch_and_tolerates_no_bash_env() {
        let origin_dir = tempfile::tempdir().unwrap();
        let origin = Repository::init(origin_dir.path()).unwrap();
        let sig = Signature::now("t", "t@example.com").unwrap();
        let tree = origin
            .find_tree(origin.index().unwrap().write_tree().unwrap())
            .unwrap();
        let first = origin
            .commit(Some("refs/heads/feature"), &sig, &sig, "one", &tree, &[])
            .unwrap();

        let work_dir = tempfile::tempdir().unwrap();
        let work = Repository::init(work_dir.path()).unwrap();
        work.remote("origin", origin_dir.path().to_str().unwrap())
            .unwrap();
        let cmd = Checkout {
            branch: "feature".to_string(),
        };

        // No BASH_ENV: still succeeds.
        cmd.run_at(work_dir.path(), None).unwrap();

        // Origin advances; the existing local branch is fast-forwarded.
        let parent = origin.find_commit(first).unwrap();
        let second = origin
            .commit(
                Some("refs/heads/feature"),
                &sig,
                &sig,
                "two",
                &tree,
                &[&parent],
            )
            .unwrap();
        let env_file = work_dir.path().join("bash_env");
        cmd.run_at(work_dir.path(), env_file.to_str()).unwrap();

        let contents = std::fs::read_to_string(&env_file).unwrap();
        assert!(contents.contains(&format!("export CIRCLE_SHA1={second}\n")));
    }
}
