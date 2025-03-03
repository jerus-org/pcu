use clap::Parser;
use color_eyre::Result;

use super::{CIExit, Commands};

/// Configuration for the Bsky command
#[derive(Debug, Parser, Clone)]
pub struct Bsky {}

impl Bsky {
    pub async fn run(&self) -> Result<CIExit> {
        // TODO: Identify blogs that have changed
        // TODO: For each blog, extract the title, description, and tags
        // TODO: For each blog, create a Bluesky post
        let _client = Commands::Bsky(self.clone()).get_client().await?;

        Ok(CIExit::PostedToBluesky)
    }
}
