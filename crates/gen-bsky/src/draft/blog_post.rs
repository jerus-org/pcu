//! # BlogPost Module Documentation
//!
//! ## Overview
//!
//! The `BlogPost` module provides functionality for processing blog posts and
//! converting them into Bluesky social media posts. It handles front matter
//! parsing, link generation, short URL creation, and Bluesky post formatting
//! with character/grapheme limits.
//!
//! ## Core Components
//!
//! ### `BlogPost` Struct
//!
//! The main struct representing a blog post with all necessary metadata and
//! functionality for Bluesky integration.
//!
//! #### Fields
//!
//! - `path: PathBuf` - Path to the original blog post file
//! - `frontmatter: front_matter::FrontMatter` - Parsed front matter containing
//!   post metadata
//! - `post_link: Url` - Full URL link to the post
//! - `redirector: Redirector` - HTML redirector for creating short links
//! - `post_short_link: Option<Url>` - Generated short URL (if created)
//! - `bluesky_count: u8` - Counter tracking how many times this post has been
//!   written to Bluesky
//!
//! ### `BlogPostError` Enum
//!
//! Comprehensive error handling for various failure scenarios:
//!
//! #### Error Variants
//!
//! - `PostTooManyCharacters(String, usize)` - Post exceeds 300 character limit
//! - `PostTooManyGraphemes(String, usize)` - Post exceeds 300 grapheme limit
//! - `BlueSkyPostNotConstructed` - Bluesky record hasn't been built
//! - `PostBasenameNotSet` - Post filename/basename is missing
//! - `BskySdk(bsky_sdk::Error)` - Bluesky SDK errors
//! - `RedirectorError(link_bridge::RedirectorError)` - Link redirector errors
//! - `Io(std::io::Error)` - File I/O errors
//! - `SerdeJsonError(serde_json::error::Error)` - JSON serialization errors
//! - `DraftNotAllowed` - Draft posts are disabled
//! - `PostTooOld(Datetime)` - Post is older than minimum allowed date
//! - `Toml(toml::de::Error)` - TOML parsing errors
//! - `UrlParse(url::ParseError)` - URL parsing errors
//!
//! ## Public API
//!
//! ### Constructor
//!
//! ```rust
//! pub fn new(
//!     blog_path: &PathBuf,
//!     min_date: Datetime,
//!     allow_draft: bool,
//!     base_url: &Url,
//!     www_src_root: &Path,
//! ) -> Result<BlogPost, BlogPostError>
//! ```
//!
//! Creates a new `BlogPost` instance from a blog file path.
//!
//! **Parameters:**
//! - `blog_path` - Path to the blog post file relative to `www_src_root`
//! - `min_date` - Minimum publication date for posts
//! - `allow_draft` - Whether to allow processing of draft posts
//! - `base_url` - Base URL for generating post links
//! - `www_src_root` - Root directory containing blog content
//!
//! **Returns:** `Result<BlogPost, BlogPostError>`
//!
//! ### Accessor Methods
//!
//! ```rust
//! pub fn title(&self) -> &str
//! ```
//! Returns the post title from front matter.
//!
//! ### Core Functionality
//!
//! ```rust
//! pub async fn get_bluesky_record(&self) -> Result<RecordData, BlogPostError>
//! ```
//!
//! Generates a Bluesky `RecordData` structure from the blog post content.
//!
//! **Features:**
//! - Builds formatted post text with title, description, tags, and link
//! - Uses rich text processing to detect facets (mentions, links, hashtags)
//! - Validates character and grapheme limits (300 max)
//! - Creates proper Bluesky API record format
//!
//! ```rust
//! pub fn write_referrer_file_to(
//!     &mut self,
//!     store_dir: &Path,
//!     base_url: &Url,
//! ) -> Result<(), BlogPostError>
//! ```
//!
//! Creates a redirect HTML file and generates a short URL for the post.
//!
//! **Parameters:**
//! - `store_dir` - Directory to write the redirect file
//! - `base_url` - Base URL for constructing the short link
//!
//! ```rust
//! pub async fn write_bluesky_record_to(&mut self, store_dir: &Path) -> Result<(), BlogPostError>
//! ```
//!
//! Writes the Bluesky post record as JSON to the specified directory.
//!
//! **Features:**
//! - Generates unique filename using base62 encoding of path components
//! - Creates JSON file with `.post` extension
//! - Increments internal post counter
//! - Handles file I/O with proper error propagation
//!
//! ## Implementation Details
//!
//! ### Post Text Format
//!
//! The generated Bluesky post follows this format:
//! ```
//! {
//!     title
//! }
//!
//! {
//!     description
//! }
//! {
//!     tags
//! }
//!
//! {
//!     short_link_or_full_link
//! }
//! ```
//!
//! ### Character Limits
//!
//! - Maximum 300 characters (byte length)
//! - Maximum 300 graphemes (Unicode grapheme clusters)
//! - Both limits are enforced to ensure Bluesky compatibility
//!
//! ### Short Link Generation
//!
//! - Uses `link_bridge::Redirector` for HTML redirect creation
//! - Generates short URLs by trimming base paths
//! - Stores short link in `post_short_link` field for reuse
//!
//! ### Unique Post Naming
//!
//! Post files are named using base62 encoding of:
//! 1. Full post path UTF-16 sum
//! 2. Filename UTF-16 sum
//! 3. Directory path (without filename) UTF-16 sum
//!
//! This ensures unique filenames even for posts with similar names.
//!
//! ## Dependencies
//!
//! - `bsky_sdk` - Bluesky API integration and rich text processing
//! - `link_bridge` - URL redirection and short link generation
//! - `toml` - Front matter parsing
//! - `url` - URL parsing and manipulation
//! - `unicode_segmentation` - Proper grapheme counting
//! - `serde_json` - JSON serialization for post records
//! - `thiserror` - Structured error handling
//!
//! ## Usage Example
//!
//! ```rust
//! use std::path::PathBuf;
//!
//! use toml::value::Datetime;
//! use url::Url;
//!
//! // Create a new blog post
//! let blog_path = PathBuf::from("content/posts/my-post.md");
//! let min_date =
//!     Datetime::from_str("2024-01-01T00:00:00Z").unwrap();
//! let base_url = Url::parse("https://example.com").unwrap();
//! let www_root = Path::new("./www");
//!
//! let mut post = BlogPost::new(
//!     &blog_path, min_date, false, // don't allow drafts
//!     &base_url, www_root,
//! )
//! .unwrap();
//!
//! // Generate short link
//! post.write_referrer_file_to(Path::new("./static"), &base_url)
//!     .unwrap();
//!
//! // Create and save Bluesky post
//! post.write_bluesky_record_to(Path::new("./output"))
//!     .await
//!     .unwrap();
//! ```
//!
//! ## Error Handling
//!
//! The module uses comprehensive error handling with the `BlogPostError` enum.
//! All errors are properly wrapped and provide context about the failure. The
//! `thiserror` crate is used for structured error definitions with automatic
//! `Display` and `Error` trait implementations.
//!
//! ## Logging
//!
//! The module includes extensive logging at various levels:
//! - `info!` - High-level operations
//! - `debug!` - Detailed processing information
//! - `trace!` - Fine-grained debugging data
//!
//! Enable logging to get insights into the post processing pipeline.

use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub(crate) mod front_matter;

use bsky_sdk::{
    api::{app::bsky::feed::post::RecordData, types::string::Datetime as BskyDatetime},
    rich_text::RichText,
};
use link_bridge::Redirector;
use thiserror::Error;
use toml::value::Datetime;
use unicode_segmentation::UnicodeSegmentation;
use url::Url;

/// Error enum for BlogPost type
#[non_exhaustive]
#[derive(Error, Debug)]
pub(super) enum BlogPostError {
    /// Generated post contains two many characters for a bluesky post.
    /// Reduce the size of the components contributing to the post such
    /// as description and tag list.
    #[error("bluesky post for `{0}` contains too many characters: {1}")]
    PostTooManyCharacters(String, usize),
    /// Generated post contains two many graphemes for a bluesky post.
    /// Reduce the size of the components contributing to the post such
    /// as description and tag list.
    #[error("bluesky post for `{0}` contains too many graphemes: {1}")]
    PostTooManyGraphemes(String, usize),
    /// The bluesky post record has not been constructed. Use
    /// the `get_bluesky_record` method to generate the bluesky post
    /// record.
    #[error("bluesky post has not been constructed")]
    BlueSkyPostNotConstructed,
    /// The post basename is has not been set.
    #[error("post basename is not set")]
    PostBasenameNotSet,
    /// Error reported by the Bluesky SDK library.
    #[error("bsky_sdk error says: {0:?}")]
    BskySdk(#[from] bsky_sdk::Error),
    /// Error reported by the link-bridge library
    #[error("link-bridge error says: {0:?}")]
    RedirectorError(#[from] link_bridge::RedirectorError),
    /// Error reported by IO library
    #[error("io error says: {0:?}")]
    Io(#[from] std::io::Error),
    /// Error reported by the serde_json library
    #[error("serde_json create_session error says: {0:?}")]
    SerdeJsonError(#[from] serde_json::error::Error),
    /// Draft posts not allowed
    #[error("processing of draft posts is not allowed")]
    DraftNotAllowed,

    /// Post too old
    #[error("Post is older than allowed by minimum date setting {0}")]
    PostTooOld(Datetime),

    /// Error reported by the Toml library.
    #[error("toml deserialization error says: {0:?}")]
    Toml(#[from] toml::de::Error),

    /// Error reported by the Url library.
    #[error("url error says: {0:?}")]
    UrlParse(#[from] url::ParseError),
}

/// Type representing the blog post.
#[derive(Debug, Clone)]
pub(super) struct BlogPost {
    /// The path to the original blog post.
    path: PathBuf,
    /// The front matter from the blog post that is salient
    /// to the production of bluesky posts.
    frontmatter: front_matter::FrontMatter,
    /// The full link to the post.
    post_link: Url,
    /// The short link redirection HTML string
    redirector: Redirector,
    /// The generated short link URL for the post.
    post_short_link: Option<Url>,
    /// Count of bluesky post writing, increment each time a post is written.
    bluesky_count: u8,
}

/// Report values in private fields
impl BlogPost {
    pub fn title(&self) -> &str {
        self.frontmatter.title()
    }

    #[cfg(test)]
    pub fn bluesky_count(&self) -> u8 {
        self.bluesky_count
    }
}

impl BlogPost {
    pub fn new(
        blog_path: &PathBuf,
        min_date: Datetime,
        allow_draft: bool,
        base_url: &Url,
        www_src_root: &Path,
    ) -> Result<BlogPost, BlogPostError> {
        let blog_file = www_src_root.join(blog_path);

        let frontmatter = match front_matter::FrontMatter::read(&blog_file, min_date, allow_draft) {
            Ok(fm) => fm,
            Err(e) => match e {
                front_matter::FrontMatterError::DraftNotAllowed => {
                    return Err(BlogPostError::DraftNotAllowed)
                }
                front_matter::FrontMatterError::PostTooOld(md) => {
                    return Err(BlogPostError::PostTooOld(md))
                }
                front_matter::FrontMatterError::Io(e) => return Err(BlogPostError::Io(e)),
                front_matter::FrontMatterError::Toml(e) => return Err(BlogPostError::Toml(e)),
            },
        };

        let mut post_link = blog_path.clone();
        post_link.set_extension("");

        log::trace!("Post link with extension stripped: `{post_link:?}`");
        // Strip root and content prefix
        let post_link = post_link.as_path().to_string_lossy().to_string();
        log::trace!("Post link as string: `{post_link}`");
        let post_link = post_link
            .trim_start_matches(&www_src_root.to_string_lossy().to_string())
            .trim_start_matches('/')
            .trim_start_matches("content");

        log::trace!("Post link as trimmed: `{post_link}`");

        let link = base_url.join(post_link)?;

        // Initialise the short link html redirector
        let redirector = Redirector::new(post_link)?;

        Ok(BlogPost {
            path: blog_path.clone(),
            frontmatter,
            post_link: link,
            redirector,
            post_short_link: None,
            bluesky_count: 0,
        })
    }

    /// Get bluesky record based on frontmatter data
    pub async fn get_bluesky_record(&self) -> Result<RecordData, BlogPostError> {
        log::info!("Blog post: {self:#?}");
        log::debug!("Building post text");
        let post_text = self.build_post_text()?;

        log::trace!("Post text: {post_text}");

        let rt = RichText::new_with_detect_facets(&post_text).await?;

        log::trace!("Rich text: {rt:#?}");

        let record_data = RecordData {
            created_at: BskyDatetime::now(),
            embed: None,
            entities: None,
            facets: rt.facets,
            labels: None,
            langs: None,
            reply: None,
            tags: None,
            text: rt.text,
        };

        log::trace!("{record_data:?}");

        Ok(record_data)
    }

    fn build_post_text(&self) -> Result<String, BlogPostError> {
        log::debug!(
            "Building post text with post dir: `{}`",
            self.path.display()
        );

        if log::log_enabled!(log::Level::Debug) {
            self.log_post_details();
        }

        let post_text = format!(
            "{}\n\n{} {}\n\n{}",
            self.frontmatter.title(),
            self.frontmatter.bluesky_description(),
            self.frontmatter.bluesky_tags().join(" "),
            if let Some(sl) = self.post_short_link.as_ref() {
                sl
            } else {
                &self.post_link
            }
        );

        if post_text.len() > 300 {
            return Err(BlogPostError::PostTooManyCharacters(
                self.frontmatter.title().to_string(),
                post_text.len(),
            ));
        }

        if post_text.graphemes(true).count() > 300 {
            return Err(BlogPostError::PostTooManyGraphemes(
                self.frontmatter.title().to_string(),
                post_text.graphemes(true).count(),
            ));
        }

        Ok(post_text)
    }

    /// Write the referrer file to the `store_dir` location.
    pub fn write_referrer_file_to(
        &mut self,
        store_dir: &Path,
        base_url: &Url,
        root: &Path,
    ) -> Result<(), BlogPostError> {
        log::debug!("Building link with `{base_url}` as root of url",);

        self.redirector.set_path(store_dir);

        let short_link = self.redirector.write_redirect()?;
        log::debug!("redirect written and short link returned: {short_link}");

        self.post_short_link = Some(
            base_url.join(
                short_link
                    .trim_start_matches(&root.to_string_lossy().to_string())
                    .trim_start_matches("/")
                    .trim_start_matches("static/"),
            )?,
        );
        log::debug!("Saved short post link {:#?}", self.post_short_link);
        Ok(())
    }

    /// Write the bluesky record to the `store_dir` location.
    /// The write function generates a short name based on post link
    /// and filename to ensure that similarly named posts have unique
    /// bluesky post names.
    pub async fn write_bluesky_record_to(&mut self, store_dir: &Path) -> Result<(), BlogPostError> {
        // let Some(bluesky_post) = self.bluesky_post.as_ref() else {
        //     return Err(BlogPostError::BlueSkyPostNotConstructed);
        // };
        log::trace!("Store path to write to bluesky record: `{store_dir:#?}`");
        log::trace!(
            "Path for basename contains a filename: {:#?}",
            self.path.is_file()
        );

        let Some(filename) = self.path.as_path().file_name() else {
            return Err(BlogPostError::PostBasenameNotSet);
        };
        let filename = filename.to_str().unwrap();

        let bluesky_post = match self.get_bluesky_record().await {
            Ok(p) => p,
            Err(e) => {
                log::warn!(
                    "failed to create bluesky record for `{}` because `{e}`",
                    self.title()
                );
                return Err(BlogPostError::BlueSkyPostNotConstructed);
            }
        };

        let postname = format!(
            "{}{}{}",
            base62::encode(self.post_link.path().encode_utf16().sum::<u16>()),
            base62::encode(filename.encode_utf16().sum::<u16>()),
            base62::encode(
                self.post_link
                    .path()
                    .trim_end_matches(filename)
                    .encode_utf16()
                    .sum::<u16>()
            )
        );

        log::trace!("Bluesky post: {bluesky_post:#?}");

        let post_file = format!("{postname}.post");
        let post_file = store_dir.to_path_buf().join(post_file);
        log::debug!("Write filename: `{filename}` as `{postname}`");
        log::debug!("Write file: `{}`", post_file.display());

        let file = File::create(post_file)?;

        serde_json::to_writer_pretty(&file, &bluesky_post)?;
        file.sync_all()?;
        self.bluesky_count += 1;

        Ok(())
    }

    fn log_post_details(&self) {
        log::debug!("Post link: {}", self.post_link);
        log::debug!(
            "Length of post link: {} characters and {} graphemes",
            self.post_link.as_str().len(),
            self.post_link.as_str().graphemes(true).count()
        );
        log::debug!(
            "Length of post short link: {} characters and {} graphemes",
            self.post_short_link
                .as_ref()
                .map_or(0, |link| link.as_str().len()),
            self.post_short_link
                .as_ref()
                .map_or(0, |link| link.as_str().graphemes(true).count())
        );
        self.frontmatter.log_post_details();
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, str::FromStr};

    use log::LevelFilter;
    use toml::value::Datetime;

    use super::*;
    use crate::util::test_utils;

    fn create_test_blog_file(path: &Path, filename: &str, content: &str) -> PathBuf {
        if !path.exists() {
            fs::create_dir_all(path).unwrap();
        }

        let file_path = path.join(filename);
        fs::write(&file_path, content).expect("Failed to write test file");
        file_path
    }

    fn create_test_frontmatter_content() -> String {
        r#"+++
title = "Test Blog Post"
date = 2024-01-15
description = "A test blog post for unit testing"
draft = false
[taxonomies]
tags = ["rust", "testing"]
+++"#
            .to_string()
    }

    #[test]
    fn test_blog_post_new_success() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Trace);
        let content = format!(
            "{}\n\nThis is the blog post content.",
            create_test_frontmatter_content()
        );
        log::debug!("Blog post content: {content:#?}");

        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "test-post.md", &content);
        log::debug!("Path to blog_file: `{blog_file:?}`");

        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();
        log::debug!("Minimum date: {min_date:#?}");

        let result = BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path());
        log::debug!("BlogPost::new result: {result:?}");

        assert!(result.is_ok());
        let post = result.unwrap();
        assert_eq!(post.title(), "Test Blog Post");
        assert_eq!(post.bluesky_count(), 0);
        assert_eq!(
            post.post_link.as_str(),
            "https://www.example.com/blog/test-post"
        );
    }

    #[test]
    fn test_blog_post_new_with_directory_path() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Debug);
        let content_dir = temp_dir.path().join("content").join("posts");
        fs::create_dir_all(&content_dir).unwrap();

        let blog_path = PathBuf::from("content/posts/");
        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();

        let result = BlogPost::new(&blog_path, min_date, false, &base_url, temp_dir.path());

        if let Ok(post) = result {
            assert_eq!(
                post.post_link.as_str(),
                "https://www.example.com/blog/posts/"
            );
        }
    }

    #[test]
    fn test_blog_post_draft_not_allowed_error() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Debug);
        let mut content = create_test_frontmatter_content();
        content = content.replace("draft = false", "draft = true");
        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "draft-post.md", &content);

        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();

        let result = BlogPost::new(
            &blog_file,
            min_date,
            false, // don't allow drafts
            &base_url,
            temp_dir.path(),
        );
        log::debug!("Result of new post generation:/n{result:#?}");
        assert!(matches!(result, Err(BlogPostError::DraftNotAllowed)));
    }

    #[test]
    fn test_blog_post_too_old_error() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Debug);
        let content = create_test_frontmatter_content();
        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "old-post.md", &content);

        let min_date = Datetime::from_str("2024-12-01T00:00:00Z").unwrap(); // Future date

        let result = BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path());

        assert!(matches!(result, Err(BlogPostError::PostTooOld(_))));
    }

    #[tokio::test]
    async fn test_get_bluesky_record_success() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Debug);
        let content = format!(
            "{}\n\nThis is the blog post content.",
            create_test_frontmatter_content()
        );
        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "test-post.md", &content);

        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();

        let post = BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path()).unwrap();

        let result = post.get_bluesky_record().await;
        assert!(result.is_ok());

        let record = result.unwrap();
        assert!(record.text.contains("Test Blog Post"));
        assert!(record
            .text
            .contains("https://www.example.com/blog/test-post"));
        assert!(record.text.len() <= 300);
    }

    #[tokio::test]
    async fn test_build_post_text_format() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Trace);
        let content = format!(
            "{}\n\nThis is the blog post content.",
            create_test_frontmatter_content()
        );
        log::debug!("Content of blog file: `{content}`");
        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "format-test.md", &content);
        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();

        let post = BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path()).unwrap();

        let post_text = post.build_post_text().unwrap();
        log::debug!("Generated post text:\n{post_text:#?}");

        // Check format: title + double newline + description + tags + double newline +
        // link
        let lines: Vec<&str> = post_text.split("\n\n").collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Test Blog Post");
        assert!(lines[1].contains("A test blog post for unit testing"));
        assert!(lines[1].contains("#Rust #Testing"));
        assert_eq!(lines[2], "https://www.example.com/blog/format-test");
    }

    #[tokio::test]
    async fn test_post_text_too_many_characters() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Trace);

        // Create content that will exceed 300 characters
        let long_description = "A".repeat(250);
        let long_content = format!(
            r#"+++
title = "Very Long Title That Will Cause Character Limit Issues"
date = 2024-01-15T10:30:00Z
description = "{long_description}"
tags = ["verylongtag1", "verylongtag2", "verylongtag3", "verylongtag4"]
draft = false
+++

Long content here."#
        );

        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "long-post.md", &long_content);

        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();

        let post_res = BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path());
        log::debug!("Post result: {post_res:#?}");
        let post = post_res.unwrap();

        let result = post.build_post_text();
        log::debug!("Result: {result:?}");
        assert!(matches!(
            result,
            Err(BlogPostError::PostTooManyCharacters(_, _))
        ));
    }

    #[test]
    fn test_write_referrer_file_to() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Debug);
        let content = format!("{}\n\nContent here.", create_test_frontmatter_content());
        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "referrer-test.md", &content);

        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();
        let store_dir = temp_dir.path().join("static");
        fs::create_dir_all(&store_dir).unwrap();

        let mut post =
            BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path()).unwrap();

        assert!(post.post_short_link.is_none());

        let result = post.write_referrer_file_to(&store_dir, &base_url, temp_dir.path());
        assert!(result.is_ok());
        assert!(post.post_short_link.is_some());
        let short_link = post.post_short_link.unwrap();
        log::debug!("The short link is: `{:?}`", short_link.as_str());
        assert!(short_link.as_str().starts_with("https://www.example.com/"));
    }

    #[tokio::test]
    async fn test_write_bluesky_record_to() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Debug);
        let content = format!("{}\n\nContent here.", create_test_frontmatter_content());
        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "bluesky-test.md", &content);

        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();
        let store_dir = temp_dir.path().join("posts");
        fs::create_dir_all(&store_dir).unwrap();

        let mut post =
            BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path()).unwrap();

        assert_eq!(post.bluesky_count(), 0);

        let result = post.write_bluesky_record_to(&store_dir).await;
        assert!(result.is_ok());
        assert_eq!(post.bluesky_count(), 1);

        // Check that a .post file was created
        let post_files: Vec<_> = fs::read_dir(&store_dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()? == "post" {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(post_files.len(), 1);

        // Verify the JSON content
        let json_content = fs::read_to_string(&post_files[0]).unwrap();
        let record_data: serde_json::Value = serde_json::from_str(&json_content).unwrap();
        log::debug!("Record data: `{record_data}`");
        assert!(record_data.get("text").is_some());
        assert!(record_data.get("createdAt").is_some());
    }

    #[tokio::test]
    async fn test_write_bluesky_record_multiple_times() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Debug);
        let content = format!("{}\n\nContent here.", create_test_frontmatter_content());
        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "multi-test.md", &content);

        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();
        let store_dir = temp_dir.path().join("posts");
        fs::create_dir_all(&store_dir).unwrap();

        let mut post =
            BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path()).unwrap();

        // Write multiple times
        post.write_bluesky_record_to(&store_dir).await.unwrap();
        assert_eq!(post.bluesky_count(), 1);

        post.write_bluesky_record_to(&store_dir).await.unwrap();
        assert_eq!(post.bluesky_count(), 2);

        post.write_bluesky_record_to(&store_dir).await.unwrap();
        assert_eq!(post.bluesky_count(), 3);
    }

    // #[tokio::test]
    // async fn test_post_basename_not_set_error() {
    //     let (temp_dir, base_url) =
    // test_utils::setup_test_environment(LevelFilter::Trace);     let content =
    // format!("{}\n\nContent here.", create_test_frontmatter_content());
    //     let blog_path = temp_dir.path().join("content").join("blog/");
    //     let _blog_file = create_test_blog_file(&blog_path, "basename-test.md",
    // &content);

    //     // Create a blog path that doesn't have a filename
    //     let _min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();
    //     let store_dir = temp_dir.path().join("posts");
    //     let mut fm = FrontMatter::new("Test", "Test desc");
    //     let taxonomies =
    // front_matter::Taxonomies::new(vec!["#test".to_string()]);
    //     fm.taxonomies = Some(taxonomies);
    //     fs::create_dir_all(&store_dir).unwrap();

    //     let mut post = BlogPost {
    //         path: blog_path,
    //         frontmatter: fm,
    //         post_link: base_url.clone(),
    //         redirector: link_bridge::Redirector::new("/test").unwrap(),
    //         post_short_link: None,
    //         bluesky_count: 0,
    //     };

    //     let result = post.write_bluesky_record_to(&store_dir).await;
    //     log::debug!("Result says: `{result:?}`");
    //     assert!(matches!(result, Err(BlogPostError::PostBasenameNotSet)));
    // }

    #[test]
    fn test_error_display_formatting() {
        let error = BlogPostError::PostTooManyCharacters("Test Post".to_string(), 350);
        assert!(format!("{error}").contains("Test Post"));
        assert!(format!("{error}").contains("350"));

        let error = BlogPostError::PostTooManyGraphemes("Another Post".to_string(), 400);
        assert!(format!("{error}").contains("Another Post"));
        assert!(format!("{error}").contains("400"));

        let date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();
        let error = BlogPostError::PostTooOld(date);
        assert!(format!("{error}").contains("2024-01-01T00:00:00Z"));
    }

    #[test]
    fn test_blog_post_accessors() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Debug);
        let content = format!("{}\n\nContent here.", create_test_frontmatter_content());
        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "accessor-test.md", &content);

        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();

        let post = BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path()).unwrap();

        assert_eq!(post.title(), "Test Blog Post");
        assert_eq!(post.bluesky_count(), 0);
    }

    #[test]
    fn test_unicode_grapheme_handling() {
        let (temp_dir, base_url) = test_utils::setup_test_environment(LevelFilter::Debug);

        // Create content with Unicode characters that have different byte vs grapheme
        // counts
        let unicode_content = r##"+++
title = "Test 👋 Post 🦀"
date = 2024-01-15T10:30:00Z
description = "Testing unicode: 🚀 émojis and àccénts"
tags = ["#test", "#unicode"]
draft = false
+++

Content with unicode characters."##
            .to_string();

        let blog_path = temp_dir.path().join("content").join("blog");
        let blog_file = create_test_blog_file(&blog_path, "unicode-test.md", &unicode_content);

        let min_date = Datetime::from_str("2024-01-01T00:00:00Z").unwrap();

        let post = BlogPost::new(&blog_file, min_date, false, &base_url, temp_dir.path()).unwrap();

        let post_text = post.build_post_text().unwrap();

        // Verify both character and grapheme counts are within limits
        assert!(post_text.len() <= 300);
        assert!(post_text.graphemes(true).count() <= 300);

        // Verify Unicode characters are preserved
        assert!(post_text.contains("👋"));
        assert!(post_text.contains("🦀"));
        assert!(post_text.contains("🚀"));
        assert!(post_text.contains("émojis"));
        assert!(post_text.contains("àccénts"));
    }
}
