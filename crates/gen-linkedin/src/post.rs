use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{
    draft::LinkedinFile,
    frontmatter_writeback::{write_linkedin_date_field, FmWriteError},
};

/// Errors that can occur during LinkedIn post publishing.
#[derive(Debug, Error)]
pub enum PostError {
    /// I/O error reading, writing, or deleting files.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON deserialization error loading a `.linkedin` file.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Frontmatter write-back error.
    #[error("frontmatter write error: {0}")]
    FmWrite(#[from] FmWriteError),
    /// LinkedIn API error.
    #[error("linkedin api error: {0}")]
    Api(String),
    /// Missing required configuration value.
    #[error("missing configuration: {0}")]
    Config(String),
}

#[derive(Debug, Clone, PartialEq)]
enum LinkedinPostState {
    Read,
    Posted,
    Deleted,
}

/// A loaded `.linkedin` draft file ready for publishing.
#[derive(Debug)]
pub struct LinkedinPost {
    file_path: PathBuf,
    record: LinkedinFile,
    state: LinkedinPostState,
}

impl LinkedinPost {
    fn new(record: LinkedinFile, file_path: PathBuf) -> Self {
        Self {
            file_path,
            record,
            state: LinkedinPostState::Read,
        }
    }

    /// Path to the source `.md` file, if recorded in the `.linkedin` file.
    pub fn source_path(&self) -> Option<&PathBuf> {
        self.record.source_path.as_ref()
    }

    /// The LinkedIn post text.
    pub fn text(&self) -> &str {
        &self.record.text
    }

    /// The optional link to attach.
    pub fn link(&self) -> Option<&str> {
        self.record.link.as_deref()
    }

    fn set_posted(&mut self) {
        self.state = LinkedinPostState::Posted;
    }

    fn set_deleted(&mut self) {
        self.state = LinkedinPostState::Deleted;
    }
}

/// Manages publishing staged `.linkedin` draft files to LinkedIn.
pub struct Post {
    access_token: String,
    author_urn: String,
    api_version: String,
    pub(crate) linkedin_posts: Vec<LinkedinPost>,
    deleted: usize,
    testing: bool,
}

impl Post {
    /// Create a new `Post` manager with the given credentials.
    pub fn new(access_token: &str, author_urn: &str) -> Self {
        let testing = std::env::var("TESTING").is_ok();
        Self {
            access_token: access_token.to_string(),
            author_urn: author_urn.to_string(),
            api_version: crate::posts::DEFAULT_API_VERSION.to_string(),
            linkedin_posts: Vec::new(),
            deleted: 0,
            testing,
        }
    }

    /// Override the LinkedIn API version sent in the `LinkedIn-Version` header.
    #[must_use]
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Force testing mode (no actual API calls). Used in unit tests.
    #[cfg(test)]
    pub fn with_testing(mut self) -> Self {
        self.testing = true;
        self
    }

    /// Load all `.linkedin` files from `store_dir`.
    pub fn load<P>(&mut self, store_dir: P) -> Result<&mut Self, PostError>
    where
        P: AsRef<Path>,
    {
        let dir = store_dir.as_ref();
        if !dir.is_dir() {
            return Ok(self);
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("linkedin") {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);
                let record: LinkedinFile = serde_json::from_reader(reader)?;
                self.linkedin_posts.push(LinkedinPost::new(record, path));
            }
        }
        Ok(self)
    }

    /// Publish all loaded posts to LinkedIn.
    ///
    /// In test mode (`TESTING=true` env var) posts are not actually sent.
    pub async fn post_to_linkedin(&mut self) -> Result<&mut Self, PostError> {
        if self.testing {
            log::info!("TESTING mode — no posts will be sent to LinkedIn.");
        }

        // Only build the HTTP client when actually needed.
        let posts_client = if !self.testing {
            use crate::{auth::StaticTokenProvider, client::Client, posts::PostsClient};
            let token = StaticTokenProvider(self.access_token.clone());
            let client = Client::new(token).map_err(|e| PostError::Api(e.to_string()))?;
            Some(PostsClient::new(client).with_api_version(&self.api_version))
        } else {
            None
        };

        for li_post in self
            .linkedin_posts
            .iter_mut()
            .filter(|p| p.state == LinkedinPostState::Read)
        {
            publish_one(
                self.testing,
                &self.author_urn,
                li_post,
                posts_client.as_ref(),
            )
            .await?;
        }

        Ok(self)
    }

    /// Delete `.linkedin` files for all successfully posted content and write
    /// `[linkedin].published = <today>` into the source markdown file.
    pub fn delete_posted_posts(&mut self) -> Result<&mut Self, PostError> {
        let today = {
            let today = chrono::Utc::now().date_naive();
            today
                .format("%Y-%m-%d")
                .to_string()
                .parse()
                .expect("valid date")
        };

        for li_post in self
            .linkedin_posts
            .iter_mut()
            .filter(|p| p.state == LinkedinPostState::Posted)
        {
            // Write published date into source frontmatter.
            if let Some(src) = li_post.source_path() {
                if let Err(e) = write_linkedin_date_field(src, "published", today) {
                    log::warn!("Failed to write [linkedin].published to {src:?}: {e}");
                }
            }

            // Delete the .linkedin file.
            if let Err(e) = fs::remove_file(&li_post.file_path) {
                log::warn!("Failed to delete {:?}: {e}", li_post.file_path);
            } else {
                li_post.set_deleted();
                self.deleted += 1;
            }
        }

        Ok(self)
    }

    /// Number of posts successfully deleted (published + cleaned up).
    pub fn count_deleted(&self) -> usize {
        self.deleted
    }
}

async fn publish_one(
    testing: bool,
    author_urn: &str,
    li_post: &mut LinkedinPost,
    posts_client: Option<&crate::posts::PostsClient<crate::auth::StaticTokenProvider>>,
) -> Result<(), PostError> {
    // Skip if source markdown already has [linkedin].published.
    if let Some(src) = li_post.source_path() {
        if crate::frontmatter_writeback::read_linkedin_date_field(src, "published").is_some() {
            log::debug!("Skipping — [linkedin].published already set in {src:?}");
            return Ok(());
        }
    }

    if testing {
        log::info!("(test) would post: {}", li_post.text());
        li_post.set_posted();
    } else {
        use crate::posts::TextPost;
        let client = posts_client.expect("client built above");
        let mut text_post = TextPost::new(author_urn, li_post.text());
        if let Some(link) = li_post.link() {
            text_post = text_post.with_link(
                link.parse()
                    .map_err(|e: url::ParseError| PostError::Api(e.to_string()))?,
            );
        }
        match client.create_text_post(&text_post).await {
            Ok(_) => {
                log::info!("Posted to LinkedIn: {}", li_post.text());
                li_post.set_posted();
            }
            Err(e) => {
                log::warn!("Failed to post to LinkedIn: {e}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_linkedin_file(dir: &Path, name: &str, text: &str, source: Option<&Path>) -> PathBuf {
        let record = LinkedinFile {
            text: text.to_string(),
            link: None,
            created_at: "2026-04-03".to_string(),
            source_path: source.map(|p| p.to_path_buf()),
        };
        let path = dir.join(name);
        let file = File::create(&path).unwrap();
        serde_json::to_writer(&file, &record).unwrap();
        path
    }

    fn write_md_with_created(dir: &Path, name: &str) -> PathBuf {
        let content = "+++\ntitle = \"Test\"\n\n[linkedin]\ndescription = \"hello\"\ncreated = 2026-04-03\n+++\n\nBody.\n";
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    // RED: Post::load reads .linkedin files

    #[test]
    fn test_load_reads_linkedin_files() {
        let dir = tempdir().unwrap();
        write_linkedin_file(dir.path(), "post.linkedin", "Hello", None);

        let mut post = Post::new("token", "urn:li:organization:123");
        post.load(dir.path()).unwrap();

        assert_eq!(post.linkedin_posts.len(), 1);
        assert_eq!(post.linkedin_posts[0].text(), "Hello");
    }

    // RED: Post::load ignores non-.linkedin files

    #[test]
    fn test_load_ignores_other_files() {
        let dir = tempdir().unwrap();
        write_linkedin_file(dir.path(), "post.linkedin", "Hello", None);
        fs::write(dir.path().join("readme.txt"), "ignore me").unwrap();

        let mut post = Post::new("token", "urn:li:organization:123");
        post.load(dir.path()).unwrap();

        assert_eq!(post.linkedin_posts.len(), 1);
    }

    // RED: Post::load reads source_path from file

    #[test]
    fn test_load_reads_source_path() {
        let dir = tempdir().unwrap();
        let md_path = dir.path().join("my-post.md");
        write_linkedin_file(dir.path(), "post.linkedin", "Hello", Some(&md_path));

        let mut post = Post::new("token", "urn:li:organization:123");
        post.load(dir.path()).unwrap();

        assert_eq!(post.linkedin_posts[0].source_path(), Some(&md_path));
    }

    // RED: delete_posted_posts writes published into source markdown

    #[test]
    fn test_delete_posted_posts_writes_published() {
        let dir = tempdir().unwrap();
        let md_path = write_md_with_created(dir.path(), "my-post.md");
        write_linkedin_file(dir.path(), "post.linkedin", "Hello", Some(&md_path));

        let mut post = Post::new("token", "urn:li:organization:123");
        post.load(dir.path()).unwrap();
        assert_eq!(post.linkedin_posts.len(), 1);

        // Manually set state to Posted (simulates a successful API call).
        post.linkedin_posts[0].set_posted();
        post.delete_posted_posts().unwrap();

        let content = fs::read_to_string(&md_path).unwrap();
        assert!(
            content.contains("published ="),
            "frontmatter should contain published: {content}"
        );
    }

    // RED: delete_posted_posts removes the .linkedin file

    #[test]
    fn test_delete_posted_posts_removes_file() {
        let dir = tempdir().unwrap();
        let md_path = write_md_with_created(dir.path(), "my-post.md");
        let li_path = write_linkedin_file(dir.path(), "post.linkedin", "Hello", Some(&md_path));

        let mut post = Post::new("token", "urn:li:organization:123");
        post.load(dir.path()).unwrap();
        post.linkedin_posts[0].set_posted();
        post.delete_posted_posts().unwrap();

        assert!(!li_path.exists(), ".linkedin file should be deleted");
        assert_eq!(post.count_deleted(), 1);
    }

    // RED: post_to_linkedin in TESTING mode marks posts as Posted

    #[tokio::test]
    async fn test_post_to_linkedin_testing_mode() {
        let dir = tempdir().unwrap();
        write_linkedin_file(dir.path(), "post.linkedin", "Hello LinkedIn", None);

        let mut post = Post::new("token", "urn:li:organization:123").with_testing();
        post.load(dir.path()).unwrap();
        post.post_to_linkedin().await.unwrap();

        assert_eq!(
            post.linkedin_posts[0].state,
            LinkedinPostState::Posted,
            "post should be marked Posted in test mode"
        );
    }

    // RED: post_to_linkedin skips posts with [linkedin].published already set

    #[tokio::test]
    async fn test_post_to_linkedin_skips_already_published() {
        let dir = tempdir().unwrap();
        let md_content = "+++\ntitle = \"T\"\n\n[linkedin]\ndescription = \"d\"\ncreated = 2026-04-01\npublished = 2026-04-01\n+++\n\nBody.\n";
        let md_path = dir.path().join("post.md");
        fs::write(&md_path, md_content).unwrap();
        write_linkedin_file(dir.path(), "post.linkedin", "Hello", Some(&md_path));

        let mut post = Post::new("token", "urn:li:organization:123").with_testing();
        post.load(dir.path()).unwrap();
        post.post_to_linkedin().await.unwrap();

        assert_eq!(
            post.linkedin_posts[0].state,
            LinkedinPostState::Read,
            "already-published post should remain in Read state"
        );
    }

    // RED: LinkedinFile roundtrips through JSON without source_path

    #[test]
    fn test_linkedin_file_deserializes_without_source_path() {
        let json = r#"{"text":"Hello","created_at":"2026-04-03"}"#;
        let f: LinkedinFile = serde_json::from_str(json).unwrap();
        assert!(f.source_path.is_none());
        assert_eq!(f.text, "Hello");
    }
}
