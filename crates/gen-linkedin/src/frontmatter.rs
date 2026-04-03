//! Frontmatter types for LinkedIn-enabled blog posts.
//!
//! Provides serde-deserializable types for parsing the `[linkedin]` section of
//! Zola/Hugo TOML frontmatter. Intentionally parallel to the `[bluesky]` parsing
//! in `gen-bsky` — the two crates target different frontmatter sections with
//! different schemas and semantics and are kept separate.
//!
//! # Example frontmatter
//!
//! ```toml
//! +++
//! title = "My Post"
//! date = 2026-04-03
//!
//! [linkedin]
//! description_file = "linkedin.md"
//! +++
//! ```

use std::{fs, io, path::Path};

use serde::Deserialize;
use toml::value::Datetime;

use crate::frontmatter_writeback::split_frontmatter;

/// The `[linkedin]` section of a blog post's TOML frontmatter.
///
/// Author-controlled fields drive draft creation; date fields are written back
/// by `pcu` as the post moves through the draft → post lifecycle.
#[derive(Debug, Default, Deserialize)]
pub struct LinkedIn {
    /// Inline post text written by the author.
    ///
    /// For substantial posts prefer [`LinkedIn::description_file`], which keeps
    /// the TOML frontmatter readable when the text spans multiple paragraphs.
    pub description: Option<String>,

    /// Path to a file containing the post text, relative to the blog post's
    /// directory.
    ///
    /// Takes precedence over [`LinkedIn::description`] when both are present.
    /// The file contents are read verbatim as the LinkedIn post text.
    pub description_file: Option<String>,

    /// Date the LinkedIn draft was created (written by `pcu linkedin draft`).
    pub created: Option<Datetime>,

    /// Date the post was published to LinkedIn (written by `pcu linkedin post`).
    pub published: Option<Datetime>,
}

impl LinkedIn {
    /// Return the post text.
    ///
    /// If [`LinkedIn::description_file`] is set, reads that file (resolved
    /// relative to `dir`) and returns its trimmed contents. Otherwise returns
    /// the inline [`LinkedIn::description`]. Returns `None` if neither field
    /// is set.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if `description_file` is set but the file cannot
    /// be read.
    pub fn post_text(&self, dir: &Path) -> Result<Option<String>, io::Error> {
        if let Some(ref file) = self.description_file {
            let text = fs::read_to_string(dir.join(file))?;
            Ok(Some(text.trim_end().to_string()))
        } else {
            Ok(self.description.clone())
        }
    }
}

/// Parsed TOML frontmatter for a LinkedIn-enabled blog post.
///
/// All fields are optional at the TOML level; only the `[linkedin]` section
/// fields relevant to draft generation are validated at call sites.
#[derive(Debug, Deserialize)]
pub struct FrontMatter {
    /// Post title.
    pub title: Option<String>,
    /// Short description of the post (not used as LinkedIn post text).
    pub description: Option<String>,
    /// Post publication date used for minimum-date filtering.
    pub date: Option<Datetime>,
    /// LinkedIn-specific metadata. `None` when the section is absent.
    pub linkedin: Option<LinkedIn>,
}

impl FrontMatter {
    /// Parse frontmatter from the `+++`-delimited TOML block in `content`.
    ///
    /// All fields are optional; an empty or missing frontmatter block returns
    /// an instance with all fields set to `None`. Returns an error only on
    /// TOML parse failure.
    pub fn from_content(content: &str) -> Result<Self, toml::de::Error> {
        let fm_str = split_frontmatter(content)
            .map(|(_, fm, _)| fm)
            .unwrap_or("");
        toml::from_str(fm_str)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tempfile::tempdir;

    // ── FrontMatter::from_content ────────────────────────────────────────────

    // RED: parse description from [linkedin] section
    #[test]
    fn test_frontmatter_parses_linkedin_description() {
        let content = "+++\ntitle = \"My Post\"\n\n[linkedin]\ndescription = \"Hello LinkedIn\"\n+++\n\nBody.\n";
        let fm = FrontMatter::from_content(content).unwrap();
        let li = fm.linkedin.expect("[linkedin] section should be present");
        assert_eq!(li.description.as_deref(), Some("Hello LinkedIn"));
    }

    // RED: parse description_file from [linkedin] section
    #[test]
    fn test_frontmatter_parses_description_file() {
        let content = "+++\ntitle = \"My Post\"\n\n[linkedin]\ndescription_file = \"linkedin.md\"\n+++\n\nBody.\n";
        let fm = FrontMatter::from_content(content).unwrap();
        let li = fm.linkedin.expect("[linkedin] section should be present");
        assert_eq!(li.description_file.as_deref(), Some("linkedin.md"));
        assert!(li.description.is_none());
    }

    // RED: linkedin section absent → fm.linkedin is None
    #[test]
    fn test_frontmatter_linkedin_absent() {
        let content = "+++\ntitle = \"My Post\"\ndate = 2026-04-03\n+++\n\nBody.\n";
        let fm = FrontMatter::from_content(content).unwrap();
        assert!(fm.linkedin.is_none());
    }

    // RED: parse created and published dates
    #[test]
    fn test_frontmatter_parses_created_and_published() {
        let content = "+++\ntitle = \"T\"\n\n[linkedin]\ndescription = \"d\"\ncreated = 2026-04-01\npublished = 2026-04-03\n+++\n\nBody.\n";
        let fm = FrontMatter::from_content(content).unwrap();
        let li = fm.linkedin.unwrap();
        assert!(li.created.is_some());
        assert!(li.published.is_some());
        assert_eq!(li.created.unwrap().to_string(), "2026-04-01");
        assert_eq!(li.published.unwrap().to_string(), "2026-04-03");
    }

    // RED: parse date at top level
    #[test]
    fn test_frontmatter_parses_date() {
        let content = "+++\ntitle = \"My Post\"\ndate = 2026-04-03\n+++\n\nBody.\n";
        let fm = FrontMatter::from_content(content).unwrap();
        assert!(fm.date.is_some());
        assert_eq!(fm.date.unwrap().to_string(), "2026-04-03");
    }

    // ── LinkedIn::post_text ──────────────────────────────────────────────────

    // RED: post_text returns inline description when no file
    #[test]
    fn test_post_text_returns_inline_description() {
        let li = LinkedIn {
            description: Some("Inline text".to_string()),
            description_file: None,
            created: None,
            published: None,
        };
        let result = li.post_text(Path::new("/any")).unwrap();
        assert_eq!(result.as_deref(), Some("Inline text"));
    }

    // RED: post_text returns None when neither field is set
    #[test]
    fn test_post_text_returns_none_when_absent() {
        let li = LinkedIn::default();
        let result = li.post_text(Path::new("/any")).unwrap();
        assert!(result.is_none());
    }

    // RED: post_text reads from description_file (relative to dir)
    #[test]
    fn test_post_text_reads_from_file() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("linkedin.md"), "File-based post text").unwrap();

        let li = LinkedIn {
            description: Some("should be ignored".to_string()),
            description_file: Some("linkedin.md".to_string()),
            created: None,
            published: None,
        };
        let result = li.post_text(dir.path()).unwrap();
        assert_eq!(result.as_deref(), Some("File-based post text"));
    }

    // RED: post_text with description_file error when file missing
    #[test]
    fn test_post_text_file_not_found_returns_error() {
        let dir = tempdir().unwrap();
        let li = LinkedIn {
            description: None,
            description_file: Some("nonexistent.md".to_string()),
            created: None,
            published: None,
        };
        let result = li.post_text(dir.path());
        assert!(result.is_err());
    }

    // RED: description_file takes precedence over description
    #[test]
    fn test_description_file_takes_precedence_over_description() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("post.md"), "From file").unwrap();

        let li = LinkedIn {
            description: Some("inline".to_string()),
            description_file: Some("post.md".to_string()),
            created: None,
            published: None,
        };
        let result = li.post_text(dir.path()).unwrap();
        assert_eq!(result.as_deref(), Some("From file"));
    }
}
