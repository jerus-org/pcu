use crate::Error;

use super::{CIExit, Commands, GitOps};

use clap::Parser;
use owo_colors::{OwoColorize, Style};

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

    pub async fn run_push(&self) -> Result<CIExit, Error> {
        let client = Commands::Push(self.clone()).get_client().await?;

        log::info!("Push the commit");
        log::trace!(
            "tag_opt: {:?} and no_push: {:?}",
            self.tag_opt(),
            self.no_push
        );

        client.push_commit(&self.prefix, self.tag_opt(), self.no_push)?;
        let hdr_style = Style::new().bold().underline();
        log::debug!("{}", "Check Push".style(hdr_style));
        log::debug!("Branch status: {}", client.branch_status()?);

        if !self.no_push {
            Ok(CIExit::Pushed(
                "Changed files committed and pushed to remote repository.".to_string(),
            ))
        } else {
            Ok(CIExit::Pushed(
                "Changed files committed and push dry run completed for logging.".to_string(),
            ))
        }
    }
}
