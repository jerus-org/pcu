mod commands;

use std::{
    env,
    fs::{self},
};

use clap::Parser;
use commands::Cmd;
use config::Config;

use crate::{Client, Error};

use super::{CIExit, Commands};

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

        // TODO: For each blog, extract the title, description, and tags
        // TODO: For each blog, create a Bluesky post
    }

    async fn setup_client(&self) -> Result<(Client, Config), Error> {
        if let Some(owner) = &self.owner {
            log::info!("Owner: {owner}");
            env::set_var("OWNER", owner);
        }
        if let Some(repo) = &self.repo {
            log::info!("Repository: {repo}");
            env::set_var("REPO", repo);
        }
        if let Some(branch) = &self.branch {
            log::info!("Branch: {branch}");
            env::set_var("BRANCH", branch);
        }
        if let Some(appid) = &self.id {
            log::info!("Appid: {appid}");
            env::set_var("PCU_APP_ID", appid);
        }
        if let Some(app_private_key) = &self.pk {
            log::info!("App Private Key file: {app_private_key}");
            let app_private_key = fs::read_to_string(app_private_key)?;
            env::set_var("PCU_PRIVATE_KEY", app_private_key);
        }
        let settings = Commands::Bsky(self.clone()).get_settings()?;
        let client = Client::new_with(&settings).await?;

        Ok((client, settings))
    }
}
