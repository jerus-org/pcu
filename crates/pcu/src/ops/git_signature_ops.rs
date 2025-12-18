use git2::{Repository, Oid, Commit};
use crate::Error;
use super::signature_ops::{CommitInfo, SignatureStatus};

/// Extract commits from a git repository in a given range
/// 
/// # Arguments
/// * `repo` - Git repository
/// * `base_ref` - Base reference (e.g., "origin/main")
/// * `head_ref` - Head reference (e.g., "HEAD")
/// 
/// # Returns
/// Vector of CommitInfo structs with signature information
pub fn extract_commits(
    repo: &Repository,
    base_ref: &str,
    head_ref: &str,
) -> Result<Vec<CommitInfo>, Error> {
    log::info!("Extracting commits from {}..{}", base_ref, head_ref);
    
    // Resolve references to OIDs
    let base_oid = resolve_reference(repo, base_ref)?;
    let head_oid = resolve_reference(repo, head_ref)?;
    
    log::debug!("Base OID: {}, Head OID: {}", base_oid, head_oid);
    
    // Find merge base
    let merge_base = repo.merge_base(base_oid, head_oid)
        .map_err(|e| Error::GitError(format!("Failed to find merge base: {}", e)))?;
    
    log::debug!("Merge base: {}", merge_base);
    
    // Walk commits from head to merge base
    let mut revwalk = repo.revwalk()
        .map_err(|e| Error::GitError(format!("Failed to create revwalk: {}", e)))?;
    
    revwalk.push(head_oid)
        .map_err(|e| Error::GitError(format!("Failed to push head: {}", e)))?;
    
    revwalk.hide(merge_base)
        .map_err(|e| Error::GitError(format!("Failed to hide merge base: {}", e)))?;
    
    // Set to topological order
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)
        .map_err(|e| Error::GitError(format!("Failed to set sorting: {}", e)))?;
    
    let mut commits = Vec::new();
    let mut count = 0;
    
    for oid_result in revwalk {
        let oid = oid_result
            .map_err(|e| Error::GitError(format!("Failed to get OID from revwalk: {}", e)))?;
        
        let commit = repo.find_commit(oid)
            .map_err(|e| Error::GitError(format!("Failed to find commit {}: {}", oid, e)))?;
        
        // Skip merge commits
        if commit.parent_count() > 1 {
            log::trace!("Skipping merge commit: {}", oid);
            continue;
        }
        
        let commit_info = extract_commit_info(&commit, repo)?;
        commits.push(commit_info);
        count += 1;
    }
    
    log::info!("Extracted {} commit(s) for verification", count);
    
    Ok(commits)
}

/// Resolve a reference string to an OID
fn resolve_reference(repo: &Repository, refname: &str) -> Result<Oid, Error> {
    // Try as a reference first
    if let Ok(reference) = repo.find_reference(refname) {
        return reference.peel_to_commit()
            .map(|c| c.id())
            .map_err(|e| Error::GitError(format!("Failed to resolve reference {}: {}", refname, e)));
    }
    
    // Try as a direct OID
    if let Ok(oid) = Oid::from_str(refname) {
        // Verify it exists
        if repo.find_commit(oid).is_ok() {
            return Ok(oid);
        }
    }
    
    // Try with refs/ prefix variations
    let prefixes = ["refs/heads/", "refs/remotes/", "refs/tags/"];
    for prefix in &prefixes {
        let full_ref = format!("{}{}", prefix, refname);
        if let Ok(reference) = repo.find_reference(&full_ref) {
            return reference.peel_to_commit()
                .map(|c| c.id())
                .map_err(|e| Error::GitError(format!("Failed to resolve reference {}: {}", full_ref, e)));
        }
    }
    
    Err(Error::GitError(format!("Could not resolve reference: {}", refname)))
}

/// Extract commit information including signature status
fn extract_commit_info(commit: &Commit, repo: &Repository) -> Result<CommitInfo, Error> {
    let sha = commit.id().to_string();
    let author = commit.author();
    let author_email = author.email()
        .unwrap_or("")
        .to_string();
    let author_name = author.name()
        .unwrap_or("")
        .to_string();
    
    let subject = commit.summary()
        .unwrap_or("")
        .to_string();
    
    // Extract signature information
    let (signature_status, key_id, signer) = extract_signature_info(commit, repo)?;
    
    log::trace!(
        "Extracted commit {}: {} (status: {:?})",
        &sha[..8],
        subject,
        signature_status
    );
    
    Ok(CommitInfo {
        sha,
        author_email,
        author_name,
        subject,
        signature_status,
        key_id,
        signer,
    })
}

/// Extract signature information from a commit
fn extract_signature_info(commit: &Commit, repo: &Repository) -> Result<(SignatureStatus, Option<String>, Option<String>), Error> {
    use std::process::Command;
    
    let oid = commit.id();
    let repo_path = repo.path().parent()
        .ok_or_else(|| Error::GitError("Failed to get repository path".to_string()))?;
    
    // Use git show to get signature information
    // %G? = signature status
    // %GK = key ID
    // %GS = signer
    let output = Command::new("git")
        .arg("show")
        .arg("-s")
        .arg("--format=%G?|%GK|%GS")
        .arg(oid.to_string())
        .current_dir(repo_path)
        .output()
        .map_err(|e| Error::GitError(format!("Failed to run git show: {}", e)))?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = output_str.trim().split('|').collect();
    
    if parts.len() < 3 {
        return Ok((SignatureStatus::None, None, None));
    }
    
    let status = SignatureStatus::from_git_format(parts[0]);
    let key_id = if parts[1].is_empty() {
        None
    } else {
        Some(parts[1].to_string())
    };
    let signer = if parts[2].is_empty() {
        None
    } else {
        Some(parts[2].to_string())
    };
    
    Ok((status, key_id, signer))
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::process::Command;
    
    fn create_test_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        
        // Configure git
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.test").unwrap();
        
        (dir, repo)
    }
    
    fn create_test_commit(repo: &Repository, message: &str) -> Oid {
        let sig = repo.signature().unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        let parent_commit = repo.head().ok()
            .and_then(|h| h.peel_to_commit().ok());
        
        let parents: Vec<&Commit> = parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();
        
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        ).unwrap()
    }
    
    #[test]
    fn test_extract_commits_from_range() {
        let (_dir, repo) = create_test_repo();
        
        // Create initial commit
        let commit1 = create_test_commit(&repo, "Initial commit");
        
        // Create a few more commits
        let _commit2 = create_test_commit(&repo, "Second commit");
        let _commit3 = create_test_commit(&repo, "Third commit");
        
        // Extract commits from first commit to HEAD
        let commits = extract_commits(&repo, &commit1.to_string(), "HEAD").unwrap();
        
        // Should have 2 commits (Second and Third)
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "Third commit");
        assert_eq!(commits[1].subject, "Second commit");
    }
    
    #[test]
    fn test_resolve_reference() {
        let (_dir, repo) = create_test_repo();
        create_test_commit(&repo, "Test commit");
        
        // Should resolve HEAD
        let oid = resolve_reference(&repo, "HEAD").unwrap();
        assert!(repo.find_commit(oid).is_ok());
        
        // Should resolve refs/heads/main or master
        let head_ref = repo.head().unwrap();
        let branch_name = head_ref.shorthand().unwrap();
        let oid2 = resolve_reference(&repo, branch_name).unwrap();
        assert_eq!(oid, oid2);
    }
    
    #[test]
    fn test_extract_commit_info() {
        let (_dir, repo) = create_test_repo();
        let oid = create_test_commit(&repo, "Test message");
        
        let commit = repo.find_commit(oid).unwrap();
        let info = extract_commit_info(&commit, &repo).unwrap();
        
        assert_eq!(info.sha, oid.to_string());
        assert_eq!(info.subject, "Test message");
        assert_eq!(info.author_email, "test@example.test");
        assert_eq!(info.author_name, "Test User");
        assert_eq!(info.signature_status, SignatureStatus::None);
    }
}
