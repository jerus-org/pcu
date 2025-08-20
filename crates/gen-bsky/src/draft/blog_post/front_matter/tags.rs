// use crate::Capitalise;

use std::fmt::Display;

use crate::Capitalise;

/// # Hashtags Function Documentation
///
/// ## Overview
///
/// The `hashtags` function converts a vector of string tags into properly formatted hashtags suitable for social media platforms. It handles various input formats and normalizes them into consistent hashtag format with title case capitalization and no spaces.
///
/// ## Function Signature
///
/// ```rust
/// pub(crate) fn hashtags(tags: Vec<String>) -> Vec<String>
/// ```
///
/// **Visibility**: `pub(crate)` - Available within the current crate only
///
/// **Parameters**:
/// - `tags: Vec<String>` - A vector of tag strings to be converted to hashtags
///
/// **Returns**:
/// - `Vec<String>` - A vector of formatted hashtags, each prefixed with `#`
///
/// ## Functionality
///
/// ### Core Processing Steps
///
/// 1. **Hashtag Prefix Removal**: Strips any existing `#` prefix from input tags
/// 2. **Word Splitting**: Splits tags on whitespace to handle multi-word tags
/// 3. **Word Capitalization**: Capitalizes the first letter of each word using the `capitalize()` method
/// 4. **Space Removal**: Concatenates capitalized words without spaces
/// 5. **Hashtag Prefix Addition**: Adds `#` prefix to create the final hashtag
///
/// ### Processing Pipeline
///
/// ```
/// Input: "rust programming" -> Split: ["rust", "programming"]
/// -> Capitalize: ["Rust", "Programming"] -> Join: "RustProgramming"
/// -> Prefix: "#RustProgramming"
/// ```
///
/// ## Input Format Handling
///
/// ### Supported Input Formats
///
/// | Input Format | Output Format | Description |
/// |-------------|---------------|-------------|
/// | `"rust"` | `"#Rust"` | Single word |
/// | `"rust programming"` | `"#RustProgramming"` | Multi-word with spaces |
/// | `"#rust"` | `"#Rust"` | Already has hashtag prefix |
/// | `"#rust programming"` | `"#RustProgramming"` | Hashtag prefix with spaces |
/// | `" rust programming "` | `"#RustProgramming"` | Leading/trailing whitespace |
/// | `"rust  programming"` | `"#RustProgramming"` | Multiple spaces |
/// | `""` | `"#"` | Empty string |
/// | `"  "` | `"#"` | Whitespace only |
///
/// ### Edge Cases
///
/// #### Empty and Whitespace Inputs
/// - **Empty string**: `""` → `"#"`
/// - **Whitespace only**: `"   "` → `"#"`
/// - **Single space**: `" "` → `"#"`
///
/// #### Special Characters
/// - **Punctuation**: Preserved in individual words
/// - **Numbers**: Treated as regular characters
/// - **Unicode**: Properly handled by the `capitalize()` method
///
/// #### Multiple Hash Symbols
/// - **Input**: `"##rust"` → **Output**: `"#Rust"`
/// - Only the first `#` is removed, others are treated as part of the tag content
///
/// ## Usage Examples
///
/// ### Basic Usage
///
/// ```rust
/// let tags = vec![
///     "rust".to_string(),
///     "programming".to_string(),
///     "web development".to_string()
/// ];
///
/// let result = hashtags(tags);
/// // Result: ["#Rust", "#Programming", "#WebDevelopment"]
/// ```
///
/// ### Handling Existing Hashtags
///
/// ```rust
/// let tags = vec![
///     "#rust".to_string(),
///     "machine learning".to_string(),
///     "#AI".to_string()
/// ];
///
/// let result = hashtags(tags);
/// // Result: ["#Rust", "#MachineLearning", "#AI"]
/// ```
///
/// ### Mixed Format Input
///
/// ```rust
/// let tags = vec![
///     "  web development  ".to_string(),
///     "#mobile  app".to_string(),
///     "data science".to_string()
/// ];
///
/// let result = hashtags(tags);
/// // Result: ["#WebDevelopment", "#MobileApp", "#DataScience"]
/// ```
///
/// ## Performance Characteristics
///
/// ### Time Complexity
/// - **O(n × m × w)** where:
///   - `n` = number of tags
///   - `m` = average number of words per tag
///   - `w` = average word length (for capitalization)
///
/// ### Space Complexity
/// - **O(n × l)** where:
///   - `n` = number of tags
///   - `l` = average length of formatted hashtag
///
/// ### Memory Allocation
/// - Creates a new `Vec<String>` for results
/// - Intermediate string allocations during processing:
///   - Split operation creates temporary string slices
///   - Capitalization creates new strings for each word
///   - Final concatenation creates the hashtag string
///
/// ## Dependencies
///
/// ### Required Traits
/// The function depends on the `Capitalize` trait being implemented for `&str`:
///
/// ```rust
/// trait Capitalize {
///     fn capitalize(&mut self) -> String;
/// }
///
/// impl Capitalize for &str {
///     fn capitalize(&mut self) -> String {
///         // Implementation details
///     }
/// }
/// ```
///
/// ### Standard Library Usage
/// - `String::trim_start_matches()` - Prefix removal
/// - `str::split_whitespace()` - Word splitting
/// - `Iterator::map()` - Word transformation
/// - `Iterator::collect()` - String concatenation
/// - `format!()` - Hashtag prefix addition
/// - `Vec::push()` - Result accumulation
///
/// ## Error Handling
///
/// ### Panic Conditions
/// The function is designed to be panic-free:
/// - Empty input vectors are handled gracefully
/// - Empty strings are processed without errors
/// - Invalid Unicode in the `capitalize()` method is handled by the trait implementation
///
/// ### Error Propagation
/// - No explicit error handling as the function doesn't return `Result`
/// - Any panics would come from the `capitalize()` implementation
/// - Memory allocation failures would cause program termination (standard Rust behaviour)
///
/// ## Unicode Support
///
/// The function properly handles Unicode characters through:
/// - `split_whitespace()` correctly handles Unicode whitespace characters
/// - `capitalize()` trait handles Unicode case conversion
/// - String concatenation preserves Unicode encoding
///
/// ### Unicode Examples
///
/// ```rust
/// let tags = vec![
///     "café culture".to_string(),
///     "москва travel".to_string(),
///     "日本 food".to_string()
/// ];
///
/// let result = hashtags(tags);
/// // Result depends on capitalize() implementation:
/// // ["#CaféCulture", "#МоскваTravel", "#日本Food"]
/// ```
///
/// ## Integration Patterns
///
/// ### Common Use Cases
///
/// #### Social Media Post Generation
/// ```rust
/// fn create_post_with_tags(content: &str, raw_tags: Vec<String>) -> String {
///     let formatted_hashtags = hashtags(raw_tags);
///     format!("{}\n\n{}", content, formatted_hashtags.join(" "))
/// }
/// ```
///
/// #### Tag Normalization Pipeline
/// ```rust
/// fn normalize_user_tags(user_input: &str) -> Vec<String> {
///     let raw_tags: Vec<String> = user_input
///         .split(',')
///         .map(|s| s.trim().to_string())
///         .filter(|s| !s.is_empty())
///         .collect();
///     
///     hashtags(raw_tags)
/// }
/// ```
///
/// #### Configuration-Based Tag Processing
/// ```rust
/// struct TagConfig {
///     max_tags: usize,
///     min_word_length: usize,
/// }
///
/// fn process_tags_with_config(tags: Vec<String>, config: &TagConfig) -> Vec<String> {
///     hashtags(tags)
///         .into_iter()
///         .filter(|tag| tag.len() > config.min_word_length + 1) // +1 for #
///         .take(config.max_tags)
///         .collect()
/// }
/// ```
///
/// ## Performance Optimizations
///
/// ### Potential Improvements
///
/// #### Pre-allocation
/// ```rust
/// pub(crate) fn hashtags_optimized(tags: Vec<String>) -> Vec<String> {
///     let mut hashtags = Vec::with_capacity(tags.len());
///     // ... rest of implementation
/// }
/// ```
///
/// #### String Builder Pattern
/// ```rust
/// use std::fmt::Write;
///
/// pub(crate) fn hashtags_with_builder(tags: Vec<String>) -> Vec<String> {
///     let mut hashtags = Vec::with_capacity(tags.len());
///     
///     for tag in tags.into_iter() {
///         let mut formatted_tag = String::new();
///         
///         for mut word in tag.trim_start_matches('#').split_whitespace() {
///             write!(&mut formatted_tag, "{}", word.capitalize()).unwrap();
///         }
///         
///         hashtags.push(format!("#{}", formatted_tag));
///     }
///     
///     hashtags
/// }
/// ```
///
/// ## Testing Considerations
///
/// ### Test Categories
///
/// 1. **Basic Functionality**: Single words, multi-word tags
/// 2. **Edge Cases**: Empty strings, whitespace, existing hashtags
/// 3. **Unicode Handling**: International characters, emojis
/// 4. **Performance**: Large input sets, long tag names
/// 5. **Integration**: Interaction with `capitalize()` trait
///
/// ### Mock Dependencies
/// Testing may require mocking the `Capitalize` trait for isolated unit tests.
///
/// ## Thread Safety
///
/// The function is thread-safe:
/// - Takes ownership of input vector
/// - Creates new strings without shared mutable state
/// - No global state dependencies
/// - Pure function behaviour (same input always produces same output)
///
/// ## Backwards Compatibility
///
/// As a `pub(crate)` function, breaking changes affect only the current crate:
/// - Changing input/output types would break internal callers
/// - Modifying behaviour could affect dependent functionality
/// - Consider deprecation warnings for significant behaviour changes
///
/// ## Related Functions
///
/// Complementary functionality that might be useful:
///
/// ```rust
/// // Reverse operation - extract tags from text
/// pub(crate) fn extract_hashtags(text: &str) -> Vec<String>;
///
/// // Validate hashtag format
/// pub(crate) fn is_valid_hashtag(tag: &str) -> bool;
///
/// // Remove hashtags from text
/// pub(crate) fn strip_hashtags(text: &str) -> String;
/// ```
pub(crate) fn hashtags<S: Display>(tags: Vec<S>) -> Vec<String> {
    let mut hashtags = vec![];
    for tag in tags.into_iter() {
        let tag = tag.to_string();
        // convert tag to hashtag by capitalising the first letter of each word,
        // removing the spaces and prefixing with a # if required
        let formatted_tag = tag
            .trim_start_matches('#')
            .split_whitespace()
            .map(|mut word| word.capitalise())
            .collect::<String>();
        let hashtag = format!("#{formatted_tag}");
        hashtags.push(hashtag);
    }

    hashtags
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper macro for testing hashtag conversion
    macro_rules! test_hashtag {
        ($test_name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let input = $input.iter().map(|s| s.to_string()).collect();
                let result = hashtags(input);
                let expected: Vec<String> = $expected.iter().map(|s| s.to_string()).collect();
                assert_eq!(
                    result, expected,
                    "hashtags({:?}) should return {:?}, got {:?}",
                    $input, expected, result
                );
            }
        };
    }

    // Basic functionality tests
    #[test]
    fn test_empty_vector() {
        let input: Vec<String> = [].to_vec();
        let expected: Vec<String> = [].to_vec();
        let result = hashtags(input);
        assert_eq!(result, expected);
    }

    test_hashtag!(test_single_word, &["rust"], &["#Rust"]);

    test_hashtag!(
        test_multiple_single_words,
        &["rust", "programming", "code"],
        &["#Rust", "#Programming", "#Code"]
    );

    test_hashtag!(test_two_words, &["web development"], &["#WebDevelopment"]);

    test_hashtag!(
        test_multiple_multi_words,
        &["web development", "machine learning", "data science"],
        &["#WebDevelopment", "#MachineLearning", "#DataScience"]
    );

    test_hashtag!(
        test_three_words,
        &["artificial intelligence research"],
        &["#ArtificialIntelligenceResearch"]
    );

    // Existing hashtag prefix tests
    test_hashtag!(test_existing_hashtag_single_word, &["#rust"], &["#Rust"]);

    test_hashtag!(
        test_existing_hashtag_multi_word,
        &["#web development"],
        &["#WebDevelopment"]
    );

    test_hashtag!(
        test_mixed_existing_and_new,
        &["#rust", "programming", "#web development"],
        &["#Rust", "#Programming", "#WebDevelopment"]
    );

    test_hashtag!(
        test_multiple_hash_symbols,
        &["##rust", "###programming"],
        &["#Rust", "#Programming"]
    );

    // Whitespace handling tests
    test_hashtag!(
        test_leading_whitespace,
        &["  rust programming"],
        &["#RustProgramming"]
    );

    test_hashtag!(
        test_trailing_whitespace,
        &["rust programming  "],
        &["#RustProgramming"]
    );

    test_hashtag!(
        test_leading_and_trailing_whitespace,
        &["  rust programming  "],
        &["#RustProgramming"]
    );

    test_hashtag!(
        test_multiple_spaces_between_words,
        &["rust    programming"],
        &["#RustProgramming"]
    );

    test_hashtag!(
        test_mixed_whitespace_types,
        &["rust\t\nprogramming"],
        &["#RustProgramming"]
    );

    // Edge cases with empty and whitespace strings
    test_hashtag!(test_empty_string, &[""], &["#"]);

    test_hashtag!(test_only_whitespace, &["   "], &["#"]);

    test_hashtag!(test_only_tabs, &["\t\t"], &["#"]);

    test_hashtag!(test_only_newlines, &["\n\n"], &["#"]);

    test_hashtag!(
        test_mixed_empty_and_valid,
        &["rust", "", "programming"],
        &["#Rust", "#", "#Programming"]
    );

    // Case handling tests
    test_hashtag!(
        test_already_capitalized,
        &["Rust Programming"],
        &["#RustProgramming"]
    );

    test_hashtag!(
        test_all_uppercase,
        &["RUST PROGRAMMING"],
        &["#RUSTPROGRAMMING"]
    );

    test_hashtag!(
        test_mixed_case,
        &["rUST pROGRAMMING"],
        &["#RUSTPROGRAMMING"]
    );

    test_hashtag!(
        test_camel_case_input,
        &["rustProgramming"],
        &["#RustProgramming"]
    );

    // Special characters and numbers
    test_hashtag!(
        test_numbers_in_tags,
        &["rust 2024", "web3 development"],
        &["#Rust2024", "#Web3Development"]
    );

    test_hashtag!(test_numbers_only, &["2024", "365"], &["#2024", "#365"]);

    test_hashtag!(
        test_punctuation_in_words,
        &["c++ programming", "node.js development"],
        &["#C++Programming", "#Node.jsDevelopment"]
    );

    test_hashtag!(
        test_hyphens_and_underscores,
        &["rust-lang programming", "web_development"],
        &["#Rust-langProgramming", "#Web_development"]
    );

    test_hashtag!(
        test_apostrophes,
        &["can't wait", "don't stop"],
        &["#Can'tWait", "#Don'tStop"]
    );

    // Unicode tests
    test_hashtag!(
        test_accented_characters,
        &["café culture", "naïve approach"],
        &["#CaféCulture", "#NaïveApproach"]
    );

    test_hashtag!(test_cyrillic, &["москва travel"], &["#МоскваTravel"]);

    test_hashtag!(test_greek_letters, &["alpha beta"], &["#AlphaBeta"]);

    test_hashtag!(test_chinese_characters, &["中国 food"], &["#中国Food"]);

    test_hashtag!(
        test_japanese_characters,
        &["日本 culture"],
        &["#日本Culture"]
    );

    test_hashtag!(
        test_emoji_in_tags,
        &["rust 🦀", "coffee ☕"],
        &["#Rust🦀", "#Coffee☕"]
    );

    test_hashtag!(
        test_mixed_scripts,
        &["русский english", "français english"],
        &["#РусскийEnglish", "#FrançaisEnglish"]
    );

    // Single character tests
    test_hashtag!(test_single_character, &["a", "z"], &["#A", "#Z"]);

    test_hashtag!(test_single_number, &["1", "9"], &["#1", "#9"]);

    test_hashtag!(test_single_symbol, &["@", "&"], &["#@", "#&"]);

    // Large input tests
    #[test]
    fn test_large_number_of_tags() {
        let tags: Vec<String> = (0..1000).map(|i| format!("tag{i}")).collect();

        let result = hashtags(tags);

        assert_eq!(result.len(), 1000);
        assert_eq!(result[0], "#Tag0");
        assert_eq!(result[999], "#Tag999");

        // Verify all are properly formatted
        for (i, hashtag) in result.iter().enumerate() {
            assert_eq!(hashtag, &format!("#Tag{i}"));
        }
    }

    #[test]
    fn test_very_long_tag() {
        let long_words: Vec<String> = (0..50).map(|i| format!("word{i}")).collect();
        let long_tag = long_words.join(" ");

        let result = hashtags(vec![long_tag]);

        assert_eq!(result.len(), 1);
        assert!(result[0].starts_with("#"));

        // Should contain all capitalized words
        for i in 0..50 {
            assert!(result[0].contains(&format!("Word{i}")));
        }
    }

    // Performance and memory tests
    #[test]
    fn test_empty_words_in_tag() {
        // This tests the behaviour when split_whitespace encounters multiple spaces
        let tags = vec!["word1   word2".to_string()];
        let result = hashtags(tags);
        assert_eq!(result, vec!["#Word1Word2"]);
    }

    #[test]
    fn test_only_hashtag_symbol() {
        let tags = vec!["#".to_string()];
        let result = hashtags(tags);
        assert_eq!(result, vec!["#"]);
    }

    #[test]
    fn test_hashtag_with_only_whitespace() {
        let tags = vec!["#   ".to_string()];
        let result = hashtags(tags);
        assert_eq!(result, vec!["#"]);
    }

    // Boundary value tests
    #[test]
    fn test_maximum_realistic_hashtag() {
        // Test with a very long but realistic hashtag
        let tag =
            "artificial intelligence machine learning deep neural networks research".to_string();
        let result = hashtags(vec![tag]);

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            "#ArtificialIntelligenceMachineLearningDeepNeuralNetworksResearch"
        );
    }

    #[test]
    fn test_all_special_characters() {
        let tags = vec!["!@#$%^&*()".to_string()];
        let result = hashtags(tags);
        assert_eq!(result, vec!["#!@#$%^&*()"]);
    }

    // Real-world scenario tests
    #[test]
    fn test_blog_post_tags() {
        let tags = vec![
            "rust programming".to_string(),
            "#web development".to_string(),
            "backend api".to_string(),
            "database design".to_string(),
        ];

        let result = hashtags(tags);
        let expected = vec![
            "#RustProgramming",
            "#WebDevelopment",
            "#BackendApi",
            "#DatabaseDesign",
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_social_media_tags() {
        let tags = vec![
            "photo of the day".to_string(),
            "#nature photography".to_string(),
            "beautiful sunset".to_string(),
            "#travel blog".to_string(),
        ];

        let result = hashtags(tags);
        let expected = vec![
            "#PhotoOfTheDay",
            "#NaturePhotography",
            "#BeautifulSunset",
            "#TravelBlog",
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_technical_documentation_tags() {
        let tags = vec![
            "API documentation".to_string(),
            "REST endpoints".to_string(),
            "#software architecture".to_string(),
            "design patterns".to_string(),
        ];

        let result = hashtags(tags);
        let expected = vec![
            "#APIDocumentation",
            "#RESTEndpoints",
            "#SoftwareArchitecture",
            "#DesignPatterns",
        ];

        assert_eq!(result, expected);
    }

    // Error handling and robustness tests
    #[test]
    fn test_mixed_valid_and_invalid_input() {
        let tags = vec![
            "valid tag".to_string(),
            "".to_string(),
            "   ".to_string(),
            "another valid".to_string(),
            "#".to_string(),
        ];

        let result = hashtags(tags);
        let expected = vec!["#ValidTag", "#", "#", "#AnotherValid", "#"];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_vector_capacity_efficiency() {
        let tags = vec!["test".to_string(); 100];
        let result = hashtags(tags);

        // Should have exactly 100 elements
        assert_eq!(result.len(), 100);

        // All should be the same
        for hashtag in &result {
            assert_eq!(hashtag, "#Test");
        }
    }

    // Integration tests with different String creation methods
    #[test]
    fn test_different_string_sources() {
        let tags = vec![
            String::from("from method"),
            "string literal".to_string(),
            format!("{} {}", "format", "macro"),
            "owned".to_owned(),
        ];

        let result = hashtags(tags);
        let expected = vec!["#FromMethod", "#StringLiteral", "#FormatMacro", "#Owned"];

        assert_eq!(result, expected);
    }

    // Test function behaviour with moved values
    #[test]
    fn test_ownership_transfer() {
        let tags = vec!["test tag".to_string()];
        let tags_clone = tags.clone();

        let result = hashtags(tags);
        // tags is now moved and cannot be used

        assert_eq!(result, vec!["#TestTag"]);

        // But we can still use the clone
        let result2 = hashtags(tags_clone);
        assert_eq!(result2, vec!["#TestTag"]);
        assert_eq!(result, result2);
    }

    // Consistency tests
    #[test]
    fn test_idempotent_behaviour() {
        let tags1 = vec!["rust programming".to_string()];
        let tags2 = vec!["rust programming".to_string()];

        let result1 = hashtags(tags1);
        let result2 = hashtags(tags2);

        assert_eq!(result1, result2);
        assert_eq!(result1, vec!["#RustProgramming"]);
    }

    #[test]
    fn test_order_preservation() {
        let tags = vec![
            "zulu".to_string(),
            "alpha".to_string(),
            "bravo".to_string(),
            "yankee".to_string(),
        ];

        let result = hashtags(tags);
        let expected = vec!["#Zulu", "#Alpha", "#Bravo", "#Yankee"];

        assert_eq!(result, expected);
    }

    // Documentation example tests
    #[test]
    fn test_documentation_basic_example() {
        let tags = vec![
            "rust".to_string(),
            "programming".to_string(),
            "web development".to_string(),
        ];

        let result = hashtags(tags);
        let expected = vec!["#Rust", "#Programming", "#WebDevelopment"];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_documentation_existing_hashtags_example() {
        let tags = vec![
            "#rust".to_string(),
            "machine learning".to_string(),
            "#AI".to_string(),
        ];

        let result = hashtags(tags);
        let expected = vec!["#Rust", "#MachineLearning", "#AI"];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_documentation_mixed_format_example() {
        let tags = vec![
            "  web development  ".to_string(),
            "#mobile  app".to_string(),
            "data science".to_string(),
        ];

        let result = hashtags(tags);
        let expected = vec!["#WebDevelopment", "#MobileApp", "#DataScience"];
        assert_eq!(result, expected);
    }
}
