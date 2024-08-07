use std::sync::Arc;

use keep_a_changelog::{Changelog, ChangelogParseOptions};
use octocrab::Octocrab;

use crate::{
    utilities::{ReleaseNotesProvider, ReleaseUnreleased},
    Client, Error, GitOps,
};

pub trait MakeRelease {
    #[allow(async_fn_in_trait)]
    async fn make_release(&self, version: &str) -> Result<(), Error>;
    fn release_unreleased(&mut self, version: &str) -> Result<(), Error>;
}

impl MakeRelease for Client {
    fn release_unreleased(&mut self, version: &str) -> Result<(), Error> {
        let opts = self.changelog_parse_options.clone();

        let mut change_log = Changelog::parse_from_file(self.changelog_as_str(), Some(opts))
            .map_err(|e| Error::KeepAChangelog(e.to_string()))?;

        let total_releases = change_log.releases().len();
        log::debug!("total_releases: {:?}", total_releases);

        change_log.release_unreleased(version).unwrap();

        change_log
            .save_to_file(self.changelog_as_str())
            .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
        Ok(())
    }

    async fn make_release(&self, version: &str) -> Result<(), Error> {
        log::debug!("Making release {version}");

        log::debug!("Creating octocrab instance {:?}", self.settings);
        log::trace!(
            "Creating octocrab for owner: {} and repo: {}",
            self.owner(),
            self.repo()
        );

        let opts = ChangelogParseOptions::default();
        let changelog = match Changelog::parse_from_file(self.changelog_as_str(), Some(opts)) {
            Ok(changelog) => changelog,
            Err(e) => {
                log::error!("Error parsing changelog: {e}");
                return Err(Error::InvalidPath(self.changelog.clone()));
            }
        };

        let release_notes = changelog.release_notes(version)?;
        log::trace!("Release notes: {:#?}", release_notes);

        log::debug!("Creating octocrab instance {:?}", self.settings);
        let octocrab = match self.settings.get::<String>("pat") {
            Ok(pat) => {
                log::debug!("Using personal access token for authentication");
                Arc::new(
                    Octocrab::builder()
                        .base_uri("https://api.github.com")?
                        .personal_token(pat)
                        .build()?,
                )
            }
            // base_uri: https://api.github.com
            // auth: None
            // self. http self.with the octocrab user agent.
            Err(_) => {
                log::debug!("Creating un-authenticated instance");
                octocrab::instance()
            }
        };

        let tag = format!("v{version}");
        let commit = Self::get_commitish_for_tag(self, &octocrab, &tag).await?;
        log::trace!("Commit: {:#?}", commit);

        let release = octocrab
            .repos(self.owner(), self.repo())
            .releases()
            .create(format!("v{version}").as_str())
            .name(&release_notes.name)
            .body(&release_notes.body)
            .send()
            .await?;

        log::trace!("Release: {:#?}", release);

        Ok(())
    }
}
