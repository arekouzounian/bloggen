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
