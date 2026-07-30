use keep_a_changelog::{Changelog, ChangelogParseOptions};
use octocrate::repos::create_release::RequestMakeLatest;

use crate::{
    utilities::{ReleaseNotesProvider, ReleaseUnreleased},
    Client, Error, GitOps,
};

pub trait MakeRelease {
    #[allow(async_fn_in_trait)]
    async fn make_release(&self, prefix: &str, version: &str, draft: bool) -> Result<(), Error>;
    fn release_unreleased(&mut self, version: &str) -> Result<(), Error>;
}

impl MakeRelease for Client {
    fn release_unreleased(&mut self, version: &str) -> Result<(), Error> {
        let opts = self.prlog_parse_options.clone();

        let mut change_log = Changelog::parse_from_file(self.prlog_as_str(), Some(opts))
            .map_err(|e| Error::KeepAChangelog(e.to_string()))?;

        let total_releases = change_log.releases().len();
        log::debug!("total_releases: {total_releases:?}");

        change_log.release_unreleased(version).unwrap();

        change_log
            .save_to_file(self.prlog_as_str())
            .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
        Ok(())
    }

    async fn make_release(&self, prefix: &str, version: &str, draft: bool) -> Result<(), Error> {
        log::debug!("Making release {version} (draft={draft})");

        let opts = ChangelogParseOptions::default();
        let prlog = match Changelog::parse_from_file(self.prlog_as_str(), Some(opts)) {
            Ok(pl) => pl,
            Err(e) => {
                log::error!("Error parsing prlog: {e}");
                return Err(Error::InvalidPath(self.prlog.clone()));
            }
        };

        let release_notes = prlog.release_notes(prefix, version)?;
        log::trace!("Release notes: {release_notes:#?}");

        let tag = format!("{prefix}{version}");
        let commit = Self::get_commitish_for_tag(self, &tag).await?;
        log::trace!("Commit: {commit:#?}");

        // `make_latest` is only set when publishing directly: GitHub documents
        // that "Drafts and prereleases cannot be set as latest", so for a draft
        // it moves to the publish call that flips `draft` to false.
        let builder = octocrate::repos::create_release::Request::builder()
            .body(release_notes.body.to_string())
            .name(release_notes.name.to_string())
            .tag_name(tag)
            .target_commitish(commit);

        let release_request = if draft {
            builder.draft(true).build()
        } else {
            builder.make_latest(RequestMakeLatest::True).build()
        };

        let release = match self
            .github_rest
            .repos
            .create_release(self.owner(), self.repo())
            .body(&release_request)
            .send()
            .await
        {
            Ok(release) => release,
            Err(e) => {
                log::error!("Error creating release: {e}");
                return Err(e.into());
            }
        };

        log::trace!("Release: {release:#?}");

        Ok(())
    }
}
