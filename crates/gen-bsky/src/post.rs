use std::{
    ffi::OsString,
    fmt::Display,
    fs,
    io::BufReader,
    path::{Path, PathBuf},
};

use bsky_sdk::{
    agent::config::Config as BskyConfig, api::app::bsky::feed::post::RecordData, BskyAgent,
};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BskyPost {
    pub post: RecordData,
    pub filename: OsString,
}

/// Post structure to load in bluesky posts and send them from.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    bsky_posts: Vec<BskyPost>,
    sent_posts: Vec<PathBuf>,
}

impl Post {
    /// Create a new default post struct
    pub fn new() -> Self {
        Default::default()
    }

    /// Load the bluesky post documents from the directory
    ///
    /// ## Parameters
    ///
    /// - directory: the directory to find the bluesky posts
    ///
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
                let post = fs::File::open(file_path)?;
                let reader = BufReader::new(post);
                let post = serde_json::from_reader(reader)?;
                let filename: OsString = format!("{directory}/{file_name}").into();
                let bsky_post = BskyPost { post, filename };
                bsky_posts.push(bsky_post);
            }
        }
        self.bsky_posts.extend(bsky_posts);
        Ok(self)
    }

    /// Post the bluesky posts to bluesky
    ///
    /// ## Parameters
    ///
    /// - identifier: login identifier to sign in to the bluesky api
    /// - password: password to sign in to the bluesky api
    ///
    pub async fn post_to_bluesky<S>(&self, identifier: S, password: S) -> Result<(), PostError>
    where
        S: ToString,
    {
        if identifier.to_string().is_empty() {
            return Err(PostError::NoBlueskyIdentifier);
        };

        if password.to_string().is_empty() {
            return Err(PostError::NoBlueskyPassword);
        };

        let bsky_config = BskyConfig::default();

        let agent = BskyAgent::builder().config(bsky_config).build().await?;

        agent
            .login(&identifier.to_string(), &password.to_string())
            .await
            .map_err(|e| PostError::BlueskyLoginError(e.to_string()))?;
        // Set labelers from preferences
        let preferences = agent.get_preferences(true).await?;
        agent.configure_labelers_from_preferences(&preferences);

        log::info!("Bluesky login successful!");

        let testing = check_for_testing();
        log::debug!("Testing: {testing:?}");

        for bsky_post in &self.bsky_posts {
            log::debug!("Post: {}", bsky_post.post.text.clone());

            if testing {
                log::debug!("Post validation: `{:?}`", "Pretending for CI");

                log::debug!("Deleting related file: {:?}", bsky_post.filename);
                fs::remove_file(&bsky_post.filename)?;

                log::info!(
                    "Successfully posted `{}` to Bluesky and deleted the source file `{}`!",
                    bsky_post.post.text,
                    bsky_post.filename.to_string_lossy()
                );
            } else {
                let result = agent.create_record(bsky_post.post.clone()).await;

                if let Err(e) = result {
                    log::error!("Error posting to Bluesky: {e}");
                    continue;
                };

                let output = result.unwrap();
                log::debug!("Post validation: `{:?}`", output.validation_status.as_ref());

                log::debug!("Deleting related file: {:?}", bsky_post.filename);
                fs::remove_file(&bsky_post.filename)?;

                log::info!(
                    "Successfully posted `{}` to Bluesky and deleted the source file `{}`!",
                    bsky_post
                        .post
                        .text
                        .split_terminator('\n')
                        .collect::<Vec<&str>>()[0],
                    bsky_post.filename.to_string_lossy()
                );
            };
        }

        Ok(())
    }
}

fn check_for_testing() -> bool {
    std::env::var("TESTING").is_ok()
}
