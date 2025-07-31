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
    pub fn new_with(semver: Option<String>, no_push: bool, mut prefix: String) -> Self {
        if prefix.is_empty() {
            prefix = "v".to_string();
        }
        Self {
            semver,
            no_push,
            prefix,
        }
    }

    pub fn tag_opt(&self) -> Option<&str> {
        if let Some(semver) = &self.semver {
            return Some(semver);
        }
        None
    }

    pub async fn run_push(&self) -> Result<CIExit, Error> {
        let client = Commands::Push(self.clone()).get_client().await?;

        let branch_status = client.branch_status()?;
        log::debug!("Branch status report: {branch_status}");

        if branch_status.ahead == 0 {
            return Ok(CIExit::NothingToPush);
        };

        log::info!("Push the commit");
        log::trace!(
            "tag_opt: {:?} and no_push: {:?}",
            self.tag_opt(),
            self.no_push
        );

        let bot_user_name = std::env::var("BOT_USER_NAME").unwrap_or_else(|_| "bot".to_string());
        log::debug!("Using bot user name: {bot_user_name}");

        client.push_commit(&self.prefix, self.tag_opt(), self.no_push, &bot_user_name)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_empty_prefix() {
        let push = Push::new_with(Some("1.0.0".to_string()), false, "".to_string());
        assert_eq!(push.prefix, "v");
        assert_eq!(push.semver, Some("1.0.0".to_string()));
        assert!(!push.no_push);
    }

    #[test]
    fn test_new_with_custom_prefix() {
        let push = Push::new_with(None, true, "ver-".to_string());
        assert_eq!(push.prefix, "ver-");
        assert_eq!(push.semver, None);
        assert!(push.no_push);
    }

    #[test]
    fn test_tag_opt() {
        let push = Push::new_with(Some("2.0.0".to_string()), false, "v".to_string());
        assert_eq!(push.tag_opt(), Some("2.0.0"));

        let push_no_semver = Push::new_with(None, false, "v".to_string());
        assert_eq!(push_no_semver.tag_opt(), None);
    }
}
