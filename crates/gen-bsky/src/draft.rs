use std::path::PathBuf;

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
}

/// Type representing the configuration required to generate
/// drafts for a list of blog posts.
///
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Draft {
    blog_posts: Vec<BlogPost>,
    bsky_store: PathBuf,
    referrer_store: PathBuf,
    base_url: Url,
}

impl Draft {
    /// Start a builder for the draft struct.
    ///
    /// ## Parameters
    ///
    /// - `base_url`: the base url for the website (e.g. `https://wwww.example.com/`)
    /// - `store`: the location to store draft posts (e.g. `bluesky`)
    ///
    pub fn builder(base_url: Url) -> DraftBuilder {
        DraftBuilder {
            base_url,
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

        if !referrer_store.exists() {
            std::fs::create_dir_all(referrer_store)?;
        }

        for blog_post in &mut self.blog_posts {
            match blog_post.write_referrer_file_to(referrer_store, &self.base_url) {
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
    pub fn write_bluesky_posts(
        &self,
        bluesky_post_store: Option<PathBuf>,
    ) -> Result<(), DraftError> {
        // create store directory if it doesn't exist
        let bluesky_post_store = if let Some(p) = bluesky_post_store.as_deref() {
            p
        } else {
            self.bsky_store.as_ref()
        };

        if !bluesky_post_store.exists() {
            std::fs::create_dir_all(bluesky_post_store)?;
        }

        for blog_post in &self.blog_posts {
            match blog_post.write_bluesky_record_to(bluesky_post_store) {
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
    ) -> Result<Vec<BlogPost>, DraftError> {
        // find the potential file in the git repo
        use walkdir::WalkDir;

        let mut blog_paths = Vec::new();

        for entry in WalkDir::new(path).into_iter().flatten() {
            if entry.path().extension().unwrap_or_default() == "md" {
                blog_paths.push(entry.into_path());
            }
        }

        let mut blog_posts = Vec::new();

        for blog_path in blog_paths {
            match BlogPost::new(&blog_path, min_date, allow_draft, base_url) {
                Ok(bp) => blog_posts.push(bp),
                Err(e) => {
                    log::warn!("`{}` excluded because `{e}`", blog_path.display());
                    continue;
                }
            }
        }

        Ok(blog_posts)
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

    pub async fn build(&mut self) -> Result<Draft, DraftError> {
        let mut blog_posts = Vec::new();

        for path in self.path_or_file.iter() {
            let mut vec_fm = DraftBuilder::add_blog_posts(
                path,
                self.minimum_date,
                self.allow_draft,
                &self.base_url,
            )?;
            blog_posts.append(&mut vec_fm);
        }

        if blog_posts.is_empty() {
            log::warn!("No blog posts found");
            return Err(DraftError::BlogPostListEmpty);
        }

        for blog_post in &mut blog_posts {
            match blog_post.get_bluesky_record().await {
                Ok(_) => {}
                Err(e) => {
                    log::warn!(
                        "failed to create bluesky record for `{}` because `{e}`",
                        blog_post.title()
                    )
                }
            };
        }

        Ok(Draft {
            blog_posts,
            bsky_store: self.bsky_store.clone(),
            referrer_store: self.refer_store.clone(),
            base_url: self.base_url.clone(),
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
