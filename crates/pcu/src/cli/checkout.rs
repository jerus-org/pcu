use clap::Parser;
use color_eyre::Result;

use super::{CIExit, Commands, GitOps};
use crate::{export_ci_branch, Error};

#[derive(Debug, Parser, Clone)]
pub struct Checkout {
    /// Target branch to switch to
    #[clap(short, long)]
    pub branch: String,
}

impl Checkout {
    pub async fn run(&self) -> Result<CIExit, Error> {
        let client = Commands::Checkout(self.clone()).get_client().await?;

        client.fetch_branch(&self.branch)?;
        client.checkout_branch(&self.branch)?;

        if let Err(e) = export_ci_branch(&self.branch) {
            log::warn!("{e}");
        }

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
