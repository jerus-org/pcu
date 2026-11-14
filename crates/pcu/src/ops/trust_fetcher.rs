use std::process::Command;

use octocrab::{models::Collaborator, Octocrab};
use serde::Deserialize;

use super::signature_ops::TrustMap;
use crate::Error;

/// The subset of GitHub's `GET /users/{username}/gpg_keys` response that
/// `process_gpg_keys` actually reads. octocrab has no dedicated wrapper for
/// this endpoint (unlike octocrate, which this replaced — see
/// jerus-org/pcu#1070), so it is fetched via `Octocrab::get`'s generic route
/// escape hatch and deserialized into this crate-owned shape rather than a
/// full upstream model.
#[derive(Debug, Clone, Deserialize)]
struct GpgKey {
    key_id: String,
    raw_key: Option<String>,
    emails: Vec<GpgKeyEmail>,
    #[serde(default)]
    subkeys: Vec<GpgSubkey>,
}

#[derive(Debug, Clone, Deserialize)]
struct GpgKeyEmail {
    email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GpgSubkey {
    key_id: Option<String>,
}

/// Fetch trusted collaborators and their GPG keys from GitHub
///
/// This function:
/// 1. Fetches all collaborators with write/admin access
/// 2. For each collaborator, fetches their GPG keys
/// 3. Builds a TrustMap (email -> key IDs)
///
/// Privacy: Only logs aggregate counts, not individual names/emails
pub async fn fetch_trust_list(
    github: &Octocrab,
    owner: &str,
    repo: &str,
) -> Result<TrustMap, Error> {
    log::info!("Fetching trusted collaborators from GitHub API");

    // Fetch collaborators with push/admin permissions
    // Every page, not just the first (30): a collaborator beyond it would
    // otherwise never have their keys imported while the trust data still
    // looked complete.
    let first_page = github
        .repos(owner, repo)
        .list_collaborators()
        .per_page(100)
        .send()
        .await?;
    let collaborators = github.all_pages(first_page).await?;

    // Filter to only those with write or admin access
    let trusted_collaborators: Vec<_> = collaborators
        .into_iter()
        .filter(|collab| collab.permissions.push || collab.permissions.admin)
        .collect();

    log::info!(
        "Found {} collaborator(s) with write access",
        trusted_collaborators.len()
    );

    let mut trust_map = TrustMap::new();
    let collaborator_count = trusted_collaborators.len();
    let (total_keys, failed_fetches) =
        process_collaborators(github, trusted_collaborators, &mut trust_map).await?;

    log::info!("Imported {total_keys} GPG key(s)");
    log::info!(
        "Built trust map with {} email→key mapping(s)",
        trust_map.len()
    );

    ensure_trust_data_complete(collaborator_count, total_keys, failed_fetches)?;

    // Also add GitHub's web-flow key for merge commits
    add_github_webflow_key(&mut trust_map)?;

    Ok(trust_map)
}

/// Refuse to verify when the trust data could not be fully obtained.
///
/// Two cases, both of which would otherwise fail open:
/// - a key fetch failed: that collaborator would be reported as an external
///   contributor and their commits pass unverified;
/// - collaborators exist but no key was imported: the trust map is empty and
///   every commit passes as external.
///
/// A collaborator with no registered key is not a failure. Counts only, never
/// logins, to keep the CI log privacy-safe.
fn ensure_trust_data_complete(
    collaborators: usize,
    imported_keys: usize,
    failed_fetches: usize,
) -> Result<(), Error> {
    if failed_fetches > 0 {
        return Err(Error::GpgError(format!(
            "Could not fetch GPG keys for {failed_fetches} of {collaborators} collaborator(s) \
             with write access; refusing to verify with incomplete trust data"
        )));
    }
    if collaborators > 0 && imported_keys == 0 {
        return Err(Error::GpgError(format!(
            "No GPG keys were imported for {collaborators} collaborator(s) with write access; \
             refusing to verify against an empty trust map, which would let every commit pass \
             as external"
        )));
    }
    Ok(())
}

/// HTTP status of a failed key fetch, when GitHub answered at all.
fn fetch_error_status(e: &octocrab::Error) -> Option<u16> {
    match e {
        octocrab::Error::GitHub { source, .. } => Some(source.status_code.as_u16()),
        _ => None,
    }
}

/// Only an explicit 404 means the account is gone. Everything else (rate
/// limit, 5xx, auth failure, network error, unparseable body) is an error in
/// getting the verification data and must still fail closed.
fn is_account_not_found(status: Option<u16>) -> bool {
    status == Some(404)
}

/// Describe a failed key fetch without echoing the request URL (which holds
/// the collaborator's login) into CI logs.
fn describe_fetch_error(e: &octocrab::Error) -> String {
    match e {
        octocrab::Error::GitHub { source, .. } => {
            format!(
                "GitHub API returned {}: {}",
                source.status_code, source.message
            )
        }
        octocrab::Error::Serde { source, .. } => {
            format!("response could not be parsed: {source}")
        }
        octocrab::Error::Json { source, .. } => {
            format!("response could not be parsed: {source}")
        }
        // The request failed before GitHub answered (bad URI, transport...).
        // Name the variant only: its payload can hold the request URL, which
        // contains the collaborator's login.
        other => {
            let debug = format!("{other:?}");
            let kind = debug
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .unwrap_or("unknown");
            format!("request failed ({kind})")
        }
    }
}

/// Fetch one user's public GPG keys. Kept separate so the request can be
/// exercised against a local server in tests.
async fn fetch_gpg_keys(github: &Octocrab, username: &str) -> Result<Vec<GpgKey>, octocrab::Error> {
    // The leading slash matters: octocrab 0.54 rejects a relative route as an
    // invalid URI before sending anything, so every key fetch failed with no
    // HTTP status (jerus-org/pcu#1131).
    let route = format!("/users/{username}/gpg_keys");
    github.get::<Vec<GpgKey>, _, ()>(route, None).await
}

/// Returns `(keys imported, collaborators whose key fetch failed)`.
async fn process_collaborators(
    github: &Octocrab,
    trusted_collaborators: Vec<Collaborator>,
    trust_map: &mut TrustMap,
) -> Result<(usize, usize), Error> {
    let mut total_keys = 0;
    let mut failed_fetches = 0;

    for collaborator in trusted_collaborators {
        let username = &collaborator.author.login;

        // Fetch GPG keys for this user — no dedicated octocrab wrapper for
        // this endpoint, so it goes through the generic route (see the
        // module-level `GpgKey` doc comment).
        let gpg_keys = match fetch_gpg_keys(github, username).await {
            Ok(keys) => keys,
            Err(e) => {
                if is_account_not_found(fetch_error_status(&e)) {
                    // A removed or renamed account has no keys to trust, and
                    // would otherwise fail every run until collaborators are
                    // edited. Nothing is lost by skipping it: it cannot sign.
                    log::warn!(
                        "A collaborator's account was not found (404); treating it as having no keys"
                    );
                } else {
                    failed_fetches += 1;
                    log::warn!(
                        "Failed to fetch GPG keys for a collaborator: {}",
                        describe_fetch_error(&e)
                    );
                }
                continue;
            }
        };

        if gpg_keys.is_empty() {
            log::trace!("No GPG keys found for collaborator");
            continue;
        }

        total_keys += process_gpg_keys(gpg_keys, &collaborator, trust_map)?;
    }

    Ok((total_keys, failed_fetches))
}

fn process_gpg_keys(
    gpg_keys: Vec<GpgKey>,
    collaborator: &Collaborator,
    trust_map: &mut TrustMap,
) -> Result<usize, Error> {
    let username = &collaborator.author.login;
    let user_id = collaborator.author.id;
    let key_count = gpg_keys.len();

    // Process each GPG key
    for key in gpg_keys {
        // Extract key ID
        let key_id = &key.key_id;

        // Import the public key into GPG keyring for git verification
        if let Some(ref raw_key) = key.raw_key {
            import_key_to_gpg(raw_key)?;
        }

        // Collect all key IDs: primary key plus all subkeys.
        // git reports the signing subkey's ID in %GK, not the primary key ID.
        // The GitHub API returns subkeys in key.subkeys[].key_id, so we add
        // them all to the trust map so subkey-signed commits verify correctly.
        let mut all_key_ids: Vec<String> = vec![key_id.clone()];
        for subkey in &key.subkeys {
            if let Some(ref subkey_id) = subkey.key_id {
                if !subkey_id.is_empty() {
                    all_key_ids.push(subkey_id.clone());
                }
            }
        }
        log::trace!(
            "Processing GPG key with {} id(s) (primary + subkeys)",
            all_key_ids.len()
        );

        // Map each email to all key IDs (primary and subkeys)
        for email_obj in &key.emails {
            if let Some(email) = &email_obj.email {
                for kid in &all_key_ids {
                    trust_map
                        .entry(email.to_lowercase())
                        .or_default()
                        .push(kid.clone());
                }
                log::trace!("Added trust mapping (aggregate count only in logs)");
            }
        }

        // Also add GitHub noreply email formats with all key IDs
        // Lowercase: verification matches author emails case-insensitively.
        let noreply_email = format!("{username}@users.noreply.github.com").to_lowercase();
        let id_email = format!("{user_id}+{username}@users.noreply.github.com").to_lowercase();
        for kid in &all_key_ids {
            trust_map
                .entry(noreply_email.clone())
                .or_default()
                .push(kid.clone());
            trust_map
                .entry(id_email.clone())
                .or_default()
                .push(kid.clone());
        }
    }

    Ok(key_count)
}

/// Import a GPG public key into the system keyring
///
/// This is needed so that `git show --show-signature` can verify signatures.
/// The key is imported via `gpg --import`.
fn import_key_to_gpg(raw_key: &str) -> Result<(), Error> {
    let mut child = Command::new("gpg")
        .arg("--import")
        .arg("--quiet")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| Error::GitError(format!("Failed to spawn gpg: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(raw_key.as_bytes())
            .map_err(|e| Error::GitError(format!("Failed to write to gpg stdin: {e}")))?;
    }

    let status = child
        .wait()
        .map_err(|e| Error::GitError(format!("Failed to wait for gpg: {e}")))?;

    if !status.success() {
        return Err(Error::GitError("GPG import failed".to_string()));
    }

    Ok(())
}

/// Add GitHub's web-flow GPG key for merge commits
///
/// GitHub signs merge commits with their web-flow key.
/// Key ID: B5690EEEBB952194
fn add_github_webflow_key(trust_map: &mut TrustMap) -> Result<(), Error> {
    const GITHUB_WEBFLOW_KEY: &str = "B5690EEEBB952194";
    const GITHUB_WEBFLOW_EMAIL: &str = "noreply@github.com";

    log::debug!("Adding GitHub web-flow key for merge commits");

    trust_map
        .entry(GITHUB_WEBFLOW_EMAIL.to_string())
        .or_default()
        .push(GITHUB_WEBFLOW_KEY.to_string());

    // Import GitHub's web-flow key into GPG
    // Fetch from GitHub's public key server
    let webflow_key_url = "https://github.com/web-flow.gpg";

    let output = Command::new("curl")
        .arg("-sL")
        .arg("--proto")
        .arg("=https")
        .arg("--tlsv1.2")
        .arg(webflow_key_url)
        .output()
        .map_err(|e| Error::GitError(format!("Failed to fetch GitHub web-flow key: {e}")))?;

    if output.status.success() {
        let key_data = String::from_utf8_lossy(&output.stdout);
        import_key_to_gpg(&key_data)?;
        log::debug!("Imported GitHub web-flow key");
    } else {
        log::warn!("Failed to fetch GitHub web-flow key, merge commits may not verify");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_collaborator(login: &str, id: i64) -> Collaborator {
        let u = "https://example.test/";
        serde_json::from_str(&format!(
            r#"{{"login":"{login}","id":{id},"node_id":"","avatar_url":"{u}","gravatar_id":"",
               "url":"{u}","html_url":"{u}","followers_url":"{u}","following_url":"{u}",
               "gists_url":"{u}","starred_url":"{u}","subscriptions_url":"{u}",
               "organizations_url":"{u}","repos_url":"{u}","events_url":"{u}",
               "received_events_url":"{u}","type":"User","site_admin":false,
               "permissions":{{"pull":true,"push":true,"admin":false}}}}"#
        ))
        .unwrap()
    }

    fn make_gpg_key(primary_id: &str, email: &str, subkey_ids: &[&str]) -> GpgKey {
        let subkeys_json: String = subkey_ids
            .iter()
            .map(|k| format!(r#"{{"key_id":"{k}"}}"#))
            .collect::<Vec<_>>()
            .join(",");

        serde_json::from_str(&format!(
            r#"{{"key_id":"{primary_id}","raw_key":null,
               "emails":[{{"email":"{email}"}}],"subkeys":[{subkeys_json}]}}"#
        ))
        .unwrap()
    }

    // Test key IDs are fictional hex strings; they are not real GPG keys.
    const TEST_PRIMARY_KEY: &str = "1A2B3C4D5E6F7A8B";
    const TEST_SUBKEY: &str = "9C0D1E2F3A4B5C6D";
    const TEST_USER: &str = "test-user-999";
    const TEST_USER_ID: i64 = 999_000_001;

    #[test]
    fn test_only_a_404_means_the_account_is_gone() {
        assert!(is_account_not_found(Some(404)));
        for status in [Some(401), Some(403), Some(429), Some(500), Some(502), None] {
            assert!(
                !is_account_not_found(status),
                "{status:?} must still fail closed"
            );
        }
    }

    #[test]
    fn test_empty_trust_map_with_collaborators_is_an_error() {
        let err = ensure_trust_data_complete(3, 0, 0).unwrap_err().to_string();
        assert!(err.contains("3 collaborator(s)"), "got: {err}");
        assert!(err.contains("empty trust map"), "got: {err}");
    }

    #[test]
    fn test_any_failed_key_fetch_is_an_error_even_if_other_keys_imported() {
        let err = ensure_trust_data_complete(3, 2, 1).unwrap_err().to_string();
        assert!(err.contains("1 of 3"), "got: {err}");
        assert!(err.contains("incomplete trust data"), "got: {err}");
    }

    #[test]
    fn test_complete_or_collaborator_free_trust_data_is_ok() {
        assert!(ensure_trust_data_complete(3, 2, 0).is_ok());
        assert!(ensure_trust_data_complete(0, 0, 0).is_ok());
    }

    #[test]
    fn test_subkey_ids_added_to_trust_map() {
        let collaborator = make_collaborator(TEST_USER, TEST_USER_ID);
        let email = format!("{TEST_USER}@example.test");
        let key = make_gpg_key(TEST_PRIMARY_KEY, &email, &[TEST_SUBKEY]);
        let mut trust_map = TrustMap::new();
        process_gpg_keys(vec![key], &collaborator, &mut trust_map).unwrap();

        let noreply = format!("{TEST_USER}@users.noreply.github.com");
        let keys = trust_map.get(&noreply).unwrap();
        assert!(
            keys.contains(&TEST_PRIMARY_KEY.to_string()),
            "primary key ID should be in trust map"
        );
        assert!(
            keys.contains(&TEST_SUBKEY.to_string()),
            "subkey ID should be in trust map"
        );
    }

    /// Trust-map keys are lowercase whatever the case GitHub reports, so the
    /// case-insensitive lookup in verification has one entry per identity.
    #[test]
    fn test_trust_map_emails_are_lowercase() {
        let collaborator = make_collaborator("Test-User-999", TEST_USER_ID);
        let key = make_gpg_key(TEST_PRIMARY_KEY, "Test-User-999@Example.Test", &[]);
        let mut trust_map = TrustMap::new();
        process_gpg_keys(vec![key], &collaborator, &mut trust_map).unwrap();

        assert!(trust_map.contains_key("test-user-999@example.test"));
        assert!(trust_map.contains_key("test-user-999@users.noreply.github.com"));
        assert!(trust_map.contains_key("999000001+test-user-999@users.noreply.github.com"));
        assert!(
            trust_map.keys().all(|k| *k == k.to_lowercase()),
            "every key lowercase: {:?}",
            trust_map.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_primary_key_only_still_works() {
        let collaborator = make_collaborator(TEST_USER, TEST_USER_ID);
        let email = format!("{TEST_USER}@example.test");
        let key = make_gpg_key(TEST_PRIMARY_KEY, &email, &[]);
        let mut trust_map = TrustMap::new();
        process_gpg_keys(vec![key], &collaborator, &mut trust_map).unwrap();

        let noreply = format!("{TEST_USER}@users.noreply.github.com");
        let keys = trust_map.get(&noreply).unwrap();
        assert_eq!(keys.len(), 1, "only primary key should be present");
        assert!(keys.contains(&TEST_PRIMARY_KEY.to_string()));
    }

    #[test]
    fn test_add_github_webflow_key() {
        let mut trust_map = TrustMap::new();

        // May fail to fetch/import key in test environment, but should add to
        // trust map
        let _ = add_github_webflow_key(&mut trust_map);

        assert!(trust_map.contains_key("noreply@github.com"));
        assert!(trust_map["noreply@github.com"].contains(&"B5690EEEBB952194".to_string()));
    }

    #[test]
    fn test_trust_map_multiple_keys_per_email() {
        let mut trust_map = TrustMap::new();

        // Simulate multiple keys for same email
        trust_map
            .entry("test@example.test".to_string())
            .or_default()
            .push("KEY1".to_string());

        trust_map
            .entry("test@example.test".to_string())
            .or_default()
            .push("KEY2".to_string());

        assert_eq!(trust_map["test@example.test"].len(), 2);
        assert!(trust_map["test@example.test"].contains(&"KEY1".to_string()));
        assert!(trust_map["test@example.test"].contains(&"KEY2".to_string()));
    }

    /// A minimal HTTP/1.1 server that answers every request with `body` and
    /// records each request line. No mock-server dependency needed.
    async fn serve_json(
        body: &'static str,
    ) -> (
        std::net::SocketAddr,
        std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = std::sync::Arc::clone(&requests);
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).await.unwrap();
                let request = String::from_utf8_lossy(&buf[..n]);
                seen.lock()
                    .unwrap()
                    .push(request.lines().next().unwrap_or("").to_string());
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
                     content-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                sock.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (addr, requests)
    }

    fn client_for(addr: std::net::SocketAddr) -> Octocrab {
        Octocrab::builder()
            .base_uri(format!("http://{addr}"))
            .unwrap()
            .build()
            .unwrap()
    }

    const KEYS_BODY: &str = r#"[{"key_id":"1A2B3C4D5E6F7A8B","raw_key":null,
        "emails":[{"email":"test-user-999@example.test","verified":true}],
        "subkeys":[{"key_id":"9C0D1E2F3A4B5C6D"}]}]"#;

    /// Regression for jerus-org/pcu#1131: the key request used a relative
    /// route, which octocrab rejected as an invalid URI before sending, so
    /// every fetch failed and the trust map was always empty.
    #[tokio::test]
    async fn gpg_keys_request_is_sent_and_parsed() {
        let (addr, requests) = serve_json(KEYS_BODY).await;

        let keys = fetch_gpg_keys(&client_for(addr), TEST_USER).await.unwrap();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_id, TEST_PRIMARY_KEY);
        let seen = requests.lock().unwrap();
        assert_eq!(seen.len(), 1, "exactly one request: {seen:?}");
        assert!(
            seen[0].starts_with(&format!("GET /users/{TEST_USER}/gpg_keys")),
            "unexpected request line: {}",
            seen[0]
        );
    }

    /// The whole fetch path: a collaborator's keys end up in the trust map.
    #[tokio::test]
    async fn collaborator_keys_are_imported_into_the_trust_map() {
        let (addr, _requests) = serve_json(KEYS_BODY).await;
        let collaborator = make_collaborator(TEST_USER, TEST_USER_ID);
        let mut trust_map = TrustMap::new();

        let (imported, failed) =
            process_collaborators(&client_for(addr), vec![collaborator], &mut trust_map)
                .await
                .unwrap();

        assert_eq!((imported, failed), (1, 0));
        let keys = trust_map.get("test-user-999@example.test").unwrap();
        assert!(keys.contains(&TEST_PRIMARY_KEY.to_string()));
        assert!(keys.contains(&TEST_SUBKEY.to_string()));
    }

    /// A relative route fails before any request is sent. Pins the octocrab
    /// behaviour behind the fix, and checks the error names its kind without
    /// echoing the URL.
    #[tokio::test]
    async fn relative_route_error_names_its_kind() {
        let (addr, requests) = serve_json(KEYS_BODY).await;

        let err = client_for(addr)
            .get::<Vec<GpgKey>, _, ()>(format!("users/{TEST_USER}/gpg_keys"), None)
            .await
            .unwrap_err();

        assert!(requests.lock().unwrap().is_empty(), "nothing was sent");
        let described = describe_fetch_error(&err);
        assert!(
            described.starts_with("request failed ("),
            "got: {described}"
        );
        assert!(!described.contains(TEST_USER), "login leaked: {described}");
    }
}
