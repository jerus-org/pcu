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
    #[error("Url says: {0:?}")]
    TomlDateTimeParse(#[from] toml::value::DatetimeParseError),
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
    /// # use gen_bsky::Draft;
    /// # use url::Url;
    /// # async fn main() -> Result<(), DraftError> {
    ///
    ///     let base_url = Url::parse("https://www.example.com/")?;
    ///     let min_date = "2025/08/04";
    ///
    ///     let mut builder = Draft::builder(base_url, none);
    ///
    ///     builder.with_minimum_date(min_date)?;
    /// #   Ok(())
    /// # }
    /// ```
    ///
    /// Method chaining is supported.
    ///
    /// ```rust
    /// # async fn main() -> Result<(), DraftError> {
    /// # use gen_bsky::Draft;
    /// # use url::Url;
    /// #
    /// #let base_url = Url::parse("https://www.example.com/")?;
    /// #let min_date = "2025/08/04";
    /// #
    /// #let mut builder = Draft::builder(base_url, None);
    ///
    ///     let posts = builder.with_minimum_date(min_date)?
    ///         .build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_minimum_date(&mut self, minimum_date: &str) -> Result<&mut Self, DraftError> {
        let minimum_date = Datetime::from_str(minimum_date)?;
        self.minimum_date = minimum_date;

        Ok(self)
    }

    /// Optionally set a
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
mod test {
    use super::Draft;
    use url::Url;

    #[tokio::test]
    async fn test_with_min_date_for_valid_date() {
        let base_url = Url::parse("https://www.example.com/").unwrap();
        let min_date = "2025-08-04";

        let mut builder = Draft::builder(base_url, None);

        let post_res = builder.with_minimum_date(min_date).unwrap().build().await;

        let error = post_res.expect_err("Expecting error as incomplete");

        assert_eq!("blog post list is empty".to_string(), error.to_string());
    }
}
