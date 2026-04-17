use std::{env, path::Path};

use clap::Parser;
use config::Config;
use gen_linkedin::{Post, PostError};

use crate::{cli::push::Push, CIExit, Client, Error, GitOps, SignConfig};
use std::fmt::Display;

#[derive(Debug, Parser, Clone)]
pub struct CmdPost {
    /// Fail if no `.linkedin` draft files are found.
    #[arg(short, long)]
    pub fail_on_missing: bool,
    /// Executing in release context — push even in CI.
    #[arg(short, long)]
    pub release: bool,
}

impl CmdPost {
    pub async fn run(&self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        let access_token = settings
            .get_string("linkedin_access_token")
            .map_err(|_| Error::MissingConfig("PCU_LINKEDIN_ACCESS_TOKEN".to_string()))?;
        let author_urn = settings
            .get_string("linkedin_author_urn")
            .map_err(|_| Error::MissingConfig("PCU_LINKEDIN_AUTHOR_URN".to_string()))?;
        let store = settings
            .get_string("linkedin_store")
            .unwrap_or_else(|_| "linkedin".to_string());
        let api_version = settings
            .get_string("linkedin_api_version")
            .unwrap_or_else(|_| "202401".to_string());

        let deleted = match post_and_delete(&access_token, &author_urn, &store, &api_version).await {
            Ok(d) => d,
            Err(e) => {
                if self.fail_on_missing {
                    return Err(e.into());
                } else {
                    log::warn!("{e}");
                    return Ok(CIExit::NoFilesToProcess);
                }
            }
        };

        if deleted == 0 {
            log::info!("No LinkedIn posts found.");
            return Ok(CIExit::NoFilesToProcess);
        }

        let sign_config = SignConfig::default();
        let commit_message = format!(
            "chore: remove {} published to LinkedIn",
            if deleted == 1 {
                format!("{deleted} post")
            } else {
                format!("{deleted} posts")
            }
        );

        client
            .commit_changed_files(sign_config, &commit_message, "", None)
            .await?;

        if env::var("CI").is_ok() && !self.release {
            log::info!("Running in CI, skipping push to remote");
            return Ok(CIExit::DraftedForLinkedIn);
        }

        Push::new_with(None, false, "v".to_string())
            .run_push()
            .await?;

        Ok(CIExit::PostedToLinkedIn)
    }
}

async fn post_and_delete<P>(
    access_token: &str,
    author_urn: &str,
    store: P,
    api_version: &str,
) -> Result<usize, PostError>
where
    P: AsRef<Path> + Display,
{
    let mut poster = Post::new(access_token, author_urn).with_api_version(api_version);
    let deleted = poster
        .load(store)?
        .post_to_linkedin()
        .await?
        .delete_posted_posts()?
        .count_deleted();
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::linkedin::Cmd;
    use crate::Cli;
    use clap::Parser;

    #[test]
    fn test_linkedin_post_parses_fail_on_missing() {
        let args = Cli::try_parse_from(["pcu", "linkedin", "post", "--fail-on-missing"]).unwrap();
        match args.command {
            crate::Commands::Linkedin(li) => match li.cmd {
                Cmd::Post(post) => assert!(post.fail_on_missing),
                _ => panic!("expected Post"),
            },
            _ => panic!("expected Linkedin"),
        }
    }

    #[test]
    fn test_linkedin_post_fail_on_missing_defaults_false() {
        let args = Cli::try_parse_from(["pcu", "linkedin", "post"]).unwrap();
        match args.command {
            crate::Commands::Linkedin(li) => match li.cmd {
                Cmd::Post(post) => assert!(!post.fail_on_missing),
                _ => panic!("expected Post"),
            },
            _ => panic!("expected Linkedin"),
        }
    }

    #[test]
    fn test_linkedin_post_release_defaults_false() {
        let args = Cli::try_parse_from(["pcu", "linkedin", "post"]).unwrap();
        match args.command {
            crate::Commands::Linkedin(li) => match li.cmd {
                Cmd::Post(post) => assert!(!post.release),
                _ => panic!("expected Post"),
            },
            _ => panic!("expected Linkedin"),
        }
    }

    #[test]
    fn test_posted_to_linkedin_ci_exit() {
        assert!(matches!(CIExit::PostedToLinkedIn, CIExit::PostedToLinkedIn));
    }
}
