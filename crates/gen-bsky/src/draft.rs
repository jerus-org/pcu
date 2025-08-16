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
    /// format:
    /// ```text
    /// Blog post: `<post_title>` skipped because of error `<error_details>`
    /// ```
    ///
    /// Ensure your logging framework is configured to capture these warnings
    /// for debugging purposes.
    pub fn write_referrers(
        &mut self,
        referrer_store: Option<PathBuf>,
    ) -> Result<&mut Self, DraftError> {
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

        Ok(self)
    }

    /// Generates Bluesky social media posts for all blog posts in the
    /// collection.
    ///
    /// This async method creates Bluesky post records based on blog post
    /// frontmatter and metadata. Each record contains the necessary
    /// information to publish posts on the Bluesky social media platform,
    /// enabling automated social media promotion of blog content.
    ///
    /// # Purpose
    ///
    /// Bluesky post generation enables:
    /// - **Automated social promotion**: Generate social media posts from blog
    ///   content
    /// - **Consistent branding**: Standardized post format across all blog
    ///   posts
    /// - **Scheduled publishing**: Create posts that can be published at
    ///   optimal times
    /// - **Content integration**: Link social media presence with blog content
    /// - **Batch processing**: Generate multiple posts efficiently in a single
    ///   operation
    ///
    /// # Parameters
    ///
    /// * `bluesky_post_store` - Optional override for the Bluesky storage
    ///   directory.
    ///   - If `Some(path)`, uses the provided path relative to the root
    ///     directory
    ///   - If `None`, uses the default `[BSKY_STORE]` configured in `Draft`
    ///   - The path is always resolved relative to the `root` directory
    ///
    /// # behaviour
    ///
    /// 1. **Directory Resolution**: Determines the target directory using
    ///    either the provided override or the configured default
    /// 2. **Directory Creation**: Ensures the Bluesky storage directory exists,
    ///    creating it if necessary
    /// 3. **Async Processing**: Asynchronously processes each blog post to
    ///    generate Bluesky records
    /// 4. **Error Handling**: Logs warnings for individual post failures but
    ///    continues processing remaining posts
    /// 5. **Best Effort**: Completes successfully even if some posts fail to
    ///    process
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful completion, even if some individual blog
    /// posts failed to generate Bluesky records. Returns `Err(DraftError)`
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
    /// cause the method to fail. These might include:
    /// - Missing required frontmatter fields
    /// - Invalid post content or metadata
    /// - Network issues (if fetching external data)
    ///
    /// # Generated File Structure
    ///
    /// The method creates a directory structure like:
    ///
    /// ```text
    /// <root>/
    /// └── <bluesky_post_store>/
    ///     ├── ppOO33.json      # Bluesky record for first post
    ///     ├── qwRWlu.json      # Bluesky record for second post
    ///     └── ...                   # Additional post records
    /// ```
    ///
    /// Each JSON file contains structured data that can be consumed by Bluesky
    /// publishing tools or APIs.
    ///
    /// # Async Considerations
    ///
    /// This method is async because:
    /// - **Individual processing**: Each `write_bluesky_record_to` call is
    ///   async
    /// - **I/O operations**: File writing operations may be async
    /// - **External APIs**: May fetch data from external services
    ///
    /// # Logging
    ///
    /// Failed blog post processing is logged at the `WARN` level with the
    /// format:
    /// ```text
    /// Blog post: `<post_title>` skipped because of error `<error_details>`
    /// ```
    ///
    /// Ensure your logging framework is configured to capture these warnings
    /// for debugging and monitoring purposes.
    pub async fn write_bluesky_posts(
        &mut self,
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

        for blog_post in self.blog_posts.iter_mut() {
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

/// Returns the current date as a TOML-compatible datetime value.
///
/// This function creates a `toml::value::Datetime` representing today's date
/// in UTC, without any time or timezone offset information. It's useful for
/// creating date-only values that can be serialized to TOML format or used
/// in TOML frontmatter.
///
///
/// # Returns
///
/// Returns a `toml::value::Datetime` with:
/// - `date`: Some(Date) containing the current UTC year, month, and day
/// - `time`: None (no time component)
/// - `offset`: None (no timezone information)
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// # use toml::value::Datetime;
/// # use chrono::{Utc, Datelike};
/// # fn today() -> toml::value::Datetime {
/// #     use toml::value::Date;
/// #     let date = Date {
/// #         year: Utc::now().year() as u16,
/// #         month: Utc::now().month() as u8,
/// #         day: Utc::now().day() as u8,
/// #     };
/// #     Datetime {
/// #         date: Some(date),
/// #         time: None,
/// #         offset: None,
/// #     }
/// # }
/// let current_date = today();
///
/// // The datetime will have only a date component
/// assert!(current_date.date.is_some());
/// assert!(current_date.time.is_none());
/// assert!(current_date.offset.is_none());
///
/// // Access the date components
/// if let Some(date) = current_date.date {
///     println!(
///         "Today is {}-{:02}-{:02}",
///         date.year, date.month, date.day
///     );
/// }
/// ```
///
/// # Implementation Notes
///
/// The function performs a single call to `Utc::now()` to get the year, month,
/// and day components. While this could theoretically result in returning
/// yesterday's date if called just before midnight UTC, this is extremely
/// unlikely in practice and the impact would be minimal.
pub(crate) fn today() -> toml::value::Datetime {
    use chrono::{Datelike, Utc};
    let now = Utc::now();
    let date = toml::value::Date {
        year: now.year() as u16,
        month: now.month() as u8,
        day: now.day() as u8,
    };

    toml::value::Datetime {
        date: Some(date),
        time: None,
        offset: None,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use std::fs;
    use std::{fs::File, io::Write, path::Path, str::FromStr};

    use log::LevelFilter;
    use tempfile::TempDir;

    use super::blog_post::front_matter::FrontMatter;
    use super::*;

    fn get_test_logger(level: LevelFilter) {
        let mut builder = env_logger::Builder::new();
        builder.filter(None, level);
        builder.format_timestamp_secs().format_module_path(false);
        let _ = builder.try_init();
    }

    // // Mock types for testing
    // #[derive(Debug, PartialEq)]
    // enum DraftError {
    //     Io(std::io::Error),
    // }

    // impl From<std::io::Error> for DraftError {
    //     fn from(e: std::io::Error) -> Self {
    //         DraftError::Io(e)
    //     }
    // }

    // impl std::fmt::Display for DraftError {
    //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //         match self {
    //             DraftError::Io(e) => write!(f, "IO error: {}", e),
    //         }
    //     }
    // }

    // impl std::error::Error for DraftError {}

    // // Mock BlogPost for testing
    // #[derive(Clone)]
    // struct BlogPost {
    //     title: String,
    //     should_fail: bool,
    //     call_count: Arc<Mutex<usize>>,
    // }

    // impl BlogPost {
    //     fn new(title: &str) -> Self {
    //         Self {
    //             title: title.to_string(),
    //             should_fail: false,
    //             call_count: Arc::new(Mutex::new(0)),
    //         }
    //     }

    //     fn new_failing(title: &str) -> Self {
    //         Self {
    //             title: title.to_string(),
    //             should_fail: true,
    //             call_count: Arc::new(Mutex::new(0)),
    //         }
    //     }

    //     fn title(&self) -> &str {
    //         &self.title
    //     }

    //     async fn write_bluesky_record_to(
    //         &self,
    //         _store_path: &std::path::Path,
    //     ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //         // Increment call count
    //         *self.call_count.lock().unwrap() += 1;

    //         if self.should_fail {
    //             Err("Simulated blog post error".into())
    //         } else {
    //             Ok(())
    //         }
    //     }

    //     fn call_count(&self) -> usize {
    //         *self.call_count.lock().unwrap()
    //     }
    // }

    // // Mock Draft struct
    // struct Draft {
    //     bsky_store: PathBuf,
    //     root: PathBuf,
    //     blog_posts: Vec<BlogPost>,
    // }

    // impl Draft {
    //     fn new() -> Self {
    //         Self {
    //             bsky_store: PathBuf::from("bluesky"),
    //             root: PathBuf::from("."),
    //             blog_posts: vec![],
    //         }
    //     }

    //     async fn write_bluesky_posts(
    //         &self,
    //         bluesky_post_store: Option<PathBuf>,
    //     ) -> Result<(), DraftError> {
    //         let bluesky_post_store = if let Some(p) = bluesky_post_store.as_deref() {
    //             p
    //         } else {
    //             self.bsky_store.as_ref()
    //         };

    //         let bluesky_post_store = self.root.join(bluesky_post_store);

    //         if !bluesky_post_store.exists() {
    //             std::fs::create_dir_all(&bluesky_post_store)?;
    //         }

    //         for blog_post in &self.blog_posts {
    //             match blog_post.write_bluesky_record_to(&bluesky_post_store).await {
    //                 Ok(_) => continue,
    //                 Err(e) => {
    //                     // In real implementation, this would use log::warn!
    //                     eprintln!(
    //                         "Blog post: `{}` skipped because of error `{e}`",
    //                         blog_post.title()
    //                     );
    //                     continue;
    //                 }
    //             }
    //         }

    //         Ok(())
    //     }
    // }

    fn setup_test_environment() -> (TempDir, Url) {
        get_test_logger(LevelFilter::Debug);
        let temp_dir = tempfile::tempdir().unwrap();
        log::debug!("Created temp directory: {temp_dir:?}");
        let base_url = Url::from_str("https://www.example.com/").unwrap();

        (temp_dir, base_url)
    }

    fn create_frontmatter_blog_post(dir: &Path, name: &str, front_matter: &FrontMatter) {
        log::debug!(
            "path: `{}`, name: `{name}`, frontmatter: {front_matter:?}",
            dir.display()
        );
        let blog_store = dir.join("content").join("blog");

        if !blog_store.exists() {
            log::debug!("creating blog store: `{}`", blog_store.display());
            std::fs::create_dir_all(&blog_store).unwrap();
        }

        let blog_name = blog_store.join(name);

        let mut fd = File::create(blog_name).unwrap();
        let buffer = format!("+++\n{}+++\n", toml::to_string(front_matter).unwrap());
        fd.write_all(buffer.as_bytes()).unwrap();
    }

    fn create_freeform_blog_post(dir: &Path, name: &str, fm_text: &str) {
        log::debug!(
            "path: `{}`, name: `{name}`, frontmatter: {fm_text:?}",
            dir.display()
        );
        let blog_store = dir.join("content").join("blog");

        if !blog_store.exists() {
            log::debug!("creating blog store: `{}`", blog_store.display());
            std::fs::create_dir_all(&blog_store).unwrap();
        }

        let blog_name = blog_store.join(name);

        let mut fd = File::create(blog_name).unwrap();
        let buffer = format!("+++\n{fm_text}+++\n");
        fd.write_all(buffer.as_bytes()).unwrap();
    }

    #[tokio::test]
    async fn test_write_bluesky_posts_with_default_store() {
        get_test_logger(LevelFilter::Debug);
        let random_name = crate::util::random_name();
        log::debug!("Random name for test directory: `{random_name}`");
        let temp_dir = PathBuf::new().join(random_name);
        fs::create_dir(&temp_dir).unwrap();
        log::trace!("Created temp directory: {temp_dir:?}");
        let base_url = Url::from_str("https://www.example.com/").unwrap();

        let first_post = FrontMatter::new("First Post", "Description of first post");
        let second_post = FrontMatter::new("Second Post", "Description of second post");

        create_frontmatter_blog_post(temp_dir.as_path(), "first-post.md", &first_post);
        create_frontmatter_blog_post(temp_dir.as_path(), "second-post.md", &second_post);

        for entry in temp_dir
            .as_path()
            .join(crate::util::default_blog_dir())
            .read_dir()
            .expect("read_dir call failed")
            .flatten()
        {
            log::debug!("Entry found: `{}`", entry.file_name().to_string_lossy());
        }

        let mut draft = Draft::builder(base_url, Some(&temp_dir))
            .build()
            .await
            .unwrap();

        let result = draft.write_bluesky_posts(None).await;

        assert!(result.is_ok());

        // Verify directory was created
        let expected_path = temp_dir.join("bluesky");
        assert!(expected_path.exists());
        assert!(expected_path.is_dir());

        // Verify all posts were processed
        for post in &draft.blog_posts {
            log::debug!("Checking if written post file: {post:#?}");
            assert_eq!(post.bluesky_count(), 1);
        }

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[tokio::test]
    async fn test_write_bluesky_posts_with_custom_store() {
        get_test_logger(LevelFilter::Debug);
        let temp_dir = tempfile::tempdir().unwrap();
        log::debug!("Created temp directory: {temp_dir:?}");
        let base_url = Url::from_str("https://www.example.com/").unwrap();

        let first_post = FrontMatter::new("Test Post", "Description of test post");
        create_frontmatter_blog_post(temp_dir.path(), "test-post.md", &first_post);

        let mut draft = Draft::builder(base_url, Some(&temp_dir.path().to_path_buf()))
            .build()
            .await
            .unwrap();

        let custom_store = PathBuf::from("custom/bluesky/path");
        let result = draft.write_bluesky_posts(Some(custom_store.clone())).await;

        assert!(result.is_ok());

        // Verify custom directory was created
        let expected_path = temp_dir.path().join(custom_store);
        assert!(expected_path.exists());
        assert!(expected_path.is_dir());

        // Verify post was processed
        assert_eq!(draft.blog_posts[0].bluesky_count(), 1);
    }

    #[tokio::test]
    async fn test_write_bluesky_posts_creates_nested_directories() {
        let (temp_dir, base_url) = setup_test_environment();

        let first_post = FrontMatter::new("Test Post", "Description of test post");
        create_frontmatter_blog_post(temp_dir.path(), "test-post.md", &first_post);

        let mut draft = Draft::builder(base_url, Some(&temp_dir.path().to_path_buf()))
            .build()
            .await
            .unwrap();

        let nested_store = PathBuf::from("deeply/nested/bluesky/directory");
        let result = draft.write_bluesky_posts(Some(nested_store.clone())).await;

        assert!(result.is_ok());

        // Verify nested directory structure was created
        let expected_path = temp_dir.path().join(nested_store);
        assert!(expected_path.exists());
        assert!(expected_path.is_dir());
    }

    #[tokio::test]
    async fn test_write_bluesky_posts_continues_on_individual_failures() {
        let (temp_dir, base_url) = setup_test_environment();

        let post_one = FrontMatter::new("Test Post One will Pass", "This post will pass");
        create_frontmatter_blog_post(temp_dir.path(), "post_1.md", &post_one);
        let post_two = "Title: Test Post Two will Fail";
        create_freeform_blog_post(temp_dir.path(), "post_2.md", post_two);
        let post_three = FrontMatter::new("Test Post Three will Pass", "This post will pass");
        create_frontmatter_blog_post(temp_dir.path(), "post_3.md", &post_three);

        let mut draft = Draft::builder(base_url, Some(&temp_dir.path().to_path_buf()))
            .build()
            .await
            .unwrap();

        let result = draft.write_bluesky_posts(None).await;

        // Method should succeed despite individual post failure
        assert!(result.is_ok());

        // Verify all posts were attempted
        for post in &draft.blog_posts {
            assert_eq!(post.bluesky_count(), 1);
        }

        assert_eq!(2, draft.blog_posts.len());

        // Directory should still be created
        let expected_path = temp_dir.path().join("bluesky");
        assert!(expected_path.exists());
    }

    #[tokio::test]
    async fn test_write_bluesky_posts_empty_blog_posts() {
        let (temp_dir, base_url) = setup_test_environment();

        let result = Draft::builder(base_url, Some(&temp_dir.path().to_path_buf()))
            .build()
            .await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            DraftError::BlogPostListEmpty.to_string()
        );

        // Directory should still be created even with no posts
        let expected_path = temp_dir.path().join("bluesky");
        assert!(!expected_path.exists());
    }

    #[tokio::test]
    async fn test_write_bluesky_posts_existing_directory() {
        let (temp_dir, base_url) = setup_test_environment();

        let post_one = FrontMatter::new("Test Post One will Pass", "This post will pass");
        create_frontmatter_blog_post(temp_dir.path(), "post_1.md", &post_one);

        let mut draft = Draft::builder(base_url, Some(&temp_dir.path().to_path_buf()))
            .build()
            .await
            .unwrap();

        // Pre-create the directory
        let store_path = temp_dir.path().join("bluesky");
        std::fs::create_dir_all(&store_path).unwrap();

        let result = draft.write_bluesky_posts(None).await;

        assert!(result.is_ok());
        assert!(store_path.exists());
        assert_eq!(draft.blog_posts[0].bluesky_count(), 1);
    }

    #[tokio::test]
    async fn test_write_bluesky_posts_path_resolution() {
        let (temp_dir, base_url) = setup_test_environment();

        let post_one = FrontMatter::new("Test Post One will Pass", "This post will pass");
        create_frontmatter_blog_post(temp_dir.path(), "post_1.md", &post_one);

        let mut draft = Draft::builder(base_url, Some(&temp_dir.path().to_path_buf()))
            .build()
            .await
            .unwrap();

        // Test with None (should use default bsky_store)
        let result = draft.write_bluesky_posts(None).await;
        assert!(result.is_ok());

        let default_path = temp_dir.path().join(crate::util::default_bluesky_dir());
        assert!(default_path.exists());

        // Test with Some (should use override)
        let override_path = PathBuf::from("override/bsky");
        let result = draft.write_bluesky_posts(Some(override_path.clone())).await;
        assert!(result.is_ok());

        let override_full_path = temp_dir.path().join(override_path);
        assert!(override_full_path.exists());
    }

    #[tokio::test]
    async fn test_write_bluesky_posts_multiple_calls() {
        let (temp_dir, base_url) = setup_test_environment();

        let post_one = FrontMatter::new("Test Post One will Pass", "This post will pass");
        create_frontmatter_blog_post(temp_dir.path(), "post_1.md", &post_one);
        let post_two = FrontMatter::new("Test Post two will Pass", "This post will pass");
        create_frontmatter_blog_post(temp_dir.path(), "post_2.md", &post_two);

        let mut draft = Draft::builder(base_url, Some(&temp_dir.path().to_path_buf()))
            .build()
            .await
            .unwrap();

        // First call
        let result1 = draft.write_bluesky_posts(None).await;
        assert!(result1.is_ok());

        // Verify first call processed posts
        for post in &draft.blog_posts {
            assert_eq!(post.bluesky_count(), 1);
        }

        // Second call
        let result2 = draft.write_bluesky_posts(None).await;
        assert!(result2.is_ok());

        // Verify second call also processed posts
        for post in &draft.blog_posts {
            assert_eq!(post.bluesky_count(), 2);
        }
    }
}
