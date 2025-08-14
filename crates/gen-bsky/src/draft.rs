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
    /// Creates a new builder for drafting Bluesky posts for one or more blog
    /// posts.
    ///
    /// This function provides the entry point for writing draft Bluesky posts
    /// and referrer short links using the builder pattern. The builder allows
    /// for step-by-step configuration of all required and optional parameters
    /// before constructing the final `Draft`.
    ///
    /// # Parameters
    ///
    /// * `base_url` - The base URL for the website or blog. This should include
    ///   the scheme (http/https) and domain, and will be used for generating
    ///   absolute URLs. The URL should typically end with a trailing slash for
    ///   proper path joining.
    ///
    /// * `root` - An optional path to the root directory for blog content and
    ///   operations. If `None`, the root of the repository will be used as the
    ///   default. If provided, this path will serve as the base for all
    ///   relative file operations.
    ///
    /// # Returns
    ///
    /// Returns a `DraftBuilder` instance that can be used to configure
    /// additional settings before building the final `Draft`.
    ///
    /// # Examples
    ///
    /// ## Basic Usage with Required Parameters
    ///
    /// ```
    /// # use std::path::PathBuf;
    /// # use url::Url;
    /// # use gen_bsky::{Draft, DraftError};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), DraftError > {
    /// let base_url = Url::parse("https://myblog.example.com/")?;
    ///
    /// let draft_res = Draft::builder(base_url, None).build().await;
    /// draft_res.is_err(); // as there are no blog posts
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## With Custom Root Directory
    ///
    /// ```
    /// # use std::path::PathBuf;
    /// # use url::Url;
    /// # use gen_bsky::{Draft, DraftError};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), DraftError > {
    /// let base_url = Url::parse("https://blog.company.com")?;
    /// let website_root = PathBuf::from("www_root");
    ///
    /// let draft_res =
    ///     Draft::builder(base_url, Some(&website_root)).build().await;
    /// draft_res.is_err(); // as there are no blog posts
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Complete Builder Chain (Typical Usage)
    ///
    /// ```should_panic
    /// # use std::path::PathBuf;
    /// # use url::Url;
    /// # use gen_bsky::{Draft, DraftError};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), DraftError > {
    /// let base_url = Url::parse("https://myblog.example.com/")?;
    /// let root_dir = PathBuf::from("www");
    /// let referrer_store = PathBuf::from("static/short");
    /// let bluesky_store = PathBuf::from("post_store");
    ///
    /// let mut draft = Draft::builder(base_url, Some(&root_dir))
    ///     .add_path_or_file("content/blog")?
    ///     .build() // assuming build found blog posts
    ///     .await?;
    ///
    /// draft.write_referrers(Some(referrer_store))?;
    /// draft.write_bluesky_posts(Some(bluesky_store)).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # URL Format Guidelines
    ///
    /// The `base_url` parameter should follow these guidelines:
    ///
    /// - **Include scheme**: Always use `http://` or `https://`
    /// - **Include domain**: The full domain name (e.g., `example.com`)
    /// - **Trailing slash**: Recommended for proper URL joining (e.g., `https://example.com/`)
    /// - **No path components**: Keep it to the root domain unless you have a
    ///   specific subdirectory setup
    ///
    /// ## Good URL Examples
    /// - `https://myblog.com/`
    /// - `https://blog.company.com/`
    /// - `http://localhost:8080/` (development)
    /// - `https://user.github.io/blog/` (if blog is in a subdirectory)
    ///
    /// ## Avoid These URL Patterns
    /// - `example.com` (missing scheme)
    /// - `https://example.com/posts` (unnecessary path component)
    /// - `https://example.com//` (double trailing slash)
    ///
    /// # Root Directory behaviour
    ///
    /// When `root` is:
    /// - **`Some(path)`**: Uses the provided path as the base directory for all
    ///   file operations
    /// - **`None`**: The builder will use a default root directory (typically
    ///   the current working directory or a configured default)
    ///
    /// The root directory affects:
    /// - Where blog post files are searched for
    /// - Root for Bluesky post drafts
    /// - Root for referrer link html files
    pub fn builder(base_url: Url, root: Option<&PathBuf>) -> DraftBuilder {
        DraftBuilder::new(base_url, root)
    }

    /// Generates referrer HTML files for all blog posts in the collection.
    ///
    /// This method creates self-hosted short link HTML files that serve as
    /// referrer endpoints for blog posts. Each referrer file acts as a
    /// redirect mechanism forwarding visitors to the actual blog post content.
    ///
    /// # Purpose
    ///
    /// Referrer files enable:
    /// - **Short links**: Provide shorter, cleaner URLs for sharing
    /// - **Redirect control**: Manage redirects without external services
    ///
    /// When a referrer file is successfully created that link will be used when
    /// generating the Bluesky post as the link to the blog to post. This saves
    /// characters in the Bluesky post for the title, description and tags.
    ///
    /// # Parameters
    ///
    /// * `referrer_store` - Optional override for the referrer storage
    ///   directory.
    ///   - If `Some(path)`, uses the provided path relative to the root
    ///     directory
    ///   - If `None`, uses the default `referrer_store` configured in `Draft`
    ///   - The path is always resolved relative to the `root` directory
    ///
    /// # behaviour
    ///
    /// 1. **Directory Resolution**: Determines the target directory using
    ///    either the provided override or the configured default
    /// 2. **Directory Creation**: Ensures the referrer storage directory
    ///    exists, creating it if necessary
    /// 3. **File Generation**: Iterates through all blog posts and generates
    ///    referrer HTML files for each
    /// 4. **Error Handling**: Logs warnings for individual post failures but
    ///    continues processing remaining posts
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful completion, even if some individual blog
    /// posts failed to generate referrer files. Returns `Err(DraftError)`
    /// only for critical failures like directory creation errors.
    ///
    /// # Errors
    ///
    /// This method can return errors for:
    /// - **I/O failures**: Directory creation or file system access issues
    /// - **Permission errors**: Insufficient permissions to create directories
    ///   or files
    /// - **Path resolution**: Issues with path construction or validation
    ///
    /// Individual blog post processing errors are logged as warnings but do not
    /// cause the method to fail.
    ///
    /// # Generated File Structure
    ///
    /// The method creates a directory structure like:
    ///
    /// ```text
    /// <root>/
    /// └── <referrer_store>/
    ///     ├── aaBB23.html      # Referrer file for first post
    ///     ├── ZZ24ss.html      # Referrer file for second post
    ///     └── ...              # Additional referrer files
    /// ```
    ///
    /// Each HTML file contains redirect logic to forward visitors to the actual
    /// blog post.
    ///
    /// # URL Structure
    ///
    /// Referrer URLs typically follow the pattern:
    /// - **Referrer URL**: `{base_url}/{referrer_store}/{post-slug}.html`
    /// - **Target URL**: `/{path-to-post}/{post-slug}` (or similar)
    ///
    /// # Error Handling Strategy
    ///
    /// The method uses a "best effort" approach:
    /// - **Critical errors** (directory creation): Method fails immediately
    /// - **Individual post errors**: Logged as warnings, processing continues
    /// - **Final result**: Success if directory operations succeed, regardless
    ///   of individual post failures
    ///
    /// This ensures that partial failures don't prevent the generation of
    /// referrer files for posts that can be processed successfully.
    ///
    /// # Logging
    ///
    /// Failed blog post processing is logged at the `WARN` level with the
    /// format: ```text
    /// Blog post: `<post_title>` skipped because of error `<error_details>`
    /// ```
    ///
    /// Ensure your logging framework is configured to capture these warnings
    /// for debugging purposes.
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
