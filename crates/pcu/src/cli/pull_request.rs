use std::env;

use clap::Parser;
use keep_a_changelog::ChangeKind;
use owo_colors::{OwoColorize, Style};

use super::CIExit;
use crate::{
    cli::{Commands, GitOps},
    Client, Error, SignConfig, UpdateFromPr,
};

const SIGNAL_HALT: &str = "halt";

#[derive(Debug, Parser, Clone)]
pub struct Pr {
    /// Signal an early exit as the prlog is already updated
    #[clap(short, long, default_value_t = false)]
    pub early_exit: bool,
    /// Run on main branch from a merge commit (post-merge PR log update)
    #[clap(short = 'M', long, default_value_t = false)]
    pub from_merge: bool,
    /// Prefix for the version tag
    #[clap(short, long, default_value_t = String::from("v"))]
    pub prefix: String,
    /// Attempt to push the changes to the remote repository
    #[clap(short = 'u', long, default_value_t = false)]
    pub push: bool,
    /// Allow git push to fail. Allows the case of two parallel updates where
    /// the second push would fail.
    #[clap(long, default_value_t = true)]
    pub allow_push_fail: bool,
    /// Hide pull request failure. Exits with success status even if no pull
    /// request was found in CI environment.
    #[clap(long, default_value_t = true)]
    pub allow_no_pull_request: bool,
    /// Append `[skip ci]` to the PRLOG commit message so the push to the default
    /// branch does not trigger a redundant CI pipeline (e.g. the second
    /// validation run after a "pr merged" PRLOG update).
    #[clap(long, default_value_t = false)]
    pub skip_ci: bool,
}

impl Pr {
    pub async fn run_pull_request(&self, sign_config: SignConfig) -> Result<CIExit, Error> {
        let branch = self.get_current_branch();

        if self.should_exit_early(&branch)? {
            return Ok(CIExit::UnChanged);
        }

        if self.from_merge {
            log::info!("Running in from-merge mode on branch: {branch}");
        }

        let mut client = match self.get_or_create_client().await {
            Ok(client) => client,
            Err(Error::EnvVarPullRequestNotFound)
                if !self.from_merge && self.allow_no_pull_request =>
            {
                log::debug!("early exit allowed - no pull request found in CI environment");
                return Ok(CIExit::UnChanged);
            }
            Err(Error::InvalidMergeCommitMessage) if self.from_merge => {
                log::info!("No pull request associated with current commit - this may be a direct commit to main");
                return Ok(CIExit::UnChanged);
            }
            Err(e) => return Err(e),
        };

        self.log_client_info(&client);

        client.create_entry()?;
        log::debug!("Proposed entry: {:?}", client.entry());

        if !self.update_and_log_prlog(&mut client)? {
            return Ok(CIExit::UnChanged);
        }

        self.commit_and_push(client, sign_config).await
    }

    fn get_current_branch(&self) -> String {
        let branch = env::var("CIRCLE_BRANCH");
        let branch = branch.unwrap_or("main".to_string());
        log::trace!("Branch: {branch:?}");
        branch
    }

    fn should_exit_early(&self, branch: &str) -> Result<bool, Error> {
        if branch == "main" && !self.from_merge {
            log::info!("On the default branch, nothing to do here!");
            if self.early_exit {
                println!("{SIGNAL_HALT}");
            }
            return Ok(true);
        }
        Ok(false)
    }

    async fn get_or_create_client(&self) -> Result<Client, Error> {
        log::trace!("*** Get Client ***");
        let client_res = Commands::Pr(self.clone()).get_client().await;
        log::trace!("client_res: {client_res:?}");
        log::trace!("allow_no_pull_request: {}", self.allow_no_pull_request);

        match client_res {
            Ok(client) => Ok(client),
            Err(e) => self.handle_client_error(e),
        }
    }

    fn handle_client_error(&self, e: Error) -> Result<Client, Error> {
        match &e {
            Error::EnvVarPullRequestNotFound => {
                log::debug!("pull request not found in environment variable");
            }
            Error::InvalidMergeCommitMessage => {
                log::debug!("no pull request associated with current commit");
            }
            _ => {
                log::error!("Error getting client: {e}");
            }
        }
        Err(e)
    }

    fn log_client_info(&self, client: &Client) {
        log::info!(
            "On the `{}` branch, so time to get to work!",
            client.branch_or_main()
        );
        log::debug!(
            "PR ID: {} - Owner: {} - Repo: {}",
            client.pr_number(),
            client.owner(),
            client.repo()
        );
        log::trace!("Full client: {client:#?}");
        log::debug!("Pull Request Title: {}", client.title());
    }

    fn update_and_log_prlog(&self, client: &mut Client) -> Result<bool, Error> {
        if log::log_enabled!(log::Level::Info) {
            if let Some((section, entry)) = client.update_prlog()? {
                let section = self.section_to_string(section);
                log::info!("Amendment: In section `{section}`, adding `{entry}`");
            } else {
                log::info!("No update required");
                if self.early_exit {
                    println!("{SIGNAL_HALT}");
                }
                return Ok(false);
            }
        } else if client.update_prlog()?.is_none() {
            return Ok(false);
        }

        log::debug!("Changelog file name: {}", client.prlog_as_str());
        log::trace!(
            "{}",
            crate::cli::print_prlog(client.prlog_as_str(), client.line_limit())
        );

        Ok(true)
    }

    fn section_to_string(&self, section: ChangeKind) -> &'static str {
        match section {
            ChangeKind::Added => "Added",
            ChangeKind::Changed => "Changed",
            ChangeKind::Deprecated => "Deprecated",
            ChangeKind::Fixed => "Fixed",
            ChangeKind::Removed => "Removed",
            ChangeKind::Security => "Security",
        }
    }

    async fn commit_and_push(
        &self,
        client: Client,
        sign_config: SignConfig,
    ) -> Result<CIExit, Error> {
        // Append the CI-skip marker only when committing the PRLOG update to the
        // default branch (the post-merge update). On a PR branch we WANT
        // validation to run, so never skip CI there.
        let on_default_branch = client.branch_or_main() == client.default_branch.as_str();
        let commit_message =
            super::with_skip_ci(&client.commit_message, self.skip_ci, on_default_branch);
        client
            .commit_changed_files(sign_config, &commit_message, &self.prefix, None)
            .await?;

        if self.push {
            self.push_the_commit(client)?;
        }

        Ok(CIExit::Updated)
    }

    #[cfg(test)]
    fn for_test(skip_ci: bool) -> Self {
        Pr {
            early_exit: false,
            from_merge: false,
            prefix: "v".to_string(),
            push: false,
            allow_push_fail: true,
            allow_no_pull_request: true,
            skip_ci,
        }
    }

    fn push_the_commit(&self, client: Client) -> Result<(), Error> {
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("*** Push the commit ***");
        } else {
            log::info!("Push the commit");
        }
        log::trace!("tag_opt: None and no_push: false");

        let bot_user_name = std::env::var("BOT_USER_NAME").unwrap_or_else(|_| "bot".to_string());
        log::debug!("Using bot user name: {bot_user_name}");

        // Read the git-configured identity — this is what git itself will use for the push,
        // independent of what we believe is configured via environment variables.
        let git_identity = client
            .git_repo
            .config()
            .ok()
            .map(|cfg| {
                let name = cfg.get_string("user.name").unwrap_or_default();
                let email = cfg.get_string("user.email").unwrap_or_default();
                format!("{name} <{email}>")
            })
            .unwrap_or_else(|| "<unknown>".to_string());

        let res = client.push_commit(&self.prefix, None, false, &bot_user_name);

        // Propagate hard errors immediately (anything other than non-fast-forward,
        // which may be a race condition that fetch-and-check can diagnose).
        if let Err(e) = &res {
            if !e
                .to_string()
                .contains("cannot push non-fastforwardable reference")
            {
                return Err(Error::GitError(e.to_string()));
            }
        }

        // Fetch to get the true remote state, then check ahead/behind to distinguish:
        //   ahead=0           → push succeeded
        //   ahead>0, behind>0 → race condition (parallel job pushed first)
        //   ahead>0, behind=0 → genuine server rejection (silent or non-fast-forward)
        client.fetch_origin()?;

        let hdr_style = Style::new().bold().underline();
        log::debug!("{}", "Check Push".style(hdr_style));
        let branch_status = client.branch_status()?;
        log::debug!("Branch status after fetch: {branch_status}");

        let ahead = branch_status.ahead;
        let behind = branch_status.behind;

        if ahead == 0 {
            Ok(())
        } else if behind > 0 {
            // Race: a parallel job pushed first; branch has diverged.
            if self.allow_push_fail {
                log::info!(
                    "Race condition: branch is {ahead} ahead and {behind} behind — \
                     assuming parallel job succeeded."
                );
                Ok(())
            } else {
                Err(Error::GitError(format!(
                    "Push race: branch is {ahead} ahead and {behind} behind remote after fetch \
                     (push identity: {git_identity})"
                )))
            }
        } else {
            // ahead > 0, behind = 0: server rejected the push (silent or non-fast-forward).
            Err(Error::GitError(format!(
                "Push rejected by server: branch is still {ahead} commit(s) ahead after fetch \
                 (push identity: {git_identity}) — check branch protection rules or \
                 authentication, and review the GitHub audit log"
            )))
        }
    }
}

#[cfg(test)]
mod commit_message_tests {
    use super::*;
    use crate::{Client, Sign, SignConfig};
    use std::path::Path;

    /// Build a temp git repo with one commit, an `origin/<branch>` tracking ref
    /// (so `branch_status` works without a real remote — it looks up
    /// `origin/<client.branch>`), and an unstaged change ready to commit.
    fn repo_with_pending_change(dir: &Path, branch: &str) {
        let repo = git2::Repository::init(dir).unwrap();
        {
            let mut cfg = repo.config().unwrap();
            cfg.set_str("user.name", "Test User").unwrap();
            cfg.set_str("user.email", "test@example.com").unwrap();
        }
        std::fs::write(dir.join("file.txt"), "initial\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = repo.signature().unwrap();
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        repo.reference(&format!("refs/remotes/origin/{branch}"), oid, true, "track")
            .unwrap();
        // Pending working-tree change for commit_and_push to stage + commit.
        std::fs::write(dir.join("file.txt"), "changed\n").unwrap();
    }

    fn head_message(dir: &Path) -> String {
        let repo = git2::Repository::open(dir).unwrap();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        let message = commit.message().unwrap().trim().to_string();
        message
    }

    /// Run commit_and_push for the given branch + skip_ci and return the
    /// resulting HEAD commit message. `client.default_branch` is "main".
    async fn commit_message_for(branch: &str, skip_ci: bool) -> String {
        let tmp = tempfile::tempdir().unwrap();
        repo_with_pending_change(tmp.path(), branch);

        let mut client = Client::new_local_at(tmp.path()).unwrap();
        client.commit_message = "chore: update prlog for pr".to_string();
        client.branch = Some(branch.to_string());

        Pr::for_test(skip_ci)
            .commit_and_push(client, SignConfig::with_signoff(Sign::None, false))
            .await
            .expect("commit_and_push should succeed");

        head_message(tmp.path())
    }

    /// On the default branch with --skip-ci, the PRLOG commit gets `[skip ci]`
    /// (suppresses the redundant post-merge validation).
    #[tokio::test]
    async fn skip_ci_appends_marker_on_default_branch() {
        assert_eq!(
            commit_message_for("main", true).await,
            "chore: update prlog for pr [skip ci]"
        );
    }

    /// Regression guard: on a PR (non-default) branch, --skip-ci must NOT add
    /// `[skip ci]` — we want validation to run for the PR. `[skip ci]` was being
    /// added to the wrong commit before this gate.
    #[tokio::test]
    async fn skip_ci_omits_marker_on_pr_branch() {
        assert_eq!(
            commit_message_for("feat/some-change", true).await,
            "chore: update prlog for pr",
            "[skip ci] must not be added on a non-default (PR) branch"
        );
    }

    /// Without --skip-ci the message is plain on any branch.
    #[tokio::test]
    async fn no_skip_ci_plain_message_on_default_branch() {
        assert_eq!(
            commit_message_for("main", false).await,
            "chore: update prlog for pr"
        );
    }
}
