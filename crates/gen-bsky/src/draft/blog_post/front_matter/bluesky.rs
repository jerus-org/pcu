use serde::{Deserialize, Serialize};
use toml::value::Datetime;

use super::tags;

/// Front-matter metadata for a Bluesky post: an optional description and tags
/// (rendered as hashtags), plus the dates the draft was generated (`created`)
/// and published (`published`).
///
/// Crate-internal type, deserialized from a blog post's `[extra.bluesky]` front
/// matter. Accessors return safe defaults for absent fields: `description()` →
/// `""`, `tags()`/`hashtags()` → empty `Vec`. `created`/`published` are skipped
/// during serialization when `None`.
///
/// For usage and the JSON/TOML round-trip behaviour, see the unit tests in this
/// module (e.g. `tests::test_default_creation`, `tests::test_serialize_deserialize`,
/// `tests::test_hashtags_with_tags`).
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Bluesky {
    description: Option<String>,
    tags: Option<Vec<String>>,
    /// Date the Bluesky draft was first generated. Set by `bsky draft`, never overwritten.
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<Datetime>,
    /// Date the post was successfully published to Bluesky. Set by `bsky post`.
    #[serde(skip_serializing_if = "Option::is_none")]
    published: Option<Datetime>,
}
impl Bluesky {
    pub(crate) fn description(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }
    #[allow(dead_code)]
    pub(crate) fn tags(&self) -> Vec<String> {
        self.tags.clone().unwrap_or_default()
    }
    pub(crate) fn hashtags(&self) -> Vec<String> {
        tags::hashtags(self.tags.clone().unwrap_or_default())
    }
    pub(crate) fn created(&self) -> Option<Datetime> {
        self.created
    }
    #[allow(dead_code)]
    pub(crate) fn published(&self) -> Option<Datetime> {
        self.published
    }
}

#[cfg(test)]
mod tests {
    // use serde_json;

    use super::*;

    #[test]
    fn test_default_creation() {
        let bluesky = Bluesky::default();
        assert_eq!(bluesky.description(), "");
        assert_eq!(bluesky.tags(), Vec::<String>::new());
        assert!(bluesky.description.is_none());
        assert!(bluesky.tags.is_none());
    }

    #[test]
    fn test_description_with_some_value() {
        let bluesky = Bluesky {
            description: Some("Test description".to_string()),
            tags: None,
            ..Default::default()
        };
        assert_eq!(bluesky.description(), "Test description");
    }

    #[test]
    fn test_description_with_none_value() {
        let bluesky = Bluesky {
            description: None,
            tags: None,
            ..Default::default()
        };
        assert_eq!(bluesky.description(), "");
    }

    #[test]
    fn test_description_with_empty_string() {
        let bluesky = Bluesky {
            description: Some("".to_string()),
            tags: None,
            ..Default::default()
        };
        assert_eq!(bluesky.description(), "");
    }

    #[test]
    fn test_tags_with_some_value() {
        let tags_vec = vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()];
        let bluesky = Bluesky {
            description: None,
            tags: Some(tags_vec.clone()),
            ..Default::default()
        };
        assert_eq!(bluesky.tags(), tags_vec);
    }

    #[test]
    fn test_tags_with_none_value() {
        let bluesky = Bluesky {
            description: None,
            tags: None,
            ..Default::default()
        };
        assert_eq!(bluesky.tags(), Vec::<String>::new());
    }

    #[test]
    fn test_tags_with_empty_vec() {
        let bluesky = Bluesky {
            description: None,
            tags: Some(vec![]),
            ..Default::default()
        };
        assert_eq!(bluesky.tags(), Vec::<String>::new());
    }

    #[test]
    fn test_hashtags_with_tags() {
        // Note: This test assumes the tags::hashtags function exists and works
        // You may need to mock this or adjust based on your actual implementation
        let tags_vec = vec!["rust".to_string(), "programming".to_string()];
        let bluesky = Bluesky {
            description: None,
            tags: Some(tags_vec),
            ..Default::default()
        };

        // This will call tags::hashtags with the tags vector
        let hashtags = bluesky.hashtags();
        assert_eq!(
            hashtags,
            vec!["#Rust".to_string(), "#Programming".to_string()]
        );
    }

    #[test]
    fn test_hashtags_with_none_tags() {
        let bluesky = Bluesky {
            description: None,
            tags: None,
            ..Default::default()
        };

        let hashtags = bluesky.hashtags();
        // This should call tags::hashtags with an empty vector
        // Adjust assertion based on your tags::hashtags implementation
        assert_eq!(hashtags, Vec::<String>::new());
    }

    #[test]
    fn test_clone() {
        let original = Bluesky {
            description: Some("Original description".to_string()),
            tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
            ..Default::default()
        };

        let cloned = original.clone();

        assert_eq!(original.description(), cloned.description());
        assert_eq!(original.tags(), cloned.tags());

        // Ensure they are independent (modify clone doesn't affect original)
        // This is automatically handled by Clone derive for owned types
    }

    #[test]
    fn test_debug_formatting() {
        let bluesky = Bluesky {
            description: Some("Debug test".to_string()),
            tags: Some(vec!["debug".to_string()]),
            ..Default::default()
        };

        let debug_string = format!("{bluesky:?}");
        assert!(debug_string.contains("Debug test"));
        assert!(debug_string.contains("debug"));
        assert!(debug_string.contains("Bluesky"));
    }

    #[test]
    fn test_serialize_deserialize() -> Result<(), serde_json::Error> {
        let original = Bluesky {
            description: Some("Serialize test".to_string()),
            tags: Some(vec!["serde".to_string(), "json".to_string()]),
            ..Default::default()
        };

        // Serialize to JSON
        let json_string = serde_json::to_string(&original)?;

        // Deserialize back
        let deserialized: Bluesky = serde_json::from_str(&json_string)?;

        assert_eq!(original.description(), deserialized.description());
        assert_eq!(original.tags(), deserialized.tags());

        Ok(())
    }

    #[test]
    fn test_serialize_deserialize_with_nulls() -> Result<(), serde_json::Error> {
        let original = Bluesky::default();

        let json_string = serde_json::to_string(&original)?;
        let deserialized: Bluesky = serde_json::from_str(&json_string)?;

        assert_eq!(original.description(), deserialized.description());
        assert_eq!(original.tags(), deserialized.tags());

        Ok(())
    }

    #[test]
    fn test_deserialize_from_json_string() -> Result<(), serde_json::Error> {
        let json = r#"{"description":"From JSON","tags":["json","test"]}"#;
        let bluesky: Bluesky = serde_json::from_str(json)?;

        assert_eq!(bluesky.description(), "From JSON");
        assert_eq!(bluesky.tags(), vec!["json".to_string(), "test".to_string()]);

        Ok(())
    }

    #[test]
    fn test_deserialize_partial_json() -> Result<(), serde_json::Error> {
        // Test with only description
        let json1 = r#"{"description":"Only description"}"#;
        let bluesky1: Bluesky = serde_json::from_str(json1)?;
        assert_eq!(bluesky1.description(), "Only description");
        assert_eq!(bluesky1.tags(), Vec::<String>::new());

        // Test with only tags
        let json2 = r#"{"tags":["only","tags"]}"#;
        let bluesky2: Bluesky = serde_json::from_str(json2)?;
        assert_eq!(bluesky2.description(), "");
        assert_eq!(
            bluesky2.tags(),
            vec!["only".to_string(), "tags".to_string()]
        );

        // Test with empty JSON (should use defaults)
        let json3 = r#"{}"#;
        let bluesky3: Bluesky = serde_json::from_str(json3)?;
        assert_eq!(bluesky3.description(), "");
        assert_eq!(bluesky3.tags(), Vec::<String>::new());

        Ok(())
    }

    // RED: created / published date fields (issue #909)

    #[test]
    fn test_bluesky_created_is_none_by_default() {
        let bluesky = Bluesky::default();
        assert!(bluesky.created().is_none());
    }

    #[test]
    fn test_bluesky_published_is_none_by_default() {
        let bluesky = Bluesky::default();
        assert!(bluesky.published().is_none());
    }

    #[test]
    fn test_bluesky_created_roundtrips_through_toml() {
        let toml = r#"
description = "My post"
created = 2026-04-03
"#;
        let bluesky: Bluesky = toml::from_str(toml).unwrap();
        let dt = bluesky.created().expect("created should be Some");
        assert_eq!(dt.to_string(), "2026-04-03");
    }

    #[test]
    fn test_bluesky_published_roundtrips_through_toml() {
        let toml = r#"
description = "My post"
published = 2026-04-03
"#;
        let bluesky: Bluesky = toml::from_str(toml).unwrap();
        let dt = bluesky.published().expect("published should be Some");
        assert_eq!(dt.to_string(), "2026-04-03");
    }

    #[test]
    fn test_bluesky_created_omitted_when_none_in_toml_serialization() {
        let bluesky = Bluesky::default();
        let toml_str = toml::to_string(&bluesky).unwrap();
        assert!(
            !toml_str.contains("created"),
            "None created should not appear in serialized TOML: {toml_str}"
        );
        assert!(
            !toml_str.contains("published"),
            "None published should not appear in serialized TOML: {toml_str}"
        );
    }

    #[test]
    fn test_bluesky_with_unicode_content() {
        let bluesky = Bluesky {
            description: Some("描述内容 🚀 émojis and ünïcödé".to_string()),
            tags: Some(vec![
                "🏷️".to_string(),
                "тег".to_string(),
                "标签".to_string(),
            ]),
            ..Default::default()
        };

        assert_eq!(bluesky.description(), "描述内容 🚀 émojis and ünïcödé");
        assert_eq!(
            bluesky.tags(),
            vec!["🏷️".to_string(), "тег".to_string(), "标签".to_string()]
        );
    }

    #[test]
    fn test_tags_independence() {
        let original_tags = vec!["tag1".to_string(), "tag2".to_string()];
        let bluesky = Bluesky {
            description: None,
            tags: Some(original_tags.clone()),
            ..Default::default()
        };

        let mut retrieved_tags = bluesky.tags();
        retrieved_tags.push("tag3".to_string());

        // Original should be unchanged since tags() clones
        assert_eq!(bluesky.tags(), original_tags);
        assert_ne!(bluesky.tags(), retrieved_tags);
    }
}
