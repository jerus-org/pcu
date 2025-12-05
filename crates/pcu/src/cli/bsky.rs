mod commands;

use std::fs::{self};

use clap::Parser;
use commands::Cmd;
use config::Config;

use super::{CIExit, Commands};
use crate::{Client, Error};

const BSKY_POSTS_DIR: &str = "bluesky";

/// Configuration for the Bsky command
#[derive(Debug, Parser, Clone)]
pub struct Bsky {
    /// owner of the repository
    #[arg(short, long)]
    pub owner: Option<String>,
    /// name of the repository
    #[arg(short, long)]
    pub repo: Option<String>,
    /// name of the branch to compare against main for file list
    #[arg(short, long)]
    pub branch: Option<String>,
    /// application id for access to the repository
    #[arg(short, long)]
    pub id: Option<String>,
    /// file with application private key for access to the repository
    #[arg(short, long)]
    pub pk: Option<String>,
    /// directory to store the Bluesky posts
    #[arg(short, long, default_value_t = BSKY_POSTS_DIR.to_string())]
    pub store: String,
    /// Command to execute
    #[command(subcommand)]
    pub cmd: Cmd,
}

impl Bsky {
    pub async fn run(&self) -> Result<CIExit, Error> {
        let (client, settings) = self.setup_client().await?;

        match self.cmd.clone() {
            Cmd::Draft(mut draft_args) => draft_args.run(&client, &settings).await,
            Cmd::Post(post_args) => post_args.run(&client, &settings).await,
        }
    }

    async fn setup_client(&self) -> Result<(Client, Config), Error> {
        let settings = Commands::Bsky(self.clone()).get_settings()?;
        let mut builder = Config::builder();
        builder = builder.add_source(settings);

        if let Some(owner) = self.owner.as_deref() {
            log::info!("Owner: {owner}");
            builder = builder.set_override("OWNER", owner)?;
        }
        if let Some(repo) = self.repo.as_deref() {
            log::info!("Repository: {repo}");
            builder = builder.set_override("REPO", repo)?;
        }
        if let Some(branch) = self.branch.as_deref() {
            log::info!("Branch: {branch}");
            builder = builder.set_override("BRANCH", branch)?;
        }
        if let Some(appid) = self.id.as_deref() {
            log::info!("Appid: {appid}");
            builder = builder.set_override("PCU_APP_ID", appid)?;
        }
        if let Some(app_private_key) = &self.pk {
            log::info!("App Private Key file: {app_private_key}");
            let app_private_key = fs::read_to_string(app_private_key)?;
            builder = builder.set_override("PCU_PRIVATE_KEY", app_private_key)?;
        }
        let settings = builder.build()?;
        let client = Client::new_with(&settings).await?;

        Ok((client, settings))
    }
}
