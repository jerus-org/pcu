use std::cmp::max;

use crate::Error;
use bsky_sdk::api::app::bsky::feed::post::RecordData;
use link_bridge::Redirector;
use serde::{Deserialize /* Deserializer */};
use toml::value::Datetime;
use unicode_segmentation::UnicodeSegmentation;

mod bluesky;
mod extra;
mod taxonomies;

pub use bluesky::Bluesky;
use extra::Extra;
use taxonomies::Taxonomies;

// +++
// title = "Blue Sky Test Blog"
// description = "A blog post to test the processing of blog posts for posting to Bluesky."
// date = 2025-01-17
// updated = 2025-01-16
// draft = false
//
// [taxonomies]
// topic = ["Technology"]
// description = "A blog post to test the processing of blog posts for posting to Bluesky."
// tags = ["bluesky", "testing", "test only", "ci"]
//
// [extra]
// bluesky.description = "This is a test blog post for Bluesky."
// bluesky.tags = ["bluesky", "testing", "test only", "ci"]
// +++
//

#[derive(Default, Debug, Clone, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub description: String,
    pub date: Option<Datetime>,
    pub updated: Option<Datetime>,
    #[serde(default)]
    pub draft: bool,
    pub taxonomies: Option<Taxonomies>,
    pub extra: Option<Extra>,
    pub bluesky: Option<Bluesky>,
    pub basename: Option<String>,
    pub path: Option<String>,
    pub bluesky_post: Option<RecordData>,
    pub post_link: Option<String>,
    pub post_short_link: Option<String>,
    pub short_link_store: Option<String>,
}

impl FrontMatter {
    pub fn from_toml(toml: &str) -> Result<Self, Error> {
        let front_matter = toml::from_str::<FrontMatter>(toml)?;
        Ok(front_matter)
    }

    fn bluesky_description(&self) -> &str {
        if self.bluesky.is_some() {
            return self.bluesky.as_ref().unwrap().description();
        }

        if self.extra.is_some() && self.extra.as_ref().unwrap().bluesky.is_some() {
            return self
                .extra
                .as_ref()
                .unwrap()
                .bluesky
                .as_ref()
                .unwrap()
                .description();
        }

        &self.description
    }

    fn bluesky_tags(&self) -> Vec<String> {
        if self.bluesky.is_some() {
            return self.bluesky.as_ref().unwrap().tags();
        }

        if self.extra.is_some() && self.extra.as_ref().unwrap().bluesky.is_some() {
            return self
                .extra
                .as_ref()
                .unwrap()
                .bluesky
                .as_ref()
                .unwrap()
                .tags();
        }

        if self.taxonomies.is_some() {
            return self.taxonomies.as_ref().unwrap().tags();
        }

        Vec::new()
    }

    pub fn most_recent_date(&self) -> Datetime {
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

    pub fn build_post_text(&mut self, base_url: &str) -> Result<String, Error> {
        let post_dir = if let Some(path) = self.path.as_ref() {
            format!("{}{}", path, "/")
        } else {
            String::new()
        };

        self.get_links(base_url, &post_dir)?;

        if log::log_enabled!(log::Level::Debug) {
            self.log_post_details();
        }

        let post_text = format!(
            "{}\n\n{} {}\n\n{}",
            self.title,
            self.bluesky_description(),
            self.taxonomies
                .as_ref()
                .map_or(String::new(), |tax| tax.hashtags().join(" ")),
            self.post_short_link.as_ref().map_or_else(
                || self.post_link.as_ref().unwrap().to_string(),
                |link| link.to_string()
            )
        );

        if post_text.len() > 300 {
            return Err(Error::PostTooCharacters(
                self.title.clone(),
                post_text.len(),
            ));
        }

        if post_text.graphemes(true).count() > 300 {
            return Err(Error::PostTooManyGraphemes(
                self.title.clone(),
                post_text.graphemes(true).count(),
            ));
        }

        Ok(post_text)
    }

    fn get_links(&mut self, base_url: &str, post_dir: &str) -> Result<(), Error> {
        log::debug!(
            "Building link with {base_url}, {post_dir} and {}",
            self.basename.as_ref().unwrap()
        );

        let post_link = format!(
            "/{}{}/index.html",
            post_dir.trim_start_matches("content/"),
            self.basename.as_ref().unwrap()
        );

        self.post_link = Some(post_link.clone());

        let mut redirect = Redirector::new(post_link)?;
        if let Some(redirect_path) = self.short_link_store.as_ref() {
            log::debug!("redirect path set as `{redirect_path}`");
            redirect.set_path(redirect_path);
        } else {
            log::debug!("redirect path set to default (`static/s`)");
            redirect.set_path("static/s");
        }
        let short_link = redirect.write_redirect()?;
        log::debug!("redirect written and short link returned: {short_link}");

        self.post_short_link = Some(format!(
            "{}/{}{}/index.html",
            base_url.trim_end_matches('/'),
            post_dir.trim_start_matches("content/"),
            self.basename.as_ref().unwrap()
        ));
        Ok(())
    }

    #[allow(dead_code)]
    fn set_short_link_store<S: ToString>(&mut self, store: S) {
        self.short_link_store = Some(store.to_string());
    }

    fn log_post_details(&self) {
        log::debug!("Post link: {}", self.post_link.as_ref().unwrap());
        log::debug!(
            "Length of post link: {} characters and {} graphemes",
            self.post_link.as_ref().unwrap().len(),
            self.post_link.as_ref().unwrap().graphemes(true).count()
        );
        log::debug!(
            "Length of post short link: {} characters and {} graphemes",
            self.post_short_link.as_ref().map_or(0, |link| link.len()),
            self.post_short_link
                .as_ref()
                .map_or(0, |link| link.graphemes(true).count())
        );
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
                .map_or(0, |e| e.tags.join("#").len() + 1),
            self.taxonomies
                .as_ref()
                .map_or(0, |e| e.tags.join("#").graphemes(true).count() + 1)
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
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Test Title");
        assert_eq!(fm.description, "Test Description");
        assert_eq!(fm.taxonomies.unwrap().tags, vec!["rust", "testing"]);
        assert!(fm.extra.is_none());
        assert!(fm.basename.is_none());
        assert!(fm.path.is_none());
        assert!(fm.bluesky_post.is_none());
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
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.taxonomies.unwrap().tags, vec!["extra"]);
        assert!(fm.extra.is_some());
        assert_eq!(
            fm.extra.unwrap().bluesky.unwrap().description,
            Some("extra_value".to_string())
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
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.taxonomies.unwrap().tags, vec!["extra"]);
        assert!(fm.extra.is_some());
        assert_eq!(
            fm.extra
                .as_ref()
                .unwrap()
                .bluesky
                .as_ref()
                .unwrap()
                .description,
            Some("extra_value".to_string())
        );
        assert_eq!(
            fm.extra.as_ref().unwrap().bluesky.as_ref().unwrap().tags,
            Some(vec!["extra_tag".to_string()])
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
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.taxonomies.unwrap().tags, vec!["extra"]);
        assert!(fm.bluesky.is_some());
        assert_eq!(
            fm.bluesky.as_ref().unwrap().description,
            Some("extra_value".to_string())
        );
        assert_eq!(
            fm.bluesky.as_ref().unwrap().tags,
            Some(vec!["extra_tag".to_string()])
        );
    }

    #[test]
    fn test_from_toml_missing_fields() {
        let toml = r#"
            title = "Missing Fields"
            description = "No taxonomies"
        "#;
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Missing Fields");
        assert_eq!(fm.description, "No taxonomies");
        assert!(fm.taxonomies.is_none());
    }

    #[test]
    fn test_from_toml_invalid() {
        let toml = r#"
            title = 123
            description = "Invalid type"
        "#;
        let result = FrontMatter::from_toml(toml);
        assert!(result.is_err());
    }
    #[test]
    fn test_hashtags_formatting() {
        let taxonomies = Taxonomies {
            tags: vec![
                "rust".to_string(),
                "blue sky".to_string(),
                "#AlreadyHashtag".to_string(),
                "multi word tag".to_string(),
                "".to_string(),
            ],
        };
        let hashtags = taxonomies.hashtags();
        assert_eq!(
            hashtags,
            vec!["#Rust", "#BlueSky", "#AlreadyHashtag", "#MultiWordTag", "#"]
        );
    }

    #[test]
    fn test_front_matter_with_basename_and_path() {
        let toml = r#"
            title = "With Path"
            description = "Has basename and path"
            basename = "post1"
            path = "/blog/post1.md"
        "#;
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.basename.as_deref(), Some("post1"));
        assert_eq!(fm.path.as_deref(), Some("/blog/post1.md"));
    }

    #[test]
    fn test_front_matter_empty_toml() {
        let toml = r#"
            title = "Extra Test"
            description = "Has extra field"
        "#;
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert_eq!(fm.title, "Extra Test");
        assert_eq!(fm.description, "Has extra field");
        assert!(fm.taxonomies.is_none());
        assert!(fm.extra.is_none());
    }

    #[test]
    fn test_taxonomies_empty_tags() {
        let taxonomies = Taxonomies { tags: vec![] };
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
        let fm = FrontMatter::from_toml(toml).unwrap();
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
        let fm = FrontMatter::from_toml(toml).unwrap();
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
        let fm = FrontMatter::from_toml(toml).unwrap();
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
        let result = FrontMatter::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_date_comparison() {
        let toml = r#"
            title = "Date Comparison"
            description = "Testing date comparison"
            date = 2025-01-17T15:30:00Z
            updated = 2025-01-18T15:30:00Z
        "#;
        let fm = FrontMatter::from_toml(toml).unwrap();
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
        let fm = FrontMatter::from_toml(toml).unwrap();
        assert!(fm.date.is_some());
        let date = fm.date.unwrap();
        assert_eq!(date.date.unwrap().year, 2025);
        assert_eq!(date.time.unwrap().nanosecond, 123456000);
    }
}
