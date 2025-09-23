use clap::Parser;

use super::{CIExit, Commands, GitOps};
use crate::Error;

/// Configuration for the Rebase command
#[derive(Debug, Parser, Clone)]
pub struct Label {
    /// Override the default allowed authors (renovate, mend) when selecting the pull
    /// request to label
    #[arg(short, long)]
    pub author: Vec<String>,
    /// Override the default label (rebase) to add to the pull request
    #[arg(short, long)]
    pub label: Option<String>,
    /// Override the default description for the label if it is created
    #[arg(short, long = "description")]
    pub desc: Option<String>,
    /// Override the default colour (B22222) for the label if it is created
    #[arg(short, long, visible_alias = "color")]
    pub colour: Option<String>,
}

impl Label {
    pub fn author(&self) -> &Vec<String> {
        &self.author
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

    pub async fn run_label(&self) -> Result<CIExit, Error> {
        let client = Commands::Label(self.clone()).get_client().await?;

        let pr_number = client
            .label_next_pr(self.author(), self.label(), self.desc(), self.colour())
            .await?;

        if let Some(pr_number) = pr_number {
            Ok(CIExit::Label(pr_number))
        } else {
            Ok(CIExit::NoLabel)
        }
    }
}
