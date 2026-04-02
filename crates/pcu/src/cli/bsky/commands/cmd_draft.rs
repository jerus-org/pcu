mod site_config;

use std::path::{Path, PathBuf};

use clap::Parser;
use config::Config;
use gen_bsky::{Draft, DraftError};
use git2::Delta;
use site_config::SiteConfig;

use crate::{CIExit, Client, Error, GitOps, SignConfig};
use std::env;

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
    /// Detect new/modified posts via git diff vs base branch instead of date filter.
    /// BASE defaults to origin/main if not specified.
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "origin/main",
        value_name = "BASE"
    )]
    pub from_branch: Option<String>,
    /// Root folder for the website source
    #[arg(short, long, default_value = ".")]
    pub www_src_root: PathBuf,
}

impl CmdDraft {
    pub async fn run(&mut self, client: &Client, settings: &Config) -> Result<CIExit, Error> {
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

        // If --from-branch is set, replace paths with git-diff-detected files
        if let Some(ref base) = self.from_branch.clone() {
            let blog_dir = self.paths[0].clone();
            log::info!(
                "--from-branch {base}: scanning {} for added/modified posts",
                blog_dir.display()
            );
            self.paths = changed_blog_files(base, &blog_dir)?;
            log::info!(
                "--from-branch {base}: {} candidate file(s) found",
                self.paths.len()
            );
        }

        let mut builder = Draft::builder(base_url, Some(&self.www_src_root));

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

/// Returns true if a diff entry (delta + path) is a candidate for Bluesky drafting:
/// the file must be added or modified, and must live under `blog_dir`.
fn is_candidate_for_draft(delta: Delta, path: &Path, blog_dir: &Path) -> bool {
    matches!(delta, Delta::Added | Delta::Modified) && path.starts_with(blog_dir)
}

/// Uses `git diff <base>...HEAD` (three-dot / merge-base diff) to find files
/// under `blog_dir` that are added or modified relative to the base branch.
fn changed_blog_files(base: &str, blog_dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let repo = git2::Repository::discover(".")
        .map_err(|e| Error::GitError(format!("failed to open repository: {e}")))?;

    let base_commit = repo
        .revparse_single(base)
        .map_err(|e| Error::GitError(format!("failed to resolve '{base}': {e}")))?
        .peel_to_commit()
        .map_err(|e| Error::GitError(format!("'{base}' is not a commit: {e}")))?;

    let head_commit = repo
        .head()
        .map_err(|e| Error::GitError(format!("failed to get HEAD: {e}")))?
        .peel_to_commit()
        .map_err(|e| Error::GitError(format!("HEAD is not a commit: {e}")))?;

    let merge_base_oid = repo
        .merge_base(base_commit.id(), head_commit.id())
        .map_err(|e| Error::GitError(format!("failed to find merge base: {e}")))?;

    let merge_base_tree = repo
        .find_commit(merge_base_oid)
        .map_err(|e| Error::GitError(format!("failed to find merge base commit: {e}")))?
        .tree()
        .map_err(|e| Error::GitError(format!("failed to get merge base tree: {e}")))?;

    let head_tree = head_commit
        .tree()
        .map_err(|e| Error::GitError(format!("failed to get HEAD tree: {e}")))?;

    let diff = repo
        .diff_tree_to_tree(Some(&merge_base_tree), Some(&head_tree), None)
        .map_err(|e| Error::GitError(format!("failed to compute diff: {e}")))?;

    let mut changed = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                if is_candidate_for_draft(delta.status(), path, blog_dir) {
                    changed.push(path.to_path_buf());
                }
            }
            true
        },
        None,
        None,
        None,
    )
    .map_err(|e| Error::GitError(format!("failed to iterate diff: {e}")))?;

    Ok(changed)
}

fn resolve_post_draft_exit(push_requested: bool, commits_ahead: usize) -> CIExit {
    if push_requested && commits_ahead > 0 {
        CIExit::Pushed("Bluesky drafts committed and pushed to remote repository.".to_string())
    } else {
        CIExit::DraftedForBluesky
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::bsky::Cmd;
    use crate::Cli;
    use clap::Parser;

    #[test]
    fn test_cmd_draft_has_allow_empty_field() {
        let cmd = CmdDraft {
            filter: None,
            paths: vec![],
            date: None,
            allow_draft: false,
            allow_empty: false,
            push: false,
            from_branch: None,
            www_src_root: PathBuf::from("."),
        };
        assert!(!cmd.allow_empty);
    }

    #[test]
    fn test_no_blog_posts_for_bluesky_ci_exit() {
        assert!(matches!(
            CIExit::NoBlogPostsForBluesky,
            CIExit::NoBlogPostsForBluesky
        ));
    }

    #[test]
    fn test_cmd_draft_has_push_field() {
        let cmd = CmdDraft {
            filter: None,
            paths: vec![],
            date: None,
            allow_draft: false,
            allow_empty: false,
            push: false,
            from_branch: None,
            www_src_root: PathBuf::from("."),
        };
        assert!(!cmd.push);
    }

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

    #[test]
    fn test_resolve_post_draft_exit_push_with_commits() {
        let exit = resolve_post_draft_exit(true, 1);
        assert!(
            matches!(exit, CIExit::Pushed(_)),
            "expected Pushed, got {exit:?}"
        );
    }

    #[test]
    fn test_resolve_post_draft_exit_push_no_commits() {
        let exit = resolve_post_draft_exit(true, 0);
        assert!(
            matches!(exit, CIExit::DraftedForBluesky),
            "expected DraftedForBluesky, got {exit:?}"
        );
    }

    #[test]
    fn test_resolve_post_draft_exit_no_push() {
        let exit = resolve_post_draft_exit(false, 5);
        assert!(
            matches!(exit, CIExit::DraftedForBluesky),
            "expected DraftedForBluesky, got {exit:?}"
        );
    }

    // RED: --from-branch CLI parsing (issue #898)

    #[test]
    fn test_bsky_draft_from_branch_defaults_none() {
        let args = Cli::try_parse_from(["pcu", "bsky", "draft"]).unwrap();
        match args.command {
            crate::Commands::Bsky(bsky) => match bsky.cmd {
                Cmd::Draft(draft) => assert!(draft.from_branch.is_none()),
                _ => panic!("expected Draft subcommand"),
            },
            _ => panic!("expected Bsky command"),
        }
    }

    #[test]
    fn test_bsky_draft_from_branch_flag_alone_defaults_origin_main() {
        let args = Cli::try_parse_from(["pcu", "bsky", "draft", "--from-branch"]).unwrap();
        match args.command {
            crate::Commands::Bsky(bsky) => match bsky.cmd {
                Cmd::Draft(draft) => {
                    assert_eq!(draft.from_branch.as_deref(), Some("origin/main"))
                }
                _ => panic!("expected Draft subcommand"),
            },
            _ => panic!("expected Bsky command"),
        }
    }

    #[test]
    fn test_bsky_draft_from_branch_with_explicit_base() {
        let args = Cli::try_parse_from(["pcu", "bsky", "draft", "--from-branch", "upstream/main"])
            .unwrap();
        match args.command {
            crate::Commands::Bsky(bsky) => match bsky.cmd {
                Cmd::Draft(draft) => {
                    assert_eq!(draft.from_branch.as_deref(), Some("upstream/main"))
                }
                _ => panic!("expected Draft subcommand"),
            },
            _ => panic!("expected Bsky command"),
        }
    }

    // GREEN: is_candidate_for_draft pure function (issue #898)

    #[test]
    fn test_candidate_added_in_blog_dir() {
        assert!(is_candidate_for_draft(
            Delta::Added,
            Path::new("content/blog/new-post.md"),
            Path::new("content/blog"),
        ));
    }

    #[test]
    fn test_candidate_modified_in_blog_dir() {
        assert!(is_candidate_for_draft(
            Delta::Modified,
            Path::new("content/blog/updated-post.md"),
            Path::new("content/blog"),
        ));
    }

    #[test]
    fn test_candidate_deleted_excluded() {
        assert!(!is_candidate_for_draft(
            Delta::Deleted,
            Path::new("content/blog/old-post.md"),
            Path::new("content/blog"),
        ));
    }

    #[test]
    fn test_candidate_outside_blog_dir_excluded() {
        assert!(!is_candidate_for_draft(
            Delta::Added,
            Path::new("src/lib.rs"),
            Path::new("content/blog"),
        ));
    }

    #[test]
    fn test_candidate_sibling_dir_excluded() {
        assert!(!is_candidate_for_draft(
            Delta::Modified,
            Path::new("content/other/post.md"),
            Path::new("content/blog"),
        ));
    }

    #[test]
    fn test_candidate_renamed_excluded() {
        assert!(!is_candidate_for_draft(
            Delta::Renamed,
            Path::new("content/blog/post.md"),
            Path::new("content/blog"),
        ));
    }
}
