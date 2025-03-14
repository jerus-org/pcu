use crate::{Error, Sign};

use clap::Parser;

use super::{CIExit, Commands, GitOps};

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

    pub async fn run_commit(&self, sign: Sign) -> Result<CIExit, Error> {
        let client = Commands::Commit(self.clone()).get_client().await?;

        client
            .commit_changed_files(sign, self.commit_message(), &self.prefix, self.tag_opt())
            .await?;

        Ok(CIExit::Committed)
    }
}
