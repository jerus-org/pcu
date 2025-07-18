use crate::Error;
use bsky_sdk::api::app::bsky::feed::post::RecordData;
use serde::Deserialize;
use unicode_segmentation::UnicodeSegmentation;

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
pub struct Taxonomies {
    #[allow(dead_code)]
    pub tags: Vec<String>,
}

impl Taxonomies {
    pub fn hashtags(&self) -> Vec<String> {
        let mut hashtags = vec![];
        for tag in &self.tags {
            // convert tag to hashtag by capitalising the first letter of each word, removing the spaces and prefixing with a # if required
            let formatted_tag = tag
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<String>();
            let hashtag = if formatted_tag.starts_with('#') {
                formatted_tag
            } else {
                format!("#{formatted_tag}")
            };
            hashtags.push(hashtag);
        }

        hashtags
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Extra {
    #[allow(dead_code)]
    pub bluesky: Bluesky,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Bluesky {
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub description: String,
    pub taxonomies: Option<Taxonomies>,
    pub extra: Option<Extra>,
    pub basename: Option<String>,
    pub path: Option<String>,
    pub bluesky_post: Option<RecordData>,
    pub post_link: Option<String>,
    pub post_short_link: Option<String>,
}

impl FrontMatter {
    pub fn from_toml(toml: &str) -> Result<Self, Error> {
        let front_matter = toml::from_str::<FrontMatter>(toml)?;
        Ok(front_matter)
    }

    pub fn build_post_text(&mut self, base_url: &str) -> Result<String, Error> {
        let post_dir = if let Some(path) = self.path.as_ref() {
            format!("{}{}", path, "/")
        } else {
            String::new()
        };

        self.get_post_link(base_url, &post_dir);
        self.get_short_post_link(&post_dir);

        if log::log_enabled!(log::Level::Debug) {
            self.log_post_details();
        }

        let post_text = format!(
            "{}\n\n{} {}\n\n{}",
            self.title,
            self.extra.as_ref().map_or_else(
                || self.description.as_str(),
                |e| e
                    .bluesky
                    .description
                    .as_deref()
                    .unwrap_or(&self.description)
            ),
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

    fn get_post_link(&mut self, base_url: &str, post_dir: &str) {
        self.post_link = Some(format!(
            "{}/{}{}/index.html",
            base_url,
            post_dir,
            self.basename.as_ref().unwrap()
        ));
    }

    fn get_short_post_link(&mut self, post_dir: &str) {
        let post_link = format!("{post_dir}{}/", self.basename.as_ref().unwrap());
        let ts = chrono::Utc::now().timestamp();
        let link = post_link.encode_utf16().sum::<u16>();
        let short_link = base62::encode(ts as u64 + link as u64);

        self.post_short_link = Some(format!("s/{short_link}.html"));
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
            self.extra.as_ref().map_or(0, |e| e
                .bluesky
                .description
                .as_ref()
                .map_or(0, |s| s.len())),
            self.extra.as_ref().map_or(0, |e| e
                .bluesky
                .description
                .as_ref()
                .map_or(0, |s| s.graphemes(true).count()))
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
            self.extra.as_ref().map_or(0, |e| e
                .bluesky
                .tags
                .as_ref()
                .map_or(0, |tags| tags.join("#").len() + 1)),
            self.extra
                .as_ref()
                .map_or(0, |e| e.bluesky.tags.as_ref().map_or(0, |tags| tags
                    .join("#")
                    .graphemes(true)
                    .count()
                    + 1))
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
            fm.extra.unwrap().bluesky.description,
            Some("extra_value".to_string())
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
}
