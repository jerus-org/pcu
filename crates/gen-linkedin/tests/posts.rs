use gen_linkedin::{
    auth::StaticTokenProvider,
    client::Client,
    posts::{PostsClient, TextPost},
};
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

    let token = StaticTokenProvider("TOKEN".to_string());
    let client = Client::new(token)
        .unwrap()
        .with_base(Url::parse(&server.uri()).unwrap());
    let posts = PostsClient::new(client);

    let resp = posts
        .create_text_post(&TextPost::new("urn:li:person:abc", "Hello LinkedIn"))
        .await
        .unwrap();
    assert_eq!(resp.id, "urn:li:activity:123");
}
