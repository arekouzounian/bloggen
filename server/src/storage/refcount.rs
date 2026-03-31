use crate::error::Result;
use crate::Blake3Hash;
use async_trait::async_trait;

/// Trait for reference counting operations on AST nodes
///
/// Reference counting is used to track which nodes are still in use by posts.
/// When a post is created or updated, ref counts are incremented for all nodes in the new tree.
/// When a post is updated or deleted, ref counts are decremented for nodes in the old tree.
/// Nodes with ref_count = 0 can be safely garbage collected.
///
/// Note: For tree-wide operations, use the helper functions `increment_tree_refs()`
/// and `decrement_tree_refs()` which combine tree walking with ref counting.
#[async_trait]
pub trait RefCountOps: Send + Sync {
    /// Increment reference counts for all nodes in the given set
    /// This is typically called when creating a post or updating to a new tree
    async fn increment_refs(&self, hashes: &[Blake3Hash]) -> Result<()>;

    /// Decrement reference counts for all nodes in the given set
    /// This is typically called when deleting a post or replacing an old tree
    async fn decrement_refs(&self, hashes: &[Blake3Hash]) -> Result<()>;
}

/// PostgreSQL implementation of RefCountOps
pub struct PostgresRefCount {
    pool: sqlx::PgPool,
}

impl PostgresRefCount {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefCountOps for PostgresRefCount {
    async fn increment_refs(&self, hashes: &[Blake3Hash]) -> Result<()> {
        if hashes.is_empty() {
            return Ok(());
        }

        let hash_bytes: Vec<Vec<u8>> = hashes.iter().map(|h| h.as_bytes().to_vec()).collect();

        sqlx::query(
            r#"
            UPDATE ast_nodes
            SET ref_count = ref_count + 1
            WHERE hash = ANY($1)
            "#,
        )
        .bind(&hash_bytes)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn decrement_refs(&self, hashes: &[Blake3Hash]) -> Result<()> {
        if hashes.is_empty() {
            return Ok(());
        }

        let hash_bytes: Vec<Vec<u8>> = hashes.iter().map(|h| h.as_bytes().to_vec()).collect();

        sqlx::query(
            r#"
            UPDATE ast_nodes
            SET ref_count = GREATEST(ref_count - 1, 0)
            WHERE hash = ANY($1)
            "#,
        )
        .bind(&hash_bytes)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Helper function to increment ref counts for an entire tree
///
/// This combines tree walking with reference counting in a single operation.
/// It walks the tree from the root, collecting all node hashes, then increments
/// their ref counts in a single batch operation.
///
/// # Transaction Safety
/// This should typically be called within a database transaction to ensure
/// atomicity with post creation/update operations.
pub async fn increment_tree_refs(
    root_hash: Blake3Hash,
    store: &impl super::NodeStore,
    refcount: &impl RefCountOps,
) -> Result<()> {
    // Walk the tree to collect all hashes
    let nodes = super::walk_tree(root_hash, store).await?;
    let hashes: Vec<Blake3Hash> = nodes.keys().copied().collect();

    // Increment ref counts for all nodes
    refcount.increment_refs(&hashes).await?;

    Ok(())
}

/// Helper function to decrement ref counts for an entire tree
///
/// This combines tree walking with reference counting in a single operation.
/// It walks the tree from the root, collecting all node hashes, then decrements
/// their ref counts in a single batch operation.
///
/// # Transaction Safety
/// This should typically be called within a database transaction to ensure
/// atomicity with post deletion/update operations.
pub async fn decrement_tree_refs(
    root_hash: Blake3Hash,
    store: &impl super::NodeStore,
    refcount: &impl RefCountOps,
) -> Result<()> {
    // Walk the tree to collect all hashes
    let nodes = super::walk_tree(root_hash, store).await?;
    let hashes: Vec<Blake3Hash> = nodes.keys().copied().collect();

    // Decrement ref counts for all nodes
    refcount.decrement_refs(&hashes).await?;

    Ok(())
}

// Tests for reference counting are integration tests that require
// a live database. The API correctness is verified through the
// helper functions increment_tree_refs and decrement_tree_refs
// which are used in Phase 4 (HTTP API) tests.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::{AstNode, Blake3Hash};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tokio::sync::Mutex as AsyncMutex;

    // ---- Mock NodeStore --------------------------------------------------------

    struct MockNodeStore {
        nodes: Arc<AsyncMutex<HashMap<Blake3Hash, AstNode>>>,
    }

    impl MockNodeStore {
        fn new() -> Self {
            Self {
                nodes: Arc::new(AsyncMutex::new(HashMap::new())),
            }
        }

        async fn insert(&self, hash: Blake3Hash, node: AstNode) {
            self.nodes.lock().await.insert(hash, node);
        }
    }

    #[async_trait]
    impl super::super::NodeStore for MockNodeStore {
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

        async fn get_many(
            &self,
            hashes: &[Blake3Hash],
        ) -> Result<HashMap<Blake3Hash, AstNode>> {
            let nodes = self.nodes.lock().await;
            Ok(hashes
                .iter()
                .filter_map(|h| nodes.get(h).map(|n| (*h, n.clone())))
                .collect())
        }
    }

    // ---- Mock RefCountOps ------------------------------------------------------

    struct MockRefCount {
        incremented: Arc<Mutex<Vec<Blake3Hash>>>,
        decremented: Arc<Mutex<Vec<Blake3Hash>>>,
    }

    impl MockRefCount {
        fn new() -> Self {
            Self {
                incremented: Arc::new(Mutex::new(Vec::new())),
                decremented: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl RefCountOps for MockRefCount {
        async fn increment_refs(&self, hashes: &[Blake3Hash]) -> Result<()> {
            self.incremented.lock().unwrap().extend_from_slice(hashes);
            Ok(())
        }

        async fn decrement_refs(&self, hashes: &[Blake3Hash]) -> Result<()> {
            self.decremented.lock().unwrap().extend_from_slice(hashes);
            Ok(())
        }
    }

    // ---- Helpers ---------------------------------------------------------------

    fn make_hash(byte: u8) -> Blake3Hash {
        Blake3Hash::new([byte; 32])
    }

    // ---- Tests -----------------------------------------------------------------

    #[tokio::test]
    async fn test_increment_tree_refs_single_node() {
        let store = MockNodeStore::new();
        let refcount = MockRefCount::new();

        let root_hash = make_hash(1);
        store
            .insert(root_hash, AstNode::Root { children: vec![] })
            .await;

        increment_tree_refs(root_hash, &store, &refcount)
            .await
            .unwrap();

        let incremented = refcount.incremented.lock().unwrap();
        assert_eq!(incremented.len(), 1);
        assert!(incremented.contains(&root_hash));
    }

    #[tokio::test]
    async fn test_increment_tree_refs_whole_tree() {
        let store = MockNodeStore::new();
        let refcount = MockRefCount::new();

        // Build: Root → Paragraph → Text
        let text_hash = make_hash(1);
        store
            .insert(
                text_hash,
                AstNode::Text {
                    value: "hello".into(),
                },
            )
            .await;

        let para_hash = make_hash(2);
        store
            .insert(
                para_hash,
                AstNode::Paragraph {
                    children: vec![text_hash],
                },
            )
            .await;

        let root_hash = make_hash(3);
        store
            .insert(
                root_hash,
                AstNode::Root {
                    children: vec![para_hash],
                },
            )
            .await;

        increment_tree_refs(root_hash, &store, &refcount)
            .await
            .unwrap();

        let incremented = refcount.incremented.lock().unwrap();
        assert_eq!(incremented.len(), 3);
        assert!(incremented.contains(&root_hash));
        assert!(incremented.contains(&para_hash));
        assert!(incremented.contains(&text_hash));
    }

    #[tokio::test]
    async fn test_decrement_tree_refs_whole_tree() {
        let store = MockNodeStore::new();
        let refcount = MockRefCount::new();

        let text_hash = make_hash(10);
        store
            .insert(
                text_hash,
                AstNode::Text {
                    value: "bye".into(),
                },
            )
            .await;

        let root_hash = make_hash(11);
        store
            .insert(
                root_hash,
                AstNode::Root {
                    children: vec![text_hash],
                },
            )
            .await;

        decrement_tree_refs(root_hash, &store, &refcount)
            .await
            .unwrap();

        let decremented = refcount.decremented.lock().unwrap();
        assert_eq!(decremented.len(), 2);
        assert!(decremented.contains(&root_hash));
        assert!(decremented.contains(&text_hash));
    }

    #[tokio::test]
    async fn test_increment_tree_refs_missing_node_returns_error() {
        let store = MockNodeStore::new();
        let refcount = MockRefCount::new();

        // Root references a child that doesn't exist in the store
        let missing_hash = make_hash(99);
        let root_hash = make_hash(100);
        store
            .insert(
                root_hash,
                AstNode::Root {
                    children: vec![missing_hash],
                },
            )
            .await;

        let result = increment_tree_refs(root_hash, &store, &refcount).await;
        assert!(result.is_err());
        // No refs should have been incremented since walk_tree failed
        let incremented = refcount.incremented.lock().unwrap();
        assert!(incremented.is_empty());
    }

    #[tokio::test]
    async fn test_ref_count_ops_empty_slice() {
        // Implementations should treat empty slices as no-ops
        let refcount = MockRefCount::new();
        refcount.increment_refs(&[]).await.unwrap();
        refcount.decrement_refs(&[]).await.unwrap();

        assert!(refcount.incremented.lock().unwrap().is_empty());
        assert!(refcount.decremented.lock().unwrap().is_empty());
    }
}
