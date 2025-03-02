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

    pub fn packages(&self) -> Option<Vec<Package>> {
        if let Some(workspace) = &self.manifest.workspace {
            let members = &workspace.members;

            let mut packages = Vec::new();

            for member in members {
                let member_file = format!("./{member}/Cargo.toml");
                let path = Path::new(&member_file);
                let manifest = Manifest::from_path(path).unwrap();
                if let Some(package) = manifest.package {
                    let name = package.name;
                    let version = package.version.get().unwrap().to_string();

                    let package = Package { name, version };

                    packages.push(package);
                }
            }

            return Some(packages);
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
}
