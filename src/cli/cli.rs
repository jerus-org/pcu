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

#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    PullRequest(PullRequest),
    Release(Release),
    Push(Push),
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::PullRequest(_) => write!(f, "pull-request"),
            Commands::Release(_) => write!(f, "release"),
            Commands::Push(_) => write!(f, "push"),
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

#[derive(Debug, Parser, Clone)]
pub struct Push {}

pub enum ClState {
    Updated,
    UnChanged,
    Pushed,
    Released,
}
