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
    pub fn new_with_path(path: &str) -> Result<Self, Error> {
        let site_config = SiteConfig::new()?;
        log::debug!("Site config: {site_config:?}");

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
            log::info!("Blog post: {blog_post:#?}");
            log::debug!("Building post text with base url {}", self.base_url);
            let post_text = blog_post.build_post_text(self.base_url.as_str())?;

            log::trace!("Post text: {post_text}");

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

    pub fn write_posts(&self) -> Result<&Self, Error> {
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

            let Some(post_link) = blog_post.post_link.as_ref() else {
                log::warn!("No post link found for blog post `{}`", blog_post.title);
                continue;
            };

            let postname = base62::encode(post_link.encode_utf16().sum::<u16>());

            log::trace!("Bluesky post: {bluesky_post:#?}");
            log::debug!("Post filename: {filename} as {postname}");

            let post_file = format!("{}/{}.post", self.store, postname);
            log::debug!("Post file: {post_file}");

            let file = File::create(post_file)?;

            serde_json::to_writer_pretty(&file, bluesky_post)?;
            file.sync_all()?;
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use log::LevelFilter;

    use super::*;

    fn get_test_logger() {
        let mut builder = env_logger::Builder::new();
        builder.filter(None, LevelFilter::Debug);
        builder.format_timestamp_secs().format_module_path(false);
        let _ = builder.try_init();
    }

    fn create_front_matter(title: &str, basename: &str, short_store: &str) -> FrontMatter {
        FrontMatter {
            title: title.to_string(),
            description: "desc".to_string(),
            basename: Some(basename.to_string()),
            path: Some("blog".to_string()),
            short_link_store: Some(short_store.to_string()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_process_posts_sets_bluesky_post() {
        let mut draft = Draft::default();
        let mut posts = vec![create_front_matter("Title", "file1", "store1")];
        draft.add_posts(&mut posts).unwrap();
        draft.process_posts().await.unwrap();
        assert!(draft.blog_posts[0].bluesky_post.is_some());
        fs::remove_dir_all("store1").unwrap();
    }

    #[test]
    fn test_add_path_sets_path() {
        let mut draft = Draft::default();
        draft.add_path("blog").unwrap();
        assert_eq!(draft.path, "blog/");
    }

    #[test]
    fn test_add_store_sets_store() {
        let mut draft = Draft::default();
        draft.add_store("store_dir").unwrap();
        assert_eq!(draft.store, "store_dir");
    }

    #[test]
    fn test_add_posts_appends_posts() {
        let mut draft = Draft::default();
        let mut posts = vec![create_front_matter("Title", "file2", "store2")];
        draft.add_posts(&mut posts).unwrap();
        assert_eq!(draft.blog_posts.len(), 1);
    }

    #[tokio::test]
    async fn test_write_posts_creates_files() {
        get_test_logger();

        let mut draft = Draft {
            store: "test_store".to_string(),
            ..Default::default()
        };
        let mut fm = create_front_matter("Title", "file3", "store3");
        fm.bluesky_post = Some(RecordData {
            created_at: Datetime::now(),
            embed: None,
            entities: None,
            facets: None,
            labels: None,
            langs: None,
            reply: None,
            tags: None,
            text: "text".to_string(),
        });
        draft.blog_posts.push(fm);
        draft.process_posts().await.unwrap();

        let psl = draft.blog_posts[0]
            .post_short_link
            .as_ref()
            .unwrap()
            .clone();
        draft.write_posts().unwrap();
        let post_file = "test_store/file3.post";
        log::debug!("Post file: {post_file}");

        let short_link = psl.trim_start_matches('/');
        log::debug!("short link: {short_link}");

        assert!(Path::new(post_file).exists());
        assert!(Path::new(&short_link).exists());
        fs::remove_file(post_file).unwrap();
        fs::remove_dir("test_store").unwrap();
        fs::remove_dir_all("store3").unwrap();
    }
}
