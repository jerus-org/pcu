use std::{env, fs};

use clap::Parser;
use color_eyre::Result;

use crate::{Client, Error};

use super::{CIExit, Commands};

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
    /// filter for files containing blog posts to broadcast on Bluesky
    #[arg(short, long)]
    pub filter: Option<String>,
}

impl Bsky {
    pub async fn run(&self) -> Result<CIExit> {
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
            log::info!("App Private Key: {app_private_key}");
            env::set_var("PCU_PRIVATE_KEY", app_private_key);
        }
        let settings = Commands::Bsky(self.clone()).get_settings()?;
        let client = Client::new_with(&settings).await?;

        // let release = client
        //     .github_rest
        //     .repos
        //     .get_latest_release(client.owner(), client.repo())
        //     .send()
        //     .await?;

        // log::info!("Release: {release:#?}");

        // let mut basehead = release.tag_name.clone();
        // basehead.push_str("...feat/add-bluesky-posting");
        // basehead.push_str("...main");
        log::trace!("branch: {:?}", settings.get::<String>("branch"));
        let pcu_branch: String = settings
            .get("branch")
            .map_err(|_| Error::EnvVarBranchNotSet)?;
        let branch = env::var(pcu_branch).map_err(|_| Error::EnvVarBranchNotFound)?;
        let basehead = format!("main...{branch}");

        log::info!("Basehead: {basehead}");

        let compare = client
            .github_rest
            .repos
            .compare_commits(client.owner(), client.repo(), &basehead)
            .send()
            .await?;

        if let Some(files) = compare.files {
            let mut changed_files = files.iter().map(|f| f.filename.clone()).collect::<Vec<_>>();
            log::info!("Changed files: {changed_files:#?}");
            changed_files = if let Some(filter) = &self.filter {
                log::info!("Filtering filenames containing: {filter}");
                let filtered_files = changed_files
                    .iter()
                    .filter(|f| f.contains(filter))
                    .cloned()
                    .collect::<Vec<_>>();
                log::info!("Filtered files: {filtered_files:#?}");
                filtered_files
            } else {
                changed_files
            };
            log::info!("Changed files: {changed_files:#?}");
        }

        // TODO: Get the list of blogs from the config

        // TODO: Identify blogs that have changed
        // TODO: For each blog, extract the title, description, and tags
        // TODO: For each blog, create a Bluesky post

        Ok(CIExit::PostedToBluesky)
    }
}
