use std::path::PathBuf;

mod blog_post;
mod draft_builder;

use blog_post::BlogPost;
use draft_builder::DraftBuilder;
use thiserror::Error;
use url::Url;

/// Error types that can occur during draft processing operations.
///
/// This enum represents all possible errors that may arise when drafting
/// bluesky posts for blogs, including file system operations, validation
/// errors, and parsing failures. The enum is marked as `#[non_exhaustive]` to
/// allow for future error variants without breaking existing code.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum DraftError {
    /// Indicates that a future capacity allocation would exceed system limits.
    #[error("Future capacity is too large")]
    FutureCapacityTooLarge,

    /// The specified file or directory path could not be found.
    #[error("path not found: `{0}`")]
    PathNotFound(String),

    /// A file has an incorrect extension for blog post processing.
    #[error("file extension invalid (must be `{1}`): {0}")]
    FileExtensionInvalid(String, String),

    /// No blog posts were found to process.
    #[error("blog post list is empty")]
    BlogPostListEmpty,

    /// No blog posts remain after applying filtering criteria.
    #[error("blog post list is empty after qualifications have been applied")]
    QualifiedBlogPostListEmpty,

    /// An I/O operation failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// URL parsing failed.
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// TOML datetime parsing failed.
    #[error("TOML datetime parse error: {0}")]
    TomlDatetimeParse(#[from] toml::value::DatetimeParseError),
}

/// Configuration for generating bluesky posts for blog posts.
///
/// The `Draft` struct encapsulates all the necessary configuration and data
/// required to generate, and save draft bluesky posts.
///
/// This struct is marked as `#[non_exhaustive]` to allow for future expansion
/// of configuration options without breaking existing code that constructs
/// or matches on `Draft` instances.
///
/// # Structure
///
/// The `Draft` configuration includes:
/// - A collection of blog posts to process
/// - Storage locations for social media and referrer data
/// - Base URL for generating absolute links
/// - Root directory for file operations
///
/// # Usage Patterns
///
/// Typically used as the first step in the two step generation and posting
/// process.
///
/// ```rust should_panic
/// # use std::path::PathBuf;
/// #
/// # use url::Url;
/// # use toml::value::Datetime;
/// #
/// # use gen_bsky::{Draft, DraftError};
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), DraftError> {
///     let base_url = Url::parse("https://www.example.com/")?;
///     let paths = vec!["content/blog".to_string()];
///     let date = Datetime {
///                   date: Some(toml::value::Date{
///                               year: 2025,
///                               month: 8,
///                               day: 4}),
///                   time: None,
///                   offset: None};
///     let allow_draft = false;
///
///     let mut posts = get_post_drafts(
///                         base_url,
///                         paths,
///                         date,
///                         allow_draft).await?;
///    
///     posts.write_referrers(None)?;
///     posts.write_bluesky_posts(None).await?;
///
///     Ok(())
///  }
///
///  async fn get_post_drafts(
///             base_url: Url,
///             paths: Vec<String>,
///             date: Datetime,
///             allow_draft: bool) -> Result<Draft, DraftError>
/// {
///     let post_store = PathBuf::new().join("bluesky_post_store");
///     let referrer_store = PathBuf::new().join("static").join("s");
///
///     let mut builder = Draft::builder(base_url, None);
///    
///     // Add the paths specified at the command line.
///     for path in paths.iter() {
///         builder.add_path_or_file(path)?;
///     }
///    
///     // Set the filters for blog posts
///     builder
///     .with_post_store(post_store)?
///     .with_referrer_store(referrer_store)?
///     .with_minimum_date(date)?
///     .with_allow_draft(allow_draft);
///    
///     builder.build().await
///
///  }
/// ```
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Draft {
    /// Collection of blog posts to be processed.
    ///
    /// This vector contains all the blog posts that will be included in
    /// the draft generation process. Posts may be filtered, sorted, or
    /// otherwise processed based on various criteria during operation.
    blog_posts: Vec<BlogPost>,

    /// File system path for storing Bluesky social media integration data.
    ///
    /// This path points to the directory where draft Bluesky posts are stored.
    /// The directory should be writable by the application. The directory will
    /// be created if it doesn't exist.
    ///
    /// The path should be within the repository so that the draft posts can
    /// retained until the post process is run.
    ///
    /// The default path is set to `bluesky`.
    bsky_store: PathBuf,

    /// File system path for storing referrer tracking data.
    ///
    /// This directory stores a generated referrer link for each blog post to
    /// provided a shortened https link to the post for inclusion in the Bluesky
    /// post. The link should be stored in the appropriate directory to copy the
    /// generated html file to the published website.
    ///
    /// The default path is set to `static/s`
    referrer_store: PathBuf,

    /// Base URL for the blog or website.
    ///
    /// This URL serves as the foundation for generating absolute URLs.
    /// It should include the scheme (http/https)
    /// and domain, but typically not include paths beyond the root.
    ///
    /// A valid Url must be set to create the builder.
    base_url: Url,

    /// Root directory for blog content and file operations.
    ///
    /// This path represents the base directory where blog content, templates,
    /// static files, and other blog-related files are located. The root
    /// directory will be pretended to the bsky_store and referrer_store to
    /// ensure that stores are within the context of the website code.
    ///
    /// The default path is empty.
    root: PathBuf,
}

impl Draft {
    /// Start a builder for the draft struct.
    ///
    /// ## Parameters
    ///
    /// - `base_url`: the base url for the website (e.g. `https://wwww.example.com/`)
    /// - `store`: the location to store draft posts (e.g. `bluesky`)
    pub fn builder(base_url: Url, root: Option<&PathBuf>) -> DraftBuilder {
        DraftBuilder::new(base_url, root)
    }

    /// Write referrer html files for blog posts.
    /// The referrer html file provides a self-hosted short link that
    pub fn write_referrers(&mut self, referrer_store: Option<PathBuf>) -> Result<(), DraftError> {
        let referrer_store = if let Some(p) = referrer_store.as_deref() {
            p
        } else {
            self.referrer_store.as_ref()
        };

        let referrer_store = self.root.join(referrer_store);

        if !referrer_store.exists() {
            std::fs::create_dir_all(&referrer_store)?;
        }

        for blog_post in &mut self.blog_posts {
            match blog_post.write_referrer_file_to(&referrer_store, &self.base_url) {
                Ok(_) => continue,
                Err(e) => {
                    log::warn!(
                        "Blog post: `{}` skipped because of error `{e}`",
                        blog_post.title()
                    );
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Write Bluesky posts for the front matter.
    pub async fn write_bluesky_posts(
        &self,
        bluesky_post_store: Option<PathBuf>,
    ) -> Result<(), DraftError> {
        // create store directory if it doesn't exist
        let bluesky_post_store = if let Some(p) = bluesky_post_store.as_deref() {
            p
        } else {
            self.bsky_store.as_ref()
        };

        let bluesky_post_store = self.root.join(bluesky_post_store);

        if !bluesky_post_store.exists() {
            std::fs::create_dir_all(&bluesky_post_store)?;
        }

        for blog_post in &self.blog_posts {
            match blog_post.write_bluesky_record_to(&bluesky_post_store).await {
                Ok(_) => continue,
                Err(e) => {
                    log::warn!(
                        "Blog post: `{}` skipped because of error `{e}`",
                        blog_post.title()
                    );
                    continue;
                }
            }
        }

        Ok(())
    }
}
