use keep_a_changelog::ChangeKind;
use log::debug;

#[derive(Debug)]
pub struct PrTitle {
    pub title: String,
    pub commit_type: Option<String>,
    pub commit_scope: Option<String>,
    pub commit_breaking: bool,
    kind: Option<ChangeKind>,
    description: String,
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
                commit_type,
                commit_scope,
                commit_breaking,
                kind: None,
                description: String::new(),
            }
        } else {
            Self {
                title: title.to_string(),
                commit_type: None,
                commit_scope: None,
                commit_breaking: false,
                kind: None,
                description: String::new(),
            }
        };

        debug!("Parsed title: {:?}", pr_title);

        pr_title
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
                    description =
                        format!("{}({})", description, self.commit_scope.as_ref().unwrap());
                }
            }
        }

        debug!("After checking scope `{}`", description);

        if self.commit_breaking {
            description = format!("BREAKING: {}", description);
        }

        self.kind = Some(kind);
        self.description = description;
    }

    fn kind(&self) -> ChangeKind {
        match &self.kind {
            Some(kind) => kind.clone(),
            None => ChangeKind::Changed,
        }
    }

    fn description(&self) -> String {
        if self.description.as_str() == "" {
            self.title.clone()
        } else {
            self.description.clone()
        }
    }
}

//test module
#[cfg(test)]
mod tests {
    use super::*;
    use log::LevelFilter;
    use log4rs_test_utils::test_logging;
    use rstest::rstest;

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
    #[case("feat: add new feature", ChangeKind::Added, "add new feature")]
    #[case(
        "fix: fix an existing feature",
        ChangeKind::Fixed,
        "fix an existing feature"
    )]
    #[case("test: update tests", ChangeKind::Changed, "test-update tests")]
    #[case(
        "fix(security): Fix security vulnerability",
        ChangeKind::Security,
        "Security: Fix security vulnerability"
    )]
    #[case(
        "chore(deps): Update dependencies",
        ChangeKind::Security,
        "Dependencies: Update dependencies"
    )]
    #[case(
        "refactor(remove): Remove unused code",
        ChangeKind::Removed,
        "Removed: Remove unused code"
    )]
    #[case(
        "docs(deprecate): Deprecate old API",
        ChangeKind::Deprecated,
        "Deprecated: Deprecate old API"
    )]
    #[case(
        "ci(other-scope): Update CI configuration",
        ChangeKind::Changed,
        "ci-Update CI configuration(other-scope)"
    )]
    #[case(
        "test!: Update test cases",
        ChangeKind::Changed,
        "BREAKING: test-Update test cases"
    )]
    fn test_calculate_kind_and_description(
        #[case] title: &str,
        #[case] expected_kind: ChangeKind,
        #[case] expected_desciption: &str,
    ) {
        test_logging::init_logging_once_for(vec![], LevelFilter::Debug, None);

        let mut pr_title = PrTitle::parse(title);
        pr_title.calculate_kind_and_description();
        assert_eq!(expected_kind, pr_title.kind());
        assert_eq!(expected_desciption, pr_title.description);
    }
}
