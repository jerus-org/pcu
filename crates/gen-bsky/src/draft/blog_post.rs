use std::fs::File;
use std::path::{Path, PathBuf};

mod front_matter;
mod link;

use bsky_sdk::api::app::bsky::feed::post::RecordData;
use bsky_sdk::api::types::string::Datetime as BskyDatetime;
use bsky_sdk::rich_text::RichText;
use link_bridge::Redirector;
use thiserror::Error;
use toml::value::Datetime;
use unicode_segmentation::UnicodeSegmentation;
use url::Url;

use crate::draft::blog_post::link::Link;

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
///
#[derive(Debug, Clone)]
pub(super) struct BlogPost {
    /// The path to the original blog post.
    path: PathBuf,
    /// The front matter from the blog post that is salient
    /// to the production of bluesky posts.
    frontmatter: front_matter::FrontMatter,
    /// The bluesky post record.
    bluesky_post: Option<RecordData>,
    /// The full link to the post.
    post_link: Link,
    /// The short link redirection HTML string
    redirector: Redirector,
    /// The generated short link URL for the post.
    post_short_link: Option<Url>,
}

/// Report values in private fields
impl BlogPost {
    pub fn title(&self) -> &str {
        self.frontmatter.title()
    }
}

impl BlogPost {
    pub fn new(
        blog_path: &PathBuf,
        min_date: Datetime,
        allow_draft: bool,
        base_url: &Url,
    ) -> Result<BlogPost, BlogPostError> {
        let frontmatter = match front_matter::FrontMatter::new(blog_path, min_date, allow_draft) {
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

        let post_link = if blog_path.is_file() {
            let mut post_link = blog_path.clone();
            post_link.set_extension("");
            post_link
                .as_path()
                .to_string_lossy()
                .trim_start_matches("content")
                .to_string()
        } else {
            blog_path
                .as_path()
                .to_string_lossy()
                .trim_start_matches("content")
                .to_string()
        };
        let link = Link::new(base_url, &post_link).map_err(|e| match e {
            link::LinkError::UrlParse(e) => BlogPostError::UrlParse(e),
        })?;

        // Initialise the short link html redirector
        let redirector = Redirector::new(&post_link)?;

        Ok(BlogPost {
            path: blog_path.clone(),
            frontmatter,
            bluesky_post: None,
            post_link: link,
            redirector,
            post_short_link: None,
        })
    }

    /// Get bluesky record based on frontmatter data
    pub async fn get_bluesky_record(&mut self) -> Result<(), BlogPostError> {
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

        self.bluesky_post = Some(record_data);
        Ok(())
    }

    fn build_post_text(&mut self) -> Result<String, BlogPostError> {
        log::debug!(
            "Building post text with post dir: `{}`",
            self.path.display()
        );

        if log::log_enabled!(log::Level::Debug) {
            self.log_post_details();
        }

        let post_text = format!(
            "{}\n\n{} {}\n\n{}",
            self.frontmatter.title,
            self.frontmatter.bluesky_description(),
            self.frontmatter.bluesky_tags().join(" "),
            if let Some(sl) = self.post_short_link.as_ref() {
                sl
            } else {
                self.post_link.public()
            }
        );

        if post_text.len() > 300 {
            return Err(BlogPostError::PostTooManyCharacters(
                self.frontmatter.title.clone(),
                post_text.len(),
            ));
        }

        if post_text.graphemes(true).count() > 300 {
            return Err(BlogPostError::PostTooManyGraphemes(
                self.frontmatter.title.clone(),
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
    ) -> Result<(), BlogPostError> {
        log::debug!("Building link with `{base_url}` as root of url",);

        self.redirector.set_path(store_dir);

        let short_link = self.redirector.write_redirect()?;
        log::debug!("redirect written and short link returned: {short_link}");

        self.post_short_link = Some(base_url.join(short_link.trim_start_matches("static/"))?);
        log::debug!("Saved short post link {:#?}", self.post_short_link);
        Ok(())
    }

    /// Write the bluesky record to the `store_dir` location.
    /// The write function generates a short name based on post link
    /// and filename to ensure that similarly named posts have unique
    /// bluesky post names.
    pub fn write_bluesky_record_to(&self, store_dir: &Path) -> Result<(), BlogPostError> {
        let Some(bluesky_post) = self.bluesky_post.as_ref() else {
            return Err(BlogPostError::BlueSkyPostNotConstructed);
        };

        let Some(filename) = self.path.as_path().file_name() else {
            return Err(BlogPostError::PostBasenameNotSet);
        };
        let filename = filename.to_str().unwrap();

        let postname = format!(
            "{}{}{}",
            base62::encode(self.post_link.local().encode_utf16().sum::<u16>()),
            base62::encode(filename.encode_utf16().sum::<u16>()),
            base62::encode(
                self.post_link
                    .local()
                    .trim_end_matches(filename)
                    .encode_utf16()
                    .sum::<u16>()
            )
        );

        log::trace!("Bluesky post: {bluesky_post:#?}");

        let post_file = format!("{postname}.post");
        let post_file = store_dir.to_path_buf().join(post_file);
        log::debug!("Write filename: {filename} as {postname}");
        log::debug!("Write file: {}", post_file.display());

        let file = File::create(post_file)?;

        serde_json::to_writer_pretty(&file, &bluesky_post)?;
        file.sync_all()?;

        Ok(())
    }

    fn log_post_details(&self) {
        log::debug!("Post link: {}", self.post_link.public());
        log::debug!(
            "Length of post link: {} characters and {} graphemes",
            self.post_link.public().as_str().len(),
            self.post_link.public().as_str().graphemes(true).count()
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
