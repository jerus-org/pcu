use std::fmt::{self, Display, Formatter};

use crate::PrTitle;

#[derive(Debug, PartialEq)]
enum Heading {
    Added,
    Changed,
    Deprecated,
    Removed,
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
            Heading::Removed => write!(f, "Removed"),
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
            "revert" => Heading::Changed,
            "build" => Heading::Changed,
            "ci" => Heading::Changed,
            "breaking" => Heading::Changed,
            "security" => Heading::Security,
            "deprecate" => Heading::Deprecated,
            "remove" => Heading::Removed,
            _ => Heading::Changed,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ChangeFrag {
    heading: Heading,
    description: String,
}

impl ChangeFrag {
    fn new(heading: &str, description: &str) -> Self {
        // <1>
        let heading = Heading::from(heading);
        Self {
            heading,
            description: description.to_string(),
        }
    }
}

impl From<&PrTitle> for ChangeFrag {
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
                "remove" => heading_str = "remove".to_string(),
                _ => (),
            }
        }

        let mut description = pr_title.title.to_string();
        if pr_title.commit_breaking {
            heading_str = "breaking:".to_string();
            description = format!("BREAKING: {}", description);
        }

        println!(
            "heading_str: {} when pr_title is {:#?}",
            heading_str, pr_title
        );

        Self::new(&heading_str, &description)
    }
}

//test module
#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_heading_display() {
        let heading = Heading::Added;
        assert_eq!(format!("{}", heading), "Added");

        let heading = Heading::Changed;
        assert_eq!(format!("{}", heading), "Changed");

        let heading = Heading::Deprecated;
        assert_eq!(format!("{}", heading), "Deprecated");

        let heading = Heading::Removed;
        assert_eq!(format!("{}", heading), "Removed");

        let heading = Heading::Fixed;
        assert_eq!(format!("{}", heading), "Fixed");

        let heading = Heading::Security;
        assert_eq!(format!("{}", heading), "Security");
    }

    #[rstest]
    #[case("feat", "add new feature", Heading::Added, "add new feature")]
    #[case(
        "fix",
        "fix an existing feature",
        Heading::Fixed,
        "fix an existing feature"
    )]
    #[case(
        "docs",
        "update documentation",
        Heading::Changed,
        "update documentation"
    )]
    #[case("style", "update style", Heading::Changed, "update style")]
    #[case("refactor", "update refactor", Heading::Changed, "update refactor")]
    #[case("perf", "update perf", Heading::Changed, "update perf")]
    #[case("test", "update test", Heading::Changed, "update test")]
    #[case("chore", "perform chore task", Heading::Changed, "perform chore task")]
    #[case("revert", "revert changes", Heading::Changed, "revert changes")]
    #[case("build", "update build", Heading::Changed, "update build")]
    #[case("ci", "update ci", Heading::Changed, "update ci")]
    #[case(
        "security",
        "fix security vulnerability",
        Heading::Security,
        "fix security vulnerability"
    )]
    #[case(
        "deprecate",
        "deprecate old feature",
        Heading::Deprecated,
        "deprecate old feature"
    )]
    #[case("remove", "remove old feature", Heading::Removed, "remove old feature")]
    #[case(
        "breaking",
        "BREAKING: remove old feature",
        Heading::Changed,
        "BREAKING: remove old feature"
    )]

    fn test_change_record_new(
        #[case] heading: &str,
        #[case] description: &str,
        #[case] expected_heading: Heading,
        #[case] expected_description: &str,
    ) {
        let change_record = ChangeFrag::new(heading, description);
        assert_eq!(expected_heading, change_record.heading);
        assert_eq!(expected_description, change_record.description);
    }

    #[rstest]
    #[case("feat: add new feature", Heading::Added, "add new feature")]
    #[case(
        "fix: fix and existing feature",
        Heading::Fixed,
        "fix and existing feature"
    )]
    #[case("docs: update to docs", Heading::Changed, "update to docs")]
    #[case("style: update to style", Heading::Changed, "update to style")]
    #[case("refactor: update to refactor", Heading::Changed, "update to refactor")]
    #[case("perf: update to perf", Heading::Changed, "update to perf")]
    #[case("test: update to test", Heading::Changed, "update to test")]
    #[case("chore: update to chore", Heading::Changed, "update to chore")]
    #[case("revert: update to revert", Heading::Changed, "update to revert")]
    #[case("build: update to build", Heading::Changed, "update to build")]
    #[case("ci: update to ci", Heading::Changed, "update to ci")]
    #[case("fix(deps): add new feature", Heading::Security, "add new feature")]
    #[case("fix(security): add new feature", Heading::Security, "add new feature")]
    #[case(
        "fix!: add breaking change",
        Heading::Changed,
        "BREAKING: add breaking change"
    )]
    #[case(
        "feat(deprecate): deprecate old feature",
        Heading::Deprecated,
        "deprecate old feature"
    )]
    #[case(
        "feat(remove): remove old feature",
        Heading::Removed,
        "remove old feature"
    )]
    #[case("feat: add new feature", Heading::Added, "add new feature")]
    #[case("feat: add new feature", Heading::Added, "add new feature")]
    #[case("feat: add new feature", Heading::Added, "add new feature")]
    #[case("feat: add new feature", Heading::Added, "add new feature")]
    fn test_change_record_from_pr_title(
        #[case] title: &str,
        #[case] expected_heading: Heading,
        #[case] expected_description: &str,
    ) {
        let pr_title = PrTitle::parse(title);
        let change_record = ChangeFrag::from(&pr_title);
        assert_eq!(expected_heading, change_record.heading);
        assert_eq!(expected_description, change_record.description);
    }
}
