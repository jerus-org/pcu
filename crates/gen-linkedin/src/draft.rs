use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::value::Datetime;

use crate::frontmatter_writeback::{
    read_linkedin_string_field, write_linkedin_date_field, FmWriteError,
};

/// Errors that can occur during LinkedIn draft generation.
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
    /// No blog posts found matching the filter criteria.
    #[error("no blog posts found matching the filter")]
    BlogPostListEmpty,
}

/// The JSON file written to the linkedin store directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedinFile {
    /// Text of the LinkedIn post (from `[linkedin].description`).
    pub text: String,
    /// Optional URL to attach to the post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// ISO date the draft was created (`YYYY-MM-DD`), sourced from frontmatter `created`.
    pub created_at: String,
    /// Absolute path to the source markdown file (for `published` write-back).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<PathBuf>,
}

/// Return today's date as a TOML `Datetime` (date-only).
fn today() -> Datetime {
    let today = chrono::Utc::now().date_naive();
    let s = today.format("%Y-%m-%d").to_string();
    s.parse().expect("valid date string")
}

/// A blog post that has opted in to LinkedIn broadcasting via a `[linkedin]` section.
#[derive(Debug)]
struct BlogPost {
    /// Absolute path to the `.md` file.
    path: PathBuf,
    /// Text from `[linkedin].description`.
    description: String,
    /// Derived post URL (`base_url` + relative path stripped of extension).
    link: Option<String>,
}

/// Manages staged LinkedIn drafts for a set of blog posts.
#[derive(Debug)]
pub struct Draft {
    posts: Vec<BlogPost>,
    written: usize,
}

impl Draft {
    /// Scan `blog_paths` under `www_src_root`, collect posts that have a
    /// `[linkedin]` section with a `description` field, and skip any that
    /// already have `[linkedin].created` set.
    ///
    /// `base_url` is used to build the post link (e.g. `https://www.example.com`).
    /// `min_date` filters posts by their frontmatter `date` field.
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
    /// and write `created = <today>` into each source file's `[linkedin]` section.
    /// Already-processed posts (those that already have `[linkedin].created`) are skipped.
    pub fn write_linkedin_drafts(&mut self, store_dir: &Path) -> Result<&mut Self, DraftError> {
        fs::create_dir_all(store_dir)?;
        let today = today();

        for post in &self.posts {
            // Primary idempotency guard: skip if created already set.
            if crate::frontmatter_writeback::read_linkedin_date_field(&post.path, "created")
                .is_some()
            {
                log::debug!(
                    "Skipping draft — [linkedin].created already set in {:?}",
                    post.path
                );
                continue;
            }

            // Write created date into frontmatter.
            write_linkedin_date_field(&post.path, "created", today)?;

            // Derive a short filename from the source path stem.
            let stem = post
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("post");
            let filename = format!("{stem}.linkedin");
            let file_path = store_dir.join(&filename);

            let record = LinkedinFile {
                text: post.description.clone(),
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

    /// Number of `.linkedin` files written in the last call to `write_linkedin_drafts`.
    pub fn count_written(&self) -> usize {
        self.written
    }
}

/// Walk `dir` recursively and add qualifying blog posts to `out`.
fn collect_posts(
    dir: &Path,
    www_src_root: &Path,
    base_url: &str,
    min_date: Option<Datetime>,
    out: &mut Vec<BlogPost>,
) -> Result<(), DraftError> {
    if !dir.is_dir() {
        // Single file path was given.
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

/// Try to load a `BlogPost` from `path`.
/// Returns `None` if the file should be skipped (no `[linkedin].description`,
/// already created, or doesn't pass the date filter).
fn try_load_post(
    path: &Path,
    www_src_root: &Path,
    base_url: &str,
    min_date: Option<Datetime>,
    // Note: `created` check is done inside `write_linkedin_drafts` so we can
    // still include the post in the list for logging purposes. We re-check there.
) -> Result<Option<BlogPost>, DraftError> {
    let description = match read_linkedin_string_field(path, "description") {
        Some(d) => d,
        None => return Ok(None), // No [linkedin].description — not opted in.
    };

    // Apply minimum date filter if provided.
    if let Some(min) = min_date {
        let post_date = read_post_date(path);
        match post_date {
            Some(d) if d < min => return Ok(None),
            None => return Ok(None), // No date in frontmatter — skip.
            _ => {}
        }
    }

    // Derive link from base_url + relative path (strip www_src_root prefix + extension).
    let link = derive_link(path, www_src_root, base_url);

    Ok(Some(BlogPost {
        path: path.to_path_buf(),
        description,
        link,
    }))
}

/// Read the `date` field from the post's TOML frontmatter.
fn read_post_date(path: &Path) -> Option<Datetime> {
    let content = fs::read_to_string(path).ok()?;
    let fm_str = extract_frontmatter_str(&content)?;
    let table: toml::Table = toml::from_str(fm_str).ok()?;
    if let Some(toml::Value::Datetime(dt)) = table.get("date") {
        Some(*dt)
    } else {
        None
    }
}

/// Extract the TOML frontmatter string from a `+++...+++` block.
fn extract_frontmatter_str(content: &str) -> Option<&str> {
    let first = content.find("+++")?;
    let after_first = &content[first + 3..];
    let fm_start = after_first.find('\n').map(|i| i + 1)?;
    let fm_rest = &after_first[fm_start..];
    let second = fm_rest.find("+++")?;
    Some(&fm_rest[..second])
}

/// Derive a public URL for the post from its filesystem path.
/// Strips `www_src_root/content/` prefix and `.md` extension, then appends to `base_url`.
fn derive_link(path: &Path, www_src_root: &Path, base_url: &str) -> Option<String> {
    // Try stripping www_src_root to get relative path.
    let rel = path.strip_prefix(www_src_root).ok()?;
    // Strip leading "content/" segment if present.
    let rel = rel.strip_prefix("content").unwrap_or(rel);
    // Strip ".md" extension.
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

    fn md_without_linkedin() -> String {
        "+++\ntitle = \"Test\"\ndate = 2026-04-03\n+++\n\nBody.\n".to_string()
    }

    // RED: Draft::new returns BlogPostListEmpty when no posts have [linkedin] section

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

    // RED: Draft::new collects posts with [linkedin].description

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

    // RED: write_linkedin_drafts creates .linkedin file

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

    // RED: write_linkedin_drafts writes created into frontmatter

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

    // RED: write_linkedin_drafts is idempotent when [linkedin].created already set

    #[test]
    fn test_write_linkedin_drafts_skips_when_created_set() {
        let src_dir = tempdir().unwrap();
        let store_dir = tempdir().unwrap();

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

    // RED: .linkedin file contains correct text and source_path

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

    // RED: derive_link builds correct URL

    #[test]
    fn test_derive_link_strips_content_prefix() {
        let root = Path::new("/site");
        let path = Path::new("/site/content/blog/my-post.md");
        let link = derive_link(path, root, "https://example.com");
        assert_eq!(link.as_deref(), Some("https://example.com/blog/my-post"));
    }

    // RED: min_date filter excludes old posts

    #[test]
    fn test_draft_new_respects_min_date() {
        let dir = tempdir().unwrap();
        // Post dated before min_date
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
}
