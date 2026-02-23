use super::signature_ops::TrustMap;
use crate::Error;
use octocrate::{Collaborator, GitHubAPI};
use std::process::Command;

/// Fetch trusted collaborators and their GPG keys from GitHub
///
/// This function:
/// 1. Fetches all collaborators with write/admin access
/// 2. For each collaborator, fetches their GPG keys
/// 3. Builds a TrustMap (email -> key IDs)
///
/// Privacy: Only logs aggregate counts, not individual names/emails
pub async fn fetch_trust_list(
    github: &GitHubAPI,
    owner: &str,
    repo: &str,
) -> Result<TrustMap, Error> {
    log::info!("Fetching trusted collaborators from GitHub API");

    // Fetch collaborators with push/admin permissions
    let collaborators = github.repos.list_collaborators(owner, repo).send().await?;

    // Filter to only those with write or admin access
    let trusted_collaborators: Vec<_> = collaborators
        .into_iter()
        .filter(|collab| {
            collab
                .permissions
                .as_ref()
                .is_some_and(|perms| perms.push || perms.admin)
        })
        .collect();

    log::info!(
        "Found {} collaborator(s) with write access",
        trusted_collaborators.len()
    );

    let mut trust_map = TrustMap::new();
    let total_keys = process_collaborators(github, trusted_collaborators, &mut trust_map).await?;

    log::info!("Imported {total_keys} GPG key(s)");
    log::info!(
        "Built trust map with {} email→key mapping(s)",
        trust_map.len()
    );

    // Also add GitHub's web-flow key for merge commits
    add_github_webflow_key(&mut trust_map)?;

    Ok(trust_map)
}

async fn process_collaborators(
    github: &GitHubAPI,
    trusted_collaborators: Vec<octocrate::Collaborator>,
    trust_map: &mut TrustMap,
) -> Result<usize, Error> {
    let mut total_keys = 0;

    for collaborator in trusted_collaborators {
        let username = &collaborator.login;

        // Fetch GPG keys for this user
        let gpg_keys = match github.users.list_gpg_keys_for_user(username).send().await {
            Ok(keys) => keys,
            Err(e) => {
                log::debug!("Failed to fetch GPG keys for user: {e}");
                continue;
            }
        };

        if gpg_keys.is_empty() {
            log::trace!("No GPG keys found for collaborator");
            continue;
        }

        total_keys += process_gpg_keys(gpg_keys, &collaborator, trust_map)?;
    }

    Ok(total_keys)
}

fn process_gpg_keys(
    gpg_keys: Vec<octocrate::GpgKey>,
    collaborator: &Collaborator,
    trust_map: &mut TrustMap,
) -> Result<usize, Error> {
    let username = &collaborator.login;
    let user_id = collaborator.id;
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
                        .entry(email.clone())
                        .or_default()
                        .push(kid.clone());
                }
                log::trace!("Added trust mapping (aggregate count only in logs)");
            }
        }

        // Also add GitHub noreply email formats with all key IDs
        let noreply_email = format!("{username}@users.noreply.github.com");
        let id_email = format!("{user_id}+{username}@users.noreply.github.com");
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
    use octocrate::{Collaborator, GpgKey};

    fn make_collaborator(login: &str, id: i64) -> Collaborator {
        serde_json::from_str(&format!(
            r#"{{"login":"{login}","id":{id},"node_id":"","avatar_url":"","gravatar_id":null,
               "url":"","html_url":"","followers_url":"","following_url":"",
               "gists_url":"","starred_url":"","subscriptions_url":"",
               "organizations_url":"","repos_url":"","events_url":"",
               "received_events_url":"","type":"User","site_admin":false,
               "permissions":{{"pull":true,"push":true,"admin":false}},
               "role_name":"write"}}"#
        ))
        .unwrap()
    }

    fn make_gpg_key(primary_id: &str, email: &str, subkey_ids: &[&str]) -> GpgKey {
        let subkeys_json: String = subkey_ids
            .iter()
            .map(|k| {
                format!(
                    r#"{{"can_certify":null,"can_encrypt_comms":null,"can_encrypt_storage":null,
                        "can_sign":true,"created_at":null,"emails":null,"expires_at":null,
                        "id":null,"key_id":"{k}","primary_key_id":null,"public_key":null,
                        "raw_key":null,"revoked":null,"subkeys":null}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        serde_json::from_str(&format!(
            r#"{{"can_certify":false,"can_encrypt_comms":false,"can_encrypt_storage":false,
               "can_sign":true,"created_at":"2025-01-01T00:00:00Z",
               "emails":[{{"email":"{email}","verified":true}}],
               "expires_at":null,"id":1,"key_id":"{primary_id}","name":null,
               "primary_key_id":null,"public_key":"test","raw_key":null,
               "revoked":false,"subkeys":[{subkeys_json}]}}"#
        ))
        .unwrap()
    }

    // Test key IDs are fictional hex strings; they are not real GPG keys.
    const TEST_PRIMARY_KEY: &str = "1A2B3C4D5E6F7A8B";
    const TEST_SUBKEY: &str = "9C0D1E2F3A4B5C6D";
    const TEST_USER: &str = "test-user-999";
    const TEST_USER_ID: i64 = 999_000_001;

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

        // May fail to fetch/import key in test environment, but should add to trust map
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
}
