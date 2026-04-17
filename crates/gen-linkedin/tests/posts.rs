use gen_linkedin::{
    auth::StaticTokenProvider,
    client::Client,
    posts::{PostsClient, TextPost},
};
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn make_posts_client(server: &MockServer) -> PostsClient<StaticTokenProvider> {
    let token = StaticTokenProvider("TOKEN".to_string());
    let client = Client::new(token)
        .unwrap()
        .with_base(Url::parse(&server.uri()).unwrap());
    PostsClient::new(client)
}

#[tokio::test]
async fn create_text_post_sends_expected_request() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/rest/posts"))
        .and(header("authorization", "Bearer TOKEN"))
        .and(header("x-restli-protocol-version", "2.0.0"))
        .and(header("content-type", "application/json"))
        .respond_with(
            ResponseTemplate::new(201).insert_header("x-restli-id", "urn:li:activity:123"),
        )
        .mount(&server)
        .await;

    let resp = make_posts_client(&server)
        .await
        .create_text_post(&TextPost::new("urn:li:person:abc", "Hello LinkedIn"))
        .await
        .unwrap();
    assert_eq!(resp.id, "urn:li:activity:123");
}

#[tokio::test]
async fn create_text_post_sends_linkedin_version_header() {
    let server = MockServer::start().await;

    // Require the LinkedIn-Version header — 404 if absent or wrong version
    Mock::given(method("POST"))
        .and(path("/rest/posts"))
        .and(header("linkedin-version", "202401"))
        .respond_with(
            ResponseTemplate::new(201).insert_header("x-restli-id", "urn:li:activity:456"),
        )
        .mount(&server)
        .await;

    let resp = make_posts_client(&server)
        .await
        .create_text_post(&TextPost::new("urn:li:person:abc", "Hello LinkedIn"))
        .await
        .unwrap();
    assert_eq!(resp.id, "urn:li:activity:456");
}

#[tokio::test]
async fn create_text_post_uses_custom_api_version() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/rest/posts"))
        .and(header("linkedin-version", "202501"))
        .respond_with(
            ResponseTemplate::new(201).insert_header("x-restli-id", "urn:li:activity:789"),
        )
        .mount(&server)
        .await;

    let resp = make_posts_client(&server)
        .await
        .with_api_version("202501")
        .create_text_post(&TextPost::new("urn:li:person:abc", "Hello LinkedIn"))
        .await
        .unwrap();
    assert_eq!(resp.id, "urn:li:activity:789");
}
