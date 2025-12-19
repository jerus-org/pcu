use octocrate::GitHubAPI;
use crate::Error;
use super::signature_ops::TrustMap;

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
    let collaborators = github
        .repos
        .list_collaborators(owner, repo)
        .send()
        .await?;
    
    // Filter to only those with write or admin access
    let trusted_collaborators: Vec<_> = collaborators
        .into_iter()
        .filter(|collab| {
            collab.permissions.as_ref().map_or(false, |perms| {
                perms.push || perms.admin
            })
        })
        .collect();
    
    log::info!(
        "Found {} collaborator(s) with write access",
        trusted_collaborators.len()
    );
    
    // Build trust map
    let mut trust_map = TrustMap::new();
    let mut total_keys = 0;
    
    for collaborator in trusted_collaborators {
        let username = &collaborator.login;
        
        // Fetch GPG keys for this user
        let gpg_keys = match github
            .users
            .list_gpg_keys_for_user(username)
            .send()
            .await
        {
            Ok(keys) => keys,
            Err(e) => {
                log::debug!("Failed to fetch GPG keys for user: {}", e);
                continue;
            }
        };
        
        if gpg_keys.is_empty() {
            log::trace!("No GPG keys found for collaborator");
            continue;
        }
        
        // Process each GPG key
        for key in gpg_keys {
            total_keys += 1;
            
            // Extract key ID (last 16 characters of the key_id)
            let key_id = key.key_id.clone();
            
            // Map each email in the key to this key ID
            for email_obj in &key.emails {
                if let Some(email) = &email_obj.email {
                    // Add to trust map
                    trust_map
                        .entry(email.clone())
                        .or_insert_with(Vec::new)
                        .push(key_id.clone());
                    
                    log::trace!("Added trust mapping (aggregate count only in logs)");
                }
            }
            
            // Also add GitHub noreply email formats
            // These are used for web commits and bot commits
            let noreply_email = format!("{}@users.noreply.github.com", username);
            trust_map
                .entry(noreply_email)
                .or_insert_with(Vec::new)
                .push(key_id.clone());
            
            // Also add numeric ID format
            let user_id = collaborator.id;
            let id_email = format!("{}+{}@users.noreply.github.com", user_id, username);
            trust_map
                .entry(id_email)
                .or_insert_with(Vec::new)
                .push(key_id.clone());
        }
    }
    
    log::info!("Imported {} GPG key(s)", total_keys);
    log::info!(
        "Built trust map with {} email→key mapping(s)",
        trust_map.len()
    );
    
    // Also add GitHub's web-flow key for merge commits
    add_github_webflow_key(&mut trust_map);
    
    Ok(trust_map)
}

/// Add GitHub's web-flow GPG key for merge commits
/// 
/// GitHub signs merge commits with their web-flow key.
/// Key ID: B5690EEEBB952194
fn add_github_webflow_key(trust_map: &mut TrustMap) {
    const GITHUB_WEBFLOW_KEY: &str = "B5690EEEBB952194";
    const GITHUB_WEBFLOW_EMAIL: &str = "noreply@github.com";
    
    log::debug!("Adding GitHub web-flow key for merge commits");
    
    trust_map
        .entry(GITHUB_WEBFLOW_EMAIL.to_string())
        .or_insert_with(Vec::new)
        .push(GITHUB_WEBFLOW_KEY.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_github_webflow_key() {
        let mut trust_map = TrustMap::new();
        
        add_github_webflow_key(&mut trust_map);
        
        assert!(trust_map.contains_key("noreply@github.com"));
        assert!(trust_map["noreply@github.com"].contains(&"B5690EEEBB952194".to_string()));
    }
    
    #[test]
    fn test_trust_map_multiple_keys_per_email() {
        let mut trust_map = TrustMap::new();
        
        // Simulate multiple keys for same email
        trust_map
            .entry("test@example.test".to_string())
            .or_insert_with(Vec::new)
            .push("KEY1".to_string());
        
        trust_map
            .entry("test@example.test".to_string())
            .or_insert_with(Vec::new)
            .push("KEY2".to_string());
        
        assert_eq!(trust_map["test@example.test"].len(), 2);
        assert!(trust_map["test@example.test"].contains(&"KEY1".to_string()));
        assert!(trust_map["test@example.test"].contains(&"KEY2".to_string()));
    }
}
