use clap::Parser;
use color_eyre::Result;

use super::{CIExit, Commands};

/// Configuration for the Bsky command
#[derive(Debug, Parser, Clone)]
pub struct Bsky {}

impl Bsky {
    pub async fn run(&self) -> Result<CIExit> {
        let client = Commands::Bsky(self.clone()).get_client().await?;

        let release = client
            .github_rest
            .repos
            .get_latest_release(client.owner(), client.repo())
            .send()
            .await?;

        let compare = client
            .github_rest
            .repos
            .compare_commits(client.owner(), client.repo(), release.tag_name)
            .send()
            .await?;

        if let Some(files) = compare.files {
            log::info!("Files: {files:?}");
            let changed_files = files.iter().map(|f| f.filename.clone()).collect::<Vec<_>>();
            log::info!("Changed files: {changed_files:#?}");
        }

        // TODO: Get the list of blogs from the config

        // TODO: Identify blogs that have changed
        // TODO: For each blog, extract the title, description, and tags
        // TODO: For each blog, create a Bluesky post
        let _client = Commands::Bsky(self.clone()).get_client().await?;

        Ok(CIExit::PostedToBluesky)
    }
}
