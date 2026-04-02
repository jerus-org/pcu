mod site_config;

use std::path::PathBuf;

use clap::Parser;
use config::Config;
use gen_bsky::{Draft, DraftError};
use site_config::SiteConfig;

use crate::{CIExit, Client, Error, GitOps, SignConfig};
use std::env;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::bsky::Cmd;
    use crate::Cli;
    use clap::Parser;

    /// RED: CmdDraft must have an `allow_empty` field (issue #813)
    #[test]
    fn test_cmd_draft_has_allow_empty_field() {
        let cmd = CmdDraft {
            filter: None,
            paths: vec![],
            date: None,
            allow_draft: false,
            allow_empty: false,
            push: false,
            www_src_root: PathBuf::from("."),
        };
        assert!(!cmd.allow_empty);
    }

    /// RED: CIExit must have a NoBlogPostsForBluesky variant (issue #813)
    #[test]
    fn test_no_blog_posts_for_bluesky_ci_exit() {
        let exit = CIExit::NoBlogPostsForBluesky;
        // Pattern match to prove the variant exists
        assert!(matches!(exit, CIExit::NoBlogPostsForBluesky));
    }

    /// RED: CmdDraft must have a `push` field (issue #899)
    #[test]
    fn test_cmd_draft_has_push_field() {
        let cmd = CmdDraft {
            filter: None,
            paths: vec![],
            date: None,
            allow_draft: false,
            allow_empty: false,
            push: false,
            www_src_root: PathBuf::from("."),
        };
        assert!(!cmd.push);
    }

    /// RED: `pcu bsky draft --push` must parse the flag (issue #899)
    #[test]
    fn test_bsky_draft_parses_push_flag() {
        let args = Cli::try_parse_from(["pcu", "bsky", "draft", "--push"]).unwrap();
        match args.command {
            crate::Commands::Bsky(bsky) => match bsky.cmd {
                Cmd::Draft(draft) => assert!(draft.push),
                _ => panic!("expected Draft subcommand"),
            },
            _ => panic!("expected Bsky command"),
        }
    }

    /// RED: `pcu bsky draft` without `--push` defaults push to false (issue #899)
    #[test]
    fn test_bsky_draft_push_defaults_false() {
        let args = Cli::try_parse_from(["pcu", "bsky", "draft"]).unwrap();
        match args.command {
            crate::Commands::Bsky(bsky) => match bsky.cmd {
                Cmd::Draft(draft) => assert!(!draft.push),
                _ => panic!("expected Draft subcommand"),
            },
            _ => panic!("expected Bsky command"),
        }
    }

    // GREEN: behaviour tests for resolve_post_draft_exit

    /// push=true, commits ahead → Pushed
    #[test]
    fn test_resolve_post_draft_exit_push_with_commits() {
        let exit = resolve_post_draft_exit(true, 1);
        assert!(
            matches!(exit, CIExit::Pushed(_)),
            "expected Pushed, got {exit:?}"
        );
    }

    /// push=true, nothing ahead → DraftedForBluesky (commit already on remote or nothing committed)
    #[test]
    fn test_resolve_post_draft_exit_push_no_commits() {
        let exit = resolve_post_draft_exit(true, 0);
        assert!(
            matches!(exit, CIExit::DraftedForBluesky),
            "expected DraftedForBluesky, got {exit:?}"
        );
    }

    /// push=false → DraftedForBluesky regardless of ahead count
    #[test]
    fn test_resolve_post_draft_exit_no_push() {
        let exit = resolve_post_draft_exit(false, 5);
        assert!(
            matches!(exit, CIExit::DraftedForBluesky),
            "expected DraftedForBluesky, got {exit:?}"
        );
    }
}

const DEFAULT_PATH: &str = "content/blog";

#[derive(Debug, Parser, Clone)]
pub struct CmdDraft {
    /// filter for files containing blog posts to broadcast on Bluesky
    #[arg(short, long)]
    pub filter: Option<String>,
    /// Optional path to file or directory of blog post(s) to process
    pub paths: Vec<PathBuf>,
    /// Optional date from which to process blog post(s)
    /// Date format: YYYY-MM-DD
    /// Default: Current date
    #[arg(short, long)]
    pub date: Option<toml::value::Datetime>,
    /// Allow bluesky posts for draft blog posts
    #[arg(long, default_value_t = false)]
    pub allow_draft: bool,
    /// Warn instead of failing when no blog posts match the date filter
    #[arg(long, default_value_t = false)]
    pub allow_empty: bool,
    /// Push committed drafts to the remote repository
    #[arg(long, default_value_t = false)]
    pub push: bool,
    /// Root folder for the website source
    #[arg(short, long, default_value = ".")]
    pub www_src_root: PathBuf,
}

impl CmdDraft {
    pub async fn run(&mut self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
        // find the potential file in the git repo

        let base_url = SiteConfig::new(&self.www_src_root, None)?.base_url();
        let store = &settings.get_string("store")?;
        if self.paths.is_empty() {
            self.paths.push(PathBuf::from(DEFAULT_PATH))
        };

        log::trace!(
            "Key parameters:\n\tbase:\t`{base_url}`\n\tstore:\t`{store}`\n\tpath:\t`{:?}`\n\troot:\t`{}`",
            self.paths,
            self.www_src_root.display(),
        );

        let mut builder = Draft::builder(base_url, Some(&self.www_src_root));

        // Add the paths specified at the command line.
        for path in self.paths.iter() {
            builder.add_path_or_file(path)?;
        }

        if let Some(d) = self.date {
            builder.with_minimum_date(d.to_string().as_str())?;
        }

        builder.with_allow_draft(self.allow_draft);

        let mut posts = match builder.build().await {
            Ok(p) => p,
            Err(DraftError::BlogPostListEmpty) if self.allow_empty => {
                log::warn!("No blog posts found matching the filter — skipping (--allow-empty)");
                return Ok(CIExit::NoBlogPostsForBluesky);
            }
            Err(e) => return Err(Error::DraftError(e)),
        };
        log::info!("Initial posts: {posts:#?}");

        posts
            .write_referrers(None)?
            .write_bluesky_posts(None)
            .await?;
        log::info!("Referrers written: {posts:#?}");
        log::info!("Bluesky posts written: {posts:#?}");

        let sign_config = SignConfig::default();
        // Commit the posts to the git repo
        let commit_message = "chore: add drafted bluesky posts to git repo";
        client
            .commit_changed_files(sign_config, commit_message, "", None)
            .await?;

        let ahead = client.branch_status()?.ahead;
        if self.push && ahead > 0 {
            let bot_user_name = env::var("BOT_USER_NAME").unwrap_or_else(|_| "bot".to_string());
            client.push_commit("v", None, false, &bot_user_name)?;
        }

        Ok(resolve_post_draft_exit(self.push, ahead))
    }
}

fn resolve_post_draft_exit(push_requested: bool, commits_ahead: usize) -> CIExit {
    if push_requested && commits_ahead > 0 {
        CIExit::Pushed("Bluesky drafts committed and pushed to remote repository.".to_string())
    } else {
        CIExit::DraftedForBluesky
    }
}
