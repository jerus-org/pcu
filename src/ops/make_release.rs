use std::{fs, path, sync::Arc};

use chrono::{Datelike, NaiveDate, Utc};
use keep_a_changelog::{
    changelog::ChangelogBuilder, Changelog, ChangelogParseOptions, Release, Version,
};
use octocrab::Octocrab;

use crate::{release_notes_provider::ReleaseNotesProvider, Client, Error, GitOps};

pub trait MakeRelease {
    #[allow(async_fn_in_trait)]
    async fn make_release(&self, version: &str) -> Result<(), Error>;
    fn release_unreleased(&mut self, version: &str) -> Result<(), Error>;
    fn update_unreleased(&mut self, version: &str) -> Result<(), Error>;
}

impl MakeRelease for Client {
    /// Update the unreleased section to the changelog to `version`
    fn update_unreleased(&mut self, version: &str) -> Result<(), Error> {
        log::debug!(
            "Updating unreleased section: {:?} with version {:?}",
            self.changelog,
            version,
        );

        if self.changelog.is_empty() {
            return Err(Error::NoChangeLogFileFound);
        }

        if !version.is_empty() {
            #[allow(clippy::needless_question_mark)]
            return Ok(self.release_unreleased(version)?);
        }

        Ok(())
    }

    fn release_unreleased(&mut self, version: &str) -> Result<(), Error> {
        if self.changelog.is_empty() {
            return Err(Error::InvalidPath(self.changelog.clone()));
        };

        let log_file = &self.changelog;
        log::debug!(
            "Releasing unreleased section in file: {:?} to version {:?}",
            log_file,
            version
        );

        let repo_url = Some(format!("https://github.com/{}/{}", self.owner, self.repo));

        let mut change_log = if path::Path::new(&log_file).exists() {
            let file_contents = fs::read_to_string(path::Path::new(&log_file))?;
            log::trace!("file contents:\n---\n{}\n---\n\n", file_contents);
            let options = if repo_url.is_some() {
                Some(ChangelogParseOptions {
                    url: repo_url.clone(),
                    ..Default::default()
                })
            } else {
                None
            };

            Changelog::parse_from_file(log_file.to_str().unwrap(), options)
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?
        } else {
            log::trace!("The changelog does not exist! Create a default changelog.");
            let mut changelog = ChangelogBuilder::default()
                .url(repo_url)
                .build()
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
            log::debug!("Changelog: {:#?}", changelog);
            let release = Release::builder()
                .build()
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
            changelog.add_release(release);
            log::debug!("Changelog: {:#?}", changelog);

            changelog
                .save_to_file(log_file.to_str().unwrap())
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
            changelog
        };

        // Get the unreleased section from the Changelog.
        // If there is no unreleased section create it and add it to the changelog
        let unreleased = if let Some(unreleased) = change_log.get_unreleased_mut() {
            unreleased
        } else {
            let release = Release::builder()
                .build()
                .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
            change_log.add_release(release);
            let unreleased = change_log.get_unreleased_mut().unwrap();
            unreleased
        };

        let version = Version::parse(version).map_err(|e| Error::InvalidVersion(e.to_string()))?;
        unreleased.set_version(version);

        let today =
            NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), Utc::now().day());
        if let Some(today) = today {
            unreleased.set_date(today);
        };

        let unreleased_string = unreleased.to_string();
        log::trace!("Release notes:\n\n---\n{}\n---\n\n", unreleased_string);
        let _ = fs::write("release_notes.md", unreleased_string.clone());

        self.unreleased = Some(unreleased_string);

        change_log
            .save_to_file(log_file.to_str().unwrap())
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
            // client: http client with the octocrab user agent.
            Err(_) => {
                log::debug!("Creating un-authenticated instance");
                octocrab::instance()
            }
        };

        let tag = format!("v{version}");
        let commit = Client::get_commitish_for_tag(self, &octocrab, &tag).await?;
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
