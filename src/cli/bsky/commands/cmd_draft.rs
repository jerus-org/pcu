mod draft;

use std::{
    env,
    fs::{self, File},
    io::{BufRead, BufReader},
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
        let mut first = true;

        for filename in changed_files {
            log::info!("File: {filename}");
            match self.get_frontmatter(&filename, first) {
                Ok(front_matter) => {
                    front_matters.push(front_matter);
                    first = false;
                }
                Err(e) => {
                    log::error!("Error: {e}");
                    first = false;
                    continue;
                }
            }
        }

        log::debug!(
            "Front matters ({}): {front_matters:#?}",
            front_matters.len()
        );

        if front_matters.is_empty() {
            log::info!("No front matters found");
            return Ok(CIExit::DraftedForBluesky);
        }

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

    fn get_frontmatter(&self, filename: &str, first: bool) -> Result<FrontMatter, Error> {
        log::debug!("Reading front matter from: {filename} with flag first: {first}");
        let file = File::open(filename)?;
        let reader = BufReader::new(file);

        let mut front_str = String::new();
        let mut quit = false;

        for line in reader.lines().map_while(Result::ok) {
            if line.starts_with("+++") & quit {
                break;
            } else if line.starts_with("+++") {
                quit = true;
                continue;
            } else {
                front_str.push_str(&line);
                front_str.push('\n');
                if first {
                    log::debug!("Front matter:\n {front_str}\n ... and quit: {quit}");
                }
            }
        }

        log::debug!("Front matter string:\n {front_str}");

        let mut front_matter = FrontMatter::from_toml(&front_str)?;

        let (path, basename) = get_path_and_basename(filename);
        // let basename = filename.split('/').last().unwrap().to_string();
        // let basename = basename.split('.').next().unwrap().to_string();
        log::debug!("Basename: {basename}");
        log::debug!("Full filename: {filename}");
        log::debug!("Front matter: {front_matter:#?}");
        front_matter.basename = Some(basename);
        front_matter.path = Some(path);

        Ok(front_matter)
    }
}

fn get_path_and_basename(path: &str) -> (String, String) {
    let (path, filename) = split_return_last_and_rest(path.to_string(), '/');
    let (basename, _ext) = split_return_last_and_rest(filename, '.');

    (path, basename)
}

fn split_return_last_and_rest(s: String, pat: char) -> (String, String) {
    let parts = s.split(pat).collect::<Vec<_>>();

    (
        parts[..parts.len() - 1].join(&String::from(pat)),
        parts.last().unwrap().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_path_and_filename_basic() {
        let result = get_path_and_basename("folder/subfolder/file.txt");
        assert_eq!(result.0, "folder/subfolder");
        assert_eq!(result.1, "file");
    }

    #[test]
    fn test_get_path_and_filename_root() {
        let result = get_path_and_basename("file.txt");
        assert_eq!(result.0, "");
        assert_eq!(result.1, "file");
    }

    #[test]
    fn test_get_path_and_filename_nested() {
        let result = get_path_and_basename("a/b/c/d/file.txt");
        assert_eq!(result.0, "a/b/c/d");
        assert_eq!(result.1, "file");
    }

    #[test]
    fn test_get_path_and_filename_with_dots() {
        let result = get_path_and_basename("path/to/my.file.with.dots.txt");
        assert_eq!(result.0, "path/to");
        assert_eq!(result.1, "my.file.with.dots");
    }

    #[test]
    fn test_get_path_and_filename_trailing_slash() {
        let result = get_path_and_basename("path/to/file/");
        assert_eq!(result.0, "path/to/file");
        assert_eq!(result.1, "");
    }
}
