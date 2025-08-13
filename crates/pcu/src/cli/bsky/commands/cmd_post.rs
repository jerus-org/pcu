use std::{env, fmt::Display, path::Path};

use clap::Parser;
use config::Config;
use gen_bsky::{Post, PostError};

use crate::{cli::push::Push, CIExit, Client, Error, GitOps, Sign};

#[derive(Debug, Parser, Clone)]
pub struct CmdPost {
    /// Fail if the files to process are missing
    #[arg(short, long)]
    pub fail_on_missing: bool,
    /// Executing in release context so execute push even if requested by CI
    #[arg(short, long)]
    pub release: bool,
}

impl CmdPost {
    pub async fn run(&self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        let id = settings.get::<String>("bsky_id")?;
        let pw = settings.get::<String>("bsky_password")?;
        let store = settings.get::<String>("store")?;

        let deleted = match post_and_delete(&id, &pw, &store).await {
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
            log::info!("No bluesky posts found.");
            return Ok(CIExit::NoFilesToProcess);
        }

        // Commit to remove the posts successfully sent to Bluesky
        let sign = Sign::Gpg;
        let commit_message = format!(
            "chore: remove {} posts that were sent to Bluesky",
            if deleted == 1 {
                format!("{deleted} post")
            } else {
                format!("{deleted} posts")
            }
        );

        client
            .commit_changed_files(sign, &commit_message, "", None)
            .await?;

        if env::var("CI").is_ok() && !self.release {
            log::info!("Running in CI, skipping push to remote");
            return Ok(CIExit::DraftedForBluesky);
        }

        Push::new_with(None, false, "v".to_string())
            .run_push()
            .await?;

        Ok(CIExit::PostedToBluesky)
    }
}

async fn post_and_delete<P>(id: &str, pw: &str, store: P) -> Result<usize, PostError>
where
    P: AsRef<Path> + Display,
{
    let mut poster = Post::new(id, pw)?;
    let deleted = poster
        .load(store)?
        .post_to_bluesky()
        .await?
        .delete_posted_posts()?
        .count_deleted();
    Ok(deleted)
}
