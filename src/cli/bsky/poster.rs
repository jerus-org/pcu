use bsky_sdk::{
    agent::config::Config,
    api::{app::bsky::feed::post::RecordData, types::string::Datetime},
    rich_text::RichText,
    BskyAgent,
};
use color_eyre::Result;

use super::front_matter::FrontMatter;

#[derive(Clone)]
pub struct Poster {
    blog_posts: Vec<FrontMatter>,
    #[allow(dead_code)]
    agent: BskyAgent,
}

impl Poster {
    pub async fn new<S>(blog_posts: Vec<FrontMatter>, identifier: S, password: S) -> Result<Self>
    where
        S: ToString,
    {
        // TODO: login to bluesky

        if identifier.to_string().is_empty() {
            return Err(color_eyre::eyre::eyre!("No identifier provided"));
        };

        if password.to_string().is_empty() {
            return Err(color_eyre::eyre::eyre!("No password provided"));
        };

        let bsky_config = Config::default();

        let agent = BskyAgent::builder().config(bsky_config).build().await?;

        agent
            .login(&identifier.to_string(), &password.to_string())
            .await?;
        // Set labelers from preferences
        let preferences = agent.get_preferences(true).await?;
        agent.configure_labelers_from_preferences(&preferences);

        log::info!("Bluesky login successful!");

        Ok(Poster { blog_posts, agent })
    }

    pub async fn post_to_bluesky(&self) -> Result<()> {
        // TODO: Create a post

        for blog_post in &self.blog_posts {
            log::info!("Blog post: {blog_post:#?}");

            let post_text = format!(
                "{}\n\n{} #{}",
                blog_post.title,
                blog_post.description,
                blog_post.taxonomies.tags.join(" #")
            );

            log::debug!("Post text: {post_text}");

            let rt = RichText::new_with_detect_facets(&post_text).await?;

            log::debug!("Rich text: {rt:#?}");

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

            log::info!("{:?}", record_data);

            // self.agent.create_record(subject).await?;
        }

        Ok(())
    }
}
