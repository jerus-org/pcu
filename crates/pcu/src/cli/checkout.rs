use std::env;

use clap::Parser;
use color_eyre::Result;

use super::{CIExit, Commands, GitOps};
use crate::{export_ci_branch, Error};

const BASH_ENV_VAR: &str = "BASH_ENV";

#[derive(Debug, Parser, Clone)]
pub struct Checkout {
    /// Target branch to switch to
    #[clap(short, long)]
    pub branch: String,
}

impl Checkout {
    pub async fn run_checkout(&self) -> Result<CIExit, Error> {
        let client = Commands::Checkout(self.clone()).get_client().await?;

        client.fetch_branch(&self.branch)?;
        client.checkout_branch(&self.branch)?;

        if env::var(BASH_ENV_VAR).is_ok() {
            export_ci_branch(&self.branch)?;
        } else {
            log::warn!("{BASH_ENV_VAR} not set; CIRCLE_BRANCH not updated in environment");
        }

        log::info!("Switched to branch: {}", self.branch);
        Ok(CIExit::SwitchedBranch(self.branch.clone()))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::Cli;

    // RED: tests written before the command is wired into the CLI.

    #[test]
    fn test_checkout_parses_branch_long_flag() {
        let args = Cli::try_parse_from(["pcu", "checkout", "--branch", "main"]).unwrap();
        match args.command {
            crate::Commands::Checkout(c) => assert_eq!(c.branch, "main"),
            _ => panic!("expected Checkout command"),
        }
    }

    #[test]
    fn test_checkout_parses_branch_short_flag() {
        let args = Cli::try_parse_from(["pcu", "checkout", "-b", "main"]).unwrap();
        match args.command {
            crate::Commands::Checkout(c) => assert_eq!(c.branch, "main"),
            _ => panic!("expected Checkout command"),
        }
    }
}
