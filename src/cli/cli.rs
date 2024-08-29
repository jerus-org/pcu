use std::fmt::Display;

use clap::{Parser, Subcommand};
use pcu_lib::Sign;

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
    PullRequest(PullRequest),
    /// Create a release on GitHub
    Release(Release),
    /// Commit changed files in the working directory
    Commit(Commit),
    /// Push the current commits to the remote repository
    Push(Push),
    /// Rebase
    Rebase(Rebase),
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::PullRequest(_) => write!(f, "pull-request"),
            Commands::Release(_) => write!(f, "release"),
            Commands::Commit(_) => write!(f, "commit"),
            Commands::Push(_) => write!(f, "push"),
            Commands::Rebase(_) => write!(f, "rebase"),
        }
    }
}

#[derive(Debug, Parser, Clone)]
pub struct PullRequest {
    /// Signal an early exit as the changelog is already updated
    #[clap(short, long, default_value_t = false)]
    pub early_exit: bool,
}

#[derive(Debug, Parser, Clone)]
pub struct Release {
    /// Semantic version number for the release
    #[arg(short, long)]
    pub semver: String,
    /// Update the changelog by renaming the unreleased section with the version number
    #[arg(short, long, default_value_t = false)]
    pub update_changelog: bool,
}

/// Configuration for the Commit command
#[derive(Debug, Parser, Clone)]
pub struct Commit {
    /// Semantic version number for a tag
    #[arg(short, long)]
    pub semver: Option<String>,
    /// Message to add to the commit when pushing
    #[arg(short, long)]
    commit_message: String,
}

impl Commit {
    pub fn commit_message(&self) -> &str {
        &self.commit_message
    }

    pub fn tag_opt(&self) -> Option<&str> {
        if let Some(semver) = &self.semver {
            return Some(semver);
        }
        None
    }
}

/// Configuration for the Push command
#[derive(Debug, Parser, Clone)]
pub struct Push {
    /// Semantic version number for a tag
    #[arg(short, long)]
    pub semver: Option<String>,
    /// Disable the push command
    #[arg(short, long, default_value_t = false)]
    pub no_push: bool,
}

impl Push {
    pub fn tag_opt(&self) -> Option<&str> {
        if let Some(semver) = &self.semver {
            return Some(semver);
        }
        None
    }
}

/// Configuration for the Rebase command
#[derive(Debug, Parser, Clone)]
pub struct Rebase {
    /// Override the default login rebase author
    #[arg(short, long)]
    pub login: Option<String>,
}

impl Rebase {
    pub fn login(&self) -> Option<&str> {
        if let Some(login) = &self.login {
            return Some(login);
        }
        None
    }
}

pub enum ClState {
    Updated,
    UnChanged,
    Committed,
    Pushed(String),
    Released,
    Rebased(String),
    NoRebase,
}
