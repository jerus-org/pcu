use keep_a_changelog::{Changelog, Version};

use crate::Error;

#[derive(Debug, Clone)]
pub struct ReleaseNotes {
    pub name: String,
    pub body: String,
}

pub trait ReleaseNotesProvider {
    fn release_notes(&self, prefix: &str, release: &str) -> Result<ReleaseNotes, Error>;
}

impl ReleaseNotesProvider for Changelog {
    fn release_notes(&self, prefix: &str, release: &str) -> Result<ReleaseNotes, Error> {
        let name = format!("{prefix}{release}");

        let version = match Version::parse(release) {
            Ok(version) => version,
            Err(e) => {
                log::error!("Error parsing version: {e}");
                return Err(Error::InvalidVersion(release.to_string()));
            }
        };

        let mut body = String::from("## What's Changed\n\n");

        body.push_str(
            &self
                .releases()
                .iter()
                .find_map(|r| {
                    if let Some(rv) = r.version() {
                        if *rv == version {
                            r.description().clone()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .unwrap_or("".to_string()),
        );

        body.push_str(
            &self
                .releases()
                .iter()
                .find_map(|r| {
                    if let Some(rv) = r.version() {
                        if *rv == version {
                            Some(r.changes().to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .unwrap_or("".to_string()),
        );

        Ok(ReleaseNotes { name, body })
    }
}
