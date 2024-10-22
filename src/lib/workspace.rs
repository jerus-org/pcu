use std::path::Path;

use crate::Error;
use cargo_toml::Manifest;

#[derive(Debug, Clone)]
pub struct Workspace {
    pub manifest: Manifest,
}

impl Workspace {
    pub fn new(ws_cargo_toml: &Path) -> Result<Self, Error> {
        let manifest = Manifest::from_path(ws_cargo_toml)?;

        Ok(Self { manifest })
    }
}
