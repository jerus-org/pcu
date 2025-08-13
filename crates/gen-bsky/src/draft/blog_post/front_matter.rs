use std::{
    cmp::max,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use serde::{Deserialize /* Deserializer */};
use thiserror::Error;
use toml::value::Datetime;
use unicode_segmentation::UnicodeSegmentation;

mod bluesky;
mod extra;
mod taxonomies;

use bluesky::Bluesky;
use extra::Extra;
use taxonomies::Taxonomies;

// +++
// title = "Blue Sky Test Blog"
// description = "A blog post to test the processing of blog posts for posting
// to Bluesky."
// date = 2025-01-17
// updated = 2025-01-16
// draft = false
//
// [taxonomies]
// topic = ["Technology"]
// description = "A blog post to test the processing of blog posts for posting
// to Bluesky."
// tags = ["bluesky", "testing", "test only", "ci"]
//
// [extra]
// bluesky.description = "This is a test blog post for Bluesky."
// bluesky.tags = ["bluesky", "testing", "test only", "ci"]
// +++
//

/// Error enum for FrontMatter type
#[non_exhaustive]
#[derive(Error, Debug)]
pub(super) enum FrontMatterError {
    /// Draft posts not allowed
    #[error("processing of draft posts is not allowed")]
    DraftNotAllowed,

    /// Post too old
    #[error("Post is older than allowed by minimum date setting {0}")]
    PostTooOld(Datetime),

    /// Error reported by IO library
    #[error("io error says: {0:?}")]
    Io(#[from] std::io::Error),

    /// Error reported by the Toml library.
    #[error("toml deserialization error says: {0:?}")]
    Toml(#[from] toml::de::Error),
}

/// Type representing the expected and optional keys in the
/// frontmatter of a markdown blog post file.
#[derive(Default, Debug, Clone, Deserialize)]
pub(super) struct FrontMatter {
    /// The title for the blog post.
    title: String,
    /// A description of the blog post.
    pub(super) description: String,
    /// The creation date of the blog post.
    pub(super) date: Option<Datetime>,
    /// The updated data of the blog post.
    pub(super) updated: Option<Datetime>,
    /// Flag to indicate if the post is draft.
    #[serde(default)]
    pub(super) draft: bool,
    /// The taxonomies section in the front matter. Expected
    /// to contain tags.
    pub(super) taxonomies: Option<Taxonomies>,
    /// The extras section in the front matter. Containing
    /// custom keys and may contain the bluesky custom keys.
    pub(super) extra: Option<Extra>,
    /// The bluesky section in the front matter. May contain
    /// the bluesky custom keys.
    pub(super) bluesky: Option<Bluesky>,
}

/// Report values in private fields
impl FrontMatter {
    pub(super) fn title(&self) -> &str {
        self.title.as_str()
    }
}

impl FrontMatter {
    pub(super) fn new(
        blog_file: &PathBuf,
        min_date: Datetime,
        allow_draft: bool,
    ) -> Result<FrontMatter, FrontMatterError> {
        log::debug!("Reading front matter from `{}` ", blog_file.display());
        let file = File::open(blog_file)?;
        let reader = BufReader::new(file);

        let mut front_str = String::new();
        let mut quit = false;

        for line in reader.lines().map_while(Result::ok) {
            if line.starts_with("+++") & quit {
                break;
            } else if line.starts_with("+++") {
                quit = true;
                continue;
            } else {
                front_str.push_str(&line);
                front_str.push('\n');
            }
        }

        #[cfg(test)]
        log::trace!("Front matter string:\n {front_str}");

        let front_matter = toml::from_str::<FrontMatter>(&front_str)?;

        if !allow_draft && front_matter.draft {
            #[cfg(test)]
            log::warn!("blog marked as draft and not allowed");
            return Err(FrontMatterError::DraftNotAllowed);
        }
        if front_matter.most_recent_date() < min_date {
            #[cfg(test)]
            log::warn!("blog post too old to process");
            return Err(FrontMatterError::PostTooOld(min_date));
        }

        #[cfg(test)]
        log::trace!("Front matter: {front_matter:#?}");

        Ok(front_matter)
    }

    pub(super) fn bluesky_description(&self) -> &str {
        if self.bluesky.is_some() {
            return self.bluesky.as_ref().unwrap().description();
        }

        if self.extra.is_some() && self.extra.as_ref().unwrap().bluesky().is_some() {
            return self
                .extra
                .as_ref()
                .unwrap()
                .bluesky()
                .unwrap()
                .description();
        }

        &self.description
    }

    pub(super) fn bluesky_tags(&self) -> Vec<String> {
        if self.bluesky.is_some() {
            return self.bluesky.as_ref().unwrap().hashtags();
        }

        if self.extra.is_some() && self.extra.as_ref().unwrap().bluesky().is_some() {
            return self.extra.as_ref().unwrap().bluesky().unwrap().hashtags();
        }

        if self.taxonomies.is_some() {
            return self.taxonomies.as_ref().unwrap().hashtags();
        }

        Vec::new()
    }

    /// Return a toml formatted datetime representing the most
    /// recent date reported in the frontmatter.
    pub(super) fn most_recent_date(&self) -> Datetime {
        match (self.date.is_some(), self.updated.is_some()) {
            (false, false) => Datetime {
                date: None,
                time: None,
                offset: None,
            },
            (true, false) => self.date.unwrap(),
            (false, true) => self.updated.unwrap(),
            (true, true) => max(self.date.unwrap(), self.updated.unwrap()),
        }
    }

    pub(super) fn log_post_details(&self) {
        log::debug!(
            "Length of title: {} characters and {} graphemes",
            self.title.len(),
            self.title.graphemes(true).count()
        );
        log::debug!(
            "Length of description: {} characters and {} graphemes",
            self.description.len(),
            self.description.graphemes(true).count()
        );
        log::debug!(
            "Length of bluesky description: {} characters and {} graphemes",
            self.bluesky_description().len(),
            self.bluesky_description().graphemes(true).count()
        );
        log::debug!(
            "Length of tag contents: {} characters and {} graphemes",
            self.taxonomies
                .as_ref()
                .map_or(0, |e| e.tags().join("#").len() + 1),
            self.taxonomies
                .as_ref()
                .map_or(0, |e| e.tags().join("#").graphemes(true).count() + 1)
        );
        log::debug!(
            "Length of bluesky tag contents: {} characters and {} graphemes",
            {
                let tags = self.bluesky_tags();
                if tags.is_empty() {
                    0
                } else {
                    tags.join("#").len() + 1
                }
            },
            {
                let tags = self.bluesky_tags();
                if tags.is_empty() {
                    0
                } else {
                    tags.join("#").graphemes(true).count() + 1
                }
            }
        );
    }
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;

    use super::*;

    fn get_test_logger() {
        let mut builder = env_logger::Builder::new();
        builder.filter(None, LevelFilter::Debug);
        builder.format_timestamp_secs().format_module_path(false);
        let _ = builder.try_init();
    }

    #[test]
    fn test_from_toml_basic() {
        let toml = r#"
            title = "Test Title"
            description = "Test Description"

            [taxonomies]
            tags = ["rust", "testing"]
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert_eq!(fm.title(), "Test Title");
        assert_eq!(fm.description, "Test Description");
        assert_eq!(fm.taxonomies.unwrap().tags(), &vec!["rust", "testing"]);
        assert!(fm.extra.is_none());
    }

    #[test]
    fn test_from_toml_with_extra() {
        get_test_logger();

        let toml = r#"
            title = "Extra Test"
            description = "Has extra field"

            [taxonomies]
            tags = ["extra"]

            [extra]
            bluesky.description = "extra_value"
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.taxonomies.unwrap().tags(), &vec!["extra"]);
        assert!(fm.extra.is_some());
        assert_eq!(
            fm.extra.unwrap().bluesky().unwrap().description(),
            "extra_value"
        );
    }

    #[test]
    fn test_from_toml_with_extra_bluesky() {
        get_test_logger();

        let toml = r#"
            title = "Extra Test"
            description = "Has extra field"

            [taxonomies]
            tags = ["extra"]

            [extra]

            [extra.bluesky]
            description = "extra_value"
            tags = ["extra_tag"]
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.taxonomies.unwrap().tags(), &vec!["extra"]);
        assert!(fm.extra.is_some());
        assert_eq!(
            fm.extra.as_ref().unwrap().bluesky().unwrap().description(),
            "extra_value"
        );
        assert_eq!(
            fm.extra.as_ref().unwrap().bluesky().unwrap().tags(),
            vec!["extra_tag".to_string()]
        );
    }

    #[test]
    fn test_from_toml_with_bluesky() {
        get_test_logger();

        let toml = r#"
            title = "Extra Test"
            description = "Has extra field"

            [taxonomies]
            tags = ["extra"]

            [bluesky]
            description = "extra_value"
            tags = ["extra_tag"]
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.taxonomies.unwrap().tags(), &vec!["extra"]);
        assert!(fm.bluesky.is_some());
        assert_eq!(fm.bluesky.as_ref().unwrap().description(), "extra_value");
        assert_eq!(
            fm.bluesky.as_ref().unwrap().tags(),
            vec!["extra_tag".to_string()]
        );
    }

    #[test]
    fn test_from_toml_missing_tags() {
        let toml = r#"
            title = "Missing Fields"
            description = "No taxonomies"
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert_eq!(fm.title, "Missing Fields");
        assert_eq!(fm.description, "No taxonomies");
        assert!(fm.taxonomies.is_none());
    }

    #[test]
    fn test_from_toml_bluesky_set_incorrectly() {
        get_test_logger();
        let toml = r#"
            title = "Overview of Our Workflow"
            description = "We’ll kick things off with a detailed overview"
            date = 2025-01-17
            updated = 2025-01-16
            draft = false
    
            [taxonomies]
            topic = ["Technology"]
            tags = ["devsecops", "software", "circleci", "security", "practices"]
    
            [extra]
            bluesky = "Covering the key steps involved."
            "#;

        let expected =
            r#"invalid type: string "Covering the key steps involved.", expected struct Bluesky"#;

        let fm_res = toml::from_str::<FrontMatter>(toml);
        assert!(fm_res.is_err());
        assert_eq!(fm_res.err().unwrap().message(), expected);
    }

    #[test]
    fn test_from_toml_invalid() {
        let toml = r#"
            title = 123
            description = "Invalid type"
        "#;
        let result = toml::from_str::<FrontMatter>(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_hashtags_formatting() {
        let taxonomies = Taxonomies::new(vec![
            "rust".to_string(),
            "blue sky".to_string(),
            "#AlreadyHashtag".to_string(),
            "multi word tag".to_string(),
            "".to_string(),
        ]);
        let hashtags = taxonomies.hashtags();
        assert_eq!(
            hashtags,
            vec!["#Rust", "#BlueSky", "#AlreadyHashtag", "#MultiWordTag", "#"]
        );
    }

    #[test]
    fn test_front_matter_empty_toml() {
        let toml = r#"
            title = "Extra Test"
            description = "Has extra field"
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.description, "Has extra field");
        assert!(fm.taxonomies.is_none());
        assert!(fm.extra.is_none());
    }

    #[test]
    fn test_taxonomies_empty_tags() {
        let taxonomies = Taxonomies::new(vec![]);
        let hashtags = taxonomies.hashtags();
        assert_eq!(hashtags, Vec::<String>::new());
    }

    #[test]
    fn test_most_recent_date_no_dates() {
        let fm = FrontMatter {
            title: "Test".to_string(),
            description: "Test".to_string(),
            date: None,
            updated: None,
            ..Default::default()
        };
        let result = fm.most_recent_date();
        assert!(result.date.is_none());
        assert!(result.time.is_none());
        assert!(result.offset.is_none());
    }

    #[test]
    fn test_most_recent_date_only_date() {
        let date = Datetime {
            date: Some(toml::value::Date {
                year: 2025,
                month: 1,
                day: 1,
            }),
            time: None,
            offset: None,
        };
        let fm = FrontMatter {
            title: "Test".to_string(),
            description: "Test".to_string(),
            date: Some(date),
            updated: None,
            ..Default::default()
        };
        let result = fm.most_recent_date();
        assert_eq!(result, date);
    }

    #[test]
    fn test_most_recent_date_only_updated() {
        let updated = Datetime {
            date: Some(toml::value::Date {
                year: 2025,
                month: 1,
                day: 2,
            }),
            time: None,
            offset: None,
        };
        let fm = FrontMatter {
            title: "Test".to_string(),
            description: "Test".to_string(),
            date: None,
            updated: Some(updated),
            ..Default::default()
        };
        let result = fm.most_recent_date();
        assert_eq!(result, updated);
    }

    #[test]
    fn test_most_recent_date_both_dates_updated_newer() {
        let date = Datetime {
            date: Some(toml::value::Date {
                year: 2025,
                month: 1,
                day: 1,
            }),
            time: None,
            offset: None,
        };
        let updated = Datetime {
            date: Some(toml::value::Date {
                year: 2025,
                month: 1,
                day: 2,
            }),
            time: None,
            offset: None,
        };
        let fm = FrontMatter {
            title: "Test".to_string(),
            description: "Test".to_string(),
            date: Some(date),
            updated: Some(updated),
            ..Default::default()
        };
        let result = fm.most_recent_date();
        assert_eq!(result, updated);
    }

    #[test]
    fn test_most_recent_date_both_dates_date_newer() {
        let date = Datetime {
            date: Some(toml::value::Date {
                year: 2025,
                month: 1,
                day: 2,
            }),
            time: None,
            offset: None,
        };
        let updated = Datetime {
            date: Some(toml::value::Date {
                year: 2025,
                month: 1,
                day: 1,
            }),
            time: None,
            offset: None,
        };
        let fm = FrontMatter {
            title: "Test".to_string(),
            description: "Test".to_string(),
            date: Some(date),
            updated: Some(updated),
            ..Default::default()
        };
        let result = fm.most_recent_date();
        assert_eq!(result, date);
    }

    #[test]
    fn test_date_from_toml_basic() {
        let toml = r#"
            title = "Date Test"
            description = "Basic date test"
            date = 2025-01-17
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert!(fm.date.is_some());
        let date = fm.date.unwrap();
        assert_eq!(date.date.unwrap().year, 2025);
        assert_eq!(date.date.unwrap().month, 1);
        assert_eq!(date.date.unwrap().day, 17);
        assert!(date.time.is_none());
        assert!(date.offset.is_none());
    }

    #[test]
    fn test_date_from_toml_with_time() {
        let toml = r#"
            title = "DateTime Test"
            description = "Date with time test"
            date = 2025-01-17T15:30:00Z
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert!(fm.date.is_some());
        let date = fm.date.unwrap();
        assert_eq!(date.date.unwrap().year, 2025);
        assert_eq!(date.date.unwrap().month, 1);
        assert_eq!(date.date.unwrap().day, 17);
        assert!(date.time.is_some());
        assert_eq!(date.time.unwrap().hour, 15);
        assert_eq!(date.time.unwrap().minute, 30);
        assert_eq!(date.time.unwrap().second, 0);
        assert!(date.offset.is_some());
    }

    #[test]
    fn test_date_from_toml_with_timezone() {
        let toml = r#"
            title = "Timezone Test"
            description = "Date with timezone test"
            date = 2025-01-17T15:30:00+02:00
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert!(fm.date.is_some());
        let date = fm.date.unwrap();
        assert_eq!(date.date.unwrap().year, 2025);
        assert_eq!(date.date.unwrap().month, 1);
        assert_eq!(date.date.unwrap().day, 17);
        assert!(date.time.is_some());
        assert_eq!(date.time.unwrap().hour, 15);
        assert_eq!(date.time.unwrap().minute, 30);
        assert!(date.offset.is_some());
        assert_eq!(
            date.offset.unwrap(),
            toml::value::Offset::Custom { minutes: 120 }
        );
    }

    #[test]
    fn test_invalid_date_format() {
        let toml = r#"
            title = "Invalid Date"
            description = "Invalid date format test"
            date = "not-a-date"
        "#;
        let result = toml::from_str::<FrontMatter>(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_date_comparison() {
        get_test_logger();

        let toml = r#"
            title = "Date Comparison"
            description = "Testing date comparison"
            date = 2025-01-17T15:30:00Z
            updated = 2025-01-18T15:30:00Z
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert!(fm.date.is_some());
        assert!(fm.updated.is_some());
        let most_recent = fm.most_recent_date();
        assert_eq!(most_recent.date.unwrap().day, 18);
    }

    #[test]
    fn test_date_with_microseconds() {
        let toml = r#"
            title = "Microseconds Test"
            description = "Date with microseconds test"
            date = 2025-01-17T15:30:00.123456Z
        "#;
        let fm = toml::from_str::<FrontMatter>(toml).unwrap();
        assert!(fm.date.is_some());
        let date = fm.date.unwrap();
        assert_eq!(date.date.unwrap().year, 2025);
        assert_eq!(date.time.unwrap().nanosecond, 123456000);
    }
}
