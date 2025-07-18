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

use crate::{CIExit, Client, Error, GitOps, Sign};

#[derive(Debug, Parser, Clone)]
pub struct CmdDraft {
    /// filter for files containing blog posts to broadcast on Bluesky
    #[arg(short, long)]
    pub filter: Option<String>,
    /// Optional path to file or directory of blog post(s) to process
    pub path: Option<String>,
    /// Optional date to from which to process blog post(s)
    /// Date format: YYYY-MM-DD
    #[arg(short, long)]
    pub date: Option<toml::value::Datetime>,
    /// Allow bluesky posts for draft blog posts
    #[arg(long, default_value_t = false)]
    pub allow_draft: bool,
}

impl CmdDraft {
    pub async fn run(&self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        // find the changed file in the git repo

        let changed_files = self.get_changed_files(client, settings).await?;

        let mut front_matters = self.get_front_matters(&changed_files)?;

        if front_matters.is_empty() {
            log::info!("No front matters found");
            return Ok(CIExit::DraftedForBluesky);
        }

        // Filter front matters by date if specified
        if let Some(date) = &self.date {
            log::info!("Filtering front matters by date: {date}");
            front_matters.retain(|fm| fm.most_recent_date() >= *date);
        }

        // Remove draft posts unless allow_draft is true
        if !self.allow_draft {
            log::info!("Filtering out draft posts");
            front_matters.retain(|fm| !fm.draft);
        }

        self.draft_posts_and_redirects(settings, &mut front_matters)
            .await?;

        let sign = Sign::Gpg;
        // Commit the posts to the git repo
        let commit_message = "chore: add drafted bluesky posts to git repo";
        client
            .commit_changed_files(sign, commit_message, "", None)
            .await?;

        Ok(CIExit::DraftedForBluesky)
    }

    /// Get the changed files from the git repo
    async fn get_changed_files(
        &self,
        client: &Client,
        settings: &Config,
    ) -> Result<Vec<(String, String)>, Error> {
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

        Ok(changed_files)
    }

    fn get_front_matters(
        &self,
        changed_files: &[(String, String)],
    ) -> Result<Vec<FrontMatter>, Error> {
        let mut front_matters = Vec::new();
        let mut first = true;

        for filename in changed_files {
            log::info!("File and path: {filename:?}");
            match self.get_frontmatter(&filename.0, first) {
                Ok(mut front_matter) => {
                    front_matter.path = Some(filename.1.clone());
                    front_matters.push(front_matter);
                    first = false;
                }
                Err(e) => {
                    log::warn!("Error: {e}");
                    first = false;
                    continue;
                }
            }
        }

        log::trace!(
            "Front matters ({}): {front_matters:#?}",
            front_matters.len()
        );

        log::info!("{} front matters found.", front_matters.len());

        Ok(front_matters)
    }

    async fn draft_posts_and_redirects(
        &self,
        settings: &Config,
        front_matters: &mut Vec<FrontMatter>,
    ) -> Result<(), Error> {
        let path = if self.path.clone().unwrap_or_default().is_empty() {
            self.filter.clone().unwrap_or_default()
        } else {
            self.path.clone().unwrap_or_default()
        };

        Draft::new_with_path(&path)?
            .add_posts(front_matters)?
            .process_posts()
            .await?
            .add_store(&settings.get_string("store")?)?
            .write_posts()?
            .write_redirects()?;

        Ok(())
    }

    /// Get the file from the path and return a list of files
    /// The path may be a single file or a directory containing files
    /// Only files ending in `.md` will be returned
    fn get_files_from_path(&self, path: &str) -> Result<Vec<(String, String)>, Error> {
        get_files(path)
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
    ) -> Result<Vec<(String, String)>, Error> {
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
        changed_files = files
            .iter()
            .map(|f| get_path_and_basename(f.filename.clone().as_str()))
            .collect::<Vec<_>>();
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
            .filter(|f| re.is_match(&f.0))
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

        log::trace!("Front matter string:\n {front_str}");

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

/// Get the file from the path and return a list of files
/// The path may be a single file or a directory containing files
/// Only files ending in `.md` will be returned
fn get_files(path: &str) -> Result<Vec<(String, String)>, Error> {
    let path = PathBuf::from(path);
    log::debug!("get_files path: {path:?}");
    if !path.exists() {
        return Err(Error::PathNotFound(path.to_string_lossy().to_string()));
    };

    if path.is_file() {
        if path.extension().unwrap_or_default() == "md" {
            Ok(vec![(
                path.to_string_lossy().to_string(),
                get_path_and_basename(path.to_str().unwrap()).0,
            )])
        } else {
            Err(Error::FileExtensionInvalid(
                path.to_string_lossy().to_string(),
                ".md".to_string(),
            ))
        }
    } else if path.is_dir() {
        let paths = fs::read_dir(&path)?;
        let mut files = Vec::new();
        for entry in paths {
            let entry_path = entry?.path();
            log::debug!("Entry path: {entry_path:?}");
            if entry_path.is_dir() {
                let mut subdir_files = get_files(entry_path.to_str().unwrap())?;
                files.append(&mut subdir_files);
                continue;
            } else if entry_path.is_file() && entry_path.extension().unwrap_or_default() == "md" {
                files.push((
                    entry_path.to_string_lossy().to_string(),
                    path.to_string_lossy().to_string(),
                ));
            }
        }
        return Ok(files);
    } else {
        return Err(Error::PathNotFound(path.to_string_lossy().to_string()));
    }
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

    use crate::cli::bsky::commands::cmd_draft::get_files;
    use crate::Error;
    use std::fs;
    // use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_get_files_single_markdown_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        fs::write(&file_path, "test content").unwrap();

        println!("File path: {file_path:?}");
        let result = get_files(file_path.to_str().unwrap());
        println!("Result: {result:#?}");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec![(
                file_path.to_string_lossy().to_string(),
                temp_dir.path().to_string_lossy().to_string()
            )]
        );
    }

    #[test]
    fn test_get_files_non_markdown_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let result = get_files(file_path.to_str().unwrap());
        assert!(matches!(result, Err(Error::FileExtensionInvalid(_, _))));
    }

    #[test]
    fn test_get_files_directory_with_markdown_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = "test1.md".to_string();
        let file2 = "test2.md".to_string();
        let file3 = "test.txt".to_string();

        let md_file1 = temp_dir.path().join(&file1);
        let md_file2 = temp_dir.path().join(&file2);
        let txt_file = temp_dir.path().join(&file3);

        let expected = [
            (
                md_file1.to_string_lossy().to_string(),
                temp_dir.path().to_string_lossy().to_string(),
            ),
            (
                md_file2.to_string_lossy().to_string(),
                temp_dir.path().to_string_lossy().to_string(),
            ),
        ];

        fs::write(&md_file1, "content1").unwrap();
        fs::write(&md_file2, "content2").unwrap();
        fs::write(&txt_file, "content3").unwrap();

        let result = get_files(temp_dir.path().to_str().unwrap());
        assert!(result.is_ok());
        let files = result.unwrap();
        println!("Files: {files:?}");
        assert_eq!(files.len(), 2);
        assert!(files.contains(&expected[0]));
        assert!(files.contains(&expected[1]));
    }

    #[test]
    fn test_get_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = get_files(temp_dir.path().to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_get_files_non_existent_path() {
        let result = get_files("/non-existent/path");
        assert!(matches!(result, Err(Error::PathNotFound(_))));
    }

    #[test]
    fn test_get_files_nested_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("nested");
        fs::create_dir(&nested_dir).unwrap();

        let md_file1 = temp_dir.path().join("test1.md");
        let md_file2 = nested_dir.join("test2.md");

        fs::write(&md_file1, "content1").unwrap();
        fs::write(&md_file2, "content2").unwrap();

        let result = get_files(temp_dir.path().to_str().unwrap());
        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&(
            md_file1.to_string_lossy().to_string(),
            temp_dir.path().to_string_lossy().to_string()
        )));
        assert!(files.contains(&(
            md_file2.to_string_lossy().to_string(),
            nested_dir.as_path().to_string_lossy().to_string()
        )));
    }
}
