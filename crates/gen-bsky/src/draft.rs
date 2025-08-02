use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::PathBuf,
};

use crate::{FrontMatter, ToStringSlash};
use chrono::{Datelike, Utc};
use serde::Deserialize;
use thiserror::Error;
use toml::value::Datetime;

/// Error enum for Draft type
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum DraftError {
    /// Array capacity too large
    #[error("Future capacity is too large")]
    FutureCapacityTooLarge,
    /// Path to add blog posts is not found
    #[error("path not found: `{0}`")]
    PathNotFound(String),
    /// Incorrect file extension for blog post (must be `.md`)
    #[error("file extension invalid (must be `{1}`): {0}")]
    FileExtensionInvalid(String, String),
    /// Blog post list is empty
    #[error("blog post list is empty")]
    BlogPostListEmpty,
    /// Blog post list is empty
    #[error("blog post list is empty after qualifications have been applied")]
    QualifiedBlogPostListEmpty,
    /// Error reported by IO library
    #[error("io error says: {0:?}")]
    IO(#[from] std::io::Error),
    /// Error reported by FrontMatter
    #[error("serde_json create_session error says: {0:?}")]
    FrontMatterError(#[from] crate::FrontMatterError),
}

/// Type representing the configuration required to generate
/// drafts for a list of blog posts.
///
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Draft {
    blog_posts: Vec<FrontMatter>,
    base_url: String,
    store: String,
}

impl Draft {
    /// Initialise a new draft configuration setting the base_url, path and store.
    ///
    /// ## Parameters
    ///
    /// - `base_url`: the base url for the website (e.g. `https://wwww.example.com/`)
    /// - `path`: the path to the source for the blog posts (e.g. `contents/blog/`)
    /// - `store`: the location to store draft posts (e.g. `bluesky`)
    ///
    pub fn builder() -> DraftBuilder {
        DraftBuilder::default()
    }

    /// Trigger processing of frontmatter posts
    pub async fn process_posts(&mut self) -> Result<&mut Self, DraftError> {
        for blog_post in &mut self.blog_posts {
            blog_post.get_bluesky_record(self.base_url.as_str()).await?;
        }

        Ok(self)
    }

    /// Write Bluesky posts for the front matter.
    pub fn write_bluesky_posts(&self) -> Result<(), DraftError> {
        // create store directory if it doesn't exist
        if !std::path::Path::new(&self.store).exists() {
            std::fs::create_dir_all(self.store.clone())?;
        }

        for blog_post in &self.blog_posts {
            match blog_post.write_bluesky_record_to(&self.store) {
                Ok(_) => continue,
                Err(e) => {
                    log::warn!(
                        "Blog post: `{}` skipped because of error `{e}`",
                        blog_post.title
                    );
                    continue;
                }
            }
        }

        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct DraftBuilder {
    blog_posts: Vec<FrontMatter>,
    base_url: String,
    store: String,
    minimum_date: Datetime,
    allow_draft: bool,
}

impl Default for DraftBuilder {
    fn default() -> Self {
        DraftBuilder {
            blog_posts: Vec::default(),
            base_url: String::default(),
            store: String::default(),
            minimum_date: today(),
            allow_draft: false,
        }
    }
}

impl DraftBuilder {
    pub fn with_base_url(&mut self, base_url: &str) -> &mut Self {
        self.base_url = base_url.to_string_slash();
        self
    }

    pub fn with_store(&mut self, store: &str) -> &mut Self {
        self.store = store.to_string_slash();
        self
    }

    pub fn add_blog_posts(&mut self, path: &str) -> Result<&mut Self, DraftError> {
        // find the potential file in the git repo
        let potential_files = get_files(path)?;

        let mut front_matters = get_front_matters(&potential_files)?;

        if front_matters.is_empty() {
            log::warn!("no front matters found for path `{path}`");
        }

        self.blog_posts.append(&mut front_matters);
        Ok(self)
    }

    /// Optionally set a minimum for blog posts
    ///
    /// ## Parameters
    ///
    /// - `minimum_date`: Optional minimum date in format `YYYY-MM-DD`
    ///
    pub fn with_minimum_date(
        &mut self,
        minimum_date: Option<Datetime>,
    ) -> Result<&mut Self, DraftError> {
        self.minimum_date = if let Some(date) = minimum_date {
            date
        } else {
            today()
        };

        Ok(self)
    }

    pub fn with_allow_draft(&mut self, allow_draft: bool) -> &mut Self {
        self.allow_draft = allow_draft;
        self
    }

    pub fn build(&mut self) -> Result<Draft, DraftError> {
        if self.blog_posts.is_empty() {
            log::warn!("No blog posts found");
            return Err(DraftError::BlogPostListEmpty);
        }

        let mut blog_posts = self.blog_posts.clone();

        blog_posts.retain(|fm| fm.most_recent_date() >= self.minimum_date);
        log::trace!(
            "Retained `{}` front matters after filtering by `{}`",
            self.blog_posts.len(),
            self.minimum_date,
        );

        if !self.allow_draft {
            blog_posts.retain(|fm| !fm.draft);
            log::trace!(
                "Retained `{}` front matters after removing drafts",
                blog_posts.len()
            );
        }

        if blog_posts.is_empty() {
            return Err(DraftError::BlogPostListEmpty);
        }

        Ok(Draft {
            blog_posts,
            base_url: self.base_url.clone(),
            store: self.store.clone(),
        })
    }
}

/// Get the file from the path and return a list of files
/// The path may be a single file or a directory containing files
/// Only files ending in `.md` will be returned
fn get_files(path: &str) -> Result<Vec<(String, String)>, DraftError> {
    let path = PathBuf::from(path);
    log::debug!("get_files path: {path:?}");
    if !path.exists() {
        return Err(DraftError::PathNotFound(path.to_string_lossy().to_string()));
    };

    if path.is_file() {
        if path.extension().unwrap_or_default() == "md" {
            Ok(vec![(
                path.to_string_lossy().to_string(),
                get_path_and_basename(path.to_str().unwrap()).0,
            )])
        } else {
            Err(DraftError::FileExtensionInvalid(
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
        return Err(DraftError::PathNotFound(path.to_string_lossy().to_string()));
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

fn get_front_matters(in_scope_files: &[(String, String)]) -> Result<Vec<FrontMatter>, DraftError> {
    let mut front_matters = Vec::new();
    let mut first = true;

    for filename in in_scope_files {
        log::info!("File and path: {filename:?}");
        match get_frontmatter(&filename.0, first) {
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

    log::info!("Total of `{}` front matters found.", front_matters.len());

    Ok(front_matters)
}

fn get_frontmatter(filename: &str, first: bool) -> Result<FrontMatter, DraftError> {
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
                log::trace!("Front matter:\n {front_str}\n ... and quit: {quit}");
            }
        }
    }

    log::trace!("Front matter string:\n {front_str}");

    let mut front_matter = FrontMatter::from_toml(&front_str)?;

    let (path, basename) = get_path_and_basename(filename);
    log::debug!("Basename: {basename}");
    log::debug!("Full filename: {filename}");
    log::trace!("Front matter: {front_matter:#?}");
    front_matter.basename = Some(basename);
    front_matter.path = Some(path);

    Ok(front_matter)
}

fn today() -> toml::value::Datetime {
    let now = Utc::now();
    let date_string = format!("date = {}-{:02}-{:02}", now.year(), now.month(), now.day());

    #[derive(Debug, Deserialize)]
    struct Current {
        #[allow(dead_code)]
        date: toml::value::Datetime,
    }
    let current_date: Current = toml::from_str(&date_string).unwrap();
    current_date.date
}

#[cfg(test)]
mod tests {
    // use std::{fs, path::Path};

    // use bsky_sdk::api::{app::bsky::feed::post::RecordData, types::string::Datetime};
    // use log::LevelFilter;

    use super::*;

    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_path_and_basename() {
        let test_cases = vec![
            (
                "/media/blog/post.md",
                ("/media/blog".to_string(), "post".to_string()),
            ),
            (
                "blog/2023/test.md",
                ("blog/2023".to_string(), "test".to_string()),
            ),
            ("single.md", ("".to_string(), "single".to_string())),
            (
                "nested/path/file.md",
                ("nested/path".to_string(), "file".to_string()),
            ),
            (
                "/absolute/path/post.md",
                ("/absolute/path".to_string(), "post".to_string()),
            ),
        ];

        for (input, expected) in test_cases {
            let result = get_path_and_basename(input);
            assert_eq!(result, expected, "Failed for input: {input}");
        }
    }

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
        assert!(matches!(
            result,
            Err(DraftError::FileExtensionInvalid(_, _))
        ));
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
        assert!(matches!(result, Err(DraftError::PathNotFound(_))));
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
