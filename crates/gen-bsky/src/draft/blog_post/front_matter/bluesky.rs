use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Bluesky {
    description: Option<String>,
    tags: Option<Vec<String>>,
}

impl Bluesky {
    pub(crate) fn description(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }

    pub(crate) fn tags(&self) -> Vec<String> {
        self.tags.clone().unwrap_or_default()
    }

    pub(crate) fn hashtags(&self) -> Vec<String> {
        let mut hashtags = vec![];
        for tag in &self.tags() {
            // convert tag to hashtag by capitalising the first letter of each word,
            // removing the spaces and prefixing with a # if required
            let formatted_tag = tag
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
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
