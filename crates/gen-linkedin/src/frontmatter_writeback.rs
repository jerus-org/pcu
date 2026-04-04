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
pub(crate) fn split_frontmatter(content: &str) -> Result<(&str, &str, &str), FmWriteError> {
    let first = content.find("+++").ok_or(FmWriteError::NoFrontmatter)?;
    let after_first = &content[first + 3..];
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

/// Read a date field from a named TOML section in Zola/Hugo frontmatter.
/// Returns `None` if the file, section, or field is absent or cannot be parsed.
fn read_section_date_field(path: &Path, section: &str, field: &str) -> Option<Datetime> {
    let content = fs::read_to_string(path).ok()?;
    let (_, fm_str, _) = split_frontmatter(&content).ok()?;
    let table: toml::Table = toml::from_str(fm_str).ok()?;
    let sec = table.get(section)?.as_table()?;
    if let Some(toml::Value::Datetime(dt)) = sec.get(field) {
        Some(*dt)
    } else {
        None
    }
}

/// Write a date field into a named TOML section in Zola/Hugo frontmatter.
/// Creates the section if absent. Overwrites existing values.
fn write_section_date_field(
    path: &Path,
    section: &str,
    field: &str,
    date: Datetime,
) -> Result<(), FmWriteError> {
    let content = fs::read_to_string(path)?;
    let (before, fm_str, body_str) = split_frontmatter(&content)?;

    let mut table: toml::Table = toml::from_str(fm_str)?;

    let sec = table
        .entry(section)
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));

    if let toml::Value::Table(ref mut t) = sec {
        t.insert(field.to_string(), toml::Value::Datetime(date));
    }

    let new_fm_str = toml::to_string(&table)?;
    let new_content = format!("{before}+++\n{new_fm_str}+++{body_str}");
    fs::write(path, new_content)?;
    Ok(())
}

const SECTION: &str = "linkedin";

/// Read an optional date field from the `[linkedin]` section of a Zola/Hugo TOML frontmatter.
/// Returns `None` if the section or field is absent, or the file cannot be read.
pub(crate) fn read_linkedin_date_field(path: &Path, field: &str) -> Option<Datetime> {
    read_section_date_field(path, SECTION, field)
}

/// Write a date field inside the `[linkedin]` section of a Zola/Hugo TOML frontmatter.
/// Creates the `[linkedin]` section if absent. Overwrites existing values.
pub(crate) fn write_linkedin_date_field(
    path: &Path,
    field: &str,
    date: Datetime,
) -> Result<(), FmWriteError> {
    write_section_date_field(path, SECTION, field, date)
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

    #[test]
    fn test_write_creates_linkedin_section_when_absent() {
        let dir = tempdir().unwrap();
        let content = "+++\ntitle = \"My Post\"\n+++\n\nBody.";
        let path = write_test_file(dir.path(), "post.md", content);

        write_linkedin_date_field(&path, "created", make_date("2026-04-03")).unwrap();

        let new = fs::read_to_string(&path).unwrap();
        assert!(new.contains("[linkedin]"), "section missing: {new}");
        assert!(new.contains("created = 2026-04-03"), "field missing: {new}");
    }

    #[test]
    fn test_write_adds_to_existing_linkedin_section() {
        let dir = tempdir().unwrap();
        let content =
            "+++\ntitle = \"My Post\"\n\n[linkedin]\ndescription = \"nice\"\n+++\n\nBody.";
        let path = write_test_file(dir.path(), "post.md", content);

        write_linkedin_date_field(&path, "created", make_date("2026-04-03")).unwrap();

        let new = fs::read_to_string(&path).unwrap();
        assert!(new.contains("created = 2026-04-03"), "field missing: {new}");
        assert!(new.contains("description"), "description removed: {new}");
    }

    #[test]
    fn test_write_published_field() {
        let dir = tempdir().unwrap();
        let content = "+++\ntitle = \"My Post\"\n\n[linkedin]\ncreated = 2026-04-02\n+++\n\nBody.";
        let path = write_test_file(dir.path(), "post.md", content);

        write_linkedin_date_field(&path, "published", make_date("2026-04-03")).unwrap();

        let new = fs::read_to_string(&path).unwrap();
        assert!(
            new.contains("published = 2026-04-03"),
            "published missing: {new}"
        );
        assert!(
            new.contains("created = 2026-04-02"),
            "created removed: {new}"
        );
    }

    #[test]
    fn test_read_date_field_returns_value() {
        let dir = tempdir().unwrap();
        let content = "+++\ntitle = \"My Post\"\n\n[linkedin]\ncreated = 2026-04-03\n+++\n\nBody.";
        let path = write_test_file(dir.path(), "post.md", content);

        let dt = read_linkedin_date_field(&path, "created");
        assert!(dt.is_some(), "expected Some, got None");
        assert_eq!(dt.unwrap().to_string(), "2026-04-03");
    }

    #[test]
    fn test_read_date_field_absent_returns_none() {
        let dir = tempdir().unwrap();
        let content = "+++\ntitle = \"My Post\"\n+++\n\nBody.";
        let path = write_test_file(dir.path(), "post.md", content);

        assert!(read_linkedin_date_field(&path, "created").is_none());
    }
}
