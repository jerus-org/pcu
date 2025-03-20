mod poster;

use clap::Parser;
use config::Config;
use poster::Poster;

use crate::{CIExit, Client, Error, GitOps, Sign};

#[derive(Debug, Parser, Clone)]
pub struct CmdPost;

impl CmdPost {
    pub async fn run(&self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        let id = settings.get::<String>("bsky_id")?;
        let pw = settings.get::<String>("bsky_password")?;
        let store = settings.get::<String>("store")?;
        Poster::new()?
            .load(store)?
            .post_to_bluesky(id, pw)
            .await?;

        // Commit to remove the posts successfully sent to Bluesky
        let sign = Sign::Gpg;
        let commit_message = "chore: remove posts that were sent to Bluesky";
        client
            .commit_changed_files(sign, commit_message, "", None)
            .await?;

        Ok(CIExit::PostedToBluesky)
    }
}
