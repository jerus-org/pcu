use std::fs;
use url::Url;

use serde::Deserialize;

use crate::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    base_url: Url,
}

impl SiteConfig {
    pub fn new() -> Result<Self, Error> {
        let site_config = fs::read_to_string("config.toml")?;
        let site_config: SiteConfig = toml::from_str(site_config.as_str())?;

        Ok(site_config)
    }

    pub fn base_url(&self) -> Url {
        self.base_url.clone()
    }
}
