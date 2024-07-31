use std::{ffi::OsStr, fs, path};

use keep_a_changelog::{
    changelog::ChangelogBuilder, ChangeKind, Changelog, ChangelogParseOptions, Release,
};
use log::debug;
use url::Url;

use crate::Error;

#[derive(Debug)]
pub struct PrTitle {
    pub title: String,
    pub pr_id: Option<u64>,
    pub pr_url: Option<Url>,
    pub commit_type: Option<String>,
    pub commit_scope: Option<String>,
    pub commit_breaking: bool,
    pub section: Option<ChangeKind>,
    pub entry: String,
}

impl PrTitle {
    pub fn parse(title: &str) -> Result<Self, Error> {
        let re = regex::Regex::new(
            r"^(?P<type>[a-z]+)(?:\((?P<scope>.+)\))?(?P<breaking>!)?: (?P<description>.*)$",
        )?;

        debug!("String to parse: `{}`", title);

        let pr_title = if let Some(captures) = re.captures(title) {
            log::trace!("Captures: {:#?}", captures);
            let commit_type = captures.name("type").map(|m| m.as_str().to_string());
            let commit_scope = captures.name("scope").map(|m| m.as_str().to_string());
            let commit_breaking = captures.name("breaking").is_some();
            let title = captures
                .name("description")
                .map(|m| m.as_str().to_string())
                .unwrap();

            Self {
                title,
                pr_id: None,
                pr_url: None,
                commit_type,
                commit_scope,
                commit_breaking,
                section: None,
                entry: String::new(),
            }
        } else {
            Self {
                title: title.to_string(),
                pr_id: None,
                pr_url: None,
                commit_type: None,
                commit_scope: None,
                commit_breaking: false,
                section: None,
                entry: String::new(),
            }
        };

        debug!("Parsed title: {:?}", pr_title);

        Ok(pr_title)
    }

    pub fn set_pr_id(&mut self, id: u64) {
        self.pr_id = Some(id);
    }

    pub fn set_pr_url(&mut self, url: Url) {
        self.pr_url = Some(url);
    }

    pub fn calculate_section_and_entry(&mut self) {
        let mut section = ChangeKind::Changed;
        let mut entry = self.title.clone();

        debug!("Initial description `{}`", entry);

        if let Some(commit_type) = &self.commit_type {
            match commit_type.as_str() {
                "feat" => section = ChangeKind::Added,
                "fix" => {
                    section = ChangeKind::Fixed;
                    if let Some(commit_scope) = &self.commit_scope {
                        log::trace!("Found scope `{}`", commit_scope);
                        entry = format!("{}: {}", commit_scope, self.title);
                    }
                }
                _ => {
                    section = ChangeKind::Changed;
                    entry = format!("{}-{}", self.commit_type.as_ref().unwrap(), entry);

                    debug!("After checking for `feat` or `fix` type: `{}`", entry);

                    if let Some(commit_scope) = &self.commit_scope {
                        log::trace!("Checking scope `{}`", commit_scope);
                        match commit_scope.as_str() {
                            "security" => {
                                section = ChangeKind::Security;
                                entry = format!("Security: {}", self.title);
                            }
                            "deps" => {
                                section = ChangeKind::Security;
                                entry = format!("Dependencies: {}", self.title);
                            }
                            "remove" => {
                                section = ChangeKind::Removed;
                                entry = format!("Removed: {}", self.title);
                            }
                            "deprecate" => {
                                section = ChangeKind::Deprecated;
                                entry = format!("Deprecated: {}", self.title);
                            }
                            _ => {
                                section = ChangeKind::Changed;
                                let split_description = entry.splitn(2, '-').collect::<Vec<&str>>();
                                log::trace!("Split description: {:#?}", split_description);
                                entry = format!(
                                    "{}({})-{}",
                                    split_description[0], commit_scope, split_description[1]
                                );
                            }
                        }
                    }
                }
            }
        }
        debug!("After checking scope `{}`", entry);

        if self.commit_breaking {
            entry = format!("BREAKING: {}", entry);
        }

        if let Some(id) = self.pr_id {
            if self.pr_url.is_some() {
                entry = format!("{}(pr [#{}])", entry, id);
            } else {
                entry = format!("{}(pr #{})", entry, id);
            }

            debug!("After checking pr id `{}`", entry);
        };

        debug!("Final entry `{}`", entry);
        self.section = Some(section);
        self.entry = entry;
    }

    fn section(&self) -> ChangeKind {
        match &self.section {
            Some(kind) => kind.clone(),
            None => ChangeKind::Changed,
        }
    }

    fn entry(&self) -> String {
        if self.entry.as_str() == "" {
            self.title.clone()
        } else {
            self.entry.clone()
        }
    }

    pub fn update_changelog(
        &mut self,
        log_file: &OsStr,
    ) -> Result<Option<(ChangeKind, String)>, Error> {
        let Some(log_file) = log_file.to_str() else {
            return Err(Error::InvalidPath(log_file.to_owned()));
        };

        let repo_url = match &self.pr_url {
            Some(pr_url) => {
                let url_string = pr_url.to_string();
                let components = url_string.split('/').collect::<Vec<&str>>();
                let url = format!("https://github.com/{}/{}", components[3], components[4]);
                Some(url)
            }
            None => None,
        };

        self.calculate_section_and_entry();

        log::trace!("Changelog entry:\n\n---\n{}\n---\n\n", self.entry());

        let mut change_log = if path::Path::new(log_file).exists() {
            let file_contents = fs::read_to_string(path::Path::new(log_file))?;
            log::trace!("file contents:\n---\n{}\n---\n\n", file_contents);
            if file_contents.contains(&self.entry) {
                log::trace!("The changelog exists and already contains the entry!");
                return Ok(None);
            } else {
                log::trace!("The changelog exists but does not contain the entry!");
            }
            let options = if repo_url.is_some() {
                Some(ChangelogParseOptions {
                    url: repo_url.clone(),
                    ..Default::default()
                })
            } else {
                None
            };

            Changelog::parse_from_file(log_file, options)
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
                .save_to_file(log_file)
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

        match self.section() {
            ChangeKind::Added => {
                unreleased.added(self.entry());
            }
            ChangeKind::Fixed => {
                unreleased.fixed(self.entry());
            }
            ChangeKind::Security => {
                unreleased.security(self.entry());
            }
            ChangeKind::Removed => {
                unreleased.removed(self.entry());
            }
            ChangeKind::Deprecated => {
                unreleased.deprecated(self.entry());
            }
            ChangeKind::Changed => {
                unreleased.changed(self.entry());
            }
        }

        // add link to the url if it exists
        if self.pr_url.is_some() {
            change_log.add_link(
                &format!("[#{}]:", self.pr_id.unwrap()),
                &self.pr_url.clone().unwrap().to_string(),
            ); // TODO: Add the PR link to the changelog.
        }

        change_log
            .save_to_file(log_file)
            .map_err(|e| Error::KeepAChangelog(e.to_string()))?;

        Ok(Some((self.section(), self.entry())))
    }
}

//test module
#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
    };

    use super::*;
    use log::LevelFilter;
    use log4rs_test_utils::test_logging;
    use rstest::rstest;
    use uuid::Uuid;

    #[test]
    fn test_pr_title_parse() {
        let pr_title = PrTitle::parse("feat: add new feature").unwrap();

        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, None);
        assert!(!pr_title.commit_breaking);

        let pr_title = PrTitle::parse("feat(core): add new feature").unwrap();
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, Some("core".to_string()));
        assert!(!pr_title.commit_breaking);

        let pr_title = PrTitle::parse("feat(core)!: add new feature").unwrap();
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, Some("core".to_string()));
        assert!(pr_title.commit_breaking);
    }

    #[test]
    fn test_pr_title_parse_with_breaking_scope() {
        let pr_title = PrTitle::parse("feat(core)!: add new feature").unwrap();
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, Some("core".to_string()));
        assert!(pr_title.commit_breaking);
    }

    #[test]
    fn test_pr_title_parse_with_security_scope() {
        let pr_title = PrTitle::parse("fix(security): fix security vulnerability").unwrap();
        assert_eq!(pr_title.title, "fix security vulnerability");
        assert_eq!(pr_title.commit_type, Some("fix".to_string()));
        assert_eq!(pr_title.commit_scope, Some("security".to_string()));
        assert!(!pr_title.commit_breaking);
    }

    #[test]
    fn test_pr_title_parse_with_deprecate_scope() {
        let pr_title = PrTitle::parse("chore(deprecate): deprecate old feature").unwrap();
        assert_eq!(pr_title.title, "deprecate old feature");
        assert_eq!(pr_title.commit_type, Some("chore".to_string()));
        assert_eq!(pr_title.commit_scope, Some("deprecate".to_string()));
        assert!(!pr_title.commit_breaking);
    }

    #[test]
    fn test_pr_title_parse_without_scope() {
        let pr_title = PrTitle::parse("docs: update documentation").unwrap();
        assert_eq!(pr_title.title, "update documentation");
        assert_eq!(pr_title.commit_type, Some("docs".to_string()));
        assert_eq!(pr_title.commit_scope, None);
        assert!(!pr_title.commit_breaking);
    }

    #[test]
    fn test_pr_title_parse_issue_172() {
        let pr_title = PrTitle::parse(
            "chore(config.yml): update jerus-org/circleci-toolkit orb version to 0.4.0",
        )
        .unwrap();
        assert_eq!(
            pr_title.title,
            "update jerus-org/circleci-toolkit orb version to 0.4.0"
        );
        assert_eq!(pr_title.commit_type, Some("chore".to_string()));
        assert_eq!(pr_title.commit_scope, Some("config.yml".to_string()));
        assert!(!pr_title.commit_breaking);
    }

    #[rstest]
    #[case(
        "feat: add new feature",
        Some(5),
        Some("https://github.com/jerus-org/pcu/pull/5"),
        ChangeKind::Added,
        "add new feature(pr [#5])"
    )]
    #[case(
        "feat: add new feature",
        Some(5),
        None,
        ChangeKind::Added,
        "add new feature(pr #5)"
    )]
    #[case(
        "feat: add new feature",
        None,
        Some("https://github.com/jerus-org/pcu/pull/5"),
        ChangeKind::Added,
        "add new feature"
    )]
    #[case(
        "feat: add new feature",
        None,
        None,
        ChangeKind::Added,
        "add new feature"
    )]
    #[case(
        "fix: fix an existing feature",
        None,
        None,
        ChangeKind::Fixed,
        "fix an existing feature"
    )]
    #[case(
        "test: update tests",
        None,
        None,
        ChangeKind::Changed,
        "test-update tests"
    )]
    #[case(
        "fix(security): Fix security vulnerability",
        None,
        None,
        ChangeKind::Fixed,
        "security: Fix security vulnerability"
    )]
    #[case(
        "chore(deps): Update dependencies",
        None,
        None,
        ChangeKind::Security,
        "Dependencies: Update dependencies"
    )]
    #[case(
        "refactor(remove): Remove unused code",
        None,
        None,
        ChangeKind::Removed,
        "Removed: Remove unused code"
    )]
    #[case(
        "docs(deprecate): Deprecate old API",
        None,
        None,
        ChangeKind::Deprecated,
        "Deprecated: Deprecate old API"
    )]
    #[case(
        "ci(other-scope): Update CI configuration",
        None,
        None,
        ChangeKind::Changed,
        "ci(other-scope)-Update CI configuration"
    )]
    #[case(
        "test!: Update test cases",
        None,
        None,
        ChangeKind::Changed,
        "BREAKING: test-Update test cases"
    )]
    #[case::issue_172(
        "chore(config.yml): update jerus-org/circleci-toolkit orb version to 0.4.0",
        Some(6),
        Some("https://github.com/jerus-org/pcu/pull/6"),
        ChangeKind::Changed,
        "chore(config.yml)-update jerus-org/circleci-toolkit orb version to 0.4.0(pr [#6])"
    )]
    fn test_calculate_kind_and_description(
        #[case] title: &str,
        #[case] pr_id: Option<u64>,
        #[case] pr_url: Option<&str>,
        #[case] expected_kind: ChangeKind,
        #[case] expected_desciption: &str,
    ) -> Result<()> {
        test_logging::init_logging_once_for(vec![], LevelFilter::Debug, None);

        let mut pr_title = PrTitle::parse(title).unwrap();
        if let Some(id) = pr_id {
            pr_title.set_pr_id(id);
        }
        if let Some(url) = pr_url {
            let url = Url::parse(url)?;
            pr_title.set_pr_url(url);
        }
        pr_title.calculate_section_and_entry();
        assert_eq!(expected_kind, pr_title.section());
        assert_eq!(expected_desciption, pr_title.entry);

        Ok(())
    }

    use color_eyre::Result;

    #[rstest]
    fn test_update_change_log_added() -> Result<()> {
        test_logging::init_logging_once_for(vec![], LevelFilter::Debug, None);

        let initial_content = fs::read_to_string("tests/data/initial_changelog.md")?;
        let expected_content = fs::read_to_string("tests/data/expected_changelog.md")?;

        let temp_dir_string = format!("tests/tmp/test-{}", Uuid::new_v4());
        let temp_dir = Path::new(&temp_dir_string);
        fs::create_dir_all(temp_dir)?;

        let file_name = temp_dir.join("CHANGELOG.md");
        debug!("filename : {:?}", file_name);

        let mut file = File::create(&file_name)?;
        file.write_all(initial_content.as_bytes())?;

        let mut pr_title = PrTitle {
            title: "add new feature".to_string(),
            pr_id: Some(5),
            pr_url: Some(Url::parse("https://github.com/jerus-org/pcu/pull/5")?),
            commit_type: Some("feat".to_string()),
            commit_scope: None,
            commit_breaking: false,
            section: Some(ChangeKind::Added),
            entry: "add new feature".to_string(),
        };

        let file_name = &file_name.into_os_string();
        pr_title.update_changelog(file_name)?;

        let actual_content = fs::read_to_string(file_name)?;

        assert_eq!(actual_content, expected_content);

        // tidy up the test environment
        std::fs::remove_dir_all(temp_dir)?;

        Ok(())
    }

    #[rstest]
    fn test_update_change_log_added_issue_172() -> Result<()> {
        test_logging::init_logging_once_for(vec![], LevelFilter::Debug, None);

        let initial_content = fs::read_to_string("tests/data/initial_changelog.md")?;
        let expected_content = fs::read_to_string("tests/data/expected_changelog_issue_172.md")?;

        let temp_dir_string = format!("tests/tmp/test-{}", Uuid::new_v4());
        let temp_dir = Path::new(&temp_dir_string);
        fs::create_dir_all(temp_dir)?;

        let file_name = temp_dir.join("CHANGELOG.md");
        debug!("filename : {:?}", file_name);

        let mut file = File::create(&file_name)?;
        file.write_all(initial_content.as_bytes())?;

        let mut pr_title = PrTitle {
            title: "add new feature".to_string(),
            pr_id: Some(5),
            pr_url: Some(Url::parse("https://github.com/jerus-org/pcu/pull/5")?),
            commit_type: Some("feat".to_string()),
            commit_scope: None,
            commit_breaking: false,
            section: Some(ChangeKind::Added),
            entry: "add new feature".to_string(),
        };

        let file_name = &file_name.into_os_string();
        pr_title.update_changelog(file_name)?;

        let mut pr_title = PrTitle::parse(
            "chore(config.yml): update jerus-org/circleci-toolkit orb version to 0.4.0",
        )?;
        pr_title.set_pr_id(6);
        pr_title.set_pr_url(Url::parse("https://github.com/jerus-org/pcu/pull/6")?);
        pr_title.calculate_section_and_entry();

        let file_name = &file_name.to_os_string();
        pr_title.update_changelog(file_name)?;

        let actual_content = fs::read_to_string(file_name)?;

        assert_eq!(actual_content, expected_content);

        // tidy up the test environment
        std::fs::remove_dir_all(temp_dir)?;

        Ok(())
    }
}
