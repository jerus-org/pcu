use serde::{Deserialize, Serialize};

use crate::draft::blog_post::front_matter::tags;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Taxonomies {
    #[allow(dead_code)]
    tags: Vec<String>,
}

#[cfg(test)]
impl Taxonomies {
    pub(crate) fn new(tags: Vec<String>) -> Self {
        Taxonomies { tags }
    }
}

impl Taxonomies {
    pub(crate) fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    pub(crate) fn hashtags(&self) -> Vec<String> {
        tags::hashtags(self.tags.clone())
    }
}
