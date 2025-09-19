mod bsky;
mod commit;
mod label;
mod pull_request;
mod push;
mod release;

use std::{env, fmt::Display, fs};

use bsky::Bsky;
use clap::{Parser, Subcommand};
use color_eyre::Result;
use commit::Commit;
use config::Config;
use label::Label;
use pull_request::Pr;
use push::Push;
use release::Release;

use crate::{Client, Error, GitOps, Sign};

const GITHUB_PAT: &str = "GITHUB_TOKEN";

pub enum CIExit {
    Updated,
    UnChanged,
    Committed,
    Pushed(String),
    Released,
    Label(String),
    NoLabel,
    DraftedForBluesky,
    PostedToBluesky,
    NoFilesToProcess,
    NothingToPush,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(flatten)]
    pub logging: clap_verbosity_flag::Verbosity,
    #[clap(short, long)]
    /// Require the user to sign the update commit with their GPG key
    pub sign: Option<Sign>,
    /// Command to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Commands for the CLI
#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    /// Update the prlog from a pull request
    Pr(Pr),
    /// Create a release on GitHub
    Release(Release),
    /// Commit changed files in the working directory
    Commit(Commit),
    /// Push the current commits to the remote repository
    Push(Push),
    /// Apply a label to a pull request.
    #[clap(long_about = "
Apply a label to a pull request.
In default use applies the `rebase` label to the pull request with 
the lowest number submitted by the `renovate` user")]
    Label(Label),
    /// Post summaries and link to new or changed blog posts to bluesky
    Bsky(Bsky),
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::Pr(_) => write!(f, "prequest"),
            Commands::Release(_) => write!(f, "release"),
            Commands::Commit(_) => write!(f, "commit"),
            Commands::Push(_) => write!(f, "push"),
            Commands::Label(_) => write!(f, "label"),
            Commands::Bsky(_) => write!(f, "bluesky"),
        }
    }
}

impl Commands {
    async fn get_client(&self) -> Result<Client, Error> {
        let settings = self.get_settings()?;
        let client = Client::new_with(&settings).await?;

        Ok(client)
    }

    fn get_settings(&self) -> Result<Config, Error> {
        let mut settings = Config::builder()
            // Set defaults for CircleCI
            .set_default("prlog", "PRLOG.md")?
            .set_default("branch", "CIRCLE_BRANCH")?
            .set_default("default_branch", "main")?
            .set_default("pull_request", "CIRCLE_PULL_REQUEST")?
            .set_default("username", "CIRCLE_PROJECT_USERNAME")?
            .set_default("reponame", "CIRCLE_PROJECT_REPONAME")?
            .set_default("commit_message", "chore: update prlog")?
            .set_default("dev_platform", "https://github.com/")?
            .set_default("version_prefix", "v")?
            // Add in settings from pcu.toml if it exists
            .add_source(config::File::with_name("pcu.toml").required(false))
            // Add in settings from the environment (with a prefix of PCU)
            .add_source(config::Environment::with_prefix("PCU"));

        log::trace!("Initial settings (default, pcu.toml and environment: {settings:#?}");

        settings = match self {
            Commands::Pr(_) => settings
                .set_override("commit_message", "chore: update prlog for pr")?
                .set_override("command", "pr")?,
            Commands::Release(_) => settings
                .set_override("commit_message", "chore: update prlog for release")?
                .set_override("command", "release")?,
            Commands::Commit(_) => settings
                .set_override("commit_message", "chore: adding changed files")?
                .set_override("command", "commit")?,
            Commands::Push(_) => settings
                .set_override("commit_message", "chore: update prlog for release")?
                .set_override("command", "push")?,
            Commands::Label(_) => settings
                .set_override("commit_message", "chore: update prlog for release")?
                .set_override("command", "label")?,
            Commands::Bsky(bsky) => settings
                .set_override("commit_message", "chore: add Bluesky posts to repository")?
                .set_override("store", bsky.store.clone())?
                .set_override("command", "bsky")?,
        };

        settings = if let Commands::Bsky(bsky) = self {
            if let Some(_owner) = &bsky.owner {
                settings.set_override("username", "OWNER")?
            } else {
                settings
            }
        } else {
            settings
        };

        settings = if let Commands::Bsky(bsky) = self {
            if let Some(_repo) = &bsky.repo {
                settings.set_override("reponame", "REPO")?
            } else {
                settings
            }
        } else {
            settings
        };

        settings = if let Commands::Bsky(bsky) = self {
            if let Some(_branch) = &bsky.branch {
                settings.set_override("branch", "BRANCH")?
            } else {
                settings
            }
        } else {
            settings
        };

        settings = if let Ok(pat) = env::var(GITHUB_PAT) {
            settings.set_override("pat", pat.to_string())?
        } else {
            settings
        };

        match settings.build() {
            Ok(settings) => Ok(settings),
            Err(e) => {
                log::error!("Error: {e}");
                Err(e.into())
            }
        }
    }
}

fn print_prlog(prlog_path: &str, mut line_limit: usize) -> String {
    let mut output = String::new();

    if let Ok(change_log) = fs::read_to_string(prlog_path) {
        let mut line_count = 0;
        if line_limit == 0 {
            line_limit = change_log.lines().count();
        };

        output.push_str("\n*****Changelog*****:\n----------------------------");
        for line in change_log.lines() {
            output.push_str(format!("{line}\n").as_str());
            line_count += 1;
            if line_count >= line_limit {
                break;
            }
        }
        output.push_str("----------------------------\n");
    };

    output
}
