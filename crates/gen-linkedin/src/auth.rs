use crate::Error;

/// Provides bearer tokens for authenticating API requests.
pub trait TokenProvider: Send + Sync {
    /// Returns a bearer token string.
    fn bearer_token(&self) -> Result<String, Error>;
}

/// A token provider backed by a fixed string.
pub struct StaticTokenProvider(pub String);
impl TokenProvider for StaticTokenProvider {
    fn bearer_token(&self) -> Result<String, Error> {
        Ok(self.0.clone())
    }
}

/// Reads the token from an environment variable.
pub struct EnvTokenProvider {
    /// The environment variable name that stores the token.
    pub var: String,
}
impl TokenProvider for EnvTokenProvider {
    fn bearer_token(&self) -> Result<String, Error> {
        std::env::var(&self.var).map_err(|_| {
            Error::Config(format!(
                "missing env var {} for linkedin access token",
                self.var
            ))
        })
    }
}
