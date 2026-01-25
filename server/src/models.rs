use bgc_ast::{AstNode, Blake3Hash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;

/// A blog post stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub slug: String,
    #[serde(with = "blake3_serde")]
    pub ast_root: Blake3Hash,
    pub title: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub published: bool,
}

/// A version snapshot of a post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostVersion {
    pub id: i32,
    pub post_id: i32,
    pub version_number: i32,
    #[serde(with = "blake3_serde")]
    pub ast_root: Blake3Hash,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Request to create a new post
#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub slug: String,
    pub title: Option<String>,
    #[serde(with = "blake3_serde")]
    pub ast_root: Blake3Hash,
    #[serde(with = "nodes_serde")]
    pub nodes: HashMap<Blake3Hash, AstNode>,
}

/// Request to update a post via delta
#[derive(Debug, Deserialize)]
pub struct DeltaUpdateRequest {
    #[serde(with = "blake3_serde")]
    pub old_root: Blake3Hash,
    #[serde(with = "blake3_serde")]
    pub new_root: Blake3Hash,
    #[serde(with = "nodes_serde")]
    pub added_nodes: HashMap<Blake3Hash, AstNode>,
    pub removed_hashes: Vec<Blake3Hash>,
}

/// Response for CAS document (AST)
#[derive(Debug, Serialize)]
pub struct CasDocumentResponse {
    #[serde(with = "blake3_serde")]
    pub root_hash: Blake3Hash,
    #[serde(with = "nodes_serde")]
    pub nodes: HashMap<Blake3Hash, AstNode>,
}

/// Response for list of posts
#[derive(Debug, Serialize)]
pub struct ListPostsResponse {
    pub posts: Vec<PostSummary>,
    pub total: i64,
}

/// Summary of a post (for list endpoint)
#[derive(Debug, Serialize)]
pub struct PostSummary {
    pub slug: String,
    pub title: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub published: bool,
}

// Custom serialization for Blake3Hash
mod blake3_serde {
    use bgc_ast::Blake3Hash;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(hash: &Blake3Hash, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hash.to_hex())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Blake3Hash, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Blake3Hash::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

// Custom serialization for HashMap<Blake3Hash, AstNode>
mod nodes_serde {
    use bgc_ast::{AstNode, Blake3Hash};
    use serde::{Deserialize, Deserializer, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(
        map: &HashMap<Blake3Hash, AstNode>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut ser_map = serializer.serialize_map(Some(map.len()))?;
        for (hash, node) in map {
            let hex_key = hash.to_hex();
            ser_map.serialize_entry(&hex_key, node)?;
        }
        ser_map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<Blake3Hash, AstNode>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_map: HashMap<String, AstNode> = HashMap::deserialize(deserializer)?;
        let mut result = HashMap::new();
        for (hex_key, node) in hex_map {
            let hash = Blake3Hash::from_hex(&hex_key).map_err(serde::de::Error::custom)?;
            result.insert(hash, node);
        }
        Ok(result)
    }
}
