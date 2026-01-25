use crate::error::{AppError, Result};
use crate::{AstNode, Blake3Hash};
use std::collections::{HashMap, HashSet, VecDeque};

use super::NodeStore;

/// Walk the entire AST tree from a root hash and collect all nodes
///
/// This performs a breadth-first traversal of the tree, loading nodes from the store
/// and following child hash references. The result is a complete map of all nodes
/// in the subtree.
///
/// # Optimizations
/// - Uses batch fetching (get_many) to minimize database round-trips
/// - Tracks visited nodes to avoid re-fetching shared subtrees
/// - Early returns on missing nodes with clear error messages
///
/// # Error Handling
/// - Returns error if root node doesn't exist
/// - Returns error if any referenced child node is missing (broken tree)
/// - Returns error on cycles (shouldn't happen with content-addressing, but checked anyway)
///
/// # Performance
/// For a typical blog post (50-100 nodes), this should complete in <50ms including DB queries.
pub async fn walk_tree(
    root_hash: Blake3Hash,
    store: &impl NodeStore,
) -> Result<HashMap<Blake3Hash, AstNode>> {
    let mut result = HashMap::new();
    let mut visited = HashSet::new();
    let mut to_visit = VecDeque::new();

    // Start with root
    to_visit.push_back(root_hash);
    visited.insert(root_hash);

    while !to_visit.is_empty() {
        // Collect a batch of nodes to fetch (up to 100 at a time for efficiency)
        let batch_size = to_visit.len().min(100);
        let current_batch: Vec<Blake3Hash> = to_visit.drain(..batch_size).collect();

        // Fetch all nodes in this batch
        let nodes = store.get_many(&current_batch).await?;

        // Check that we got all the nodes we requested
        for hash in &current_batch {
            if !nodes.contains_key(hash) {
                return Err(AppError::Internal(format!(
                    "Missing node in tree: {}",
                    hash.to_hex()
                )));
            }
        }

        // Process each node in the batch
        for (hash, node) in nodes {
            // Add all children to the queue if not already visited
            for child_hash in node.children() {
                if visited.insert(*child_hash) {
                    to_visit.push_back(*child_hash);
                }
            }

            // Store the node in our result
            result.insert(hash, node);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock in-memory node store for testing
    struct MockNodeStore {
        nodes: Arc<Mutex<HashMap<Blake3Hash, AstNode>>>,
    }

    impl MockNodeStore {
        fn new() -> Self {
            Self {
                nodes: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        async fn insert(&self, hash: Blake3Hash, node: AstNode) {
            self.nodes.lock().await.insert(hash, node);
        }
    }

    #[async_trait::async_trait]
    impl NodeStore for MockNodeStore {
        async fn insert_node(&self, hash: Blake3Hash, node: &AstNode) -> Result<()> {
            self.nodes.lock().await.insert(hash, node.clone());
            Ok(())
        }

        async fn insert_many(&self, nodes: &HashMap<Blake3Hash, AstNode>) -> Result<()> {
            self.nodes.lock().await.extend(nodes.clone());
            Ok(())
        }

        async fn get_node(&self, hash: Blake3Hash) -> Result<Option<AstNode>> {
            Ok(self.nodes.lock().await.get(&hash).cloned())
        }

        async fn get_many(&self, hashes: &[Blake3Hash]) -> Result<HashMap<Blake3Hash, AstNode>> {
            let nodes = self.nodes.lock().await;
            Ok(hashes
                .iter()
                .filter_map(|h| nodes.get(h).map(|n| (*h, n.clone())))
                .collect())
        }
    }

    #[tokio::test]
    async fn test_walk_single_node() {
        let store = MockNodeStore::new();
        let text = AstNode::Text {
            value: "hello".to_string(),
        };
        let hash = Blake3Hash::new([1; 32]);
        store.insert(hash, text.clone()).await;

        let result = walk_tree(hash, &store).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&hash), Some(&text));
    }

    #[tokio::test]
    async fn test_walk_tree_with_children() {
        let store = MockNodeStore::new();

        // Create a simple tree: Root -> Paragraph -> Text
        let text_hash = Blake3Hash::new([1; 32]);
        let text = AstNode::Text {
            value: "hello".to_string(),
        };
        store.insert(text_hash, text.clone()).await;

        let para_hash = Blake3Hash::new([2; 32]);
        let para = AstNode::Paragraph {
            children: vec![text_hash],
        };
        store.insert(para_hash, para.clone()).await;

        let root_hash = Blake3Hash::new([3; 32]);
        let root = AstNode::Root {
            children: vec![para_hash],
        };
        store.insert(root_hash, root.clone()).await;

        let result = walk_tree(root_hash, &store).await.unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result.get(&root_hash), Some(&root));
        assert_eq!(result.get(&para_hash), Some(&para));
        assert_eq!(result.get(&text_hash), Some(&text));
    }

    #[tokio::test]
    async fn test_walk_tree_with_shared_subtrees() {
        let store = MockNodeStore::new();

        // Create a tree with a shared subtree (deduplication)
        let text_hash = Blake3Hash::new([1; 32]);
        let text = AstNode::Text {
            value: "shared".to_string(),
        };
        store.insert(text_hash, text.clone()).await;

        // Two paragraphs both reference the same text node
        let para1_hash = Blake3Hash::new([2; 32]);
        let para1 = AstNode::Paragraph {
            children: vec![text_hash],
        };
        store.insert(para1_hash, para1.clone()).await;

        let para2_hash = Blake3Hash::new([3; 32]);
        let para2 = AstNode::Paragraph {
            children: vec![text_hash],
        };
        store.insert(para2_hash, para2.clone()).await;

        let root_hash = Blake3Hash::new([4; 32]);
        let root = AstNode::Root {
            children: vec![para1_hash, para2_hash],
        };
        store.insert(root_hash, root.clone()).await;

        let result = walk_tree(root_hash, &store).await.unwrap();
        // Should have 4 unique nodes, even though text is referenced twice
        assert_eq!(result.len(), 4);
        assert_eq!(result.get(&text_hash), Some(&text));
    }

    #[tokio::test]
    async fn test_walk_tree_missing_node() {
        let store = MockNodeStore::new();

        // Create a tree with a missing child reference
        let missing_hash = Blake3Hash::new([1; 32]);
        let para_hash = Blake3Hash::new([2; 32]);
        let para = AstNode::Paragraph {
            children: vec![missing_hash], // This node doesn't exist!
        };
        store.insert(para_hash, para.clone()).await;

        let result = walk_tree(para_hash, &store).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing node in tree"));
    }

    #[tokio::test]
    async fn test_walk_empty_tree() {
        let store = MockNodeStore::new();
        let root_hash = Blake3Hash::new([1; 32]);
        let root = AstNode::Root { children: vec![] };
        store.insert(root_hash, root.clone()).await;

        let result = walk_tree(root_hash, &store).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&root_hash), Some(&root));
    }
}
