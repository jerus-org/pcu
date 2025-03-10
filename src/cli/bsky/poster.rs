use std::{ffi::OsString, fmt::Display, fs, io::BufReader, path::Path};

use bsky_sdk::{
    agent::config::Config as BskyConfig, api::app::bsky::feed::post::RecordData, BskyAgent,
};
use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BskyPost {
    pub post: RecordData,
    pub filename: OsString,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Poster {
    bsky_posts: Vec<BskyPost>,
}

impl Poster {
    pub fn new() -> Result<Self, Error> {
        Ok(Default::default())
    }

    pub fn load<P>(&mut self, directory: P) -> Result<&mut Self, Error>
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
                let filename: OsString = format!("{}/{}", directory, file_name).into();
                let bsky_post = BskyPost { post, filename };
                bsky_posts.push(bsky_post);
            }
        }
        self.bsky_posts.extend(bsky_posts);
        Ok(self)
    }

    pub async fn post_to_bluesky<S>(&self, identifier: S, password: S) -> Result<(), Error>
    where
        S: ToString,
    {
        if identifier.to_string().is_empty() {
            return Err(Error::NoBlueskyIdentifier);
        };

        if password.to_string().is_empty() {
            return Err(Error::NoBlueskyPassword);
        };

        let bsky_config = BskyConfig::default();

        let agent = BskyAgent::builder().config(bsky_config).build().await?;

        agent
            .login(&identifier.to_string(), &password.to_string())
            .await
            .map_err(|e| Error::BlueskyLoginError(e.to_string()))?;
        // Set labelers from preferences
        let preferences = agent.get_preferences(true).await?;
        agent.configure_labelers_from_preferences(&preferences);

        log::info!("Bluesky login successful!");

        let ci = check_for_ci();

        for bsky_post in &self.bsky_posts {
            log::debug!("Post: {}", bsky_post.post.text.clone());

            if ci {
                log::debug!("Post validation: `{:?}`", "Pretending for CI");

                log::debug!("Deleting related file: {:?}", bsky_post.filename);
                fs::remove_file(&bsky_post.filename)?;

                log::info!(
                    "Successfully posted `{}` to Bluesky and deleted the source file `{}`!",
                    bsky_post.post.text,
                    bsky_post.filename.to_string_lossy().to_string()
                );
            } else {
                let result = agent.create_record(bsky_post.post.clone()).await;

                if result.is_err() {
                    let e = result.unwrap_err();
                    log::error!("Error posting to Bluesky: {}", e);
                    continue;
                };

                let output = result.unwrap();
                log::debug!("Post validation: `{:?}`", output.validation_status.as_ref());

                log::debug!("Deleting related file: {:?}", bsky_post.filename);
                fs::remove_file(&bsky_post.filename)?;

                log::info!(
                    "Successfully posted `{}` to Bluesky and deleted the source file `{}`!",
                    bsky_post.post.text,
                    bsky_post.filename.to_string_lossy().to_string()
                );
            };
        }

        Ok(())
    }
}

fn check_for_ci() -> bool {
    std::env::var("CI").is_ok()
}
