use chrono::{Datelike, NaiveDate, Utc};
use keep_a_changelog::{Changelog, Version};

use crate::Error;

pub trait ReleaseUnreleased {
    fn release_unreleased(&mut self, version: &str) -> Result<(), Error>;
}

impl ReleaseUnreleased for Changelog {
    /// Release unreleased section with the version number specified and add
    /// a compare link for the new release to the changelog
    ///
    fn release_unreleased(&mut self, version: &str) -> Result<(), Error> {
        let url = self.url();
        log::debug!("Changelog url: {:?}", url);

        let anchor = version.to_string();
        log::debug!("anchor: {:?}", anchor);
        let repo_url = self.url().clone().unwrap();

        let url = format!(
            "{}/compare/{}..{}",
            repo_url,
            self.releases()
                .get(1)
                .map(|r| {
                    r.version()
                        .clone()
                        .unwrap_or_else(|| Version::new(0, 0, 0))
                        .to_string()
                })
                .unwrap_or_else(|| "0.0.0".to_string()),
            anchor
        );

        log::debug!("url: {:?}", url);

        let Some(unreleased) = self.get_unreleased_mut() else {
            return Err(Error::NoUnreleasedSection);
        };

        let version = Version::parse(version).map_err(|e| Error::InvalidVersion(e.to_string()))?;

        log::debug!("version: {:?}", version);
        unreleased.set_version(version);

        let today =
            NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), Utc::now().day());

        log::debug!("today: {:?}", today);
        if let Some(today) = today {
            unreleased.set_date(today);
        };

        log::debug!("unreleased: {:?}", unreleased);

        self.add_link(anchor, url);

        Ok(())
    }
}

//test module
#[cfg(test)]
mod tests {
    use std::fs;

    use crate::Error;
    use keep_a_changelog::ChangelogParseOptions;
    use log::LevelFilter;

    use super::*;

    fn get_test_logger() {
        let mut builder = env_logger::Builder::new();
        builder.filter(None, LevelFilter::Debug);
        builder.format_timestamp_secs().format_module_path(false);
        let _ = builder.try_init();
    }

    #[allow(dead_code)]
    fn are_the_same(file_a: &str, file_b: &str) -> Result<bool, Error> {
        let file_a_contents = fs::read_to_string(file_a)?;
        let file_b_contents = fs::read_to_string(file_b)?;

        if file_a_contents.len() != file_b_contents.len() {
            return Ok(false);
        }

        let a_lines: Vec<_> = file_a_contents.lines().collect();
        let b_lines: Vec<_> = file_b_contents.lines().collect();

        if a_lines.len() != b_lines.len() {
            return Ok(false);
        }

        for (a, b) in a_lines.iter().zip(b_lines.iter()) {
            if a != b {
                return Ok(false);
            }
        }

        Ok(true)
    }

    #[test]
    fn test_release_unreleased_creates_unreleased_section() -> Result<(), Error> {
        get_test_logger();

        let original_path = "tests/data/release_changelog.md";

        // setup

        let opts = ChangelogParseOptions {
            url: Some("https://github.com/jerus-org/ci-container".to_string()),
            head: Some("main".to_string()),
            tag_prefix: None,
        };

        let mut changelog = Changelog::parse_from_file(original_path, Some(opts))
            .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
        let new_version = "0.1.2";

        let total_releases = changelog.releases().len();
        log::debug!("total_releases: {:?}", total_releases);

        // test
        changelog.release_unreleased(new_version).unwrap();

        // verify

        let releases = changelog.releases();
        log::debug!("releases: {:?}", releases);

        let new_latest_version = releases
            .first()
            .map(|r| {
                r.version()
                    .clone()
                    .unwrap_or_else(|| Version::new(0, 0, 0))
                    .to_string()
            })
            .unwrap_or_else(|| "0.0.0".to_string());

        log::debug!("new_latest_version: {:?}", new_latest_version);

        assert_eq!(total_releases, releases.len());
        assert!(changelog.get_unreleased().is_none());
        assert_eq!(new_latest_version, new_version);

        Ok(())
    }
}
