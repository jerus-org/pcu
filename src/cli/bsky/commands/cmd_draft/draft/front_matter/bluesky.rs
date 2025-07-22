use serde::Deserialize;

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Bluesky {
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl Bluesky {
    pub fn description(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }

    pub fn tags(&self) -> Vec<String> {
        self.tags.clone().unwrap_or_default()
    }
}
