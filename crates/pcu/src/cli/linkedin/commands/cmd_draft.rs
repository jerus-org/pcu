use std::path::PathBuf;

use clap::Parser;
use config::Config;
use gen_linkedin::{Draft, DraftError};

use crate::{CIExit, Client, Error, GitOps, SignConfig};
use std::env;

const DEFAULT_PATH: &str = "content/blog";
const DEFAULT_STORE: &str = "linkedin";

#[derive(Debug, Parser, Clone)]
pub struct CmdDraft {
    /// Optional path to file or directory of blog post(s) to process.
    pub paths: Vec<PathBuf>,
    /// Optional minimum date filter (YYYY-MM-DD). Posts with a `date` before this are skipped.
    #[arg(short, long)]
    pub date: Option<toml::value::Datetime>,
    /// Warn instead of failing when no blog posts match the filter.
    #[arg(long, default_value_t = false)]
    pub allow_empty: bool,
    /// Push committed drafts to the remote repository.
    #[arg(long, default_value_t = false)]
    pub push: bool,
    /// Root folder for the website source (default: `.`).
    #[arg(short, long, default_value = ".")]
    pub www_src_root: PathBuf,
    /// Directory to store the LinkedIn draft files (default: `linkedin`).
    #[arg(short, long, default_value = DEFAULT_STORE)]
    pub store: String,
}

impl CmdDraft {
    pub async fn run(&mut self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        let base_url = settings
            .get_string("base_url")
            .unwrap_or_else(|_| String::new());
        let store = settings
            .get_string("linkedin_store")
            .unwrap_or_else(|_| self.store.clone());

        if self.paths.is_empty() {
            self.paths.push(PathBuf::from(DEFAULT_PATH));
        }

        log::trace!(
            "linkedin draft: base_url={base_url:?} store={store:?} paths={:?} root={}",
            self.paths,
            self.www_src_root.display(),
        );

        let mut draft = match Draft::new(&self.www_src_root, &self.paths, &base_url, self.date) {
            Ok(d) => d,
            Err(DraftError::BlogPostListEmpty) if self.allow_empty => {
                log::warn!("No blog posts found matching the filter — skipping (--allow-empty)");
                return Ok(CIExit::NoBlogPostsForLinkedIn);
            }
            Err(e) => return Err(Error::LinkedinDraftError(e)),
        };

        let store_dir = self.www_src_root.join(&store);
        draft.write_linkedin_drafts(&store_dir)?;

        log::info!("LinkedIn drafts written: {}", draft.count_written());

        let sign_config = SignConfig::default();
        let commit_message = "chore: add drafted linkedin posts to git repo";
        client
            .commit_changed_files(sign_config, commit_message, "", None)
            .await?;

        let ahead = client.branch_status()?.ahead;
        if self.push && ahead > 0 {
            let bot_user_name = env::var("BOT_USER_NAME").unwrap_or_else(|_| "bot".to_string());
            client.push_commit("v", None, false, &bot_user_name)?;
        }

        Ok(resolve_draft_exit(self.push, ahead))
    }
}

fn resolve_draft_exit(push_requested: bool, commits_ahead: usize) -> CIExit {
    if push_requested && commits_ahead > 0 {
        CIExit::Pushed("LinkedIn drafts committed and pushed to remote repository.".to_string())
    } else {
        CIExit::DraftedForLinkedIn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::linkedin::Cmd;
    use crate::Cli;
    use clap::Parser;

    #[test]
    fn test_linkedin_draft_parses_push_flag() {
        let args = Cli::try_parse_from(["pcu", "linkedin", "draft", "--push"]).unwrap();
        match args.command {
            crate::Commands::Linkedin(li) => match li.cmd {
                Cmd::Draft(draft) => assert!(draft.push),
                _ => panic!("expected Draft"),
            },
            _ => panic!("expected Linkedin"),
        }
    }

    #[test]
    fn test_linkedin_draft_push_defaults_false() {
        let args = Cli::try_parse_from(["pcu", "linkedin", "draft"]).unwrap();
        match args.command {
            crate::Commands::Linkedin(li) => match li.cmd {
                Cmd::Draft(draft) => assert!(!draft.push),
                _ => panic!("expected Draft"),
            },
            _ => panic!("expected Linkedin"),
        }
    }

    #[test]
    fn test_linkedin_draft_allow_empty_defaults_false() {
        let args = Cli::try_parse_from(["pcu", "linkedin", "draft"]).unwrap();
        match args.command {
            crate::Commands::Linkedin(li) => match li.cmd {
                Cmd::Draft(draft) => assert!(!draft.allow_empty),
                _ => panic!("expected Draft"),
            },
            _ => panic!("expected Linkedin"),
        }
    }

    #[test]
    fn test_resolve_draft_exit_push_with_commits() {
        let exit = resolve_draft_exit(true, 1);
        assert!(matches!(exit, CIExit::Pushed(_)));
    }

    #[test]
    fn test_resolve_draft_exit_push_no_commits() {
        let exit = resolve_draft_exit(true, 0);
        assert!(matches!(exit, CIExit::DraftedForLinkedIn));
    }

    #[test]
    fn test_resolve_draft_exit_no_push() {
        let exit = resolve_draft_exit(false, 5);
        assert!(matches!(exit, CIExit::DraftedForLinkedIn));
    }

    #[test]
    fn test_no_blog_posts_for_linkedin_ci_exit() {
        assert!(matches!(
            CIExit::NoBlogPostsForLinkedIn,
            CIExit::NoBlogPostsForLinkedIn
        ));
    }
}
