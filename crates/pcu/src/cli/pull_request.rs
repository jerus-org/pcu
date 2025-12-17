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
}

impl Pr {
    pub async fn run_pull_request(&self, sign_config: SignConfig) -> Result<CIExit, Error> {
        let branch = env::var("CIRCLE_BRANCH");
        let branch = branch.unwrap_or("main".to_string());
        log::trace!("Branch: {branch:?}");

        if branch == "main" && !self.from_merge {
            log::info!("On the default branch, nothing to do here!");
            if self.early_exit {
                println!("{SIGNAL_HALT}");
            }

            return Ok(CIExit::UnChanged);
        }

        if self.from_merge {
            log::info!("Running in from-merge mode on branch: {branch}");
        }

        log::trace!("*** Get Client ***");
        let client_res = Commands::Pr(self.clone()).get_client().await;
        log::trace!("client_res: {client_res:?}");
        log::trace!("allow_no_pull_request: {}", self.allow_no_pull_request);
        let mut client = match client_res {
            Ok(client) => client,
            Err(e) => {
                match e {
                    Error::EnvVarPullRequestNotFound => {
                        if self.allow_no_pull_request {
                            log::debug!("early exit allowed even though no pull request found in CI environment");
                            return Ok(CIExit::UnChanged);
                        } else {
                            log::debug!("pull request not found and not allowed");
                            return Err(e);
                        }
                    }
                    _ => {
                        log::error!("Error getting client: {e}");
                        return Err(e);
                    }
                };
            }
        };

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
        let title = client.title();

        log::debug!("Pull Request Title: {title}");

        client.create_entry()?;

        log::debug!("Proposed entry: {:?}", client.entry());

        if log::log_enabled!(log::Level::Info) {
            if let Some((section, entry)) = client.update_prlog()? {
                let section = match section {
                    ChangeKind::Added => "Added",
                    ChangeKind::Changed => "Changed",
                    ChangeKind::Deprecated => "Deprecated",
                    ChangeKind::Fixed => "Fixed",
                    ChangeKind::Removed => "Removed",
                    ChangeKind::Security => "Security",
                };
                log::info!("Amendment: In section `{section}`, adding `{entry}`");
            } else {
                log::info!("No update required");
                if self.early_exit {
                    println!("{SIGNAL_HALT}");
                }
                return Ok(CIExit::UnChanged);
            };
        } else if client.update_prlog()?.is_none() {
            return Ok(CIExit::UnChanged);
        }

        log::debug!("Changelog file name: {}", client.prlog_as_str());

        log::trace!(
            "{}",
            crate::cli::print_prlog(client.prlog_as_str(), client.line_limit())
        );

        // Commit the pull request log
        let commit_message = "chore: update prlog for pr";
        client
            .commit_changed_files(sign_config, commit_message, &self.prefix, None)
            .await?;

        if self.push {
            // Push the pull request log (and other commits)
            self.push_the_commit(client)?;
        };

        Ok(CIExit::Updated)
    }

    fn push_the_commit(&self, client: Client) -> Result<CIExit, Error> {
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("*** Push the commit ***");
        } else {
            log::info!("Push the commit");
        }
        log::trace!("tag_opt: None and no_push: false");

        let bot_user_name = std::env::var("BOT_USER_NAME").unwrap_or_else(|_| "bot".to_string());
        log::debug!("Using bot user name: {bot_user_name}");

        let res = client.push_commit(&self.prefix, None, false, &bot_user_name);
        let hdr_style = Style::new().bold().underline();
        log::debug!("{}", "Check Push".style(hdr_style));
        log::debug!("Branch status: {}", client.branch_status()?);

        match res {
            Ok(()) => Ok(CIExit::Updated),
            Err(e) => {
                if self.allow_push_fail
                    && e.to_string()
                        .contains("cannot push non-fastforwardable reference")
                {
                    log::info!(
                        "Cannot psh non-fastforwardable reference, presuming change made already in parallel job."
                    );
                    Ok(CIExit::UnChanged)
                } else {
                    Err(e)
                }
            }
        }
    }
}
