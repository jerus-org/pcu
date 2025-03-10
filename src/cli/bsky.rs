mod commands;
mod draft;
mod front_matter;
mod poster;

use std::{
    env,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use clap::Parser;
use commands::Cmd;
use config::Config;
use draft::Draft;
use front_matter::FrontMatter;
use poster::Poster;
use regex::Regex;

use crate::{Client, Error};

use super::{CIExit, Commands};

const BSKY_POSTS_DIR: &str = "bluesky";

/// Configuration for the Bsky command
#[derive(Debug, Parser, Clone)]
pub struct Bsky {
    /// Optional path to file or directory of blog post(s) to process
    path: Option<String>,
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
    /// Command to execute
    #[command(subcommand)]
    pub cmd: Cmd,
}

impl Bsky {
    pub async fn run(&self) -> Result<CIExit, Error> {
        let (client, settings) = self.setup_client().await?;

        let changed_files = if let Some(path) = &self.path {
            log::info!("Path: {path}");
            self.get_files_from_path(path)?
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

        match self.cmd {
            Cmd::Draft => {
                let path = self.filter.clone().unwrap_or_default();
                Draft::new_with_path(&path)?
                    .add_posts(&mut front_matters)?
                    .process_posts()
                    .await?
                    .write_posts()?;
                Ok(CIExit::DraftedForBluesky)
            }
            Cmd::Post => {
                let id = settings.get::<String>("bsky_id")?;
                let pw = settings.get::<String>("bsky_password")?;
                Poster::new()?
                    .load(BSKY_POSTS_DIR)?
                    .post_to_bluesky(id, pw)
                    .await?;
                Ok(CIExit::PostedToBluesky)
            }
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

    /// Get the file from the path and return a list of files
    /// The path may be a single file or a directory containing files
    /// Only files ending in `.md` will be returned
    fn get_files_from_path(&self, path: &str) -> Result<Vec<String>, Error> {
        let path = PathBuf::from(path);
        if !path.exists() {
            return Err(Error::PathNotFound(path.to_string_lossy().to_string()));
        };

        if path.is_file() {
            if path.extension().unwrap_or_default() == "md" {
                Ok(vec![path.to_string_lossy().to_string()])
            } else {
                Err(Error::FileExtensionInvalid(
                    path.to_string_lossy().to_string(),
                    ".md".to_string(),
                ))
            }
        } else if path.is_dir() {
            let paths = fs::read_dir(path)?;
            let mut files = Vec::new();
            for path in paths {
                let path = path?.path();
                if path.is_dir() {
                    // files.append(&mut self.get_files_from_path(&path.to_string_lossy())?);
                    continue;
                } else if path.is_file() && path.extension().unwrap_or_default() == "md" {
                    files.push(path.to_string_lossy().to_string());
                }
            }
            return Ok(files);
        } else {
            return Err(Error::PathNotFound(path.to_string_lossy().to_string()));
        }
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
    ) -> Result<Vec<String>, Error> {
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

    fn get_frontmatter(&self, filename: &str) -> Result<FrontMatter, Error> {
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
