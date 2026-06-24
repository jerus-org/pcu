/// Capitalises the first character of a string, preserving the rest, with
/// correct Unicode handling (e.g. `"café"` → `"Café"`, German `"ß"` → `"SS"`).
///
/// Crate-internal trait, implemented for `&str`. Empty input yields an empty
/// `String`; the original string is never mutated.
///
/// For behaviour across scripts and edge cases, see the unit tests in this
/// module (e.g. `test::test_documentation_examples`, `test::test_german_eszett`,
/// `test::test_cyrillic_lowercase`).
pub(crate) trait Capitalise: ToString {
    fn capitalise(&self) -> String;
}

impl Capitalise for &str {
    /// Capitalises the first character in s.
    fn capitalise(&self) -> String {
        let output = self.to_string();

        let mut c = output.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }
}

mod test {
    #[allow(unused_imports)]
    use super::Capitalise;
    // Helper macro to test capitalise functionality
    macro_rules! test_capitalise {
        ($test_name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let s = $input;
                let result = s.capitalise();
                assert_eq!(
                    result, $expected,
                    "capitalise('{}') should return '{}', got '{}'",
                    $input, $expected, result
                );
            }
        };
    }

    // Basic ASCII tests
    test_capitalise!(test_empty_string, "", "");
    test_capitalise!(test_single_lowercase_char, "a", "A");
    test_capitalise!(test_single_uppercase_char, "A", "A");
    test_capitalise!(test_lowercase_word, "hello", "Hello");
    test_capitalise!(test_uppercase_word, "HELLO", "HELLO");
    test_capitalise!(test_mixed_case_word, "hELLO", "HELLO");
    test_capitalise!(test_sentence, "hello world", "Hello world");
    test_capitalise!(test_already_capitalised, "Hello", "Hello");

    // Special character tests
    test_capitalise!(test_number_first, "123abc", "123abc");
    test_capitalise!(test_space_first, " hello", " hello");
    test_capitalise!(test_punctuation_first, "!hello", "!hello");
    test_capitalise!(test_underscore_first, "_hello", "_hello");
    test_capitalise!(test_hyphen_first, "-hello", "-hello");

    // Unicode tests - Basic Latin Extended
    test_capitalise!(test_accented_lowercase, "café", "Café");
    test_capitalise!(test_accented_uppercase, "Café", "Café");
    test_capitalise!(test_umlaut, "über", "Über");
    test_capitalise!(test_cedilla, "ça", "Ça");
    test_capitalise!(test_tilde, "niño", "Niño");

    // Unicode tests - Cyrillic
    test_capitalise!(test_cyrillic_lowercase, "москва", "Москва");
    test_capitalise!(test_cyrillic_uppercase, "Москва", "Москва");
    test_capitalise!(test_cyrillic_mixed, "мОСКВА", "МОСКВА");

    // Unicode tests - Greek
    test_capitalise!(test_greek_lowercase, "αβγ", "Αβγ");
    test_capitalise!(test_greek_uppercase, "ΑΒΓ", "ΑΒΓ");
    test_capitalise!(test_greek_mixed, "αΒΓ", "ΑΒΓ");

    // Unicode tests - Arabic (no case distinction, should remain unchanged)
    test_capitalise!(test_arabic, "السلام", "السلام");
    test_capitalise!(test_arabic_with_latin, "مرحبا hello", "مرحبا hello");

    // Unicode tests - Chinese/Japanese (no case, should remain unchanged)
    test_capitalise!(test_chinese, "你好", "你好");
    test_capitalise!(test_japanese_hiragana, "こんにちは", "こんにちは");
    test_capitalise!(test_japanese_katakana, "コンニチハ", "コンニチハ");

    // Special Unicode case conversions
    #[test]
    fn test_german_eszett() {
        // German ß (eszett) converts to SS when uppercased
        let s = "ß";
        let result = s.capitalise();
        assert_eq!(result, "SS", "German ß should convert to SS");
    }

    #[test]
    fn test_german_eszett_in_word() {
        let s = "straße";
        let result = s.capitalise();
        assert_eq!(
            result, "Straße",
            "First character should be capitalised, ß should remain"
        );
    }

    #[test]
    fn test_turkish_dotted_i() {
        // Turkish has special i/I rules, but basic conversion should work
        let s = "istanbul";
        let result = s.capitalise();
        assert_eq!(result, "Istanbul");
    }

    // Emoji and symbol tests
    test_capitalise!(test_emoji_first, "🦀rust", "🦀rust");
    test_capitalise!(test_emoji_mixed, "🎉hello", "🎉hello");
    test_capitalise!(test_symbol_first, "©copyright", "©copyright");
    test_capitalise!(test_mathematical_symbol, "∑math", "∑math");

    // Edge cases with whitespace
    test_capitalise!(test_leading_whitespace, "  hello", "  hello");
    test_capitalise!(test_trailing_whitespace, "hello  ", "Hello  ");
    test_capitalise!(test_only_whitespace, "   ", "   ");
    test_capitalise!(test_newline_first, "\nhello", "\nhello");
    test_capitalise!(test_tab_first, "\thello", "\thello");

    // Mixed script tests
    test_capitalise!(test_latin_cyrillic_mix, "hмосква", "Hмосква");
    test_capitalise!(test_number_letter_mix, "1hello", "1hello");
    test_capitalise!(test_symbol_letter_mix, "@hello", "@hello");

    // Long string tests
    #[test]
    fn test_very_long_string() {
        let long_string = "a".repeat(1000);
        let s = long_string.as_str();
        let result = s.capitalise();
        assert!(result.starts_with("A"));
        assert_eq!(result.len(), 1000);
        assert_eq!(result.chars().skip(1).collect::<String>(), "a".repeat(999));
    }

    #[test]
    fn test_long_unicode_string() {
        let long_unicode = "café".repeat(250); // 1000 chars with Unicode
        let s = long_unicode.as_str();
        let result = s.capitalise();
        assert!(result.starts_with("C"));
        assert!(result.chars().nth(1) == Some('a'));
        assert!(result.chars().nth(2) == Some('f'));
        assert!(result.chars().nth(3) == Some('é'));
    }

    // Test immutability of original string
    #[test]
    fn test_original_string_unchanged() {
        let original = "hello";
        let s = original;
        let result = s.capitalise();

        // The original string should be unchanged
        assert_eq!(s, "hello");
        assert_eq!(result, "Hello");
        assert_ne!(s, result);
    }

    // Test multiple calls
    #[test]
    fn test_multiple_calls_same_result() {
        let s = "hello";
        let result1 = s.capitalise();
        let result2 = s.capitalise();
        let result3 = s.capitalise();

        assert_eq!(result1, "Hello");
        assert_eq!(result2, "Hello");
        assert_eq!(result3, "Hello");
        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    // Test with different string sources
    #[test]
    fn test_string_literal() {
        let s = "hello";
        assert_eq!(s.capitalise(), "Hello");
    }

    #[test]
    fn test_string_from_string_type() {
        let string_obj = String::from("hello");
        let s = string_obj.as_str();
        assert_eq!(s.capitalise(), "Hello");
    }

    #[test]
    fn test_string_from_format() {
        let formatted = format!("{}{}", "hel", "lo");
        let s = formatted.as_str();
        assert_eq!(s.capitalise(), "Hello");
    }

    // Performance and memory tests
    #[test]
    fn test_no_allocation_for_empty() {
        let s = "";
        let result = s.capitalise();
        assert_eq!(result, "");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_single_byte_char() {
        let s = "h";
        let result = s.capitalise();
        assert_eq!(result, "H");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_multi_byte_char_first() {
        let s = "ábc";
        let result = s.capitalise();
        assert_eq!(result, "Ábc");
        // á is 2 bytes, Á is also 2 bytes in UTF-8
        assert!(result.len() >= 4); // At least 2 (Á) + 1 (b) + 1 (c)
    }

    // Boundary condition tests
    #[test]
    fn test_all_punctuation() {
        let s = "!@#$%^&*()";
        let result = s.capitalise();
        assert_eq!(result, "!@#$%^&*()");
    }

    #[test]
    fn test_all_numbers() {
        let s = "1234567890";
        let result = s.capitalise();
        assert_eq!(result, "1234567890");
    }

    #[test]
    fn test_mixed_punctuation_and_letters() {
        let s = "!hello";
        let result = s.capitalise();
        assert_eq!(result, "!hello");
    }

    // Character category tests
    #[test]
    fn test_control_characters() {
        let s = "\x01hello"; // SOH control character
        let result = s.capitalise();
        assert_eq!(result, "\x01hello");
    }

    #[test]
    fn test_zero_width_characters() {
        let s = "\u{200B}hello"; // Zero-width space + hello
        let result = s.capitalise();
        assert_eq!(result, "\u{200B}hello");
    }

    // Test trait behaviour consistency
    #[test]
    fn test_trait_multiple_implementations_consistent() {
        // This test demonstrates that if we had multiple implementations,
        // they should behave consistently
        let test_cases = vec![
            ("", ""),
            ("a", "A"),
            ("hello", "Hello"),
            ("Hello", "Hello"),
            ("123", "123"),
            ("!hello", "!hello"),
            ("café", "Café"),
            ("москва", "Москва"),
            ("🦀rust", "🦀rust"),
        ];

        for (input, expected) in test_cases {
            let s = input;
            let result = s.capitalise();
            assert_eq!(result, expected, "Inconsistent result for input: '{input}'");
        }
    }

    // Regression tests for edge cases
    #[test]
    fn test_surrogate_pairs() {
        // Test with characters that use surrogate pairs in UTF-16
        let s = "𝕳ello"; // Mathematical double-struck H (U+1D573)
        let result = s.capitalise();
        // This character should remain unchanged as it doesn't have a simple uppercase
        assert!(result.starts_with("𝕳"));
        assert!(result.ends_with("ello"));
    }

    #[test]
    fn test_combining_characters() {
        // Test with combining diacritical marks
        let s = "e\u{0301}llo"; // e + combining acute accent
        let result = s.capitalise();
        assert!(result.starts_with("E\u{0301}") || result.starts_with("É"));
        assert!(result.ends_with("llo"));
    }

    // Documentation example tests
    #[test]
    fn test_documentation_examples() {
        // These tests verify that examples in documentation work correctly

        // Basic example
        let text = "hello world";
        let capitalised = text.capitalise();
        assert_eq!(capitalised, "Hello world");

        // Empty string example
        let empty = "";
        let result = empty.capitalise();
        assert_eq!(result, "");

        // Unicode example
        let unicode = "café";
        let result = unicode.capitalise();
        assert_eq!(result, "Café");

        // Single character example
        let single = "a";
        let result = single.capitalise();
        assert_eq!(result, "A");
    }
}
