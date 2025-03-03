mod commit;
mod label;
mod pull_request;
mod push;
mod release;

use commit::Commit;
use label::Label;
use pull_request::Pr;
use push::Push;
use release::Release;

use std::{env, fmt::Display, fs};

use clap::{Parser, Subcommand};
use color_eyre::Result;
use config::Config;
use owo_colors::{OwoColorize, Style};

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
    /// Update the changelog from a pull request
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
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::Pr(_) => write!(f, "prequest"),
            Commands::Release(_) => write!(f, "release"),
            Commands::Commit(_) => write!(f, "commit"),
            Commands::Push(_) => write!(f, "push"),
            Commands::Label(_) => write!(f, "label"),
        }
    }
}

impl Commands {
    async fn get_client(&self) -> Result<Client, Error> {
        let settings = self.get_settings()?;
        let client = Client::new_with(settings).await?;

        Ok(client)
    }

    fn get_settings(&self) -> Result<Config, Error> {
        let mut settings = Config::builder()
            // Set defaults for CircleCI
            .set_default("log", "CHANGELOG.md")?
            .set_default("branch", "CIRCLE_BRANCH")?
            .set_default("default_branch", "main")?
            .set_default("pull_request", "CIRCLE_PULL_REQUEST")?
            .set_default("username", "CIRCLE_PROJECT_USERNAME")?
            .set_default("reponame", "CIRCLE_PROJECT_REPONAME")?
            .set_default("commit_message", "chore: update changelog")?
            .set_default("dev_platform", "https://github.com/")?
            .set_default("version_prefix", "v")?
            // Add in settings from pcu.toml if it exists
            .add_source(config::File::with_name("pcu.toml").required(false))
            // Add in settings from the environment (with a prefix of PCU)
            .add_source(config::Environment::with_prefix("PCU"));

        settings = match self {
            Commands::Pr(_) => settings
                .set_override("commit_message", "chore: update changelog for pr")?
                .set_override("command", "pr")?,
            Commands::Release(_) => settings
                .set_override("commit_message", "chore: update changelog for release")?
                .set_override("command", "release")?,
            Commands::Commit(_) => settings
                .set_override("commit_message", "chore: adding changed files")?
                .set_override("command", "commit")?,
            Commands::Push(_) => settings
                .set_override("commit_message", "chore: update changelog for release")?
                .set_override("command", "push")?,
            Commands::Label(_) => settings
                .set_override("commit_message", "chore: update changelog for release")?
                .set_override("command", "label")?,
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

fn print_changelog(changelog_path: &str, mut line_limit: usize) -> String {
    let mut output = String::new();

    if let Ok(change_log) = fs::read_to_string(changelog_path) {
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

async fn commit_changed_files(
    client: &Client,
    sign: Sign,
    commit_message: &str,
    prefix: &str,
    tag_opt: Option<&str>,
) -> Result<()> {
    let hdr_style = Style::new().bold().underline();
    log::debug!("{}", "Check WorkDir".style(hdr_style));

    let files_in_workdir = client.repo_files_not_staged()?;

    log::debug!("WorkDir files:\n\t{:?}", files_in_workdir);
    log::debug!("Staged files:\n\t{:?}", client.repo_files_staged()?);
    log::debug!("Branch status: {}", client.branch_status()?);

    log::info!("Stage the changes for commit");

    client.stage_files(files_in_workdir)?;

    log::debug!("{}", "Check Staged".style(hdr_style));
    log::debug!("WorkDir files:\n\t{:?}", client.repo_files_not_staged()?);

    let files_staged_for_commit = client.repo_files_staged()?;

    log::debug!("Staged files:\n\t{:?}", files_staged_for_commit);
    log::debug!("Branch status: {}", client.branch_status()?);

    log::info!("Commit the staged changes");

    client.commit_staged(sign, commit_message, prefix, tag_opt)?;

    log::debug!("{}", "Check Committed".style(hdr_style));
    log::debug!("WorkDir files:\n\t{:?}", client.repo_files_not_staged()?);

    let files_staged_for_commit = client.repo_files_staged()?;

    log::debug!("Staged files:\n\t{:?}", files_staged_for_commit);
    log::debug!("Branch status: {}", client.branch_status()?);

    Ok(())
}

async fn push_committed(
    client: &Client,
    prefix: &str,
    tag_opt: Option<&str>,
    no_push: bool,
) -> Result<()> {
    log::info!("Push the commit");
    log::trace!("tag_opt: {tag_opt:?} and no_push: {no_push}");

    client.push_commit(prefix, tag_opt, no_push)?;
    let hdr_style = Style::new().bold().underline();
    log::debug!("{}", "Check Push".style(hdr_style));
    log::debug!("Branch status: {}", client.branch_status()?);

    Ok(())
}
