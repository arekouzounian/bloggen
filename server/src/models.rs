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

#[cfg(test)]
mod tests {
    use super::*;
    use bgc_ast::{AstNode, Blake3Hash};
    use std::collections::HashMap;

    #[test]
    fn test_blake3_serde_roundtrip() {
        // Wrap a hash in a struct that uses the blake3_serde module
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct Wrapper {
            #[serde(with = "blake3_serde")]
            hash: Blake3Hash,
        }

        let original = Wrapper {
            hash: Blake3Hash::new([0xab; 32]),
        };
        let json = serde_json::to_string(&original).unwrap();
        // The hash should appear as a hex string in JSON
        assert!(json.contains('"'));
        let recovered: Wrapper = serde_json::from_str(&json).unwrap();
        assert_eq!(original.hash.as_bytes(), recovered.hash.as_bytes());
    }

    #[test]
    fn test_blake3_serde_invalid_hex() {
        // Deserialization should fail for a non-hex value
        #[derive(serde::Deserialize, Debug)]
        struct Wrapper {
            #[serde(with = "blake3_serde")]
            hash: Blake3Hash,
        }

        let bad_json = r#"{"hash": "not-valid-hex"}"#;
        assert!(serde_json::from_str::<Wrapper>(bad_json).is_err());
    }

    #[test]
    fn test_nodes_serde_roundtrip() {
        // Wrap a nodes map in a struct that uses nodes_serde
        #[derive(serde::Serialize, serde::Deserialize, Debug)]
        struct Wrapper {
            #[serde(with = "nodes_serde")]
            nodes: HashMap<Blake3Hash, AstNode>,
        }

        let mut map = HashMap::new();
        let hash = Blake3Hash::new([0x01; 32]);
        let node = AstNode::Text {
            value: "hello".to_string(),
        };
        map.insert(hash, node.clone());

        let original = Wrapper { nodes: map };
        let json = serde_json::to_string(&original).unwrap();
        let recovered: Wrapper = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.nodes.len(), 1);
        assert!(recovered.nodes.contains_key(&hash));
        let recovered_node = &recovered.nodes[&hash];
        assert_eq!(recovered_node, &node);
    }

    #[test]
    fn test_nodes_serde_empty_map() {
        #[derive(serde::Serialize, serde::Deserialize, Debug)]
        struct Wrapper {
            #[serde(with = "nodes_serde")]
            nodes: HashMap<Blake3Hash, AstNode>,
        }

        let original = Wrapper {
            nodes: HashMap::new(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let recovered: Wrapper = serde_json::from_str(&json).unwrap();
        assert!(recovered.nodes.is_empty());
    }
}
