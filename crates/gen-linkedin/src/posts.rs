use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

use crate::{auth::TokenProvider, client::Client, Error};

/// Response returned when a post is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    /// The LinkedIn-generated ID/URN of the new post (from `x-restli-id` or `location`).
    pub id: String,
}

/// Parameters for creating a simple text post.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPost {
    /// The actor URN (e.g., `urn:li:person:...` or `urn:li:organization:...`).
    pub author_urn: String,
    /// The post text content.
    pub text: String,
    /// Optional link to include with the post (will be appended to text for now).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<url::Url>,
    /// Visibility setting, default PUBLIC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}

impl TextPost {
    /// Construct a new text post.
    pub fn new(author_urn: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            author_urn: author_urn.into(),
            text: text.into(),
            link: None,
            visibility: None,
        }
    }
    /// Attach a link to the post.
    pub fn with_link(mut self, link: Url) -> Self {
        self.link = Some(link);
        self
    }
    /// Set a visibility value such as "PUBLIC" or "CONNECTIONS".
    pub fn with_visibility(mut self, vis: impl Into<String>) -> Self {
        self.visibility = Some(vis.into());
        self
    }
}

/// Client for interacting with LinkedIn's Posts API.
pub struct PostsClient<TP: TokenProvider> {
    inner: Client<TP>,
}

impl<TP: TokenProvider> PostsClient<TP> {
    /// Construct a new Posts client from the base client.
    #[must_use]
    pub fn new(inner: Client<TP>) -> Self {
        Self { inner }
    }

    /// Create a text post using the LinkedIn Posts REST API.
    ///
    /// Notes
    /// - Requires the header `X-Restli-Protocol-Version: 2.0.0`.
    /// - On success, LinkedIn typically returns 201 with `x-restli-id` or `location` headers.
    pub async fn create_text_post(&self, post: &TextPost) -> Result<PostResponse, Error> {
        // Build endpoint
        let url = self
            .inner
            .base()
            .join("rest/posts")
            .map_err(|e| Error::Config(format!("invalid base url: {e}")))?;

        // Compose commentary, appending link if provided.
        let mut commentary = post.text.trim().to_string();
        if let Some(link) = &post.link {
            if !commentary.is_empty() {
                commentary.push_str("\n\n");
            }
            commentary.push_str(link.as_str());
        }

        // Minimal payload per Posts API examples (PUBLIC visibility, main feed)
        let body = json!({
            "author": post.author_urn,
            "commentary": commentary,
            "visibility": post.visibility.clone().unwrap_or_else(|| "PUBLIC".to_string()),
            "distribution": { "feedDistribution": "MAIN_FEED" },
            "lifecycleState": "PUBLISHED",
            "isReshareDisabledByAuthor": false
        });

        // Headers
        let mut headers = self.inner.auth_headers()?;
        headers.insert("X-Restli-Protocol-Version", "2.0.0".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        // Execute
        let resp = self
            .inner
            .http()
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() || resp.status().as_u16() == 201 {
            // Prefer x-restli-id, else location
            let id = resp
                .headers()
                .get("x-restli-id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .or_else(|| {
                    resp.headers()
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "created".to_string());
            return Ok(PostResponse { id });
        }

        if resp.status().as_u16() == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            return Err(Error::RateLimited { retry_after });
        }

        let status = resp.status().as_u16();
        let message = resp.text().await.unwrap_or_default();
        Err(Error::Api { status, message })
    }
}
