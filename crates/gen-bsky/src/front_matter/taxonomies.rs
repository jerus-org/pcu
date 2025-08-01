use serde::Deserialize;

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Taxonomies {
    #[allow(dead_code)]
    pub tags: Vec<String>,
}

impl Taxonomies {
    pub fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    pub fn hashtags(&self) -> Vec<String> {
        let mut hashtags = vec![];
        for tag in &self.tags() {
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
