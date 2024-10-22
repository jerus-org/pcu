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

#[derive(Debug, Parser, Clone)]
pub struct Pr {
    /// Signal an early exit as the changelog is already updated
    #[clap(short, long, default_value_t = false)]
    pub early_exit: bool,
    /// Prefix for the version tag
    #[clap(short, long, default_value_t = String::from("v"))]
    pub prefix: String,
}

#[derive(Debug, Parser, Clone)]
pub struct Release {
    /// Semantic version number for the release
    #[arg(short, long)]
    pub semver: String,
    /// Update the changelog by renaming the unreleased section with the version number
    #[arg(short, long, default_value_t = false)]
    pub update_changelog: bool,
    /// Prefix for the version tag
    #[clap(short, long, default_value_t = String::from("v"))]
    pub prefix: String,
    /// Process packages in the workspace s
    #[clap(short, long, default_value_t = false)]
    pub workspace: bool,
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
    /// Prefix for the version tag
    #[clap(short, long, default_value_t = String::from("v"))]
    pub prefix: String,
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
    /// Prefix for the version tag
    #[clap(short, long, default_value_t = String::from("v"))]
    pub prefix: String,
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
pub struct Label {
    /// Override the default author login (renovate) when selecting the pull request to label
    #[arg(short, long)]
    pub author: Option<String>,
    /// Override the default label (rebase) to add to the pull request
    #[arg(short, long)]
    pub label: Option<String>,
    /// Override the default description for the label if it is creaated
    #[arg(short, long = "description")]
    pub desc: Option<String>,
    /// Override the default colour (B22222) for the label if it is creaated
    #[arg(short, long, visible_alias = "color")]
    pub colour: Option<String>,
}

impl Label {
    pub fn author(&self) -> Option<&str> {
        if let Some(l) = &self.author {
            return Some(l);
        }
        None
    }

    pub fn label(&self) -> Option<&str> {
        if let Some(l) = &self.label {
            return Some(l);
        }
        None
    }

    pub fn desc(&self) -> Option<&str> {
        if let Some(d) = &self.desc {
            return Some(d);
        }
        None
    }

    pub fn colour(&self) -> Option<&str> {
        if let Some(c) = &self.colour {
            return Some(c);
        }
        None
    }
}

pub enum CIExit {
    Updated,
    UnChanged,
    Committed,
    Pushed(String),
    Released,
    Label(String),
    NoLabel,
}
