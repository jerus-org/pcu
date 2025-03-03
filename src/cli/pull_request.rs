use std::env;

use crate::{
    cli::{commit_changed_files, Commands},
    Sign, UpdateFromPr,
};

use super::CIExit;

use clap::Parser;
use color_eyre::Result;
use keep_a_changelog::ChangeKind;

const SIGNAL_HALT: &str = "halt";

#[derive(Debug, Parser, Clone)]
pub struct Pr {
    /// Signal an early exit as the changelog is already updated
    #[clap(short, long, default_value_t = false)]
    pub early_exit: bool,
    /// Prefix for the version tag
    #[clap(short, long, default_value_t = String::from("v"))]
    pub prefix: String,
    /// Allow git push to fail. Allows the case of two parallel updates where the second push would fail.
    #[clap(short, long, default_value_t = false)]
    pub allow_push_fail: bool,
}

impl Pr {
    pub async fn run_pull_request(&self, sign: Sign) -> Result<CIExit> {
        let branch = env::var("CIRCLE_BRANCH");
        let branch = branch.unwrap_or("main".to_string());
        log::trace!("Branch: {branch:?}");

        if branch == "main" {
            log::info!("On the default branch, nothing to do here!");
            if self.early_exit {
                println!("{SIGNAL_HALT}");
            }

            return Ok(CIExit::UnChanged);
        }

        log::trace!("*** Get Client ***");
        let mut client = crate::cli::get_client(Commands::Pr(self.clone())).await?;

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

        log::trace!("Full client: {:#?}", client);
        let title = client.title();

        log::debug!("Pull Request Title: {title}");

        client.create_entry()?;

        log::debug!("Proposed entry: {:?}", client.entry());

        if log::log_enabled!(log::Level::Info) {
            if let Some((section, entry)) = client.update_changelog()? {
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
        } else if client.update_changelog()?.is_none() {
            return Ok(CIExit::UnChanged);
        }

        log::debug!("Changelog file name: {}", client.changelog_as_str());

        log::trace!(
            "{}",
            crate::cli::print_changelog(client.changelog_as_str(), client.line_limit())
        );

        let commit_message = "chore: update changelog for pr";

        commit_changed_files(&client, sign, commit_message, &self.prefix, None).await?;

        let res = crate::cli::push_committed(&client, &self.prefix, None, false).await;
        match res {
            Ok(()) => Ok(CIExit::Updated),
            Err(e) => {
                if self.allow_push_fail
                    && e.to_string()
                        .contains("cannot push non-fastforwardable reference")
                {
                    log::info!("Cannot psh non-fastforwardable reference, presuming change made already in parallel job.");
                    Ok(CIExit::UnChanged)
                } else {
                    Err(e)
                }
            }
        }
    }
}
