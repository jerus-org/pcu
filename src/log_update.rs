// #[derive(Debug)]
// struct ChangeRecord {
//     heading: String,
//     description: String,
// }

// impl ChangeRecord {
//     fn new(heading: &str, description: &str) -> Self {
//         // <1>
//         Self {
//             heading: heading.to_string(),
//             description: description.to_string(),
//         }
//     }
// }

#[derive(Debug)]
struct PrTitle {
    title: String,
    commit_type: Option<String>,
    commit_scope: Option<String>,
    commit_breaking: bool,
}

impl PrTitle {
    fn parse_title(title: &str) -> Self {
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
        let pr_title = PrTitle::parse_title("feat: add new feature");
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, None);
        assert_eq!(pr_title.commit_breaking, false);

        let pr_title = PrTitle::parse_title("feat(core): add new feature");
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, Some("core".to_string()));
        assert_eq!(pr_title.commit_breaking, false);

        let pr_title = PrTitle::parse_title("feat(core)!: add new feature");
        assert_eq!(pr_title.title, "add new feature");
        assert_eq!(pr_title.commit_type, Some("feat".to_string()));
        assert_eq!(pr_title.commit_scope, Some("core".to_string()));
        assert_eq!(pr_title.commit_breaking, true);
    }
}
