mod poster;

use clap::Parser;
use config::Config;
use poster::Poster;

use crate::{CIExit, Error};

use super::super::BSKY_POSTS_DIR;

#[derive(Debug, Parser, Clone)]
pub struct CmdPost;

impl CmdPost {
    pub async fn run(&self, settings: &Config) -> Result<CIExit, Error> {
        let id = settings.get::<String>("bsky_id")?;
        let pw = settings.get::<String>("bsky_password")?;
        Poster::new()?
            .load(BSKY_POSTS_DIR)?
            .post_to_bluesky(id, pw)
            .await?;
        Ok(CIExit::PostedToBluesky)
    }
}
