use std::{fs, path::Path};

use serde::Deserialize;
use url::Url;

use crate::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    base_url: Url,
}

impl SiteConfig {
    pub fn new(www_src_root: &Path, filename: Option<&str>) -> Result<Self, Error> {
        let filename = filename.unwrap_or("config.toml");

        let file_path = www_src_root.join(filename);

        let site_config = match fs::read_to_string(&file_path) {
            Ok(sc) => sc,
            Err(e) => {
                log::error!("failed to read to string {}", file_path.display());
                return Err(e.into());
            }
        };

        let site_config: SiteConfig = toml::from_str(site_config.as_str())?;

        Ok(site_config)
    }

    pub fn base_url(&self) -> Url {
        self.base_url.clone()
    }
}
