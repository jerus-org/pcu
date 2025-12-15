use thiserror::Error;

/// Errors that can occur when interacting with the LinkedIn API.
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP transport error from reqwest.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    /// Invalid or missing configuration.
    #[error("invalid configuration: {0}")]
    Config(String),
    /// The LinkedIn API returned an error response.
    #[error("api error: status={status}, message={message}")]
    Api {
        /// HTTP status code returned by the API.
        status: u16,
        /// Human-readable error message from the API.
        message: String,
    },
    /// The request was rate limited; wait before retrying.
    #[error("rate limited; retry after {retry_after}s")]
    RateLimited {
        /// Number of seconds to wait before retrying.
        retry_after: u64,
    },
}
