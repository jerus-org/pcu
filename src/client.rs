use std::env;

#[derive(Debug, Default)]
pub struct Client {
    branch: String,
    pull_request: String,
}

impl Client {
    pub fn new() -> Self {
        // Use the PCU_BRANCH env variable to direct to the appropriate CI environment variable to find the branch data
        let pcu_branch = env::var("PCU_BRANCH").unwrap_or("".to_string());
        let branch = env::var(pcu_branch).unwrap_or("".to_string());

        // Use the PCU_PULL_REQUEST env variable to direct to the appropriate CI environment variable to find the PR data
        let pcu_pull_request = env::var("PCU_PULL_REQUEST").unwrap_or("".to_string());
        let pull_request = if let Ok(pr) = env::var(pcu_pull_request) {
            pr.clone()
        } else {
            String::new()
        };

        Self {
            branch,
            pull_request,
        }
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn pull_release(&self) -> &str {
        &self.pull_request
    }

    pub fn pr_number(&self) -> &str {
        if self.pull_request.contains("github.com") {
            let parts = self.pull_request.splitn(7, '/').collect::<Vec<&str>>();
            parts[6]
        } else {
            ""
        }
    }

    pub fn owner(&self) -> &str {
        if self.pull_request.contains("github.com") {
            let parts = self.pull_request.splitn(7, '/').collect::<Vec<&str>>();
            parts[3]
        } else {
            ""
        }
    }

    pub fn repo(&self) -> &str {
        if self.pull_request.contains("github.com") {
            let parts = self.pull_request.splitn(7, '/').collect::<Vec<&str>>();
            parts[4]
        } else {
            ""
        }
    }
}
