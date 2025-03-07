mod front_matter;
mod poster;

use std::{
    env,
    fs::{self, File},
    io::Read,
};

use clap::Parser;
use color_eyre::Result;
use config::Config;
use front_matter::FrontMatter;
use poster::Poster;
use regex::Regex;

use crate::{Client, Error};

use super::{CIExit, Commands};

/// Configuration for the Bsky command
#[derive(Debug, Parser, Clone)]
pub struct Bsky {
    /// Optional blog post file to process
    file: Option<String>,
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
        let (client, settings) = self.setup_client().await?;

        let changed_files = if let Some(file) = &self.file {
            log::info!("File: {file}");
            vec![file.clone()]
        } else {
            self.get_filtered_changed_files(&client, &settings).await?
        };
        log::debug!("Changed files: {changed_files:#?}");

        let mut front_matters = Vec::new();

        for filename in changed_files {
            log::info!("File: {filename}");
            match self.get_frontmatter(&filename) {
                Ok(front_matter) => front_matters.push(front_matter),
                Err(e) => {
                    log::error!("Error: {e}");
                    continue;
                }
            }
        }

        log::debug!("Front matters: {front_matters:#?}");

        let id = settings.get::<String>("bsky_id")?;
        let pw = settings.get::<String>("bsky_password")?;
        let dir = self.filter.as_deref().unwrap_or_default();

        let poster = Poster::new(front_matters, id, pw, dir).await?;
        poster.post_to_bluesky().await?;

        // TODO: For each blog, extract the title, description, and tags
        // TODO: For each blog, create a Bluesky post

        Ok(CIExit::PostedToBluesky)
    }

    async fn setup_client(&self) -> Result<(Client, Config)> {
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

    /// Filter for Markdown files containing blog posts
    ///
    /// Markdown files are filtered based on ending with ".md" and blog
    /// files are identified based on the filter string specified
    /// at the command line.
    async fn get_filtered_changed_files(
        &self,
        client: &Client,
        settings: &Config,
    ) -> Result<Vec<String>> {
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

        let mut changed_files = Vec::new();
        let Some(files) = compare.files else {
            log::warn!("No files found in compare");
            return Ok(changed_files);
        };
        changed_files = files.iter().map(|f| f.filename.clone()).collect::<Vec<_>>();
        log::debug!("Changed files: {changed_files:#?}");

        let re = if let Some(filter) = &self.filter {
            log::info!("Filtering filenames containing: {filter}");
            let regex_str = format!(r"^.+{filter}.+\.md$");
            Regex::new(&regex_str).unwrap()
        } else {
            Regex::new(r"^.+\.md$").unwrap()
        };

        let filtered_files = changed_files
            .iter()
            .filter(|f| re.is_match(f))
            .cloned()
            .collect::<Vec<_>>();
        log::trace!("Filtered files: {filtered_files:#?}");

        Ok(filtered_files)
    }

    fn get_frontmatter(&self, filename: &str) -> Result<FrontMatter> {
        let mut file = File::open(filename)?;
        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents)?;
        log::debug!("File contents: {file_contents}");
        let lines: Vec<String> = file_contents.lines().map(|l| l.to_string()).collect();
        log::debug!("Lines: {lines:#?}");

        let mut front_str = String::new();

        let mut quit = false;

        for line in lines {
            if line.starts_with("+++") && quit {
                break;
            } else if line.starts_with("+++") {
                quit = true;
                continue;
            } else {
                front_str.push_str(&line);
                front_str.push('\n');
                log::debug!("Front matter: {front_str} and quit: {quit}");
            }
        }

        let mut front_matter = FrontMatter::from_toml(&front_str)?;
        let basename = filename.split('/').last().unwrap().to_string();
        let basename = basename.split('.').next().unwrap().to_string();
        log::trace!("Basename: {basename}");
        front_matter.filename = Some(basename);

        Ok(front_matter)
    }
}
