mod poster;

use clap::Parser;
use config::Config;
use poster::Poster;

use crate::{CIExit, Client, Error, GitOps, Sign};

#[derive(Debug, Parser, Clone)]
pub struct CmdPost {
    /// Fail if the files to process are missing
    #[arg(short, long)]
    pub fail_on_missing: bool,
}

impl CmdPost {
    pub async fn run(&self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        let id = settings.get::<String>("bsky_id")?;
        let pw = settings.get::<String>("bsky_password")?;
        let store = settings.get::<String>("store")?;
        let mut poster = Poster::new()?;
        match poster.load(store) {
            Ok(_) => {}
            Err(e) => {
                if self.fail_on_missing {
                    return Err(e);
                } else {
                    log::warn!("{e}");
                    return Ok(CIExit::NoFilesToProcess);
                }
            }
        };
        poster.post_to_bluesky(id, pw).await?;

        // Commit to remove the posts successfully sent to Bluesky
        let sign = Sign::Gpg;
        let commit_message = "chore: remove posts that were sent to Bluesky";
        client
            .commit_changed_files(sign, commit_message, "", None)
            .await?;

        Ok(CIExit::PostedToBluesky)
    }
}
