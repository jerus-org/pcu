use std::path;

use keep_a_changelog::{changelog::ChangelogBuilder, ChangeKind, Changelog, Release};
use log::debug;
use url::Url;

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
    pub fn parse(title: &str) -> Self {
        let re = regex::Regex::new(
            r"^(?P<type>[a-z]+)(?:\((?P<scope>.+)\))?(?P<breaking>!)?: (?P<description>.*)$",
        )
        .unwrap();

        debug!("String to parse: `{}`", title);

        let pr_title = if let Some(captures) = re.captures(title) {
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

        pr_title
    }

    pub fn set_pr_id(&mut self, id: u64) {
        self.pr_id = Some(id);
    }

    pub fn set_pr_url(&mut self, url: Url) {
        self.pr_url = Some(url);
    }

    fn calculate_kind_and_description(&mut self) {
        let mut kind = ChangeKind::Changed;
        let mut description = self.title.clone();

        debug!("Initial description `{}`", description);

        if let Some(commit_type) = &self.commit_type {
            match commit_type.as_str() {
                "feat" => kind = ChangeKind::Added,
                "fix" => kind = ChangeKind::Fixed,
                _ => {
                    kind = ChangeKind::Changed;
                    description = format!("{}-{}", self.commit_type.as_ref().unwrap(), description);
                }
            }
        }

        debug!("After checking type `{}`", description);

        if let Some(commit_scope) = &self.commit_scope {
            match commit_scope.as_str() {
                "security" => {
                    kind = ChangeKind::Security;
                    description = format!("Security: {}", self.title);
                }
                "deps" => {
                    kind = ChangeKind::Security;
                    description = format!("Dependencies: {}", self.title);
                }
                "remove" => {
                    kind = ChangeKind::Removed;
                    description = format!("Removed: {}", self.title);
                }
                "deprecate" => {
                    kind = ChangeKind::Deprecated;
                    description = format!("Deprecated: {}", self.title);
                }
                _ => {
                    kind = ChangeKind::Changed;
                    let split_description = description.splitn(2, '-').collect::<Vec<&str>>();
                    description = format!(
                        "{}({})-{}",
                        split_description[0],
                        self.commit_scope.as_ref().unwrap(),
                        split_description[1]
                    );
                }
            }
        }

        debug!("After checking scope `{}`", description);

        if self.commit_breaking {
            description = format!("BREAKING: {}", description);
        }

        if let Some(id) = self.pr_id {
            description = format!("{}(pr #{})", description, id);

            debug!("After checking pr id `{}`", description);

            if let Some(url) = &self.pr_url {
                let split_description = description.splitn(2, '(').collect::<Vec<&str>>();
                description = format!("{}(pr [#{}]({}))", split_description[0], id, url);
            }
        };

        self.section = Some(kind);
        self.entry = description;
    }

    fn kind(&self) -> ChangeKind {
        match &self.section {
            Some(kind) => kind.clone(),
            None => ChangeKind::Changed,
        }
    }

    fn description(&self) -> String {
        if self.entry.as_str() == "" {
            self.title.clone()
        } else {
            self.entry.clone()
        }
    }

    pub fn update_change_log(&mut self, log_file: &str) {
        self.calculate_kind_and_description();

        let mut change_log = if path::Path::new(log_file).exists() {
            println!("The changelog exists!");
            Changelog::parse_from_file(log_file, None).unwrap()
        } else {
            println!("The changelog does not exist!");
            let mut changelog = ChangelogBuilder::default().build().unwrap();
            log::debug!("Changelog: {:#?}", changelog);
            let release = Release::builder().build().unwrap();
            changelog.add_release(release);
            log::debug!("Changelog: {:#?}", changelog);

            changelog.save_to_file(log_file).unwrap();
            changelog
        };

        //          Changelog::parse_from_file(log_file, None).unwrap();

        let unreleased = change_log.get_unreleased_mut().unwrap();

        match self.kind() {
            ChangeKind::Added => {
                unreleased.added(self.description());
            }
            ChangeKind::Fixed => {
                unreleased.fixed(self.description());
            }
            ChangeKind::Security => {
                unreleased.security(self.description());
            }
            ChangeKind::Removed => {
                unreleased.removed(self.description());
            }
            ChangeKind::Deprecated => {
                unreleased.deprecated(self.description());
            }
            ChangeKind::Changed => {
                unreleased.changed(self.description());
            }
        }

        change_log.save_to_file(log_file).unwrap();
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
        let pr_title = PrTitle::parse("feat: add new feature");
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, None);
        assert!(!pr_title.commit_breaking);

        let pr_title = PrTitle::parse("feat(core): add new feature");
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, Some("core".to_string()));
        assert!(!pr_title.commit_breaking);

        let pr_title = PrTitle::parse("feat(core)!: add new feature");
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, Some("core".to_string()));
        assert!(pr_title.commit_breaking);
    }

    #[test]
    fn test_pr_title_parse_with_breaking_scope() {
        let pr_title = PrTitle::parse("feat(core)!: add new feature");
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, Some("core".to_string()));
        assert!(pr_title.commit_breaking);
    }

    #[test]
    fn test_pr_title_parse_with_security_scope() {
        let pr_title = PrTitle::parse("fix(security): fix security vulnerability");
        assert_eq!(pr_title.title, "fix security vulnerability");
        assert_eq!(pr_title.commit_type, Some("fix".to_string()));
        assert_eq!(pr_title.commit_scope, Some("security".to_string()));
        assert!(!pr_title.commit_breaking);
    }

    #[test]
    fn test_pr_title_parse_with_deprecate_scope() {
        let pr_title = PrTitle::parse("chore(deprecate): deprecate old feature");
        assert_eq!(pr_title.title, "deprecate old feature");
        assert_eq!(pr_title.commit_type, Some("chore".to_string()));
        assert_eq!(pr_title.commit_scope, Some("deprecate".to_string()));
        assert!(!pr_title.commit_breaking);
    }

    #[test]
    fn test_pr_title_parse_without_scope() {
        let pr_title = PrTitle::parse("docs: update documentation");
        assert_eq!(pr_title.title, "update documentation");
        assert_eq!(pr_title.commit_type, Some("docs".to_string()));
        assert_eq!(pr_title.commit_scope, None);
        assert!(!pr_title.commit_breaking);
    }

    #[rstest]
    #[case(
        "feat: add new feature",
        Some(5),
        Some("https://github.com/jerus-org/pcu/pull/5"),
        ChangeKind::Added,
        "add new feature(pr [#5](https://github.com/jerus-org/pcu/pull/5))"
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
        ChangeKind::Security,
        "Security: Fix security vulnerability"
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
    fn test_calculate_kind_and_description(
        #[case] title: &str,
        #[case] pr_id: Option<u64>,
        #[case] pr_url: Option<&str>,
        #[case] expected_kind: ChangeKind,
        #[case] expected_desciption: &str,
    ) -> Result<()> {
        test_logging::init_logging_once_for(vec![], LevelFilter::Debug, None);

        let mut pr_title = PrTitle::parse(title);
        if let Some(id) = pr_id {
            pr_title.set_pr_id(id);
        }
        if let Some(url) = pr_url {
            let url = Url::parse(url)?;
            pr_title.set_pr_url(url);
        }
        pr_title.calculate_kind_and_description();
        assert_eq!(expected_kind, pr_title.kind());
        assert_eq!(expected_desciption, pr_title.entry);

        Ok(())
    }

    use eyre::Result;

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

        let mut file = File::create(&file_name).unwrap();
        file.write_all(initial_content.as_bytes()).unwrap();

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

        pr_title.update_change_log(file_name.to_str().unwrap());

        let actual_content = fs::read_to_string(&file_name).unwrap();

        assert_eq!(actual_content, expected_content);

        // tidy up the test environment
        std::fs::remove_dir_all(temp_dir)?;

        Ok(())
    }
}
