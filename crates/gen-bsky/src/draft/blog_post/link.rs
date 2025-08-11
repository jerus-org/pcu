use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub(crate) enum LinkError {
    /// Error reported by the Url library.
    #[error("url error says: {0:?}")]
    UrlParse(#[from] url::ParseError),
}

#[derive(Debug, Clone)]
pub(crate) struct Link {
    local: String,
    public: Url,
}

impl Link {
    pub(crate) fn new(base_url: &Url, local: &str) -> Result<Self, LinkError> {
        let public = base_url.join(local)?;
        Ok(Link {
            local: local.to_string(),
            public,
        })
    }

    pub(crate) fn local(&self) -> &str {
        &self.local
    }

    pub(crate) fn public(&self) -> &Url {
        &self.public
    }
}
