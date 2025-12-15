use clap::Parser;
use config::Config;
use gen_linkedin::posts::{PostsClient, TextPost};
use gen_linkedin::{auth::StaticTokenProvider, client::Client as LiClient};

use crate::utilities::linkedin_post::{build_release_text, compute_release_url};
use crate::{CIExit, Error};

#[derive(Debug, Parser, Clone)]
pub struct CmdShare {
    /// Post text. If omitted, we will build from release notes when --from-release is used.
    #[arg(long)]
    pub text: Option<String>,
    /// Build content from PRLOG release notes
    #[arg(long)]
    pub from_release: bool,
    /// Release version to use when --from-release is set (e.g., 0.6.2)
    #[arg(long)]
    pub version: Option<String>,
    /// Version tag prefix (default "v") when --from-release is used
    #[arg(long, default_value = "v")]
    pub prefix: String,
    /// Allow empty content (will no-op instead of failing)
    #[arg(long)]
    pub allow_empty: bool,
}

impl CmdShare {
    pub async fn run(
        &mut self,
        settings: &Config,
        author_urn_cli: Option<String>,
    ) -> Result<CIExit, Error> {
        let token = match settings.get::<String>("linkedin_access_token") {
            Ok(v) => v,
            Err(_) => std::env::var("LINKEDIN_ACCESS_TOKEN")
                .map_err(|_| config::ConfigError::NotFound("linkedin_access_token".into()))?,
        };
        let author_urn = match author_urn_cli
            .or_else(|| settings.get::<String>("linkedin_author_urn").ok())
            .or_else(|| std::env::var("LINKEDIN_AUTHOR_URN").ok())
        {
            Some(u) => u,
            None => {
                return Err(Error::Config(config::ConfigError::NotFound(
                    "linkedin_author_urn".into(),
                )))
            }
        };

        // Build text either from CLI, release notes, or fail/skip
        let mut text = self.text.take();
        if text.is_none() && self.from_release {
            let ver = self.version.clone().ok_or(Error::MissingSemver)?;
            text = Some(build_release_text(settings, &self.prefix, &ver)?);
        }
        let text = match text {
            Some(t) if !t.trim().is_empty() => t,
            _ if self.allow_empty => return Ok(CIExit::NoContentForLinkedIn),
            _ => {
                return Err(Error::Config(config::ConfigError::NotFound(
                    "linkedin text".into(),
                )))
            }
        };

        let li = LiClient::new(StaticTokenProvider(token))?;
        let pc = PostsClient::new(li);
        // Try attach a release link if we can compute one
        let link = compute_release_url(settings, &self.prefix, self.version.as_deref())?;
        let mut post = TextPost::new(author_urn, text);
        if let Some(u) = link {
            post = post.with_link(u);
        }
        let _resp = pc.create_text_post(&post).await?;
        Ok(CIExit::SharedToLinkedIn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_build_release_text_and_url() {
        // create temp PRLOG
        let d = tempdir().unwrap();
        let prlog_path = d.path().join("PRLOG.md");
        let contents = r#"# Changelog

## [1.2.3] - 2025-01-01

### Added
- New feature
"#;
        fs::write(&prlog_path, contents).unwrap();

        // config with overrides
        let mut builder = Config::builder();
        builder = builder
            .set_override("prlog", prlog_path.to_string_lossy().to_string())
            .unwrap()
            .set_override("dev_platform", "https://github.com/")
            .unwrap()
            .set_override("username", "TEST_OWNER_VAR")
            .unwrap()
            .set_override("reponame", "TEST_REPO_VAR")
            .unwrap();
        let cfg = builder.build().unwrap();

        // env vars for owner/repo
        env::set_var("TEST_OWNER_VAR", "jerus-org");
        env::set_var("TEST_REPO_VAR", "pcu");

        let text = build_release_text(&cfg, "v", "1.2.3").unwrap();
        assert!(text.contains("v1.2.3"));
        assert!(text.contains("releases/tag/v1.2.3"));

        let url = compute_release_url(&cfg, "v", Some("1.2.3")).unwrap();
        assert!(url.is_some());
    }
}
