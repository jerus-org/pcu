mod site_config;

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
    pub path: Option<String>,
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
    pub async fn run(&self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        // find the potential file in the git repo

        let base_url = SiteConfig::new()?.base_url();
        let store = &settings.get_string("store")?;
        let path = if let Some(path) = self.path.as_ref() {
            path
        } else {
            DEFAULT_PATH
        };

        log::trace!(
            "Key parameters:\n\tBase:\t`{base_url}`\n\tstore:\t`{store}`\n\tpath:\t`{path}`"
        );

        let mut posts_builder = Draft::builder();
        let posts_res = posts_builder
            .with_base_url(&base_url)
            .with_store(store)
            .add_blog_posts(path)?
            .with_minimum_date(self.date)?
            .with_allow_draft(self.allow_draft)
            .build();

        let Ok(mut posts) = posts_res else {
            log::warn!("{}", posts_res.err().unwrap());
            return Ok(CIExit::DraftedForBluesky);
        };

        posts.process_posts().await?;
        posts.write_bluesky_posts()?;

        let sign = Sign::Gpg;
        // Commit the posts to the git repo
        let commit_message = "chore: add drafted bluesky posts to git repo";
        client
            .commit_changed_files(sign, commit_message, "", None)
            .await?;

        Ok(CIExit::DraftedForBluesky)
    }
}
