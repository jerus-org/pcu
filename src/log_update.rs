use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
enum Heading {
    Added,
    Changed,
    Deprecated,
    Remnoved,
    Fixed,
    Security,
}

impl Display for Heading {
    // <1>
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Heading::Added => write!(f, "Added"),
            Heading::Changed => write!(f, "Changed"),
            Heading::Deprecated => write!(f, "Deprecated"),
            Heading::Remnoved => write!(f, "Remnoved"),
            Heading::Fixed => write!(f, "Fixed"),
            Heading::Security => write!(f, "Security"),
        }
    }
}

impl From<&str> for Heading {
    // <2>
    fn from(s: &str) -> Self {
        match s {
            "feat" => Heading::Added,
            "fix" => Heading::Fixed,
            "docs" => Heading::Changed,
            "style" => Heading::Changed,
            "refactor" => Heading::Changed,
            "perf" => Heading::Changed,
            "test" => Heading::Changed,
            "chore" => Heading::Changed,
            "revert" => Heading::Remnoved,
            "build" => Heading::Changed,
            "ci" => Heading::Changed,
            "breaking" => Heading::Changed,
            "security" => Heading::Security,
            "deprecate" => Heading::Deprecated,
            "remove" => Heading::Remnoved,
            _ => Heading::Changed,
        }
    }
}

#[derive(Debug)]
pub struct ChangeRecord {
    heading: Heading,
    description: String,
}

impl ChangeRecord {
    fn new(heading: &str, description: &str) -> Self {
        // <1>
        let heading = Heading::from(heading);
        Self {
            heading,
            description: description.to_string(),
        }
    }
}

impl From<&PrTitle> for ChangeRecord {
    fn from(pr_title: &PrTitle) -> Self {
        // <2>
        let mut heading_str = pr_title
            .commit_type
            .as_ref()
            .unwrap_or(&"".to_string())
            .to_string();
        if pr_title.commit_scope.is_some() {
            match pr_title.commit_scope.as_ref().unwrap().as_str() {
                "deps" => heading_str = "security".to_string(),
                "security" => heading_str = "security".to_string(),
                "deprecate" => heading_str = "deprecate".to_string(),
                _ => (),
            }
        }

        if pr_title.commit_breaking {
            heading_str = "breaking:".to_string()
        }

        Self::new(&heading_str, &pr_title.title)
    }
}

#[derive(Debug)]
pub struct PrTitle {
    title: String,
    commit_type: Option<String>,
    commit_scope: Option<String>,
    commit_breaking: bool,
}

impl PrTitle {
    pub fn parse(title: &str) -> Self {
        let re = regex::Regex::new(
            r"^(?P<type>[a-z]+)(?:\((?P<scope>[a-z]+)\))?(?P<breaking>!)?: (?P<description>.*)$",
        )
        .unwrap();

        if let Some(captures) = re.captures(title) {
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
            }
        } else {
            Self {
                title: title.to_string(),
                commit_type: None,
                commit_scope: None,
                commit_breaking: false,
            }
        }
    }
}

//test module
#[cfg(test)]
mod tests {
    use super::*;

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
}
