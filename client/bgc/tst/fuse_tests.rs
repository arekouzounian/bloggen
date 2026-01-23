use bgc::fuse::BlogGenFS;
use bgc::http::Client;
use mockito::Server;

/// Test creating a BlogGenFS instance
#[test]
fn test_bloggen_fs_new() {
    let fs = BlogGenFS::new("http://localhost:3000".to_string());
    // Just check it doesn't panic
    drop(fs);
}

/// Test HTTP client integration with mock server
#[tokio::test]
async fn test_http_client_list_posts() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/posts")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "posts": [
                {
                    "slug": "test-post-1",
                    "title": "Test Post 1",
                    "created_at": "2026-01-19T00:00:00Z",
                    "updated_at": "2026-01-19T00:00:00Z",
                    "published": true
                },
                {
                    "slug": "test-post-2",
                    "title": "Test Post 2",
                    "created_at": "2026-01-19T01:00:00Z",
                    "updated_at": "2026-01-19T01:00:00Z",
                    "published": false
                }
            ],
            "total": 2
        }"#)
        .create_async()
        .await;

    let client = Client::new(server.url());
    let result = client.list_posts().await;

    mock.assert_async().await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.total, 2);
    assert_eq!(response.posts.len(), 2);
    assert_eq!(response.posts[0].slug, "test-post-1");
    assert_eq!(response.posts[1].slug, "test-post-2");
}

/// Test HTTP client download post with mock server
#[tokio::test]
async fn test_http_client_download_post() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/posts/test-post/ast")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"root_hash":"0000000000000000000000000000000000000000000000000000000000000001","nodes":{}}"#)
        .create_async()
        .await;

    let client = Client::new(server.url());
    let result = client.download_post("test-post").await;

    mock.assert_async().await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.nodes.len(), 0);
}

/// Test HTTP client upload post with mock server
#[tokio::test]
async fn test_http_client_upload_post() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/posts")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "slug": "new-post",
            "ast_root": "0000000000000000000000000000000000000000000000000000000000000001",
            "created_at": "2026-01-19T00:00:00Z"
        }"#)
        .create_async()
        .await;

    let client = Client::new(server.url());

    use bgc_ast::{AstNode, Blake3Hash};
    use std::collections::HashMap;

    let root_hash = Blake3Hash::from_hex(
        "0000000000000000000000000000000000000000000000000000000000000001"
    ).unwrap();

    let mut nodes = HashMap::new();
    nodes.insert(root_hash, AstNode::Root { children: vec![] });

    let result = client.upload_post("new-post", Some("New Post"), root_hash, nodes).await;

    mock.assert_async().await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.slug, "new-post");
}

/// Test BlogGenFS can be created
#[test]
fn test_fs_creation() {
    let fs = BlogGenFS::new("http://localhost:3000".to_string());
    // Just verify it doesn't panic
    drop(fs);

    // Test with different server URLs
    let fs2 = BlogGenFS::new("https://example.com".to_string());
    drop(fs2);

    let fs3 = BlogGenFS::new("http://192.168.1.1:8080".to_string());
    drop(fs3);
}
