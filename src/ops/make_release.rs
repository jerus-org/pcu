use crate::{
    utilities::{ReleaseNotesProvider, ReleaseUnreleased},
    Client, Error, GitOps,
};
use keep_a_changelog::{Changelog, ChangelogParseOptions};
use octocrate::repos::create_release::RequestMakeLatest;

pub trait MakeRelease {
    #[allow(async_fn_in_trait)]
    async fn make_release(&self, prefix: &str, version: &str) -> Result<(), Error>;
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

    async fn make_release(&self, prefix: &str, version: &str) -> Result<(), Error> {
        log::debug!("Making release {version}");

        let opts = ChangelogParseOptions::default();
        let changelog = match Changelog::parse_from_file(self.changelog_as_str(), Some(opts)) {
            Ok(changelog) => changelog,
            Err(e) => {
                log::error!("Error parsing changelog: {e}");
                return Err(Error::InvalidPath(self.changelog.clone()));
            }
        };

        let release_notes = changelog.release_notes(prefix, version)?;
        log::trace!("Release notes: {:#?}", release_notes);

        let tag = format!("{prefix}{version}");
        let commit = Self::get_commitish_for_tag(self, &tag).await?;
        log::trace!("Commit: {:#?}", commit);

        let release_request = octocrate::repos::create_release::Request {
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

        let release = self
            .github_rest
            .repos
            .create_release(self.owner(), self.repo())
            .body(&release_request)
            .send()
            .await?;

        log::trace!("Release: {:#?}", release);

        Ok(())
    }
}
