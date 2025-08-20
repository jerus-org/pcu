use serde::{Deserialize, Serialize};

/// Additional metadata extracted from TOML frontmatter in markdown files.
///
/// The `Extra` struct serves as a container for optional metadata that can be
/// included in markdown files' TOML frontmatter. Currently, it supports Bluesky
/// social media configuration for automated posting.
///
/// # Examples
///
/// ## TOML Frontmatter Usage
///
/// ```toml
/// # In your markdown file's frontmatter
/// [extra]
/// [extra.bluesky]
/// description = "Check out this amazing blog post!"
/// tags = ["rust", "programming", "blog"]
/// ```
///
/// ## Rust Usage
///
/// ```rust
/// use your_crate::Extra;
///
/// // Create an empty Extra struct
/// let extra = Extra::default();
/// assert!(extra.bluesky().is_none());
///
/// // Access Bluesky configuration if present
/// if let Some(bluesky_config) = extra.bluesky() {
///     // Use the Bluesky configuration for social media posting
/// }
/// ```
///
/// # Serialization
///
/// This struct implements both `Serialize` and `Deserialize` from serde,
/// making it suitable for use with TOML, JSON, and other supported formats.
///
/// # Thread Safety
///
/// This struct derives `Clone`, making it easy to share across threads
/// when wrapped in appropriate synchronization primitives.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Extra {
    /// Optional Bluesky social media configuration.
    ///
    /// When present, this field contains metadata such as custom descriptions
    /// and hashtags that should be used when automatically posting content
    /// to Bluesky social media platform.
    ///
    /// The `#[allow(dead_code)]` attribute is used because this field might
    /// only be accessed through the public accessor method `bluesky()`.
    #[allow(dead_code)]
    bluesky: Option<super::Bluesky>,
}

impl Extra {
    /// Returns a reference to the Bluesky configuration, if present.
    ///
    /// This method provides read-only access to the Bluesky social media
    /// configuration stored in the frontmatter. It returns `None` if no
    /// Bluesky configuration was specified.
    ///
    /// # Returns
    ///
    /// * `Some(&Bluesky)` - A reference to the Bluesky configuration
    /// * `None` - No Bluesky configuration is present
    ///
    /// # Examples
    ///
    /// ```rust
    /// use your_crate::Extra;
    ///
    /// let extra = Extra::default();
    ///
    /// match extra.bluesky() {
    ///     Some(config) => {
    ///         // Process Bluesky configuration
    ///         println!("Bluesky config found");
    ///     }
    ///     None => {
    ///         println!("No Bluesky configuration");
    ///     }
    /// }
    /// ```
    pub(crate) fn bluesky(&self) -> Option<&super::Bluesky> {
        self.bluesky.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock Bluesky struct for testing
    // In a real implementation, this would be imported from super::Bluesky
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct MockBluesky {
        pub description: Option<String>,
        pub tags: Option<Vec<String>>,
    }

    impl MockBluesky {
        pub fn new(description: Option<String>, tags: Option<Vec<String>>) -> Self {
            Self { description, tags }
        }
    }

    // For testing purposes, we'll create a test version of Extra that uses
    // MockBluesky
    #[derive(Default, Debug, Clone, Serialize, Deserialize)]
    struct TestExtra {
        bluesky: Option<MockBluesky>,
    }

    impl TestExtra {
        fn bluesky(&self) -> Option<&MockBluesky> {
            self.bluesky.as_ref()
        }

        fn with_bluesky(bluesky: MockBluesky) -> Self {
            Self {
                bluesky: Some(bluesky),
            }
        }

        fn has_config(&self) -> bool {
            self.bluesky.is_some()
        }
    }

    #[test]
    fn test_default_extra() {
        let extra = TestExtra::default();
        assert!(extra.bluesky().is_none());
        assert!(!extra.has_config());
    }

    #[test]
    fn test_extra_with_bluesky() {
        let bluesky_config = MockBluesky::new(
            Some("Test description".to_string()),
            Some(vec!["rust".to_string(), "test".to_string()]),
        );

        let extra = TestExtra::with_bluesky(bluesky_config.clone());

        assert!(extra.bluesky().is_some());
        assert!(extra.has_config());

        let retrieved_config = extra.bluesky().unwrap();
        assert_eq!(retrieved_config, &bluesky_config);
    }

    #[test]
    fn test_bluesky_accessor() {
        let mut extra = TestExtra::default();

        // Test None case
        assert!(extra.bluesky().is_none());

        // Test Some case
        let bluesky_config = MockBluesky::new(
            Some("Accessor test".to_string()),
            Some(vec!["testing".to_string()]),
        );
        extra = TestExtra::with_bluesky(bluesky_config);

        let retrieved = extra.bluesky();
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().description,
            Some("Accessor test".to_string())
        );
        assert_eq!(retrieved.unwrap().tags, Some(vec!["testing".to_string()]));
    }

    #[test]
    fn test_has_config() {
        // Empty configuration
        let empty_extra = TestExtra::default();
        assert!(!empty_extra.has_config());

        // With Bluesky configuration
        let bluesky_config = MockBluesky::new(None, None);
        let extra_with_bluesky = TestExtra::with_bluesky(bluesky_config);
        assert!(extra_with_bluesky.has_config());
    }

    #[test]
    fn test_clone() {
        let bluesky_config = MockBluesky::new(
            Some("Clone test".to_string()),
            Some(vec!["clone".to_string(), "test".to_string()]),
        );

        let original = TestExtra::with_bluesky(bluesky_config);
        let cloned = original.clone();

        assert_eq!(original.bluesky(), cloned.bluesky());
        assert!(cloned.has_config());
    }

    #[test]
    fn test_serialization_deserialization() {
        use serde_json;

        // Test empty Extra
        let empty_extra = TestExtra::default();
        let json = serde_json::to_string(&empty_extra).unwrap();
        let deserialized: TestExtra = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.has_config());

        // Test Extra with Bluesky config
        let bluesky_config = MockBluesky::new(
            Some("Serialization test".to_string()),
            Some(vec!["serde".to_string(), "json".to_string()]),
        );
        let extra_with_config = TestExtra::with_bluesky(bluesky_config);

        let json = serde_json::to_string(&extra_with_config).unwrap();
        let deserialized: TestExtra = serde_json::from_str(&json).unwrap();

        assert!(deserialized.has_config());
        let bluesky = deserialized.bluesky().unwrap();
        assert_eq!(bluesky.description, Some("Serialization test".to_string()));
        assert_eq!(
            bluesky.tags,
            Some(vec!["serde".to_string(), "json".to_string()])
        );
    }

    #[test]
    fn test_toml_integration() {
        use toml;

        // Test deserializing from TOML string (simulating frontmatter)
        let toml_str = r#"
[bluesky]
description = "TOML integration test"
tags = ["toml", "frontmatter", "markdown"]
"#;

        let extra: TestExtra = toml::from_str(toml_str).unwrap();
        assert!(extra.has_config());

        let bluesky = extra.bluesky().unwrap();
        assert_eq!(
            bluesky.description,
            Some("TOML integration test".to_string())
        );
        assert_eq!(
            bluesky.tags,
            Some(vec![
                "toml".to_string(),
                "frontmatter".to_string(),
                "markdown".to_string()
            ])
        );

        // Test serializing to TOML
        let serialized_toml = toml::to_string(&extra).unwrap();
        assert!(serialized_toml.contains("description = \"TOML integration test\""));
        assert!(serialized_toml.contains("tags = [\"toml\", \"frontmatter\", \"markdown\"]"));
    }

    #[test]
    fn test_empty_toml_deserialization() {
        use toml;

        // Test that empty TOML creates default Extra
        let empty_toml = "";
        let extra: TestExtra = toml::from_str(empty_toml).unwrap();
        assert!(!extra.has_config());
        assert!(extra.bluesky().is_none());
    }

    #[test]
    fn test_partial_bluesky_config() {
        use toml;

        // Test with only description
        let toml_str = r#"
[bluesky]
description = "Only description"
"#;

        let extra: TestExtra = toml::from_str(toml_str).unwrap();
        let bluesky = extra.bluesky().unwrap();
        assert_eq!(bluesky.description, Some("Only description".to_string()));
        assert_eq!(bluesky.tags, None);

        // Test with only tags
        let toml_str = r#"
[bluesky]
tags = ["only", "tags"]
"#;

        let extra: TestExtra = toml::from_str(toml_str).unwrap();
        let bluesky = extra.bluesky().unwrap();
        assert_eq!(bluesky.description, None);
        assert_eq!(
            bluesky.tags,
            Some(vec!["only".to_string(), "tags".to_string()])
        );
    }

    #[test]
    fn test_debug_implementation() {
        let extra = TestExtra::default();
        let debug_str = format!("{extra:?}");
        assert!(debug_str.contains("TestExtra"));

        let bluesky_config = MockBluesky::new(
            Some("Debug test".to_string()),
            Some(vec!["debug".to_string()]),
        );
        let extra_with_config = TestExtra::with_bluesky(bluesky_config);
        let debug_str = format!("{extra_with_config:?}");
        assert!(debug_str.contains("Debug test"));
        assert!(debug_str.contains("debug"));
    }
}
