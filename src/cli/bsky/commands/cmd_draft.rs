mod draft;

use std::{
    env,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use clap::Parser;
use config::Config;
use draft::{Draft, FrontMatter};
use regex::Regex;

use crate::{CIExit, Client, Error};

#[derive(Debug, Parser, Clone)]
pub struct CmdDraft {
    /// filter for files containing blog posts to broadcast on Bluesky
    #[arg(short, long)]
    pub filter: Option<String>,
    /// Optional path to file or directory of blog post(s) to process
    pub path: Option<String>,
}

impl CmdDraft {
    pub async fn run(&self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        let path = &self.path.clone().unwrap_or_default();

        let filter = &self.filter.clone().unwrap_or_default();

        let changed_files = if path != &String::new() {
            log::info!("Path: {path}");
            self.get_files_from_path(path)?
        } else {
            self.get_filtered_changed_files(client, settings, filter)
                .await?
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

        let path = if path.is_empty() {
            filter.to_string()
        } else {
            path.to_string()
        };

        Draft::new_with_path(&path)?
            .add_posts(&mut front_matters)?
            .process_posts()
            .await?
            .write_posts()?;
        Ok(CIExit::DraftedForBluesky)
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
        filter: &str,
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

        let re = if !filter.is_empty() {
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
