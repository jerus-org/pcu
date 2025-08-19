use serde::{Deserialize, Serialize};

use super::tags;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Bluesky {
    description: Option<String>,
    tags: Option<Vec<String>>,
}

impl Bluesky {
    pub(crate) fn description(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }

    #[allow(dead_code)]
    pub(crate) fn tags(&self) -> Vec<String> {
        self.tags.clone().unwrap_or_default()
    }

    pub(crate) fn hashtags(&self) -> Vec<String> {
        tags::hashtags(self.tags.clone().unwrap_or_default())
    }
}
