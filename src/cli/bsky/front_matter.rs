use serde::Deserialize;

/// +++
/// title = "Blue Sky Test Blog"
/// description = "A blog post to test the processing of blog posts for posting to Bluesky."
/// date = 2025-01-17
/// updated = 2025-01-16
/// draft = false
///
/// [taxonomies]
/// topic = ["Technology"]
/// tags = ["bluesky", "testing", "test only", "ci"]
/// +++
///

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Taxonomies {
    #[allow(dead_code)]
    pub tags: Vec<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct FrontMatter {
    #[allow(dead_code)]
    pub title: String,
    #[allow(dead_code)]
    pub description: String,
    #[allow(dead_code)]
    pub taxonomies: Taxonomies,
}
