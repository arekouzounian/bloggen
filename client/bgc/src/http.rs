//! HTTP client for communicating with the BlogGen v2 server

use anyhow::{Context, Result};
use bgc_ast::{AstNode, Blake3Hash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP client for the BlogGen v2 server
pub struct Client {
    base_url: String,
    http_client: reqwest::Client,
}

impl Client {
    /// Create a new client with the given server base URL
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Upload a new post to the server
    pub async fn upload_post(
        &self,
        slug: &str,
        title: Option<&str>,
        ast_root: Blake3Hash,
        nodes: HashMap<Blake3Hash, AstNode>,
    ) -> Result<CreatePostResponse> {
        let url = format!("{}/posts", self.base_url);

        let request = CreatePostRequest {
            slug: slug.to_string(),
            title: title.map(|s| s.to_string()),
            ast_root,
            nodes,
        };

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send upload request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Server returned error {}: {}", status, error_body);
        }

        response
            .json()
            .await
            .context("Failed to parse upload response")
    }

    /// Download a post from the server by slug
    pub async fn download_post(&self, slug: &str) -> Result<CasDocumentResponse> {
        let url = format!("{}/posts/{}/ast", self.base_url, slug);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send download request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Server returned error {}: {}", status, error_body);
        }

        response
            .json()
            .await
            .context("Failed to parse download response")
    }

    /// Update a post using delta update
    pub async fn update_post_delta(
        &self,
        slug: &str,
        old_root: Blake3Hash,
        new_root: Blake3Hash,
        added_nodes: HashMap<Blake3Hash, AstNode>,
        removed_hashes: Vec<Blake3Hash>,
    ) -> Result<DeltaUpdateResponse> {
        let url = format!("{}/posts/{}/delta", self.base_url, slug);

        let request = DeltaUpdateRequest {
            old_root,
            new_root,
            added_nodes,
            removed_hashes,
        };

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send delta update request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Server returned error {}: {}", status, error_body);
        }

        response
            .json()
            .await
            .context("Failed to parse delta update response")
    }

    /// List all posts on the server
    pub async fn list_posts(&self) -> Result<ListPostsResponse> {
        let url = format!("{}/posts", self.base_url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send list request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Server returned error {}: {}", status, error_body);
        }

        response
            .json()
            .await
            .context("Failed to parse list response")
    }

    /// Delete a post from the server
    pub async fn delete_post(&self, slug: &str) -> Result<()> {
        let url = format!("{}/posts/{}", self.base_url, slug);

        let response = self
            .http_client
            .delete(&url)
            .send()
            .await
            .context("Failed to send delete request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Server returned error {}: {}", status, error_body);
        }

        Ok(())
    }

    /// Get markdown rendering of a post
    pub async fn get_markdown(&self, slug: &str) -> Result<String> {
        let url = format!("{}/posts/{}/markdown", self.base_url, slug);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send markdown request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Server returned error {}: {}", status, error_body);
        }

        response
            .text()
            .await
            .context("Failed to get markdown response")
    }
}

// Request/Response types
// These match the server's API models with custom Blake3Hash serialization

#[derive(Debug, Serialize)]
struct CreatePostRequest {
    slug: String,
    title: Option<String>,
    #[serde(serialize_with = "serialize_blake3hash")]
    ast_root: Blake3Hash,
    #[serde(serialize_with = "serialize_nodes")]
    nodes: HashMap<Blake3Hash, AstNode>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePostResponse {
    pub slug: String,
    #[serde(deserialize_with = "deserialize_blake3hash")]
    pub ast_root: Blake3Hash,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
struct DeltaUpdateRequest {
    #[serde(serialize_with = "serialize_blake3hash")]
    old_root: Blake3Hash,
    #[serde(serialize_with = "serialize_blake3hash")]
    new_root: Blake3Hash,
    #[serde(serialize_with = "serialize_nodes")]
    added_nodes: HashMap<Blake3Hash, AstNode>,
    #[serde(serialize_with = "serialize_blake3hash_vec")]
    removed_hashes: Vec<Blake3Hash>,
}

#[derive(Debug, Deserialize)]
pub struct DeltaUpdateResponse {
    pub slug: String,
    #[serde(deserialize_with = "deserialize_blake3hash")]
    pub old_root: Blake3Hash,
    #[serde(deserialize_with = "deserialize_blake3hash")]
    pub new_root: Blake3Hash,
    pub nodes_added: usize,
    pub nodes_removed: usize,
}

#[derive(Debug, Deserialize)]
pub struct CasDocumentResponse {
    #[serde(deserialize_with = "deserialize_blake3hash")]
    pub root_hash: Blake3Hash,
    #[serde(deserialize_with = "deserialize_nodes")]
    pub nodes: HashMap<Blake3Hash, AstNode>,
}

#[derive(Debug, Deserialize)]
pub struct ListPostsResponse {
    pub posts: Vec<PostSummary>,
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostSummary {
    pub slug: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub published: bool,
}

// Custom serialization/deserialization for Blake3Hash
fn serialize_blake3hash<S>(hash: &Blake3Hash, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hash.to_hex())
}

fn deserialize_blake3hash<'de, D>(deserializer: D) -> Result<Blake3Hash, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Blake3Hash::from_hex(&s).map_err(serde::de::Error::custom)
}

fn serialize_blake3hash_vec<S>(hashes: &[Blake3Hash], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(hashes.len()))?;
    for hash in hashes {
        seq.serialize_element(&hash.to_hex())?;
    }
    seq.end()
}

fn serialize_nodes<S>(
    nodes: &HashMap<Blake3Hash, AstNode>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(Some(nodes.len()))?;
    for (hash, node) in nodes {
        map.serialize_entry(&hash.to_hex(), node)?;
    }
    map.end()
}

fn deserialize_nodes<'de, D>(
    deserializer: D,
) -> Result<HashMap<Blake3Hash, AstNode>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let map: HashMap<String, AstNode> = HashMap::deserialize(deserializer)?;
    let mut result = HashMap::new();
    for (hash_str, node) in map {
        let hash = Blake3Hash::from_hex(&hash_str).map_err(serde::de::Error::custom)?;
        result.insert(hash, node);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_upload_post_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/posts")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"slug":"test-post","ast_root":"0000000000000000000000000000000000000000000000000000000000000001","created_at":"2026-01-18T12:00:00Z"}"#)
            .create_async()
            .await;

        let client = Client::new(server.url());
        let result = client
            .upload_post(
                "test-post",
                Some("Test Post"),
                Blake3Hash::from_hex("0000000000000000000000000000000000000000000000000000000000000001")
                    .unwrap(),
                HashMap::new(),
            )
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.slug, "test-post");
    }

    #[tokio::test]
    async fn test_upload_post_server_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/posts")
            .with_status(500)
            .with_body("Internal server error")
            .create_async()
            .await;

        let client = Client::new(server.url());
        let result = client
            .upload_post(
                "test-post",
                Some("Test Post"),
                Blake3Hash::from_hex("0000000000000000000000000000000000000000000000000000000000000001")
                    .unwrap(),
                HashMap::new(),
            )
            .await;

        mock.assert_async().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("500"));
    }

    #[tokio::test]
    async fn test_download_post_success() {
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
    }

    #[tokio::test]
    async fn test_download_post_not_found() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/posts/missing/ast")
            .with_status(404)
            .with_body("Post not found")
            .create_async()
            .await;

        let client = Client::new(server.url());
        let result = client.download_post("missing").await;

        mock.assert_async().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }

    #[tokio::test]
    async fn test_update_post_delta_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/posts/test-post/delta")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"slug":"test-post","old_root":"0000000000000000000000000000000000000000000000000000000000000001","new_root":"0000000000000000000000000000000000000000000000000000000000000002","nodes_added":1,"nodes_removed":0}"#)
            .create_async()
            .await;

        let client = Client::new(server.url());
        let result = client
            .update_post_delta(
                "test-post",
                Blake3Hash::from_hex("0000000000000000000000000000000000000000000000000000000000000001")
                    .unwrap(),
                Blake3Hash::from_hex("0000000000000000000000000000000000000000000000000000000000000002")
                    .unwrap(),
                HashMap::new(),
                vec![],
            )
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.nodes_added, 1);
        assert_eq!(response.nodes_removed, 0);
    }

    #[tokio::test]
    async fn test_list_posts_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/posts")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"posts":[],"total":0}"#)
            .create_async()
            .await;

        let client = Client::new(server.url());
        let result = client.list_posts().await;

        mock.assert_async().await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.total, 0);
    }

    #[tokio::test]
    async fn test_delete_post_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/posts/test-post")
            .with_status(204)
            .create_async()
            .await;

        let client = Client::new(server.url());
        let result = client.delete_post("test-post").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_markdown_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/posts/test-post/markdown")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("# Test Post\n\nThis is a test.")
            .create_async()
            .await;

        let client = Client::new(server.url());
        let result = client.get_markdown("test-post").await;

        mock.assert_async().await;
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("# Test Post"));
    }
}
