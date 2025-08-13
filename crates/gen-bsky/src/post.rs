use std::{fmt::Display, fs, io::BufReader, path::Path};

const TESTING_FLAG: &str = "TESTING";
mod bsky_post;

use bsky_post::{BskyPost, BskyPostState};
use bsky_sdk::{agent::config::Config as BskyConfig, BskyAgent};
// use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error enum for Draft type
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum PostError {
    /// Bluesky identifier required to post to bluesky not provided
    #[error("No bluesky identifier provided")]
    NoBlueskyIdentifier,

    /// Bluesky password required to post to bluesky not provided
    #[error("No bluesky password provided")]
    NoBlueskyPassword,

    /// Bluesky sdk reports a login error
    #[error("bsky_sdk create_session error says: {0:?}")]
    BlueskyLoginError(String),

    /// Error report from the std::io module
    #[error("io error says: {0:?}")]
    Io(#[from] std::io::Error),

    /// Error report from the serde_json crate's create_session type
    #[error("serde_json create_session error says: {0:?}")]
    SerdeJsonError(#[from] serde_json::error::Error),

    /// Error report from the bsky_sdk crate
    #[error("bsky_sdk error says: {0:?}")]
    BskySdk(#[from] bsky_sdk::Error),
}

/// Post structure to load in bluesky posts and send them from.
#[derive(Default, Debug, Clone)]
pub struct Post {
    bsky_posts: Vec<BskyPost>,
    id: String,
    pwd: String,
    // sent_posts: Vec<PathBuf>,
}

impl Post {
    /// Create a new default post struct
    pub fn new(id: &str, password: &str) -> Result<Self, PostError> {
        if id.is_empty() {
            return Err(PostError::NoBlueskyIdentifier);
        };

        if password.is_empty() {
            return Err(PostError::NoBlueskyPassword);
        };

        Ok(Post {
            id: id.to_string(),
            pwd: password.to_string(),
            ..Default::default()
        })
    }

    /// Load the bluesky post documents from the directory
    ///
    /// ## Parameters
    ///
    /// - directory: the directory to find the bluesky posts
    pub fn load<P>(&mut self, directory: P) -> Result<&mut Self, PostError>
    where
        P: AsRef<Path> + Display,
    {
        let files = fs::read_dir(&directory)?;
        let mut bsky_posts = Vec::new();
        for file in files {
            let file = file?;
            let file_name = file.file_name().into_string().unwrap();
            if file_name.ends_with(".post") {
                let file_path = file.path();
                let post = fs::File::open(&file_path)?;
                let reader = BufReader::new(post);
                let post = serde_json::from_reader(reader)?;
                let bsky_post = BskyPost::new(post, file_path);
                bsky_posts.push(bsky_post);
            }
        }
        self.bsky_posts.extend(bsky_posts);
        Ok(self)
    }

    /// Post the bluesky posts to bluesky
    pub async fn post_to_bluesky(&mut self) -> Result<&mut Self, PostError> {
        let bsky_config = BskyConfig::default();

        let agent = BskyAgent::builder().config(bsky_config).build().await?;

        agent
            .login(&self.id, &self.pwd)
            .await
            .map_err(|e| PostError::BlueskyLoginError(e.to_string()))?;
        // Set labelers from preferences
        let preferences = agent.get_preferences(true).await?;
        agent.configure_labelers_from_preferences(&preferences);
        log::info!("Bluesky login successful!");

        let testing = std::env::var(TESTING_FLAG).is_ok();
        if testing {
            log::info!("No posts will be made to bluesky as this is a test.");
        }

        for bsky_post in &mut self
            .bsky_posts
            .iter_mut()
            .filter(|p| p.state() == &BskyPostState::Read)
        {
            log::debug!("Post: {}", bsky_post.post().text.clone());

            if testing {
                // log::debug!("Deleting related file: {:?}", bsky_post.file_path);
                // fs::remove_file(&bsky_post.file_path)?;

                log::info!(
                    "Successfully posted `{}` to Bluesky",
                    bsky_post.file_path().to_string_lossy()
                );
            } else {
                let result = agent.create_record(bsky_post.post().clone()).await;

                let Ok(output) = result else {
                    log::warn!("Error posting to Bluesky: {}", result.err().unwrap());
                    continue;
                };

                log::debug!("Post validation: `{:?}`", output.validation_status.as_ref());

                log::info!(
                    "Successfully posted `{}` to Bluesky",
                    bsky_post
                        .post()
                        .text
                        .split_terminator('\n')
                        .collect::<Vec<&str>>()[0],
                );

                bsky_post.set_state(BskyPostState::Posted);
            };
        }

        Ok(self)
    }

    /// Delete the successfully posted bluesky posts
    pub fn delete_posted_posts(&mut self) -> Result<&mut Self, PostError> {
        for bsky_post in &mut self
            .bsky_posts
            .iter_mut()
            .filter(|p| p.state() == &BskyPostState::Posted)
        {
            log::debug!("Deleting related file: {:?}", bsky_post.file_path());
            fs::remove_file(bsky_post.file_path())?;

            log::info!(
                "Successfully deleted `{}` bluesky post file",
                bsky_post.file_path().to_string_lossy()
            );

            bsky_post.set_state(BskyPostState::Deleted);
        }

        Ok(self)
    }

    /// Count the deleted posts
    pub fn count_deleted(&self) -> usize {
        self.bsky_posts
            .iter()
            .filter(|b| b.state() == &BskyPostState::Deleted)
            .count()
    }
}
