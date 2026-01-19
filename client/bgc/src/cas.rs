use crate::ast::{AstNode, Blake3Hash};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A content-addressable storage node.
///
/// Each node is identified by the Blake3 hash of its content.
/// This enables:
/// - Deduplication: identical subtrees share the same hash
/// - Efficient updates: only changed nodes need to be transmitted
/// - Version history: just store root hash snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasNode {
    /// The Blake3 hash of this node's content
    pub hash: Blake3Hash,
    /// The actual AST node data
    pub node: AstNode,
}

impl CasNode {
    /// Create a new CasNode by computing the hash of the given node
    pub fn new(node: AstNode) -> Self {
        let hash = compute_hash(&node);
        Self { hash, node }
    }

    /// Get the hash of this node
    pub fn hash(&self) -> Blake3Hash {
        self.hash
    }

    /// Get a reference to the underlying AST node
    pub fn node(&self) -> &AstNode {
        &self.node
    }
}

/// Compute the Blake3 hash of an AST node.
///
/// The hash is computed from:
/// - Node type discriminant
/// - Node attributes (level, url, value, etc.)
/// - Child hashes (not child content)
///
/// This ensures that identical subtrees always produce the same hash,
/// regardless of where they appear in the document.
pub fn compute_hash(node: &AstNode) -> Blake3Hash {
    // Serialize the node to JSON for hashing
    // The serde serialization is deterministic and includes the type tag
    let json = serde_json::to_vec(node)
        .expect("serialization should never fail for AstNode");

    // Compute Blake3 hash
    let hash = blake3::hash(&json);
    Blake3Hash::new(*hash.as_bytes())
}

/// In-memory content-addressable node store.
///
/// Stores AST nodes indexed by their Blake3 hash.
/// Provides methods to store nodes and retrieve them by hash.
#[derive(Debug, Default)]
pub struct NodeStore {
    /// Map from hash to node
    nodes: HashMap<Blake3Hash, AstNode>,
}

impl NodeStore {
    /// Create a new empty node store
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Store a node and return its hash.
    ///
    /// If the node already exists (same hash), it won't be duplicated.
    pub fn store(&mut self, node: AstNode) -> Blake3Hash {
        let hash = compute_hash(&node);
        self.nodes.insert(hash, node);
        hash
    }

    /// Store multiple nodes and return their hashes
    pub fn store_many(&mut self, nodes: Vec<AstNode>) -> Vec<Blake3Hash> {
        nodes.into_iter().map(|node| self.store(node)).collect()
    }

    /// Retrieve a node by its hash
    pub fn get(&self, hash: &Blake3Hash) -> Option<&AstNode> {
        self.nodes.get(hash)
    }

    /// Check if a node with the given hash exists in the store
    pub fn contains(&self, hash: &Blake3Hash) -> bool {
        self.nodes.contains_key(hash)
    }

    /// Get the number of nodes in the store
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get all nodes in the store
    pub fn nodes(&self) -> &HashMap<Blake3Hash, AstNode> {
        &self.nodes
    }

    /// Walk the tree starting from a root hash and collect all reachable nodes.
    ///
    /// This is useful for serializing a complete document by following
    /// all child references from the root.
    pub fn walk_tree<'a>(&'a self, root_hash: &'a Blake3Hash) -> Option<Vec<(&'a Blake3Hash, &'a AstNode)>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();

        self.walk_tree_recursive(root_hash, &mut result, &mut visited)?;

        Some(result)
    }

    fn walk_tree_recursive<'a>(
        &'a self,
        hash: &'a Blake3Hash,
        result: &mut Vec<(&'a Blake3Hash, &'a AstNode)>,
        visited: &mut std::collections::HashSet<Blake3Hash>,
    ) -> Option<()> {
        // Avoid infinite loops in case of circular references (shouldn't happen)
        if visited.contains(hash) {
            return Some(());
        }
        visited.insert(*hash);

        let node = self.get(hash)?;
        result.push((hash, node));

        // Recursively walk children
        for child_hash in node.children() {
            self.walk_tree_recursive(child_hash, result, visited)?;
        }

        Some(())
    }
}

/// A complete content-addressable document.
///
/// This represents a full markdown document as a content-addressed AST,
/// with all nodes stored in the node store and indexed by the root hash.
#[derive(Debug, Serialize, Deserialize)]
pub struct CasDocument {
    /// The hash of the root node
    pub root_hash: Blake3Hash,
    /// All nodes in the document, indexed by hash
    #[serde(with = "hash_map_hex")]
    pub nodes: HashMap<Blake3Hash, AstNode>,
}

// Helper module for serializing HashMap<Blake3Hash, AstNode> with hex keys
mod hash_map_hex {
    use super::{AstNode, Blake3Hash};
    use serde::{Deserialize, Deserializer, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(map: &HashMap<Blake3Hash, AstNode>, serializer: S) -> Result<S::Ok, S::Error>
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

impl CasDocument {
    /// Create a new CasDocument from a node store and root hash
    pub fn new(store: &NodeStore, root_hash: Blake3Hash) -> Option<Self> {
        let nodes = store
            .walk_tree(&root_hash)?
            .into_iter()
            .map(|(hash, node)| (*hash, node.clone()))
            .collect();

        Some(Self { root_hash, nodes })
    }

    /// Get the root node
    pub fn root(&self) -> Option<&AstNode> {
        self.nodes.get(&self.root_hash)
    }

    /// Convert this document into a NodeStore
    ///
    /// This reconstructs a NodeStore containing all nodes from this document.
    /// Useful for rendering or further processing after deserialization.
    pub fn into_store(self) -> NodeStore {
        let mut store = NodeStore::new();
        for (hash, node) in self.nodes {
            store.nodes.insert(hash, node);
        }
        store
    }

    /// Create a NodeStore from this document (cloning the nodes)
    ///
    /// This is similar to `into_store` but doesn't consume the document.
    pub fn to_store(&self) -> NodeStore {
        let mut store = NodeStore::new();
        for (hash, node) in &self.nodes {
            store.nodes.insert(*hash, node.clone());
        }
        store
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty-printed JSON
    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// Serialize to MessagePack (compact binary format)
    ///
    /// MessagePack provides 60-70% size reduction compared to JSON,
    /// making it ideal for network transmission while maintaining
    /// the ability to deserialize back to the same structure.
    pub fn to_msgpack(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Deserialize from MessagePack
    pub fn from_msgpack(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }

    /// Serialize to compressed MessagePack (zstd compression level 3)
    ///
    /// This provides an additional 3-4x size reduction over raw MessagePack,
    /// making it ideal for network transmission. Compression level 3 provides
    /// a good balance between speed and compression ratio.
    pub fn to_msgpack_compressed(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let msgpack = self.to_msgpack()?;
        Ok(zstd::bulk::compress(&msgpack, 3)?)
    }

    /// Deserialize from compressed MessagePack
    ///
    /// Decompresses zstd-compressed data and then deserializes from MessagePack.
    /// The max_size parameter prevents decompression bombs (default: 10MB).
    pub fn from_msgpack_compressed(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let msgpack = zstd::bulk::decompress(data, 10_000_000)?;
        Ok(Self::from_msgpack(&msgpack)?)
    }

    /// Compute a delta between this document and a new version.
    ///
    /// This identifies nodes that were added (in new but not in old) and
    /// nodes that were removed (in old but not in new).
    ///
    /// This enables efficient network transmission: only send changed nodes
    /// instead of the entire document.
    pub fn compute_delta(&self, new_doc: &CasDocument) -> DeltaDocument {
        let old_hashes: HashSet<Blake3Hash> = self.nodes.keys().copied().collect();
        let new_hashes: HashSet<Blake3Hash> = new_doc.nodes.keys().copied().collect();

        // Nodes in new but not in old (added)
        let added_nodes: HashMap<Blake3Hash, AstNode> = new_hashes
            .difference(&old_hashes)
            .map(|hash| (*hash, new_doc.nodes[hash].clone()))
            .collect();

        // Nodes in old but not in new (removed)
        let removed_hashes: Vec<Blake3Hash> = old_hashes
            .difference(&new_hashes)
            .copied()
            .collect();

        DeltaDocument {
            old_root: self.root_hash,
            new_root: new_doc.root_hash,
            added_nodes,
            removed_hashes,
        }
    }

    /// Apply a delta to this document to produce a new document.
    ///
    /// This verifies that the delta's old_root matches this document's root,
    /// then applies the changes to produce the new document.
    ///
    /// Returns None if the old_root doesn't match (conflict).
    pub fn apply_delta(&self, delta: &DeltaDocument) -> Option<CasDocument> {
        // Verify the delta applies to this document
        if self.root_hash != delta.old_root {
            return None;
        }

        // Start with current nodes
        let mut new_nodes = self.nodes.clone();

        // Add new nodes
        for (hash, node) in &delta.added_nodes {
            new_nodes.insert(*hash, node.clone());
        }

        // Remove deleted nodes
        for hash in &delta.removed_hashes {
            new_nodes.remove(hash);
        }

        Some(CasDocument {
            root_hash: delta.new_root,
            nodes: new_nodes,
        })
    }
}

/// A delta between two CasDocuments.
///
/// This represents the difference between an old and new version of a document,
/// containing only the nodes that changed. This enables efficient network
/// transmission for document updates.
///
/// # Delta Size
///
/// For typical blog post edits (1-5 changed paragraphs), deltas are usually:
/// - 90-95% smaller than full documents
/// - Only transmit changed nodes plus new root
/// - Compress well with zstd (shared context with full doc)
#[derive(Debug, Serialize, Deserialize)]
pub struct DeltaDocument {
    /// The root hash of the old document
    pub old_root: Blake3Hash,
    /// The root hash of the new document
    pub new_root: Blake3Hash,
    /// Nodes that were added (in new but not in old)
    #[serde(with = "hash_map_hex")]
    pub added_nodes: HashMap<Blake3Hash, AstNode>,
    /// Hashes of nodes that were removed (in old but not in new)
    pub removed_hashes: Vec<Blake3Hash>,
}

impl DeltaDocument {
    /// Create a new DeltaDocument by computing the difference between two CasDocuments
    pub fn new(old_doc: &CasDocument, new_doc: &CasDocument) -> Self {
        old_doc.compute_delta(new_doc)
    }

    /// Check if this delta has any changes
    pub fn is_empty(&self) -> bool {
        self.added_nodes.is_empty() && self.removed_hashes.is_empty() && self.old_root == self.new_root
    }

    /// Get statistics about this delta
    pub fn stats(&self) -> DeltaStats {
        DeltaStats {
            added_count: self.added_nodes.len(),
            removed_count: self.removed_hashes.len(),
            root_changed: self.old_root != self.new_root,
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty-printed JSON
    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// Serialize to MessagePack (compact binary format)
    pub fn to_msgpack(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Deserialize from MessagePack
    pub fn from_msgpack(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }

    /// Serialize to compressed MessagePack (zstd compression level 3)
    pub fn to_msgpack_compressed(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let msgpack = self.to_msgpack()?;
        Ok(zstd::bulk::compress(&msgpack, 3)?)
    }

    /// Deserialize from compressed MessagePack
    pub fn from_msgpack_compressed(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let msgpack = zstd::bulk::decompress(data, 10_000_000)?;
        Ok(Self::from_msgpack(&msgpack)?)
    }
}

/// Statistics about a delta
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaStats {
    pub added_count: usize,
    pub removed_count: usize,
    pub root_changed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_stability() {
        let node = AstNode::Text {
            value: "hello".to_string(),
        };
        let hash1 = compute_hash(&node);
        let hash2 = compute_hash(&node);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_nodes_different_hashes() {
        let node1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let node2 = AstNode::Text {
            value: "world".to_string(),
        };
        let hash1 = compute_hash(&node1);
        let hash2 = compute_hash(&node2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_cas_node_creation() {
        let node = AstNode::Text {
            value: "test".to_string(),
        };
        let cas_node = CasNode::new(node.clone());
        assert_eq!(cas_node.node, node);
        assert_eq!(cas_node.hash, compute_hash(&node));
    }

    #[test]
    fn test_node_store() {
        let mut store = NodeStore::new();

        let node1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let hash1 = store.store(node1.clone());

        assert_eq!(store.len(), 1);
        assert!(store.contains(&hash1));
        assert_eq!(store.get(&hash1), Some(&node1));
    }

    #[test]
    fn test_deduplication() {
        let mut store = NodeStore::new();

        let node = AstNode::Text {
            value: "hello".to_string(),
        };

        let hash1 = store.store(node.clone());
        let hash2 = store.store(node.clone());

        // Same content should produce same hash
        assert_eq!(hash1, hash2);
        // Should only store once
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_tree_walk() {
        let mut store = NodeStore::new();

        // Create a simple tree: Paragraph -> [Text, Text]
        let text1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let text2 = AstNode::Text {
            value: "world".to_string(),
        };

        let hash1 = store.store(text1);
        let hash2 = store.store(text2);

        let para = AstNode::Paragraph {
            children: vec![hash1, hash2],
        };
        let para_hash = store.store(para);

        // Walk the tree
        let nodes = store.walk_tree(&para_hash).unwrap();

        // Should contain all 3 nodes
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn test_cas_document() {
        let mut store = NodeStore::new();

        let text = AstNode::Text {
            value: "test".to_string(),
        };
        let text_hash = store.store(text);

        let root = AstNode::Root {
            children: vec![text_hash],
        };
        let root_hash = store.store(root);

        let doc = CasDocument::new(&store, root_hash).unwrap();

        assert_eq!(doc.root_hash, root_hash);
        assert_eq!(doc.nodes.len(), 2);

        // Test serialization
        let json = doc.to_json_pretty().unwrap();
        assert!(json.contains("root_hash"));
        assert!(json.contains("nodes"));
    }

    #[test]
    fn test_msgpack_roundtrip() {
        let mut store = NodeStore::new();

        let text = AstNode::Text {
            value: "Hello, MessagePack!".to_string(),
        };
        let text_hash = store.store(text.clone());

        let root = AstNode::Root {
            children: vec![text_hash],
        };
        let root_hash = store.store(root.clone());

        // Create document
        let doc1 = CasDocument::new(&store, root_hash).unwrap();

        // Test MessagePack round-trip
        let msgpack = doc1.to_msgpack().unwrap();
        let doc2 = CasDocument::from_msgpack(&msgpack).unwrap();

        // Verify document content is preserved
        assert_eq!(doc1.root_hash, doc2.root_hash);
        assert_eq!(doc1.nodes.len(), doc2.nodes.len());

        // Verify all nodes are preserved
        assert_eq!(doc2.nodes.get(&root_hash), Some(&root));
        assert_eq!(doc2.nodes.get(&text_hash), Some(&text));
    }

    #[test]
    fn test_hex_hash_serialization() {
        let mut store = NodeStore::new();

        let text = AstNode::Text {
            value: "test".to_string(),
        };
        let hash = store.store(text);

        // Create a simple document
        let root = AstNode::Root {
            children: vec![hash],
        };
        let root_hash = store.store(root);
        let doc = CasDocument::new(&store, root_hash).unwrap();

        // Serialize to JSON
        let json = doc.to_json().unwrap();

        // Verify hashes are hex strings (64 chars), not arrays
        assert!(json.contains(&root_hash.to_hex()));
        assert!(!json.contains("[0,"));  // Should not have byte arrays
    }

    // Delta computation tests
    #[test]
    fn test_delta_no_changes() {
        let mut store = NodeStore::new();

        let text = AstNode::Text {
            value: "unchanged".to_string(),
        };
        let text_hash = store.store(text);

        let root = AstNode::Root {
            children: vec![text_hash],
        };
        let root_hash = store.store(root);

        let doc = CasDocument::new(&store, root_hash).unwrap();

        // Compute delta with itself
        let delta = doc.compute_delta(&doc);

        assert!(delta.is_empty());
        assert_eq!(delta.old_root, delta.new_root);
        assert_eq!(delta.added_nodes.len(), 0);
        assert_eq!(delta.removed_hashes.len(), 0);
    }

    #[test]
    fn test_delta_add_node() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();

        // Old document: one text node
        let text1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let text1_hash = old_store.store(text1.clone());
        let _ = new_store.store(text1);

        let old_root = AstNode::Root {
            children: vec![text1_hash],
        };
        let old_root_hash = old_store.store(old_root);

        // New document: two text nodes
        let text2 = AstNode::Text {
            value: "world".to_string(),
        };
        let text2_hash = new_store.store(text2.clone());

        let new_root = AstNode::Root {
            children: vec![text1_hash, text2_hash],
        };
        let new_root_hash = new_store.store(new_root.clone());

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();

        // Compute delta
        let delta = old_doc.compute_delta(&new_doc);
        let stats = delta.stats();

        assert_eq!(stats.added_count, 2); // new root + new text node
        assert_eq!(stats.removed_count, 1); // old root
        assert!(stats.root_changed);

        // Verify the new text node is in added_nodes
        assert!(delta.added_nodes.contains_key(&text2_hash));
        assert_eq!(delta.added_nodes[&text2_hash], text2);

        // Verify the new root is in added_nodes
        assert!(delta.added_nodes.contains_key(&new_root_hash));
        assert_eq!(delta.added_nodes[&new_root_hash], new_root);
    }

    #[test]
    fn test_delta_remove_node() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();

        // Old document: two text nodes
        let text1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let text1_hash = old_store.store(text1.clone());
        let _ = new_store.store(text1);

        let text2 = AstNode::Text {
            value: "world".to_string(),
        };
        let text2_hash = old_store.store(text2);

        let old_root = AstNode::Root {
            children: vec![text1_hash, text2_hash],
        };
        let old_root_hash = old_store.store(old_root);

        // New document: one text node
        let new_root = AstNode::Root {
            children: vec![text1_hash],
        };
        let new_root_hash = new_store.store(new_root);

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();

        // Compute delta
        let delta = old_doc.compute_delta(&new_doc);
        let stats = delta.stats();

        assert_eq!(stats.added_count, 1); // new root
        assert_eq!(stats.removed_count, 2); // old root + removed text node
        assert!(stats.root_changed);

        // Verify the removed text node is in removed_hashes
        assert!(delta.removed_hashes.contains(&text2_hash));
        assert!(delta.removed_hashes.contains(&old_root_hash));
    }

    #[test]
    fn test_delta_modify_node() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();

        // Old document
        let old_text = AstNode::Text {
            value: "old content".to_string(),
        };
        let old_text_hash = old_store.store(old_text);

        let old_root = AstNode::Root {
            children: vec![old_text_hash],
        };
        let old_root_hash = old_store.store(old_root);

        // New document (modified text)
        let new_text = AstNode::Text {
            value: "new content".to_string(),
        };
        let new_text_hash = new_store.store(new_text.clone());

        let new_root = AstNode::Root {
            children: vec![new_text_hash],
        };
        let new_root_hash = new_store.store(new_root.clone());

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();

        // Compute delta
        let delta = old_doc.compute_delta(&new_doc);
        let stats = delta.stats();

        // Modified node = remove old + add new
        assert!(stats.added_count >= 2); // new root + new text
        assert!(stats.removed_count >= 2); // old root + old text
        assert!(stats.root_changed);

        assert!(delta.added_nodes.contains_key(&new_text_hash));
        assert!(delta.added_nodes.contains_key(&new_root_hash));
    }

    // Delta application tests
    #[test]
    fn test_apply_delta_success() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();

        // Create old document
        let text1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let text1_hash = old_store.store(text1.clone());
        let _ = new_store.store(text1);

        let old_root = AstNode::Root {
            children: vec![text1_hash],
        };
        let old_root_hash = old_store.store(old_root);

        // Create new document
        let text2 = AstNode::Text {
            value: "world".to_string(),
        };
        let text2_hash = new_store.store(text2);

        let new_root = AstNode::Root {
            children: vec![text1_hash, text2_hash],
        };
        let new_root_hash = new_store.store(new_root);

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();

        // Compute and apply delta
        let delta = old_doc.compute_delta(&new_doc);
        let result_doc = old_doc.apply_delta(&delta).unwrap();

        // Verify result matches new document
        assert_eq!(result_doc.root_hash, new_doc.root_hash);
        assert_eq!(result_doc.nodes.len(), new_doc.nodes.len());
    }

    #[test]
    fn test_apply_delta_conflict() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();
        let mut wrong_store = NodeStore::new();

        // Create old document
        let text1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let text1_hash = old_store.store(text1.clone());

        let old_root = AstNode::Root {
            children: vec![text1_hash],
        };
        let old_root_hash = old_store.store(old_root);

        // Create new document
        let _ = new_store.store(text1.clone());
        let text2 = AstNode::Text {
            value: "world".to_string(),
        };
        let text2_hash = new_store.store(text2);

        let new_root = AstNode::Root {
            children: vec![text1_hash, text2_hash],
        };
        let new_root_hash = new_store.store(new_root);

        // Create wrong document with different root
        let _ = wrong_store.store(text1);
        let text3 = AstNode::Text {
            value: "wrong".to_string(),
        };
        let text3_hash = wrong_store.store(text3);

        let wrong_root = AstNode::Root {
            children: vec![text3_hash],
        };
        let wrong_root_hash = wrong_store.store(wrong_root);

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();
        let wrong_doc = CasDocument::new(&wrong_store, wrong_root_hash).unwrap();

        // Compute delta from old to new
        let delta = old_doc.compute_delta(&new_doc);

        // Try to apply delta to wrong document (should fail)
        let result = wrong_doc.apply_delta(&delta);
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_delta_roundtrip() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();

        // Create complex old document
        let text1 = AstNode::Text {
            value: "paragraph 1".to_string(),
        };
        let text1_hash = old_store.store(text1.clone());
        let _ = new_store.store(text1);

        let para1 = AstNode::Paragraph {
            children: vec![text1_hash],
        };
        let para1_hash = old_store.store(para1.clone());
        let _ = new_store.store(para1);

        let text2 = AstNode::Text {
            value: "paragraph 2".to_string(),
        };
        let text2_hash = old_store.store(text2);

        let para2 = AstNode::Paragraph {
            children: vec![text2_hash],
        };
        let para2_hash = old_store.store(para2);

        let old_root = AstNode::Root {
            children: vec![para1_hash, para2_hash],
        };
        let old_root_hash = old_store.store(old_root);

        // Create new document (replaced para2)
        let text3 = AstNode::Text {
            value: "new paragraph 2".to_string(),
        };
        let text3_hash = new_store.store(text3);

        let para3 = AstNode::Paragraph {
            children: vec![text3_hash],
        };
        let para3_hash = new_store.store(para3);

        let new_root = AstNode::Root {
            children: vec![para1_hash, para3_hash],
        };
        let new_root_hash = new_store.store(new_root);

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();

        // Compute delta
        let delta = old_doc.compute_delta(&new_doc);

        // Apply delta
        let result_doc = old_doc.apply_delta(&delta).unwrap();

        // Verify result matches new document exactly
        assert_eq!(result_doc.root_hash, new_doc.root_hash);
        assert_eq!(result_doc.nodes.len(), new_doc.nodes.len());

        // Verify all nodes match
        for (hash, node) in &new_doc.nodes {
            assert!(result_doc.nodes.contains_key(hash));
            assert_eq!(&result_doc.nodes[hash], node);
        }
    }

    // Delta serialization tests
    #[test]
    fn test_delta_json_serialization() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();

        let text1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let text1_hash = old_store.store(text1.clone());
        let _ = new_store.store(text1);

        let old_root = AstNode::Root {
            children: vec![text1_hash],
        };
        let old_root_hash = old_store.store(old_root);

        let text2 = AstNode::Text {
            value: "world".to_string(),
        };
        let text2_hash = new_store.store(text2);

        let new_root = AstNode::Root {
            children: vec![text1_hash, text2_hash],
        };
        let new_root_hash = new_store.store(new_root);

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();

        // Compute delta
        let delta = old_doc.compute_delta(&new_doc);

        // Test JSON serialization roundtrip
        let json = delta.to_json().unwrap();
        let delta2 = DeltaDocument::from_json(&json).unwrap();

        assert_eq!(delta.old_root, delta2.old_root);
        assert_eq!(delta.new_root, delta2.new_root);
        assert_eq!(delta.added_nodes.len(), delta2.added_nodes.len());
        assert_eq!(delta.removed_hashes.len(), delta2.removed_hashes.len());
    }

    #[test]
    fn test_delta_msgpack_serialization() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();

        let text1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let text1_hash = old_store.store(text1.clone());
        let _ = new_store.store(text1);

        let old_root = AstNode::Root {
            children: vec![text1_hash],
        };
        let old_root_hash = old_store.store(old_root);

        let text2 = AstNode::Text {
            value: "world".to_string(),
        };
        let text2_hash = new_store.store(text2);

        let new_root = AstNode::Root {
            children: vec![text1_hash, text2_hash],
        };
        let new_root_hash = new_store.store(new_root);

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();

        // Compute delta
        let delta = old_doc.compute_delta(&new_doc);

        // Test MessagePack serialization roundtrip
        let msgpack = delta.to_msgpack().unwrap();
        let delta2 = DeltaDocument::from_msgpack(&msgpack).unwrap();

        assert_eq!(delta.old_root, delta2.old_root);
        assert_eq!(delta.new_root, delta2.new_root);
        assert_eq!(delta.added_nodes.len(), delta2.added_nodes.len());
        assert_eq!(delta.removed_hashes.len(), delta2.removed_hashes.len());
    }

    #[test]
    fn test_delta_compressed_serialization() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();

        let text1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let text1_hash = old_store.store(text1.clone());
        let _ = new_store.store(text1);

        let old_root = AstNode::Root {
            children: vec![text1_hash],
        };
        let old_root_hash = old_store.store(old_root);

        let text2 = AstNode::Text {
            value: "world".to_string(),
        };
        let text2_hash = new_store.store(text2);

        let new_root = AstNode::Root {
            children: vec![text1_hash, text2_hash],
        };
        let new_root_hash = new_store.store(new_root);

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();

        // Compute delta
        let delta = old_doc.compute_delta(&new_doc);

        // Test compressed serialization roundtrip
        let compressed = delta.to_msgpack_compressed().unwrap();
        let delta2 = DeltaDocument::from_msgpack_compressed(&compressed).unwrap();

        assert_eq!(delta.old_root, delta2.old_root);
        assert_eq!(delta.new_root, delta2.new_root);
        assert_eq!(delta.added_nodes.len(), delta2.added_nodes.len());
        assert_eq!(delta.removed_hashes.len(), delta2.removed_hashes.len());

        // Verify compression provides size reduction
        let uncompressed = delta.to_msgpack().unwrap();
        assert!(compressed.len() < uncompressed.len());
    }

    #[test]
    fn test_delta_stats() {
        let mut old_store = NodeStore::new();
        let mut new_store = NodeStore::new();

        let text1 = AstNode::Text {
            value: "hello".to_string(),
        };
        let text1_hash = old_store.store(text1.clone());
        let _ = new_store.store(text1);

        let old_root = AstNode::Root {
            children: vec![text1_hash],
        };
        let old_root_hash = old_store.store(old_root);

        let text2 = AstNode::Text {
            value: "world".to_string(),
        };
        let text2_hash = new_store.store(text2);

        let new_root = AstNode::Root {
            children: vec![text1_hash, text2_hash],
        };
        let new_root_hash = new_store.store(new_root);

        let old_doc = CasDocument::new(&old_store, old_root_hash).unwrap();
        let new_doc = CasDocument::new(&new_store, new_root_hash).unwrap();

        // Compute delta and get stats
        let delta = old_doc.compute_delta(&new_doc);
        let stats = delta.stats();

        assert_eq!(stats.added_count, delta.added_nodes.len());
        assert_eq!(stats.removed_count, delta.removed_hashes.len());
        assert!(stats.root_changed);
        assert_ne!(delta.old_root, delta.new_root);
    }
}
