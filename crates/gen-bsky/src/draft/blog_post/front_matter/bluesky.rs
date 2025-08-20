/// # Bluesky Struct Documentation
///
/// ## Overview
///
/// The `Bluesky` struct represents metadata for Bluesky social media posts,
/// encapsulating optional description text and tags that can be converted to
/// hashtags. It provides a clean API for handling post content with proper
/// serialization support and safe access to optional fields.
///
/// ## Struct Definition
///
/// ```rust
/// #[derive(Default, Debug, Clone, Serialize, Deserialize)]
/// pub(crate) struct Bluesky {
///     description: Option<String>,
///     tags: Option<Vec<String>>,
/// }
/// ```
///
/// ### Derives and Attributes
///
/// - **`Default`**: Provides automatic default implementation (both fields as
///   `None`)
/// - **`Debug`**: Enables debug formatting for development and logging
/// - **`Clone`**: Allows deep copying of the struct and its contents
/// - **`Serialize`**: Enables JSON/TOML serialization via serde
/// - **`Deserialize`**: Enables JSON/TOML deserialization via serde
/// - **`pub(crate)`**: Visible within the current crate only
///
/// ### Fields
///
/// #### `description: Option<String>`
/// - **Purpose**: Optional description text for the Bluesky post
/// - **Type**: `Option<String>` - can be `None` (no description) or
///   `Some(String)`
/// - **Usage**: Contains the main descriptive text content for the post
/// - **Default**: `None` when using `Default::default()`
///
/// #### `tags: Option<Vec<String>>`
/// - **Purpose**: Optional list of tags to be converted to hashtags
/// - **Type**: `Option<Vec<String>>` - can be `None` (no tags) or
///   `Some(Vec<String>)`
/// - **Usage**: Raw tag strings that will be processed into hashtag format
/// - **Default**: `None` when using `Default::default()`
///
/// ## Public API Methods
///
/// ### `description(&self) -> &str`
///
/// Returns the description as a string slice, providing a safe default for
/// `None` values.
///
/// ```rust ignore
/// pub(crate) fn description(&self) -> &str {
///     self.description.as_deref().unwrap_or("")
/// }
/// ```
///
/// **Behaviour**:
/// - **When `Some(description)`**: Returns `&str` reference to the description
/// - **When `None`**: Returns empty string `""`
/// - **Memory**: No allocation, returns borrowed reference or static string
///
/// **Usage Examples**:
/// ```rust ignore
/// let bluesky = Bluesky {
///     description: Some("Hello world".to_string()),
///     tags: None,
/// };
/// assert_eq!(bluesky.description(), "Hello world");
///
/// let empty = Bluesky::default();
/// assert_eq!(empty.description(), "");
/// ```
///
/// ### `tags(&self) -> Vec<String>`
///
/// Returns a clone of the tags vector, providing an empty vector for `None`
/// values.
///
/// ```rust ignore
/// #[allow(dead_code)]
/// pub(crate) fn tags(&self) -> Vec<String> {
///     self.tags.clone().unwrap_or_default()
/// }
/// ```
///
/// **behaviour**:
/// - **When `Some(tags)`**: Returns cloned `Vec<String>` with all tags
/// - **When `None`**: Returns empty `Vec<String>`
/// - **Memory**: Always allocates a new vector (clone or empty)
///
/// **Note**: Marked with `#[allow(dead_code)]`, indicating it may not be
/// actively used in the codebase but is retained for API completeness.
///
/// **Usage Examples**:
/// ```rust ignore
/// let bluesky = Bluesky {
///     description: None,
///     tags: Some(vec![
///         "rust".to_string(),
///         "programming".to_string(),
///     ]),
/// };
/// assert_eq!(bluesky.tags(), vec!["rust", "programming"]);
///
/// let empty = Bluesky::default();
/// assert_eq!(empty.tags(), Vec::<String>::new());
/// ```
///
/// ### `hashtags(&self) -> Vec<String>`
///
/// Processes the tags through the `tags::hashtags` function to create properly
/// formatted hashtags.
///
/// ```rust ignore
/// pub(crate) fn hashtags(&self) -> Vec<String> {
///     tags::hashtags(self.tags.clone().unwrap_or_default())
/// }
/// ```
///
/// **behaviour**:
/// - **When `Some(tags)`**: Processes tags through `tags::hashtags()` function
/// - **When `None`**: Processes empty vector, returns empty vector
/// - **Processing**: Tags are capitalized, spaces removed, and prefixed with
///   `#`
///
/// **Dependencies**: Requires the `tags::hashtags` function from the parent
/// module.
///
/// **Usage Examples**:
/// ```rust ignore
/// let bluesky = Bluesky {
///     description: Some("My post".to_string()),
///     tags: Some(vec![
///         "rust programming".to_string(),
///         "web dev".to_string(),
///     ]),
/// };
/// assert_eq!(
///     bluesky.hashtags(),
///     vec!["#RustProgramming", "#WebDev"]
/// );
///
/// let empty = Bluesky::default();
/// assert_eq!(empty.hashtags(), Vec::<String>::new());
/// ```
///
/// ## Serialization Support
///
/// The struct supports JSON and TOML serialization/deserialization through
/// serde.
///
/// ### JSON Format
///
/// #### Serialization Examples
///
/// **Full struct**:
/// ```json
/// {
///   "description": "A great blog post about Rust",
///   "tags": ["rust programming", "web development", "tutorial"]
/// }
/// ```
///
/// **Partial struct**:
/// ```json
/// {
///   "description": "Just a description",
///   "tags": null
/// }
/// ```
///
/// **Empty struct** (Default):
/// ```json
/// {
///   "description": null,
///   "tags": null
/// }
/// ```
///
/// #### Deserialization Flexibility
///
/// The struct can be deserialized from various JSON formats:
///
/// ```json
/// // Complete
/// {
///   "description": "Post content",
///   "tags": ["tag1", "tag2"]
/// }
///
/// // Missing fields (will be None)
/// {
///   "description": "Only description"
/// }
///
/// // Empty object (all fields None)
/// {}
///
/// // Explicit nulls
/// {
///   "description": null,
///   "tags": null
/// }
/// ```
///
/// ### TOML Format
///
/// #### Serialization Examples
///
/// **Full struct**:
/// ```toml
/// description = "A great blog post about Rust"
/// tags = ["rust programming", "web development", "tutorial"]
/// ```
///
/// **Partial struct**:
/// ```toml
/// description = "Just a description"
/// ```
///
/// **Empty struct**: Produces no TOML content (all fields are `None`)
///
/// ## Usage Patterns
///
/// ### Construction Methods
///
/// #### Default Construction
/// ```rust
/// let bluesky = Bluesky::default();
/// // Both description and tags are None
/// ```
///
/// #### Field-by-Field Construction
/// ```rust
/// let bluesky = Bluesky {
///     description: Some("My post description".to_string()),
///     tags: Some(vec![
///         "rust".to_string(),
///         "programming".to_string(),
///     ]),
/// };
/// ```
///
/// ### Common Usage Scenarios
///
/// #### Blog Post Metadata
/// ```rust ignore
/// let post_metadata = Bluesky {
///     description: Some(
///         "Introduction to Rust programming language".to_string(),
///     ),
///     tags: Some(vec![
///         "rust programming".to_string(),
///         "beginners guide".to_string(),
///         "systems programming".to_string(),
///     ]),
/// };
///
/// let hashtag_string = post_metadata.hashtags().join(" ");
/// // Result: "#RustProgramming #BeginnersGuide #SystemsProgramming"
/// ```
///
/// #### Social Media Post Generation
/// ```rust ignore
/// fn generate_bluesky_post(metadata: &Bluesky) -> String {
///     let description = metadata.description();
///     let hashtags = metadata.hashtags();
///
///     if description.is_empty() && hashtags.is_empty() {
///         "No content available".to_string()
///     } else if hashtags.is_empty() {
///         description.to_string()
///     } else {
///         format!("{}\n\n{}", description, hashtags.join(" "))
///     }
/// }
/// ```
///
/// #### Configuration File Processing
/// ```rust ignore
/// use serde_json;
///
/// // Load from JSON configuration
/// let json_data = r#"
/// {
///     "description": "My awesome project",
///     "tags": ["rust", "web development", "open source"]
/// }
/// "#;
///
/// let bluesky: Bluesky = serde_json::from_str(json_data)?;
/// println!(
///     "Post: {} {}",
///     bluesky.description(),
///     bluesky.hashtags().join(" ")
/// );
/// ```
///
/// ## Memory and Performance Characteristics
///
/// ### Memory Usage
/// - **Base struct**: Minimal overhead (two `Option` pointers)
/// - **With data**: Memory proportional to string content length
/// - **Cloning**: Deep clone includes all string data
///
/// ### Performance Characteristics
///
/// #### Time Complexity
/// - **`description()`**: O(1) - just reference or static string
/// - **`tags()`**: O(n) where n = number of tags (due to cloning)
/// - **`hashtags()`**: O(n × m) where n = number of tags, m = average tag
///   processing time
///
/// #### Space Complexity
/// - **`description()`**: O(1) - no additional allocation
/// - **`tags()`**: O(n) - full vector clone
/// - **`hashtags()`**: O(n) - new vector with processed strings
///
/// #### Memory Allocations
/// - **Construction**: Minimal allocations for `Option` wrappers
/// - **`tags()`**: One allocation for vector clone
/// - **`hashtags()`**: Multiple allocations for tag processing and result
///   vector
///
/// ## Error Handling
///
/// ### Panic Conditions
/// The current implementation is panic-free:
/// - All `Option` access uses safe methods (`as_deref()`,
///   `unwrap_or_default()`)
/// - No direct indexing or unwrapping of potentially `None` values
///
/// ### Serialization Errors
/// - **Deserialization**: May fail if JSON/TOML format is invalid
/// - **Serialization**: Generally panic-free for valid struct content
/// - **Field validation**: No built-in validation; accepts any string content
///
/// ## Thread Safety
///
/// The struct is thread-safe for immutable operations:
/// - **Reading**: All getter methods are safe for concurrent access
/// - **Cloning**: Safe to clone across threads
/// - **Serialization**: Safe to serialize concurrently
/// - **Mutation**: Direct field mutation would require synchronization
///
/// ## Integration with External Systems
///
/// ### Bluesky API Integration
/// ```rust ignore
/// // Example integration with Bluesky posting API
/// async fn post_to_bluesky(
///     metadata: &Bluesky,
///     client: &BlueSkyClient,
/// ) -> Result<()> {
///     let text = format!(
///         "{}\n\n{}",
///         metadata.description(),
///         metadata.hashtags().join(" ")
///     );
///
///     client.create_post(text).await
/// }
/// ```
///
/// ### Content Management Systems
/// ```rust ignore
/// // Example CMS integration
/// struct BlogPost {
///     title: String,
///     content: String,
///     bluesky_meta: Bluesky,
/// }
///
/// impl BlogPost {
///     fn generate_social_post(&self) -> String {
///         let description =
///             if self.bluesky_meta.description().is_empty() {
///                 &self.title
///             } else {
///                 self.bluesky_meta.description()
///             };
///
///         format!(
///             "{}\n\n{}",
///             description,
///             self.bluesky_meta.hashtags().join(" ")
///         )
///     }
/// }
/// ```
///
/// ## Best Practices
///
/// ### Construction
/// - Use `Default::default()` for empty instances
/// - Prefer `Some(value)` for explicit content over default empty strings
/// - Consider validation of tag content before storage
///
/// ### API Usage
/// - Use `description()` method rather than direct field access
/// - Prefer `hashtags()` over `tags()` for social media integration
/// - Clone sparingly due to performance implications
///
/// ### Serialization
/// - Include explicit `null` handling in deserialization code
/// - Validate deserialized content for business logic requirements
/// - Consider using custom deserializers for advanced validation
///
/// ### Error Handling
/// - Wrap deserialization in proper error handling
/// - Validate tag content for length and character restrictions
/// - Consider implementing custom validation methods
///
/// ## Future Enhancements
///
/// ### Potential Additions
/// - **Validation methods**: Tag length limits, content validation
/// - **Builder pattern**: Fluent API for construction
/// - **Custom serialization**: More control over JSON/TOML format
/// - **Metadata fields**: Created date, author, post ID
/// - **Character counting**: For Bluesky post length limits
///
/// ### API Extensions
/// ```rust ignore
/// impl Bluesky {
///     // Validation
///     pub(crate) fn is_valid(&self) -> bool { /* ... */ }
///     pub(crate) fn validate(&self) -> Result<(), ValidationError> { /* ... */ }
///     
///     // Content manipulation
///     pub(crate) fn add_tag(&mut self, tag: String) { /* ... */ }
///     pub(crate) fn set_description(&mut self, desc: String) { /* ... */ }
///     
///     // Utility methods
///     pub(crate) fn is_empty(&self) -> bool { /* ... */ }
///     pub(crate) fn character_count(&self) -> usize { /* ...
/// ```
use serde::{Deserialize, Serialize};

use super::tags;
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Bluesky {
    description: Option<String>,
    tags: Option<Vec<String>>,
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
        };
        assert_eq!(bluesky.description(), "Test description");
    }

    #[test]
    fn test_description_with_none_value() {
        let bluesky = Bluesky {
            description: None,
            tags: None,
        };
        assert_eq!(bluesky.description(), "");
    }

    #[test]
    fn test_description_with_empty_string() {
        let bluesky = Bluesky {
            description: Some("".to_string()),
            tags: None,
        };
        assert_eq!(bluesky.description(), "");
    }

    #[test]
    fn test_tags_with_some_value() {
        let tags_vec = vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()];
        let bluesky = Bluesky {
            description: None,
            tags: Some(tags_vec.clone()),
        };
        assert_eq!(bluesky.tags(), tags_vec);
    }

    #[test]
    fn test_tags_with_none_value() {
        let bluesky = Bluesky {
            description: None,
            tags: None,
        };
        assert_eq!(bluesky.tags(), Vec::<String>::new());
    }

    #[test]
    fn test_tags_with_empty_vec() {
        let bluesky = Bluesky {
            description: None,
            tags: Some(vec![]),
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

    #[test]
    fn test_with_unicode_content() {
        let bluesky = Bluesky {
            description: Some("描述内容 🚀 émojis and ünïcödé".to_string()),
            tags: Some(vec![
                "🏷️".to_string(),
                "тег".to_string(),
                "标签".to_string(),
            ]),
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
        };

        let mut retrieved_tags = bluesky.tags();
        retrieved_tags.push("tag3".to_string());

        // Original should be unchanged since tags() clones
        assert_eq!(bluesky.tags(), original_tags);
        assert_ne!(bluesky.tags(), retrieved_tags);
    }
}
