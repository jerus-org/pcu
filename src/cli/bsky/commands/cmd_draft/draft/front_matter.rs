use crate::Error;
use bsky_sdk::api::app::bsky::feed::post::RecordData;
use serde::Deserialize;

// +++
// title = "Blue Sky Test Blog"
// description = "A blog post to test the processing of blog posts for posting to Bluesky."
// date = 2025-01-17
// updated = 2025-01-16
// draft = false
//
// [taxonomies]
// topic = ["Technology"]
// tags = ["bluesky", "testing", "test only", "ci"]
// +++
//

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Taxonomies {
    #[allow(dead_code)]
    pub tags: Vec<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Extra {
    #[allow(dead_code)]
    pub bluesky: String,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub description: String,
    pub taxonomies: Taxonomies,
    pub extra: Option<Extra>,
    pub basename: Option<String>,
    pub path: Option<String>,
    pub bluesky_post: Option<RecordData>,
}

impl FrontMatter {
    pub fn from_toml(toml: &str) -> Result<Self, Error> {
        let front_matter = toml::from_str::<FrontMatter>(toml)?;
        Ok(front_matter)
    }
}
