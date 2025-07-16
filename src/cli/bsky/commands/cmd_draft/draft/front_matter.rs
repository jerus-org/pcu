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

impl Taxonomies {
    pub fn hashtags(&self) -> Vec<String> {
        let mut hashtags = vec![];
        for tag in &self.tags {
            // convert tag to hashtag by capitalising the first letter of each word, removing the spaces and prefixing with a # if required
            let formatted_tag = tag
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<String>();
            let hashtag = if formatted_tag.starts_with('#') {
                formatted_tag
            } else {
                format!("#{formatted_tag}")
            };
            hashtags.push(hashtag);
        }

        hashtags
    }
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
    pub taxonomies: Option<Taxonomies>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_toml_basic() {
        let toml = r#"
            title = "Test Title"
            description = "Test Description"

            [taxonomies]
            tags = ["rust", "testing"]
        "#;
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Test Title");
        assert_eq!(fm.description, "Test Description");
        assert_eq!(fm.taxonomies.unwrap().tags, vec!["rust", "testing"]);
        assert!(fm.extra.is_none());
        assert!(fm.basename.is_none());
        assert!(fm.path.is_none());
        assert!(fm.bluesky_post.is_none());
    }

    #[test]
    fn test_from_toml_with_extra() {
        let toml = r#"
            title = "Extra Test"
            description = "Has extra field"

            [taxonomies]
            tags = ["extra"]

            [extra]
            bluesky = "extra_value"
        "#;
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.taxonomies.unwrap().tags, vec!["extra"]);
        assert!(fm.extra.is_some());
        assert_eq!(fm.extra.unwrap().bluesky, "extra_value");
    }

    #[test]
    fn test_from_toml_missing_fields() {
        let toml = r#"
            title = "Missing Fields"
            description = "No taxonomies"
        "#;
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Missing Fields");
        assert_eq!(fm.description, "No taxonomies");
        assert!(fm.taxonomies.is_none());
    }

    #[test]
    fn test_from_toml_invalid() {
        let toml = r#"
            title = 123
            description = "Invalid type"
        "#;
        let result = FrontMatter::from_toml(toml);
        assert!(result.is_err());
    }
    #[test]
    fn test_hashtags_formatting() {
        let taxonomies = Taxonomies {
            tags: vec![
                "rust".to_string(),
                "blue sky".to_string(),
                "#AlreadyHashtag".to_string(),
                "multi word tag".to_string(),
                "".to_string(),
            ],
        };
        let hashtags = taxonomies.hashtags();
        assert_eq!(
            hashtags,
            vec!["#Rust", "#BlueSky", "#AlreadyHashtag", "#MultiWordTag", "#"]
        );
    }

    #[test]
    fn test_front_matter_with_basename_and_path() {
        let toml = r#"
            title = "With Path"
            description = "Has basename and path"
            basename = "post1"
            path = "/blog/post1.md"
        "#;
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.basename.as_deref(), Some("post1"));
        assert_eq!(fm.path.as_deref(), Some("/blog/post1.md"));
    }

    #[test]
    fn test_front_matter_empty_toml() {
        let toml = r#"
            title = "Extra Test"
            description = "Has extra field"
        "#;
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.description, "Has extra field");
        assert!(fm.taxonomies.is_none());
        assert!(fm.extra.is_none());
    }

    #[test]
    fn test_taxonomies_empty_tags() {
        let taxonomies = Taxonomies { tags: vec![] };
        let hashtags = taxonomies.hashtags();
        assert_eq!(hashtags, Vec::<String>::new());
    }
}
