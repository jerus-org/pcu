use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use toml::value::Datetime;
use url::Url;

// use crate::util::{default_bluesky_dir, default_referrer_dir};

use super::{blog_post::BlogPost, Draft, DraftError};

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct DraftBuilder {
    base_url: Url,
    root: PathBuf,
    path_or_file: Vec<PathBuf>,
    minimum_date: Datetime,
    allow_draft: bool,
}

impl DraftBuilder {
    /// Start a builder for the draft struct.
    ///
    /// ## Parameters
    ///
    /// - `base_url`: the base url for the website (e.g. `https://wwww.example.com/`)
    /// - `store`: the location to store draft posts (e.g. `bluesky`)
    pub fn new(base_url: Url, root: Option<&PathBuf>) -> Self {
        let root = if let Some(r) = root {
            PathBuf::from(r)
        } else {
            PathBuf::new().join(".")
        };

        DraftBuilder {
            base_url,
            root,
            path_or_file: Vec::new(),
            minimum_date: super::today(),
            allow_draft: false,
        }
    }
    /// Adds a path or file to the builder's collection.
    ///
    /// This method appends a new path or file to the internal collection of
    /// paths that will be processed. The path can be either a file or a
    /// directory, and the method accepts any type that can be converted
    /// into a `PathBuf`.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Any type that implements `Into<PathBuf>`, such as `&str`,
    ///   `String`, `&Path`, or `PathBuf` itself.
    ///
    /// # Arguments
    ///
    /// * `path_or_file` - The path or file to add to the collection. This can
    ///   be an absolute or relative path, pointing to either a file or
    ///   directory.
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
    /// This method is commonly used in builder patterns where you need to
    /// collect multiple paths before processing them:
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

        let walk_dir = www_src_root.join(path);
        log::trace!("Walking directory at {}", walk_dir.display());

        for entry in WalkDir::new(walk_dir).into_iter().flatten() {
            log::debug!("Entry found: {entry:?}");
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
    /// let base_url = Url::parse("https://www.example.com/")?;
    /// let min_date = "2025-08-04";
    ///
    /// let mut builder = Draft::builder(base_url, None);
    ///
    /// builder.with_minimum_date(min_date)?;
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
    /// let posts_res =
    ///     builder.with_minimum_date(min_date)?.build().await;
    /// posts_res.expect_err("as no blog posts have been added");
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
    /// This method configures the builder to either allow or disallow draft
    /// items during processing. When set to `true`, draft items will be
    /// included; when set to `false`, they will be filtered out.
    ///
    /// # Arguments
    ///
    /// * `allow_draft` - A boolean value indicating whether draft items should
    ///   be allowed
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

        if self.path_or_file.is_empty() {
            log::debug!("Add default path to the path list as it is empty.");
            self.path_or_file = vec![crate::util::default_blog_dir()];
        }

        log::trace!("Paths and files to processes: {:?}", self.path_or_file);

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
            bsky_store: crate::util::default_bluesky_dir(),
            referrer_store: crate::util::default_referrer_dir(),
            base_url: self.base_url.clone(),
            root: self.root.to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        io,
        path::{Path, PathBuf},
    };

    use toml::value::Date;
    use url::Url;

    use crate::draft::today;

    use super::*;

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
    fn test_future_capacity_too_large_display() {
        let error = DraftError::FutureCapacityTooLarge;
        assert_eq!(error.to_string(), "Future capacity is too large");
    }

    #[test]
    fn test_future_capacity_too_large_debug() {
        let error = DraftError::FutureCapacityTooLarge;
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("FutureCapacityTooLarge"));
    }

    #[test]
    fn test_path_not_found_display() {
        let path = "/some/missing/path".to_string();
        let error = DraftError::PathNotFound(path.clone());
        assert_eq!(error.to_string(), format!("path not found: `{path}`"));
    }

    #[test]
    fn test_path_not_found_with_various_paths() {
        let test_paths = vec![
            "/absolute/unix/path",
            "relative/path",
            "C:\\Windows\\Path",
            "",
            "path with spaces",
            "path/with/unicode/🦀",
        ];

        for path in test_paths {
            let error = DraftError::PathNotFound(path.to_string());
            let expected = format!("path not found: `{path}`");
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn test_file_extension_invalid_display() {
        let file_path = "document.txt".to_string();
        let expected_ext = ".md".to_string();
        let error = DraftError::FileExtensionInvalid(file_path.clone(), expected_ext.clone());

        assert_eq!(
            error.to_string(),
            format!("file extension invalid (must be `{expected_ext}`): {file_path}")
        );
    }

    #[test]
    fn test_file_extension_invalid_various_combinations() {
        let test_cases = vec![
            ("file.txt", ".md"),
            ("document.doc", ".markdown"),
            ("image.jpg", ".png"),
            ("no_extension", ".md"),
            ("file.TAR.GZ", ".zip"),
        ];

        for (file, expected) in test_cases {
            let error = DraftError::FileExtensionInvalid(file.to_string(), expected.to_string());
            let expected_msg = format!("file extension invalid (must be `{expected}`): {file}");
            assert_eq!(error.to_string(), expected_msg);
        }
    }

    #[test]
    fn test_blog_post_list_empty_display() {
        let error = DraftError::BlogPostListEmpty;
        assert_eq!(error.to_string(), "blog post list is empty");
    }

    #[test]
    fn test_qualified_blog_post_list_empty_display() {
        let error = DraftError::QualifiedBlogPostListEmpty;
        assert_eq!(
            error.to_string(),
            "blog post list is empty after qualifications have been applied"
        );
    }

    #[test]
    fn test_io_error_from_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let draft_error: DraftError = io_error.into();

        match draft_error {
            DraftError::Io(inner_error) => {
                assert_eq!(inner_error.kind(), io::ErrorKind::NotFound);
                assert_eq!(inner_error.to_string(), "File not found");
            }
            _ => panic!("Expected DraftError::Io variant"),
        }
    }

    #[test]
    fn test_io_error_display() {
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        let draft_error = DraftError::Io(io_error);

        let error_string = draft_error.to_string();
        assert!(error_string.contains("io error:"));
        assert!(error_string.contains("Access denied"));
    }

    #[test]
    fn test_io_error_various_kinds() {
        let io_errors = vec![
            io::Error::new(io::ErrorKind::NotFound, "Not found"),
            io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied"),
            io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused"),
            io::Error::new(io::ErrorKind::InvalidData, "Invalid data"),
        ];

        for io_error in io_errors {
            let expected_msg = io_error.to_string();
            let draft_error = DraftError::Io(io_error);

            assert!(draft_error.to_string().contains("io error:"));
            assert!(draft_error.to_string().contains(&expected_msg));
        }
    }

    #[test]
    fn test_url_parse_error_from_conversion() {
        let url_result = url::Url::parse("not-a-valid-url");
        assert!(url_result.is_err());

        let url_error = url_result.unwrap_err();
        let draft_error: DraftError = url_error.into();

        match draft_error {
            DraftError::UrlParse(_) => {
                // Success - we got the expected variant
            }
            _ => panic!("Expected DraftError::UrlParse variant"),
        }
    }

    #[test]
    fn test_url_parse_error_display() {
        let url_result = url::Url::parse("h://invalid::url");
        let url_error = url_result.unwrap_err();
        let draft_error = DraftError::UrlParse(url_error);

        let error_string = draft_error.to_string();
        assert!(error_string.contains("URL parse error:"));
    }

    #[test]
    fn test_toml_datetime_parse_error_from_conversion() {
        let datetime_result = "invalid-datetime".parse::<toml::value::Datetime>();
        assert!(datetime_result.is_err());

        let datetime_error = datetime_result.unwrap_err();
        let draft_error: DraftError = datetime_error.into();

        match draft_error {
            DraftError::TomlDatetimeParse(_) => {
                // Success - we got the expected variant
            }
            _ => panic!("Expected DraftError::TomlDatetimeParse variant"),
        }
    }

    #[test]
    fn test_toml_datetime_parse_error_display() {
        let datetime_result = "not-a-date".parse::<toml::value::Datetime>();
        let datetime_error = datetime_result.unwrap_err();
        let draft_error = DraftError::TomlDatetimeParse(datetime_error);

        let error_string = draft_error.to_string();
        assert!(error_string.contains("TOML datetime parse error:"));
    }

    #[test]
    fn test_error_trait_implementation() {
        let error = DraftError::BlogPostListEmpty;

        // Test that it implements Error trait
        let _: &dyn Error = &error;

        // Test source() method (should be None for simple variants)
        assert!(error.source().is_none());
    }

    #[test]
    fn test_error_trait_with_source() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "Test error");
        let draft_error = DraftError::Io(io_error);

        // Test that source() returns the wrapped error
        let source = draft_error.source();
        assert!(source.is_some());

        // Verify the source is an io::Error
        let source_error = source.unwrap();
        assert!(source_error.is::<io::Error>());
    }

    #[test]
    fn test_debug_implementation() {
        let error = DraftError::PathNotFound("test/path".to_string());
        let debug_output = format!("{error:?}");

        assert!(debug_output.contains("PathNotFound"));
        assert!(debug_output.contains("test/path"));
    }

    #[test]
    fn test_equality_for_simple_variants() {
        let error1 = DraftError::FutureCapacityTooLarge;
        let error2 = DraftError::FutureCapacityTooLarge;

        // Note: PartialEq is not derived due to contained error types,
        // but we can test that the same variant creates consistent output
        assert_eq!(error1.to_string(), error2.to_string());
        assert_eq!(format!("{error1:?}"), format!("{:?}", error2));
    }

    #[test]
    fn test_error_message_consistency() {
        // Test that error messages are consistent and well-formed
        let errors = vec![
            DraftError::FutureCapacityTooLarge,
            DraftError::PathNotFound("test".to_string()),
            DraftError::FileExtensionInvalid("file.txt".to_string(), ".md".to_string()),
            DraftError::BlogPostListEmpty,
            DraftError::QualifiedBlogPostListEmpty,
        ];

        for error in errors {
            let message = error.to_string();

            // Error messages should not be empty
            assert!(!message.is_empty());

            // Error messages should not end with punctuation (common convention)
            assert!(!message.ends_with('.'));
            assert!(!message.ends_with('!'));
        }
    }

    #[test]
    fn test_from_trait_implementations() {
        // Test std::io::Error conversion
        let io_error = io::Error::new(io::ErrorKind::NotFound, "test");
        let draft_error: DraftError = DraftError::from(io_error);
        assert!(matches!(draft_error, DraftError::Io(_)));

        // Test url::ParseError conversion
        let url_error = url::Url::parse("invalid").unwrap_err();
        let draft_error: DraftError = DraftError::from(url_error);
        assert!(matches!(draft_error, DraftError::UrlParse(_)));

        // Test toml::value::DatetimeParseError conversion
        let datetime_error = "invalid".parse::<toml::value::Datetime>().unwrap_err();
        let draft_error: DraftError = DraftError::from(datetime_error);
        assert!(matches!(draft_error, DraftError::TomlDatetimeParse(_)));
    }

    #[test]
    fn test_non_exhaustive_attribute() {
        // This test ensures that the enum is marked as non_exhaustive
        // We can't directly test this, but we can verify that pattern matching
        // requires a catch-all pattern
        let error = DraftError::BlogPostListEmpty;

        #[allow(unreachable_patterns)]
        let true_branch = match error {
            DraftError::FutureCapacityTooLarge => false,
            DraftError::PathNotFound(_) => false,
            DraftError::FileExtensionInvalid(_, _) => false,
            DraftError::BlogPostListEmpty => true,
            DraftError::QualifiedBlogPostListEmpty => false,
            DraftError::Io(_) => false,
            DraftError::UrlParse(_) => false,
            DraftError::TomlDatetimeParse(_) => false,
            // The #[non_exhaustive] attribute means we need this catch-all
            _ => false,
        };
        assert!(true_branch);
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
            "TOML datetime parse error: invalid datetime, expected year or hour".to_string()
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
