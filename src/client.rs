use std::env;

#[derive(Debug, Default)]
pub struct Client {
    branch: String,
}

impl Client {
    pub fn new() -> Self {
        let pcu_branch = env::var("PCU_BRANCH").unwrap_or("".to_string());
        let branch = env::var(pcu_branch).unwrap_or("".to_string());

        Self { branch }
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }
}
