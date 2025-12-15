use reqwest::{header, Client as HttpClient};
use url::Url;

use crate::{auth::TokenProvider, Error};

const DEFAULT_BASE: &str = "https://api.linkedin.com";

/// A thin HTTP client wrapper configured for LinkedIn's API.
pub struct Client<TP: TokenProvider> {
    http: HttpClient,
    base: Url,
    token_provider: TP,
    user_agent: String,
}

impl<TP: TokenProvider> Client<TP> {
    /// Constructs a new client using the provided token provider.
    pub fn new(token_provider: TP) -> Result<Self, Error> {
        let http = HttpClient::builder().build()?;
        let base = Url::parse(DEFAULT_BASE).map_err(|e| Error::Config(e.to_string()))?;
        Ok(Self {
            http,
            base,
            token_provider,
            user_agent: format!("gen-linkedin/{}", env!("CARGO_PKG_VERSION")),
        })
    }

    /// Override the base URL (useful for testing).
    #[must_use]
    pub fn with_base(mut self, base: Url) -> Self {
        self.base = base;
        self
    }

    /// Retrieve the current bearer token from the provider.
    pub fn bearer(&self) -> Result<String, Error> {
        self.token_provider.bearer_token()
    }

    /// The underlying reqwest client.
    pub fn http(&self) -> &HttpClient {
        &self.http
    }

    /// The base API URL.
    pub fn base(&self) -> &Url {
        &self.base
    }

    /// The default User-Agent used in requests.
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub(crate) fn auth_headers(&self) -> Result<header::HeaderMap, Error> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {}", self.bearer()?).parse().unwrap(),
        );
        headers.insert(header::USER_AGENT, self.user_agent.parse().unwrap());
        Ok(headers)
    }
}
