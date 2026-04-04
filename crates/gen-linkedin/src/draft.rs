//! LinkedIn draft generation from blog post frontmatter.
//!
//! Scans a content directory for Markdown files that have a `[linkedin]`
//! frontmatter section with post text, builds [`LinkedinFile`] JSON draft
//! files in a store directory, and records the draft date in the source
//! frontmatter for idempotency.
//!
//! # Workflow
//!
//! 1. Author writes a blog post and adds a `[linkedin]` section with either
//!    `description` (inline) or `description_file` (path to a separate file).
//! 2. `pcu linkedin draft` calls [`Draft::new`] then [`Draft::write_linkedin_drafts`].
//! 3. Draft files (`.linkedin` JSON) appear in the store directory.
//! 4. `pcu linkedin post` reads the store and publishes each draft.
//!
//! # Character limit
//!
//! LinkedIn posts are validated against [`LINKEDIN_POST_MAX_CHARS`] (default
//! 3 000). The generated link is included in the count. Adjust the constant if
//! LinkedIn changes its limit.

use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::value::Datetime;

use crate::{
    frontmatter::FrontMatter,
    frontmatter_writeback::{read_linkedin_date_field, write_linkedin_date_field, FmWriteError},
};

/// Maximum number of Unicode characters allowed in a LinkedIn post, including
/// the appended link.
///
/// LinkedIn's documented limit as of 2025 is 3 000 characters. The value is
/// exposed as a public constant so callers can surface it in error messages or
/// UI. Override at the application level if LinkedIn updates its limit.
pub const LINKEDIN_POST_MAX_CHARS: usize = 3_000;

/// Errors that can occur during LinkedIn draft generation.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum DraftError {
    /// I/O error reading or writing files.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization error writing the `.linkedin` file.
    #[error("json serialization error: {0}")]
    Json(#[from] serde_json::Error),
    /// Frontmatter write-back error.
    #[error("frontmatter write error: {0}")]
    FmWrite(#[from] FmWriteError),
    /// TOML parse error reading a blog post's frontmatter.
    #[error("toml parse error in {file:?}: {source}")]
    Toml {
        /// Source TOML error.
        source: toml::de::Error,
        /// Path of the offending file.
        file: PathBuf,
    },
    /// Post text exceeds the LinkedIn character limit.
    ///
    /// The character count includes the post text and the appended link
    /// (separated by a single space).
    #[error("post too long in {file:?}: {chars} chars exceeds maximum {max}")]
    PostTooLong {
        /// Total character count (text + space + link).
        chars: usize,
        /// Configured maximum ([`LINKEDIN_POST_MAX_CHARS`]).
        max: usize,
        /// Path of the offending source file.
        file: PathBuf,
    },
    /// No blog posts found matching the filter criteria.
    #[error("no blog posts found matching the filter")]
    BlogPostListEmpty,
}

/// The JSON file written to the LinkedIn store directory.
///
/// Each `.linkedin` file corresponds to one staged post. [`crate::Post`] reads
/// these files and publishes them via the LinkedIn API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedinFile {
    /// Text of the LinkedIn post.
    pub text: String,
    /// Optional URL appended to the post (derived from `base_url` + slug).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// ISO date the draft was created (`YYYY-MM-DD`).
    pub created_at: String,
    /// Absolute path to the source markdown file (used for `published` write-back).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<PathBuf>,
}

/// Return today's date as a TOML `Datetime` (date-only).
fn today() -> Datetime {
    let today = chrono::Utc::now().date_naive();
    today
        .format("%Y-%m-%d")
        .to_string()
        .parse()
        .expect("valid date string")
}

/// An internal representation of a qualifying blog post.
#[derive(Debug)]
struct BlogPost {
    /// Absolute path to the `.md` file.
    path: PathBuf,
    /// Post text (from `description` or `description_file`).
    text: String,
    /// Derived post URL.
    link: Option<String>,
}

/// Manages staged LinkedIn drafts for a set of blog posts.
///
/// Create with [`Draft::new`], then call [`Draft::write_linkedin_drafts`] to
/// write `.linkedin` JSON files to the store directory and stamp the source
/// frontmatter with the creation date.
#[derive(Debug)]
pub struct Draft {
    posts: Vec<BlogPost>,
    written: usize,
}

impl Draft {
    /// Scan `blog_paths` under `www_src_root`, collect posts that have a
    /// `[linkedin]` section with post text, and validate their length.
    ///
    /// Posts are skipped when:
    /// - The `[linkedin]` section is absent.
    /// - Neither `description` nor `description_file` yields text.
    /// - `[linkedin].created` is already set (already drafted).
    /// - `[linkedin].published` is already set (already posted).
    /// - The post's `date` field predates `min_date` (when provided).
    ///
    /// Returns [`DraftError::PostTooLong`] if any post's text + link exceeds
    /// [`LINKEDIN_POST_MAX_CHARS`] characters.
    ///
    /// Returns [`DraftError::BlogPostListEmpty`] when no qualifying posts remain.
    pub fn new(
        www_src_root: &Path,
        blog_paths: &[PathBuf],
        base_url: &str,
        min_date: Option<Datetime>,
    ) -> Result<Self, DraftError> {
        let mut posts = Vec::new();

        for blog_path in blog_paths {
            let dir = if blog_path.is_absolute() {
                blog_path.clone()
            } else {
                www_src_root.join(blog_path)
            };
            collect_posts(&dir, www_src_root, base_url, min_date, &mut posts)?;
        }

        if posts.is_empty() {
            return Err(DraftError::BlogPostListEmpty);
        }

        Ok(Self { posts, written: 0 })
    }

    /// Write `.linkedin` JSON files to `store_dir` for all collected posts,
    /// and write `created = <today>` into each source file's `[linkedin]`
    /// section.
    ///
    /// Posts that already have `[linkedin].created` set are skipped
    /// (idempotent). The store directory is created if absent.
    pub fn write_linkedin_drafts(&mut self, store_dir: &Path) -> Result<&mut Self, DraftError> {
        fs::create_dir_all(store_dir)?;
        let today = today();

        for post in &self.posts {
            // Idempotency guard: skip if created is already stamped.
            if read_linkedin_date_field(&post.path, "created").is_some() {
                log::debug!(
                    "Skipping — [linkedin].created already set in {:?}",
                    post.path
                );
                continue;
            }

            // Stamp the creation date into the source frontmatter.
            write_linkedin_date_field(&post.path, "created", today)?;

            // Derive a filename from the source file stem.
            let stem = post
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("post");
            let filename = format!("{stem}.linkedin");
            let file_path = store_dir.join(&filename);

            let record = LinkedinFile {
                text: post.text.clone(),
                link: post.link.clone(),
                created_at: today.to_string(),
                source_path: Some(post.path.clone()),
            };

            let file = File::create(&file_path)?;
            serde_json::to_writer_pretty(&file, &record)?;
            log::debug!("Wrote LinkedIn draft: {file_path:?}");
            self.written += 1;
        }

        Ok(self)
    }

    /// Number of `.linkedin` files written in the last call to
    /// [`Draft::write_linkedin_drafts`].
    pub fn count_written(&self) -> usize {
        self.written
    }
}

/// Walk `dir` recursively and add qualifying posts to `out`.
fn collect_posts(
    dir: &Path,
    www_src_root: &Path,
    base_url: &str,
    min_date: Option<Datetime>,
    out: &mut Vec<BlogPost>,
) -> Result<(), DraftError> {
    if !dir.is_dir() {
        if dir.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Some(post) = try_load_post(dir, www_src_root, base_url, min_date)? {
                out.push(post);
            }
        }
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_posts(&path, www_src_root, base_url, min_date, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Some(post) = try_load_post(&path, www_src_root, base_url, min_date)? {
                out.push(post);
            }
        }
    }

    Ok(())
}

/// Try to load a `BlogPost` from `path`, applying all skip conditions.
///
/// Returns `None` when the post should be skipped (no `[linkedin]` section,
/// no text, already drafted/published, or predates `min_date`).
fn try_load_post(
    path: &Path,
    www_src_root: &Path,
    base_url: &str,
    min_date: Option<Datetime>,
) -> Result<Option<BlogPost>, DraftError> {
    let content = fs::read_to_string(path)?;
    let fm = FrontMatter::from_content(&content).map_err(|e| DraftError::Toml {
        source: e,
        file: path.to_path_buf(),
    })?;

    // Skip posts with no [linkedin] section.
    let li = match fm.linkedin {
        Some(ref li) => li,
        None => return Ok(None),
    };

    // Skip already-published posts.
    if li.published.is_some() {
        log::debug!("Skipping — [linkedin].published already set in {path:?}");
        return Ok(None);
    }

    // Apply minimum date filter.
    if let Some(min) = min_date {
        match fm.date {
            Some(d) if d < min => return Ok(None),
            None => return Ok(None),
            _ => {}
        }
    }

    // Resolve post text (description_file takes precedence over description).
    let dir = path.parent().unwrap_or(Path::new("."));
    let text = match li.post_text(dir)? {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None), // No text — not opted in.
    };

    // Derive the post link.
    let link = derive_link(path, www_src_root, base_url);

    // Validate character count: text + space + link.
    let char_count = text.chars().count()
        + link
            .as_deref()
            .map(|l| 1 + l.chars().count()) // " " + link
            .unwrap_or(0);

    if char_count > LINKEDIN_POST_MAX_CHARS {
        return Err(DraftError::PostTooLong {
            chars: char_count,
            max: LINKEDIN_POST_MAX_CHARS,
            file: path.to_path_buf(),
        });
    }

    Ok(Some(BlogPost {
        path: path.to_path_buf(),
        text,
        link,
    }))
}

/// Derive a public URL for the post from its filesystem path.
///
/// Strips `www_src_root` and a leading `content/` segment, removes the `.md`
/// extension, then prepends `base_url`.
fn derive_link(path: &Path, www_src_root: &Path, base_url: &str) -> Option<String> {
    let rel = path.strip_prefix(www_src_root).ok()?;
    let rel = rel.strip_prefix("content").unwrap_or(rel);
    let without_ext = rel.with_extension("");
    let slug = without_ext.to_string_lossy();
    let base = base_url.trim_end_matches('/');
    Some(format!("{base}/{slug}"))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use tempfile::tempdir;

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn md_with_linkedin(description: &str) -> String {
        format!(
            "+++\ntitle = \"Test\"\ndate = 2026-04-03\n\n[linkedin]\ndescription = \"{description}\"\n+++\n\nBody.\n"
        )
    }

    fn md_with_linkedin_and_created(description: &str) -> String {
        format!(
            "+++\ntitle = \"Test\"\ndate = 2026-04-03\n\n[linkedin]\ndescription = \"{description}\"\ncreated = 2026-04-01\n+++\n\nBody.\n"
        )
    }

    fn md_with_linkedin_and_published(description: &str) -> String {
        format!(
            "+++\ntitle = \"Test\"\ndate = 2026-04-03\n\n[linkedin]\ndescription = \"{description}\"\ncreated = 2026-04-01\npublished = 2026-04-02\n+++\n\nBody.\n"
        )
    }

    fn md_without_linkedin() -> String {
        "+++\ntitle = \"Test\"\ndate = 2026-04-03\n+++\n\nBody.\n".to_string()
    }

    // ── existing tests (must stay GREEN) ────────────────────────────────────

    #[test]
    fn test_draft_new_empty_when_no_linkedin_section() {
        let dir = tempdir().unwrap();
        write_file(dir.path(), "post.md", &md_without_linkedin());

        let result = Draft::new(
            dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        );
        assert!(
            matches!(result, Err(DraftError::BlogPostListEmpty)),
            "expected BlogPostListEmpty, got {result:?}"
        );
    }

    #[test]
    fn test_draft_new_collects_opted_in_posts() {
        let dir = tempdir().unwrap();
        write_file(dir.path(), "post.md", &md_with_linkedin("Hello LinkedIn"));

        let result = Draft::new(
            dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        );
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let draft = result.unwrap();
        assert_eq!(draft.posts.len(), 1);
    }

    #[test]
    fn test_write_linkedin_drafts_creates_file() {
        let src_dir = tempdir().unwrap();
        let store_dir = tempdir().unwrap();

        write_file(
            src_dir.path(),
            "post.md",
            &md_with_linkedin("Hello LinkedIn"),
        );

        let mut draft = Draft::new(
            src_dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        )
        .unwrap();
        draft.write_linkedin_drafts(store_dir.path()).unwrap();

        let files: Vec<_> = fs::read_dir(store_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(files.len(), 1, "expected 1 .linkedin file");
        assert!(
            files[0]
                .file_name()
                .to_string_lossy()
                .ends_with(".linkedin"),
            "expected .linkedin extension"
        );
    }

    #[test]
    fn test_write_linkedin_drafts_sets_created_in_frontmatter() {
        let src_dir = tempdir().unwrap();
        let store_dir = tempdir().unwrap();

        let md_path = write_file(src_dir.path(), "post.md", &md_with_linkedin("Hello"));

        let mut draft = Draft::new(
            src_dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        )
        .unwrap();
        draft.write_linkedin_drafts(store_dir.path()).unwrap();

        let content = fs::read_to_string(&md_path).unwrap();
        assert!(
            content.contains("created ="),
            "frontmatter should contain created: {content}"
        );
    }

    #[test]
    fn test_write_linkedin_drafts_skips_when_created_set() {
        let src_dir = tempdir().unwrap();
        let store_dir = tempdir().unwrap();

        // Post with created already set is included in Draft::new (we keep
        // it) but write_linkedin_drafts skips writing it.
        write_file(
            src_dir.path(),
            "post.md",
            &md_with_linkedin_and_created("Hello"),
        );

        let mut draft = Draft::new(
            src_dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        )
        .unwrap();
        draft.write_linkedin_drafts(store_dir.path()).unwrap();

        assert_eq!(
            draft.count_written(),
            0,
            "should have skipped already-created post"
        );
    }

    #[test]
    fn test_linkedin_file_content() {
        let src_dir = tempdir().unwrap();
        let store_dir = tempdir().unwrap();

        write_file(
            src_dir.path(),
            "my-post.md",
            &md_with_linkedin("My post text"),
        );

        let mut draft = Draft::new(
            src_dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        )
        .unwrap();
        draft.write_linkedin_drafts(store_dir.path()).unwrap();

        let file_path = store_dir.path().join("my-post.linkedin");
        let content = fs::read_to_string(&file_path).unwrap();
        let record: LinkedinFile = serde_json::from_str(&content).unwrap();

        assert_eq!(record.text, "My post text");
        assert!(record.source_path.is_some());
    }

    #[test]
    fn test_derive_link_strips_content_prefix() {
        let root = Path::new("/site");
        let path = Path::new("/site/content/blog/my-post.md");
        let link = derive_link(path, root, "https://example.com");
        assert_eq!(link.as_deref(), Some("https://example.com/blog/my-post"));
    }

    #[test]
    fn test_draft_new_respects_min_date() {
        let dir = tempdir().unwrap();
        let old = "+++\ntitle = \"Old\"\ndate = 2026-01-01\n\n[linkedin]\ndescription = \"old\"\n+++\n\nBody.\n";
        write_file(dir.path(), "old.md", old);

        let min_date = Datetime::from_str("2026-04-01").unwrap();
        let result = Draft::new(
            dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            Some(min_date),
        );
        assert!(
            matches!(result, Err(DraftError::BlogPostListEmpty)),
            "old post should be filtered out: {result:?}"
        );
    }

    // ── new tests ────────────────────────────────────────────────────────────

    // RED: posts with [linkedin].published are skipped entirely
    #[test]
    fn test_draft_new_skips_published_posts() {
        let dir = tempdir().unwrap();
        write_file(
            dir.path(),
            "post.md",
            &md_with_linkedin_and_published("Hello"),
        );

        let result = Draft::new(
            dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        );
        assert!(
            matches!(result, Err(DraftError::BlogPostListEmpty)),
            "published post should be skipped: {result:?}"
        );
    }

    // RED: description_file is read and used as post text
    #[test]
    fn test_draft_new_reads_description_file() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("linkedin.md"), "Text from file").unwrap();
        let content = "+++\ntitle = \"T\"\ndate = 2026-04-03\n\n[linkedin]\ndescription_file = \"linkedin.md\"\n+++\n\nBody.\n";
        write_file(dir.path(), "post.md", content);

        let mut draft = Draft::new(
            dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        )
        .unwrap();
        let store_dir = tempdir().unwrap();
        draft.write_linkedin_drafts(store_dir.path()).unwrap();

        let file_path = store_dir.path().join("post.linkedin");
        let record: LinkedinFile =
            serde_json::from_str(&fs::read_to_string(&file_path).unwrap()).unwrap();
        assert_eq!(record.text, "Text from file");
    }

    // RED: post text that exceeds LINKEDIN_POST_MAX_CHARS returns PostTooLong
    #[test]
    fn test_draft_new_fails_on_post_too_long() {
        let dir = tempdir().unwrap();
        // Generate text longer than the limit
        let long_text: String = "a".repeat(LINKEDIN_POST_MAX_CHARS + 1);
        let content = format!(
            "+++\ntitle = \"T\"\ndate = 2026-04-03\n\n[linkedin]\ndescription = \"{long_text}\"\n+++\n\nBody.\n"
        );
        write_file(dir.path(), "post.md", &content);

        let result = Draft::new(
            dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        );
        assert!(
            matches!(result, Err(DraftError::PostTooLong { .. })),
            "expected PostTooLong, got {result:?}"
        );
    }

    // RED: text within limit (no link) is accepted
    #[test]
    fn test_draft_new_accepts_text_within_limit() {
        let dir = tempdir().unwrap();
        // Text at exactly the limit minus space+link chars still fits
        let text: String = "a".repeat(100);
        write_file(dir.path(), "post.md", &md_with_linkedin(&text));

        let result = Draft::new(
            dir.path(),
            &[PathBuf::from(".")],
            "https://example.com",
            None,
        );
        assert!(
            result.is_ok(),
            "should accept text within limit: {result:?}"
        );
    }

    // RED: link is appended to the .linkedin file
    #[test]
    fn test_linkedin_file_includes_link() {
        let src_dir = tempdir().unwrap();
        let store_dir = tempdir().unwrap();

        write_file(
            src_dir.path(),
            "content/blog/my-post.md",
            &md_with_linkedin("Hello"),
        );

        let mut draft = Draft::new(
            src_dir.path(),
            &[PathBuf::from("content/blog")],
            "https://example.com",
            None,
        )
        .unwrap();
        draft.write_linkedin_drafts(store_dir.path()).unwrap();

        let file_path = store_dir.path().join("my-post.linkedin");
        let record: LinkedinFile =
            serde_json::from_str(&fs::read_to_string(&file_path).unwrap()).unwrap();
        assert!(
            record.link.is_some(),
            "link should be set in .linkedin file"
        );
        assert!(
            record.link.unwrap().contains("example.com"),
            "link should contain base_url"
        );
    }

    // RED: LinkedinFile deserializes without source_path
    #[test]
    fn test_linkedin_file_deserializes_without_source_path() {
        let json = r#"{"text":"Hello","created_at":"2026-04-03"}"#;
        let f: LinkedinFile = serde_json::from_str(json).unwrap();
        assert!(f.source_path.is_none());
        assert_eq!(f.text, "Hello");
    }
}
