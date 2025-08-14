use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

mod blog_post;

use blog_post::BlogPost;
use chrono::{Datelike, Utc};
use serde::Deserialize;
use thiserror::Error;
use toml::value::Datetime;
use url::Url;

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
    Io(#[from] std::io::Error),
    /// Error reported by url crate parse
    #[error("Url says: {0:?}")]
    UrlParse(#[from] url::ParseError),
    /// Error reported by toml
    #[error("Toml Datetime Parse says: {0:?}")]
    TomlDatetimeParse(#[from] toml::value::DatetimeParseError),
}

/// Type representing the configuration required to generate
/// drafts for a list of blog posts.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Draft {
    blog_posts: Vec<BlogPost>,
    bsky_store: PathBuf,
    referrer_store: PathBuf,
    base_url: Url,
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
        let root = if let Some(r) = root {
            PathBuf::from(r)
        } else {
            PathBuf::new().join(".")
        };

        DraftBuilder {
            base_url,
            root,
            bsky_store: PathBuf::from("bluesky"),
            refer_store: PathBuf::from("static").join("s"),
            path_or_file: Vec::new(),
            minimum_date: today(),
            allow_draft: false,
        }
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

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct DraftBuilder {
    base_url: Url,
    root: PathBuf,
    bsky_store: PathBuf,
    refer_store: PathBuf,
    path_or_file: Vec<PathBuf>,
    minimum_date: Datetime,
    allow_draft: bool,
}

// impl Default for DraftBuilder {
//     fn default() -> Self {
//         DraftBuilder {
//             base_url: Url::,
//             bsky_store: PathBuf::from("bluesky"),
//             refer_store: PathBuf::from("static").join("s"),
//             path_or_file: Vec::new(),
//             minimum_date: today(),
//             allow_draft: false,
//         }
//     }
// }

impl DraftBuilder {
    /// Adds a path or file to the builder's collection.
    ///
    /// This method appends a new path or file to the internal collection of paths
    /// that will be processed. The path can be either a file or a directory, and
    /// the method accepts any type that can be converted into a `PathBuf`.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Any type that implements `Into<PathBuf>`, such as `&str`, `String`,
    ///   `&Path`, or `PathBuf` itself.
    ///
    /// # Arguments
    ///
    /// * `path_or_file` - The path or file to add to the collection. This can be
    ///   an absolute or relative path, pointing to either a file or directory.
    ///
    /// # Returns
    ///
    /// Returns `Ok(&mut Self)` on success, allowing for method chaining.
    /// Returns `Err(DraftError)` if an error occurs during processing.
    ///
    /// # Errors
    ///
    /// This method may return a `DraftError` in future implementations if path
    /// validation is added, though the current implementation always succeeds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::path::PathBuf;
    /// # #[derive(Debug)]
    /// # enum DraftError {
    /// #     // Error variants
    /// # }
    /// # struct PathBuilder {
    /// #     path_or_file: Vec<PathBuf>,
    /// # }
    /// # impl PathBuilder {
    /// #     pub fn new() -> Self { Self { path_or_file: Vec::new() } }
    /// #     pub fn add_path_or_file<P: Into<PathBuf>>(
    /// #         &mut self,
    /// #         path_or_file: P,
    /// #     ) -> Result<&mut Self, DraftError> {
    /// #         self.path_or_file.push(path_or_file.into());
    /// #         Ok(self)
    /// #     }
    /// # }
    /// let mut builder = gdb();
    ///
    /// // Add a file using a string literal
    /// builder.add_path_or_file("./config.toml")?;
    ///
    /// // Add a directory using a String
    /// let dir = String::from("/home/user/documents");
    /// builder.add_path_or_file(dir)?;
    ///
    /// // Add using a PathBuf
    /// use std::path::PathBuf;
    /// let path = PathBuf::from("../assets/images");
    /// builder.add_path_or_file(path)?;
    ///
    /// // Method chaining is supported
    /// builder
    ///     .add_path_or_file("file1.txt")?
    ///     .add_path_or_file("file2.txt")?
    ///     .add_path_or_file("/absolute/path/file3.txt")?;
    /// # Ok::<(), DraftError>(())
    /// ```
    ///
    /// # Usage Patterns
    ///
    /// This method is commonly used in builder patterns where you need to collect
    /// multiple paths before processing them:
    ///
    /// ```
    /// # use std::path::PathBuf;
    /// # #[derive(Debug)]
    /// # enum DraftError {}
    /// # struct PathBuilder {
    /// #     path_or_file: Vec<PathBuf>,
    /// # }
    /// # impl PathBuilder {
    /// #     pub fn new() -> Self { Self { path_or_file: Vec::new() } }
    /// #     pub fn add_path_or_file<P: Into<PathBuf>>(
    /// #         &mut self,
    /// #         path_or_file: P,
    /// #     ) -> Result<&mut Self, DraftError> {
    /// #         self.path_or_file.push(path_or_file.into());
    /// #         Ok(self)
    /// #     }
    /// #     pub fn build(self) -> Vec<PathBuf> { self.path_or_file }
    /// # }
    /// let paths = gdb()
    ///     .add_path_or_file("src/")?
    ///     .add_path_or_file("tests/")?
    ///     .add_path_or_file("Cargo.toml")?
    ///     .build();
    /// # Ok::<(), DraftError>(())
    /// ```
    pub fn add_path_or_file<P: Into<PathBuf>>(
        &mut self,
        path_or_file: P,
    ) -> Result<&mut Self, DraftError> {
        self.path_or_file.push(path_or_file.into());

        Ok(self)
    }

    fn add_blog_posts(
        path: &PathBuf,
        min_date: Datetime,
        allow_draft: bool,
        base_url: &Url,
        www_src_root: &Path,
    ) -> Result<Vec<BlogPost>, DraftError> {
        // find the potential file in the git repo
        use walkdir::WalkDir;

        let mut blog_paths = Vec::new();

        log::trace!("Walking directory at {}", path.display());

        for entry in WalkDir::new(www_src_root.join(path)).into_iter().flatten() {
            if entry.path().extension().unwrap_or_default() == "md" {
                let entry_as_path = entry.into_path();
                let Ok(path_to_blog) = entry_as_path.strip_prefix(www_src_root) else {
                    println!("Failed to strip prefix from `{}`", entry_as_path.display());
                    continue;
                };

                blog_paths.push(path_to_blog.to_path_buf());
            }
        }
        log::trace!("Found blog paths:\n{blog_paths:#?}");

        let mut blog_posts = Vec::new();

        for blog_path in blog_paths {
            match BlogPost::new(&blog_path, min_date, allow_draft, base_url, www_src_root) {
                Ok(bp) => blog_posts.push(bp),
                Err(e) => {
                    log::warn!("`{}` excluded because `{e}`", blog_path.display());
                    continue;
                }
            }
        }

        Ok(blog_posts)
    }

    /// Sets the minimum date constraint for this instance.
    ///
    /// This method configures the minimum allowable date, which can be used
    /// for validation or filtering purposes depending on the context of the
    /// implementing type.
    ///
    /// # Arguments
    ///
    /// * `minimum_date` - A `Datetime` value representing the earliest
    ///   acceptable date
    ///
    /// # Returns
    ///
    /// Returns `Ok(&mut Self)` on success, allowing for method chaining.
    /// Returns `Err(DraftError)` if the operation fails.
    ///
    /// # Errors
    ///
    /// This method currently always returns `Ok`, but the `Result` return type
    /// allows for future error conditions to be added without breaking the API.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use gen_bsky::{Draft, DraftError};
    /// # use url::Url;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), DraftError> {
    ///
    ///     let base_url = Url::parse("https://www.example.com/")?;
    ///     let min_date = "2025-08-04";
    ///
    ///     let mut builder = Draft::builder(base_url, None);
    ///
    ///     builder.with_minimum_date(min_date)?;
    /// #   Ok(())
    /// # }
    /// ```
    ///
    /// Method chaining is supported.
    ///
    /// ```rust
    /// # use gen_bsky::{Draft, DraftError};
    /// # use url::Url;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), DraftError> {
    /// #   
    /// #   let base_url = Url::parse("https://www.example.com/")?;
    /// #   let min_date = "2025-08-04";
    /// #   
    /// #   let mut builder = Draft::builder(base_url, None);
    ///
    ///     let posts_res = builder.with_minimum_date(min_date)?
    ///         .build().await;
    ///     posts_res.expect_err("as no blog posts have been added");
    /// #   Ok(())
    /// # }
    /// ```
    pub fn with_minimum_date(&mut self, minimum_date: &str) -> Result<&mut Self, DraftError> {
        let minimum_date = Datetime::from_str(minimum_date)?;
        self.minimum_date = minimum_date;

        Ok(self)
    }

    /// Sets whether draft items should be allowed.
    ///
    /// This method configures the builder to either allow or disallow draft items
    /// during processing. When set to `true`, draft items will be included; when
    /// set to `false`, they will be filtered out.
    ///
    /// # Arguments
    ///
    /// * `allow_draft` - A boolean value indicating whether draft items should be allowed
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to `self` for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// # struct MyBuilder { allow_draft: bool }
    /// # impl MyBuilder {
    /// #     pub fn new() -> Self { Self { allow_draft: false } }
    /// #     pub fn with_allow_draft(&mut self, allow_draft: bool) -> &mut Self {
    /// #         self.allow_draft = allow_draft;
    /// #         self
    /// #     }
    /// # }
    /// let mut builder = MyBuilder::new();
    ///
    /// // Allow draft items
    /// builder.with_allow_draft(true);
    ///
    /// // Method chaining is supported
    /// builder
    ///     .with_allow_draft(false)
    ///     .with_allow_draft(true);
    /// ```
    pub fn with_allow_draft(&mut self, allow_draft: bool) -> &mut Self {
        self.allow_draft = allow_draft;
        self
    }

    pub async fn build(&mut self) -> Result<Draft, DraftError> {
        let mut blog_posts = Vec::new();

        for path in self.path_or_file.iter() {
            let mut vec_fm = DraftBuilder::add_blog_posts(
                path,
                self.minimum_date,
                self.allow_draft,
                &self.base_url,
                &self.root,
            )?;
            blog_posts.append(&mut vec_fm);
        }

        if blog_posts.is_empty() {
            log::warn!("No blog posts found");
            return Err(DraftError::BlogPostListEmpty);
        }

        Ok(Draft {
            blog_posts,
            bsky_store: self.bsky_store.clone(),
            referrer_store: self.refer_store.clone(),
            base_url: self.base_url.clone(),
            root: self.root.to_path_buf(),
        })
    }
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
    use super::*;

    use std::path::{Path, PathBuf};

    use toml::value::Date;
    use url::Url;

    // get draft builder
    fn gdb() -> DraftBuilder {
        let base_url = Url::parse("https://www.example.com/").unwrap();

        Draft::builder(base_url, None)
    }

    // Generate expected date
    fn ged(year: u16, month: u8, day: u8) -> Option<Date> {
        Some(Date { year, month, day })
    }

    #[test]
    fn test_add_path_or_file_with_string_literal() {
        let mut builder = gdb();
        let result = builder.add_path_or_file("test/path.txt");

        assert!(result.is_ok());
        assert_eq!(builder.path_or_file.len(), 1);
        assert_eq!(builder.path_or_file[0], PathBuf::from("test/path.txt"));
    }

    #[test]
    fn test_add_path_or_file_with_string() {
        let mut builder = gdb();
        let path_string = String::from("/home/user/document.md");
        let result = builder.add_path_or_file(path_string);

        assert!(result.is_ok());
        assert_eq!(builder.path_or_file.len(), 1);
        assert_eq!(
            builder.path_or_file[0],
            PathBuf::from("/home/user/document.md")
        );
    }

    #[test]
    fn test_add_path_or_file_with_pathbuf() {
        let mut builder = gdb();
        let path_buf = PathBuf::from("src/main.rs");
        let expected_path = path_buf.clone();
        let result = builder.add_path_or_file(path_buf);

        assert!(result.is_ok());
        assert_eq!(builder.path_or_file.len(), 1);
        assert_eq!(builder.path_or_file[0], expected_path);
    }

    #[test]
    fn test_add_path_or_file_with_path_ref() {
        let mut builder = gdb();
        let path = Path::new("config/settings.json");
        let result = builder.add_path_or_file(path);

        assert!(result.is_ok());
        assert_eq!(builder.path_or_file.len(), 1);
        assert_eq!(
            builder.path_or_file[0],
            PathBuf::from("config/settings.json")
        );
    }

    #[test]
    fn test_add_path_or_file_multiple_paths() {
        let mut builder = gdb();

        builder.add_path_or_file("file1.txt").unwrap();
        builder.add_path_or_file("file2.txt").unwrap();
        builder.add_path_or_file("dir/file3.txt").unwrap();

        assert_eq!(builder.path_or_file.len(), 3);
        assert_eq!(builder.path_or_file[0], PathBuf::from("file1.txt"));
        assert_eq!(builder.path_or_file[1], PathBuf::from("file2.txt"));
        assert_eq!(builder.path_or_file[2], PathBuf::from("dir/file3.txt"));
    }

    #[test]
    fn test_add_path_or_file_returns_mutable_reference() {
        let mut builder = gdb();
        let result = builder.add_path_or_file("test.txt");

        assert!(result.is_ok());
        let builder_ref = result.unwrap();

        // Should be able to continue modifying through the returned reference
        let second_result = builder_ref.add_path_or_file("test2.txt");
        assert!(second_result.is_ok());
        assert_eq!(builder.path_or_file.len(), 2);
    }

    #[test]
    fn test_add_path_or_file_method_chaining() {
        let mut builder = gdb();

        let result = builder
            .add_path_or_file("path1.txt")
            .and_then(|b| b.add_path_or_file("path2.txt"))
            .and_then(|b| b.add_path_or_file("path3.txt"));

        assert!(result.is_ok());
        assert_eq!(builder.path_or_file.len(), 3);
        assert_eq!(builder.path_or_file[0], PathBuf::from("path1.txt"));
        assert_eq!(builder.path_or_file[1], PathBuf::from("path2.txt"));
        assert_eq!(builder.path_or_file[2], PathBuf::from("path3.txt"));
    }

    #[test]
    fn test_add_path_or_file_empty_path() {
        let mut builder = gdb();
        let result = builder.add_path_or_file("");

        assert!(result.is_ok());
        assert_eq!(builder.path_or_file.len(), 1);
        assert_eq!(builder.path_or_file[0], PathBuf::from(""));
    }

    #[test]
    fn test_add_path_or_file_absolute_paths() {
        let mut builder = gdb();

        #[cfg(unix)]
        let absolute_path = "/usr/local/bin/tool";
        #[cfg(windows)]
        let absolute_path = r"C:\Program Files\Tool\tool.exe";

        let result = builder.add_path_or_file(absolute_path);
        assert!(result.is_ok());
        assert_eq!(builder.path_or_file.len(), 1);
        assert_eq!(builder.path_or_file[0], PathBuf::from(absolute_path));
    }

    #[test]
    fn test_add_path_or_file_relative_paths() {
        let mut builder = gdb();

        let relative_paths = vec![
            "./current_dir/file.txt",
            "../parent_dir/file.txt",
            "../../grandparent/file.txt",
            "subdir/file.txt",
        ];

        for path in &relative_paths {
            builder.add_path_or_file(*path).unwrap();
        }

        assert_eq!(builder.path_or_file.len(), relative_paths.len());
        for (i, expected_path) in relative_paths.iter().enumerate() {
            assert_eq!(builder.path_or_file[i], PathBuf::from(expected_path));
        }
    }

    #[test]
    fn test_add_path_or_file_special_characters() {
        let mut builder = gdb();

        let special_paths = vec![
            "file with spaces.txt",
            "file-with-dashes.txt",
            "file_with_underscores.txt",
            "file.with.dots.txt",
            "file[with]brackets.txt",
        ];

        for path in &special_paths {
            let result = builder.add_path_or_file(*path);
            assert!(result.is_ok(), "Failed to add path: {path}");
        }

        assert_eq!(builder.path_or_file.len(), special_paths.len());
        for (i, expected_path) in special_paths.iter().enumerate() {
            assert_eq!(builder.path_or_file[i], PathBuf::from(expected_path));
        }
    }

    #[test]
    fn test_add_path_or_file_preserves_order() {
        let mut builder = gdb();

        let paths_in_order = vec!["first.txt", "second.txt", "third.txt", "fourth.txt"];

        for path in &paths_in_order {
            builder.add_path_or_file(*path).unwrap();
        }

        // Verify paths are stored in the order they were added
        for (i, expected_path) in paths_in_order.iter().enumerate() {
            assert_eq!(builder.path_or_file[i], PathBuf::from(expected_path));
        }
    }

    #[test]
    fn test_add_path_or_file_duplicate_paths() {
        let mut builder = gdb();

        // Add the same path multiple times
        builder.add_path_or_file("duplicate.txt").unwrap();
        builder.add_path_or_file("duplicate.txt").unwrap();
        builder.add_path_or_file("duplicate.txt").unwrap();

        // Should allow duplicates (behaviour may vary based on requirements)
        assert_eq!(builder.path_or_file.len(), 3);
        for path in &builder.path_or_file {
            assert_eq!(*path, PathBuf::from("duplicate.txt"));
        }
    }

    #[test]
    fn test_add_path_or_file_mixed_types_in_sequence() {
        let mut builder = gdb();

        // Add paths using different input types in sequence
        builder.add_path_or_file("string_literal.txt").unwrap();
        builder
            .add_path_or_file(String::from("owned_string.txt"))
            .unwrap();
        builder
            .add_path_or_file(PathBuf::from("pathbuf.txt"))
            .unwrap();
        builder.add_path_or_file(Path::new("path_ref.txt")).unwrap();

        assert_eq!(builder.path_or_file.len(), 4);
        assert_eq!(builder.path_or_file[0], PathBuf::from("string_literal.txt"));
        assert_eq!(builder.path_or_file[1], PathBuf::from("owned_string.txt"));
        assert_eq!(builder.path_or_file[2], PathBuf::from("pathbuf.txt"));
        assert_eq!(builder.path_or_file[3], PathBuf::from("path_ref.txt"));
    }

    #[test]
    fn test_add_path_or_file_builder_state_after_success() {
        let mut builder = gdb();

        // Initial state
        assert_eq!(builder.path_or_file.len(), 0);

        // Add first path
        builder.add_path_or_file("first.txt").unwrap();
        assert_eq!(builder.path_or_file.len(), 1);

        // Add second path
        builder.add_path_or_file("second.txt").unwrap();
        assert_eq!(builder.path_or_file.len(), 2);

        // Verify both paths are present
        assert_eq!(builder.path_or_file[0], PathBuf::from("first.txt"));
        assert_eq!(builder.path_or_file[1], PathBuf::from("second.txt"));
    }

    #[test]
    fn test_with_minimum_date_valid_date() {
        let mut builder = gdb();
        let result = builder.with_minimum_date("2023-12-25");
        let expected_date = ged(2023, 12, 25);

        assert!(result.is_ok());
        assert_eq!(builder.minimum_date.date, expected_date);
    }

    #[test]
    fn test_with_minimum_date_different_valid_formats() {
        let mut builder = gdb();

        // Expected dates:
        let expected_dates = [
            ged(2023, 1, 1),
            ged(2024, 2, 29), // leap year
            ged(1999, 12, 31),
            ged(2000, 6, 15),
        ];

        // Test various valid date formats
        let valid_dates = [
            "2023-01-01",
            "2024-02-29", // leap year
            "1999-12-31",
            "2000-06-15",
        ];

        for (i, date) in valid_dates.iter().enumerate() {
            let result = builder.with_minimum_date(date);
            assert!(result.is_ok(), "Date '{date}' should be valid");
            assert_eq!(builder.minimum_date.date, expected_dates[i]);
        }
    }

    #[test]
    fn test_with_minimum_date_returns_mutable_reference() {
        let mut builder = gdb();
        let result = builder.with_minimum_date("2023-05-15").unwrap();

        // Verify we get back a mutable reference to the same instance
        assert_eq!(result.minimum_date.date, ged(2023, 5, 15));

        // Should be able to continue modifying through the returned reference
        let chained_result = result.with_minimum_date("2023-06-20");
        assert!(chained_result.is_ok());
        assert_eq!(result.minimum_date.date, ged(2023, 6, 20));
    }

    #[test]
    fn test_with_minimum_date_method_chaining_success() {
        let mut builder = gdb();

        let result = builder
            .with_minimum_date("2023-01-01")
            .and_then(|b| b.with_minimum_date("2023-12-31"));

        assert!(result.is_ok());
        assert_eq!(builder.minimum_date.date, ged(2023, 12, 31));
    }

    #[test]
    fn test_with_minimum_date_empty_string() {
        let mut builder = gdb();
        let original_date = builder.minimum_date;

        let result = builder.with_minimum_date("");

        assert!(result.is_err());
        assert_eq!(
            result.expect_err("expecting failure").to_string(),
            "Toml Datetime Parse says: DatetimeParseError { what: None, expected: Some(\"year or hour\") }".to_string()
        );
        // Original value should remain unchanged
        assert_eq!(builder.minimum_date, original_date);
    }

    #[test]
    fn test_with_minimum_date_invalid_format() {
        let mut builder = gdb();
        let original_date = builder.minimum_date;

        let invalid_dates = vec!["invalid", "2023", "not-a-date", "123", "abcd-ef-gh"];

        for invalid_date in invalid_dates {
            let result = builder.with_minimum_date(invalid_date);
            assert!(result.is_err(), "Date '{invalid_date}' should be invalid");
            let valid_error = matches!(
                result.expect_err("certain of failure"),
                DraftError::TomlDatetimeParse(_)
            );
            assert!(valid_error);
            // Original value should remain unchanged after error
            assert_eq!(builder.minimum_date, original_date);
        }
    }

    #[test]
    fn test_with_minimum_date_updates_field() {
        let mut builder = gdb();
        let initial_date = builder.minimum_date;

        // Verify initial state
        assert_eq!(builder.minimum_date, today());

        // Update with valid date
        let result = builder.with_minimum_date("2024-03-15");
        assert!(result.is_ok());

        // Verify field was updated
        assert_ne!(builder.minimum_date, initial_date);
        assert_eq!(builder.minimum_date.date, ged(2024, 3, 15));
    }

    #[test]
    fn test_with_minimum_date_multiple_successful_calls() {
        let mut builder = gdb();

        // Test multiple successful updates
        builder.with_minimum_date("2023-01-01").unwrap();
        assert_eq!(builder.minimum_date.date, ged(2023, 1, 1));

        builder.with_minimum_date("2023-06-15").unwrap();
        assert_eq!(builder.minimum_date.date, ged(2023, 6, 15));

        builder.with_minimum_date("2023-12-31").unwrap();
        assert_eq!(builder.minimum_date.date, ged(2023, 12, 31));
    }

    #[test]
    fn test_with_minimum_date_error_preserves_state() {
        let mut builder = gdb();

        // Set a valid date first
        builder.with_minimum_date("2023-05-20").unwrap();
        let valid_date = builder.minimum_date;

        // Try to set an invalid date
        let result = builder.with_minimum_date("invalid-date");
        assert!(result.is_err());

        // Verify the previous valid date is preserved
        assert_eq!(builder.minimum_date, valid_date);
        assert_eq!(builder.minimum_date.date, ged(2023, 5, 20));
    }

    #[test]
    fn test_with_minimum_date_error_types() {
        let mut builder = gdb();

        let result = builder.with_minimum_date("");
        match result {
            Err(DraftError::TomlDatetimeParse(_)) => {
                // Expected error type
            }
            _ => panic!("Expected InvalidDateFormat error"),
        }
    }

    #[tokio::test]
    async fn test_with_min_date_for_valid_date() {
        let min_date = "2025-08-04";

        let mut builder = gdb();

        let post_res = builder.with_minimum_date(min_date).unwrap().build().await;

        let error = post_res.expect_err("Expecting error as incomplete");

        assert_eq!("blog post list is empty".to_string(), error.to_string());
    }

    #[test]
    fn test_with_allow_draft_sets_true() {
        let mut builder = gdb();
        assert!(!builder.allow_draft);

        builder.with_allow_draft(true);
        assert!(builder.allow_draft);
    }

    #[test]
    fn test_with_allow_draft_sets_false() {
        let mut builder = gdb();
        builder.allow_draft = true; // Set initial state to true

        builder.with_allow_draft(false);
        assert!(!builder.allow_draft);
    }

    #[test]
    fn test_with_allow_draft_returns_mutable_reference() {
        let mut builder = gdb();
        let result = builder.with_allow_draft(true);

        // Verify we get back a mutable reference to the same instance
        assert!(result.allow_draft);

        // Should be able to continue modifying through the returned reference
        result.with_allow_draft(false);
        assert!(!result.allow_draft);
    }

    #[test]
    fn test_with_allow_draft_method_chaining() {
        let mut builder = gdb();

        // Test method chaining works
        builder
            .with_allow_draft(true)
            .with_allow_draft(false)
            .with_allow_draft(true);

        assert!(builder.allow_draft);
    }

    #[test]
    fn test_with_allow_draft_multiple_calls() {
        let mut builder = gdb();

        // Test that multiple calls properly update the value
        builder.with_allow_draft(true);
        assert!(builder.allow_draft);

        builder.with_allow_draft(false);
        assert!(!builder.allow_draft);

        builder.with_allow_draft(true);
        assert!(builder.allow_draft);
    }

    #[test]
    fn test_with_allow_draft_idempotent() {
        let mut builder = gdb();

        // Test that setting the same value multiple times works correctly
        builder.with_allow_draft(true);
        builder.with_allow_draft(true);
        builder.with_allow_draft(true);
        assert!(builder.allow_draft);

        builder.with_allow_draft(false);
        builder.with_allow_draft(false);
        builder.with_allow_draft(false);
        assert!(!builder.allow_draft);
    }
}
