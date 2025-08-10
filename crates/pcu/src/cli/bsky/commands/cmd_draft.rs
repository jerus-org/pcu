mod site_config;

use std::path::PathBuf;

use clap::Parser;
use config::Config;
use gen_bsky::Draft;
use site_config::SiteConfig;

use crate::{CIExit, Client, Error, GitOps, Sign};

const DEFAULT_PATH: &str = "content/blog";

#[derive(Debug, Parser, Clone)]
pub struct CmdDraft {
    /// filter for files containing blog posts to broadcast on Bluesky
    #[arg(short, long)]
    pub filter: Option<String>,
    /// Optional path to file or directory of blog post(s) to process
    pub paths: Vec<PathBuf>,
    /// Optional date from which to process blog post(s)
    /// Date format: YYYY-MM-DD
    /// Default: Current date
    #[arg(short, long)]
    pub date: Option<toml::value::Datetime>,
    /// Allow bluesky posts for draft blog posts
    #[arg(long, default_value_t = false)]
    pub allow_draft: bool,
}

impl CmdDraft {
    pub async fn run(&mut self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        // find the potential file in the git repo

        let base_url = SiteConfig::new()?.base_url();
        let store = &settings.get_string("store")?;
        if !self.paths.is_empty() {
            self.paths.push(PathBuf::from(DEFAULT_PATH))
        };

        log::trace!(
            "Key parameters:\n\tBase:\t`{base_url}`\n\tstore:\t`{store}`\n\tpath:\t`{:#?}`",
            self.paths
        );

        let mut builder = Draft::builder(base_url);

        // Add the paths specified at the command line.
        for path in self.paths.iter() {
            builder.add_path_or_file(path)?;
        }

        // Set the filters to qualify the blog posts
        builder
            .with_minimum_date(self.date)?
            .with_allow_draft(self.allow_draft);

        let mut posts = builder.build().await?;
        log::info!("Initial posts: {posts:#?}");

        posts.write_referrers(None)?;
        log::info!("Referrers written: {posts:#?}");
        posts.write_bluesky_posts(None).await?;
        log::info!("Bluesky posts written: {posts:#?}");

        let sign = Sign::Gpg;
        // Commit the posts to the git repo
        let commit_message = "chore: add drafted bluesky posts to git repo";
        client
            .commit_changed_files(sign, commit_message, "", None)
            .await?;

        Ok(CIExit::DraftedForBluesky)
    }
}
