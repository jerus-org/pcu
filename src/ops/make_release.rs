use crate::{
    utilities::{ReleaseNotesProvider, ReleaseUnreleased},
    Client, Error, GitOps,
};
use keep_a_changelog::{Changelog, ChangelogParseOptions};
use octocrate::repos::create_release;
use octocrate::repos::create_release::RequestMakeLatest;
use octocrate::repos::GitHubReposAPI;
use octocrate::APIConfig;
use octocrate::PersonalAccessToken;

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

        log::debug!("Creating octocrate instance {:?}", self.settings);
        log::trace!(
            "Creating octocate for owner: {} and repo: {}",
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

        log::debug!("Creating octocrate instance {:?}", self.settings);
        let config = match self.settings.get::<String>("pat") {
            Ok(pat) => {
                log::debug!("Using personal access token for authentication");
                // Create a personal access token
                let personal_access_token = PersonalAccessToken::new(&pat);

                // Use the personal access token to create a API configuration
                APIConfig::with_token(personal_access_token).shared()
            }
            Err(_) => {
                log::debug!("Creating un-authenticated instance");
                APIConfig::default().shared()
            }
        };

        let api = GitHubReposAPI::new(&config);

        let tag = format!("v{version}");
        let commit = Self::get_commitish_for_tag(self, &api, &tag).await?;
        log::trace!("Commit: {:#?}", commit);

        let release_request = create_release::Request {
            body: Some(release_notes.body.to_string()),
            discussion_category_name: None,
            draft: Some(false),
            generate_release_notes: Some(false),
            make_latest: Some(RequestMakeLatest::True),
            name: Some(release_notes.name.to_string()),
            prerelease: Some(false),
            tag_name: tag,
            target_commitish: Some(commit),
        };

        let release = api
            .create_release(self.owner(), self.repo())
            .body(&release_request)
            .send()
            .await?;

        log::trace!("Release: {:#?}", release);

        Ok(())
    }
}
