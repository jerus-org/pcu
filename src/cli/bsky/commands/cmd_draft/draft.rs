mod front_matter;
mod site_config;

use std::fs::File;

use bsky_sdk::{
    api::{app::bsky::feed::post::RecordData, types::string::Datetime},
    rich_text::RichText,
};

pub use front_matter::FrontMatter;
use site_config::SiteConfig;

use crate::Error;

#[derive(Clone, Default)]
pub struct Draft {
    blog_posts: Vec<FrontMatter>,
    base_url: String,
    path: String,
    store: String,
}
impl Draft {
    // pub fn new() -> Result<Self, Error> {
    //     let site_config = SiteConfig::new()?;

    //     Ok(Builder {
    //         base_url: site_config.base_url(),
    //         ..Default::default()
    //     })
    // }

    pub fn new_with_path(path: &str) -> Result<Self, Error> {
        let site_config = SiteConfig::new()?;

        let path = if !path.is_empty() {
            log::debug!("Path to blog files: `{path}`");
            format!("{path}/")
        } else {
            "".to_string()
        };

        Ok(Draft {
            base_url: site_config.base_url(),
            path,
            ..Default::default()
        })
    }

    #[allow(dead_code)]
    pub fn add_path(&mut self, path: &str) -> Result<&mut Self, Error> {
        let path = if !path.is_empty() {
            log::debug!("Path to blog files: `{path}`");
            format!("{path}/")
        } else {
            "".to_string()
        };

        self.path = path;
        Ok(self)
    }

    pub fn add_posts(&mut self, blog_posts: &mut Vec<FrontMatter>) -> Result<&mut Self, Error> {
        // if isize::MAX < (self.blog_posts.capacity() + blog_posts.capacity()) as isize {
        //     return Err(Error::FutureCapacityTooLarge);
        // }

        let a_size = self.blog_posts.capacity() as isize;
        let b_size = blog_posts.capacity() as isize;

        let Some(_c_size) = a_size.checked_add(b_size) else {
            return Err(Error::FutureCapacityTooLarge);
        };

        self.blog_posts.append(blog_posts);

        Ok(self)
    }

    pub async fn process_posts(&mut self) -> Result<&mut Self, Error> {
        for blog_post in &mut self.blog_posts {
            log::trace!("Blog post: {blog_post:#?}");

            let post_dir = if let Some(path) = blog_post.path.as_ref() {
                format!("{}{}", path, "/")
            } else {
                String::new()
            };

            let post_link = format!(
                "{}/{}{}/index.html",
                self.base_url,
                post_dir,
                blog_post.basename.as_ref().unwrap()
            );
            log::debug!("Post link: {post_link}");

            let post_text = format!(
                "{}\n\n{} #{}\n\n{}",
                blog_post.title,
                blog_post
                    .extra
                    .as_ref()
                    .map_or_else(|| blog_post.description.as_str(), |e| e.bluesky.as_str()),
                blog_post.taxonomies.tags.join(" #"),
                post_link
            );

            log::debug!("Post text: {post_text}");

            let rt = RichText::new_with_detect_facets(&post_text).await?;

            log::trace!("Rich text: {rt:#?}");

            let record_data = RecordData {
                created_at: Datetime::now(),
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

            blog_post.bluesky_post = Some(record_data);
        }

        Ok(self)
    }

    pub fn add_store(&mut self, store: &str) -> Result<&mut Self, Error> {
        self.store = store.to_string();
        Ok(self)
    }
    pub fn write_posts(&self) -> Result<(), Error> {
        // create store directory if it doesn't exist
        if !std::path::Path::new(&self.store).exists() {
            std::fs::create_dir_all(self.store.clone())?;
        }

        for blog_post in &self.blog_posts {
            let Some(bluesky_post) = &blog_post.bluesky_post else {
                log::warn!("No Bluesky post found for blog post: {}", blog_post.title);
                continue;
            };

            let Some(filename) = &blog_post.basename else {
                log::warn!("No filename found for blog post: {}", blog_post.title);
                continue;
            };

            log::trace!("Bluesky post: {bluesky_post:#?}");
            log::debug!("Post filename: {filename}");

            let post_file = format!("{}/{}.post", self.store, filename.trim_end_matches(".md"));
            log::debug!("Post file: {post_file}");

            let file = File::create(post_file)?;

            serde_json::to_writer_pretty(&file, bluesky_post)?;
            file.sync_all()?;
        }

        Ok(())
    }
}
