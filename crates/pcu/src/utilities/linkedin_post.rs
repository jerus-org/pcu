use config::Config;
use keep_a_changelog::Changelog;
use url::Url;

use super::ReleaseNotesProvider;
use crate::Error;

pub fn build_release_text(settings: &Config, prefix: &str, version: &str) -> Result<String, Error> {
    let prlog = settings
        .get::<String>("prlog")
        .unwrap_or_else(|_| "PRLOG.md".to_string());

    let pr = Changelog::parse_from_file(&prlog, None)
        .map_err(|e| Error::KeepAChangelog(e.to_string()))?;
    let rn = pr.release_notes(prefix, version)?;

    let mut body = format!("{} released\n\n", rn.name);
    let mut desc = rn.body.trim().to_string();
    if desc.len() > 2600 {
        desc.truncate(2600);
        desc.push('…');
    }
    body.push_str(&desc);

    if let Ok(Some(url)) = compute_release_url(settings, prefix, Some(version)) {
        body.push_str("\n\n");
        body.push_str(url.as_str());
    }

    Ok(body)
}

pub fn compute_release_url(
    settings: &Config,
    prefix: &str,
    version_opt: Option<&str>,
) -> Result<Option<Url>, Error> {
    use std::env;

    let owner_var = settings
        .get::<String>("username")
        .unwrap_or_else(|_| "CIRCLE_PROJECT_USERNAME".into());
    let repo_var = settings
        .get::<String>("reponame")
        .unwrap_or_else(|_| "CIRCLE_PROJECT_REPONAME".into());
    let base = settings
        .get::<String>("dev_platform")
        .unwrap_or_else(|_| "https://github.com/".into());

    let owner = match env::var(owner_var) {
        Ok(v) if !v.is_empty() => v,
        _ => return Ok(None),
    };
    let repo = match env::var(repo_var) {
        Ok(v) if !v.is_empty() => v,
        _ => return Ok(None),
    };

    if let Some(ver) = version_opt {
        let tag = format!("{prefix}{ver}");
        let url_str = format!("{base}{owner}/{repo}/releases/tag/{tag}");
        return Ok(Url::parse(&url_str).ok());
    }
    Ok(None)
}
