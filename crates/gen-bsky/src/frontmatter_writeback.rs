use std::{fs, io, path::Path};

use thiserror::Error;
use toml::value::Datetime;

/// Error type for frontmatter write-back operations.
#[derive(Debug, Error)]
pub enum FmWriteError {
    #[error("io error: {0:?}")]
    Io(#[from] io::Error),
    #[error("toml deserialization error: {0:?}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialization error: {0:?}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("frontmatter delimiters not found in file")]
    NoFrontmatter,
}

/// Split a Zola/Hugo markdown file into `(before_first_marker, frontmatter_str, body_str)`.
///
/// The file is expected to start with `+++`, contain TOML frontmatter, and close
/// with another `+++`.  Returns `FmWriteError::NoFrontmatter` if the delimiters are
/// not found.
fn split_frontmatter(content: &str) -> Result<(&str, &str, &str), FmWriteError> {
    // Files typically start with `+++\n`, so before_marker is empty.
    let first = content.find("+++").ok_or(FmWriteError::NoFrontmatter)?;
    let after_first = &content[first + 3..];
    // Skip past the newline that follows the opening `+++`
    let fm_start = after_first
        .find('\n')
        .map(|i| i + 1)
        .ok_or(FmWriteError::NoFrontmatter)?;
    let fm_rest = &after_first[fm_start..];
    let second = fm_rest.find("+++").ok_or(FmWriteError::NoFrontmatter)?;
    let fm_str = &fm_rest[..second];
    let body_str = &fm_rest[second + 3..];
    let before_marker = &content[..first];
    Ok((before_marker, fm_str, body_str))
}

/// Read an optional date field from the `[bluesky]` section of a Zola/Hugo TOML frontmatter
/// block in `path`.  Returns `None` if the file has no `[bluesky]` section, the field is absent,
/// or the path cannot be read.
pub(crate) fn read_bluesky_date_field(path: &Path, field: &str) -> Option<Datetime> {
    let content = fs::read_to_string(path).ok()?;
    let (_, fm_str, _) = split_frontmatter(&content).ok()?;
    let table: toml::Table = toml::from_str(fm_str).ok()?;
    let bluesky = table.get("bluesky")?.as_table()?;
    if let Some(toml::Value::Datetime(dt)) = bluesky.get(field) {
        Some(*dt)
    } else {
        None
    }
}

/// Write a date field inside the `[bluesky]` section of a Zola/Hugo TOML frontmatter
/// block in `path`.  If the `[bluesky]` section does not yet exist it is created.
/// Existing field values are overwritten.
pub(crate) fn write_bluesky_date_field(
    path: &Path,
    field: &str,
    date: Datetime,
) -> Result<(), FmWriteError> {
    let content = fs::read_to_string(path)?;
    let (before, fm_str, body_str) = split_frontmatter(&content)?;

    let mut table: toml::Table = toml::from_str(fm_str)?;

    let bluesky_table = table
        .entry("bluesky")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));

    if let toml::Value::Table(ref mut bt) = bluesky_table {
        bt.insert(field.to_string(), toml::Value::Datetime(date));
    }

    let new_fm_str = toml::to_string(&table)?;
    let new_content = format!("{before}+++\n{new_fm_str}+++{body_str}");
    fs::write(path, new_content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use tempfile::tempdir;

    fn make_date(s: &str) -> Datetime {
        Datetime::from_str(s).unwrap()
    }

    fn write_test_file(dir: &std::path::Path, filename: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(filename);
        fs::write(&path, content).unwrap();
        path
    }

    // RED: write_bluesky_date_field (issue #909)

    #[test]
    fn test_write_creates_bluesky_section_when_absent() {
        let dir = tempdir().unwrap();
        let content = "+++\ntitle = \"My Post\"\ndescription = \"test\"\n+++\n\nBody here.";
        let path = write_test_file(dir.path(), "post.md", content);

        write_bluesky_date_field(&path, "created", make_date("2026-04-03")).unwrap();

        let new_content = fs::read_to_string(&path).unwrap();
        assert!(
            new_content.contains("[bluesky]"),
            "bluesky section should be created: {new_content}"
        );
        assert!(
            new_content.contains("created = 2026-04-03"),
            "created field should be present: {new_content}"
        );
    }

    #[test]
    fn test_write_adds_field_to_existing_bluesky_section() {
        let dir = tempdir().unwrap();
        let content =
            "+++\ntitle = \"My Post\"\n\n[bluesky]\ndescription = \"nice post\"\n+++\n\nBody.";
        let path = write_test_file(dir.path(), "post.md", content);

        write_bluesky_date_field(&path, "created", make_date("2026-04-03")).unwrap();

        let new_content = fs::read_to_string(&path).unwrap();
        assert!(
            new_content.contains("created = 2026-04-03"),
            "created field should be added: {new_content}"
        );
        // Original description should still be present
        assert!(
            new_content.contains("description"),
            "existing description should remain: {new_content}"
        );
    }

    #[test]
    fn test_write_published_field() {
        let dir = tempdir().unwrap();
        let content = "+++\ntitle = \"My Post\"\n\n[bluesky]\ncreated = 2026-04-02\n+++\n\nBody.";
        let path = write_test_file(dir.path(), "post.md", content);

        write_bluesky_date_field(&path, "published", make_date("2026-04-03")).unwrap();

        let new_content = fs::read_to_string(&path).unwrap();
        assert!(
            new_content.contains("published = 2026-04-03"),
            "published field should be added: {new_content}"
        );
        assert!(
            new_content.contains("created = 2026-04-02"),
            "existing created should remain: {new_content}"
        );
    }

    #[test]
    fn test_write_body_preserved() {
        let dir = tempdir().unwrap();
        let body = "\n\nThis is the blog post body.\n\n## Section\n\nMore content here.\n";
        let content = format!("+++\ntitle = \"My Post\"\n+++{body}");
        let path = write_test_file(dir.path(), "post.md", &content);

        write_bluesky_date_field(&path, "created", make_date("2026-04-03")).unwrap();

        let new_content = fs::read_to_string(&path).unwrap();
        assert!(
            new_content.contains("This is the blog post body."),
            "body should be preserved: {new_content}"
        );
        assert!(
            new_content.contains("## Section"),
            "body headings should be preserved: {new_content}"
        );
    }

    #[test]
    fn test_write_no_frontmatter_returns_error() {
        let dir = tempdir().unwrap();
        let content = "No frontmatter here — just plain text.";
        let path = write_test_file(dir.path(), "post.md", content);

        let result = write_bluesky_date_field(&path, "created", make_date("2026-04-03"));
        assert!(
            matches!(result, Err(FmWriteError::NoFrontmatter)),
            "should return NoFrontmatter: {result:?}"
        );
    }
}
