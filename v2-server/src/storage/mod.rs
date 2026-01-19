use crate::error::Result;
use crate::{AstNode, Blake3Hash};
use async_trait::async_trait;
use std::collections::HashMap;

pub mod gc;
pub mod refcount;
pub mod walker;

pub use gc::{start_gc_background_task, GarbageCollector, PostgresGarbageCollector};
pub use refcount::{decrement_tree_refs, increment_tree_refs, PostgresRefCount, RefCountOps};
pub use walker::walk_tree;

/// Trait for content-addressable AST node storage
#[async_trait]
pub trait NodeStore: Send + Sync {
    /// Insert a single node into the store
    /// Uses ON CONFLICT DO NOTHING to handle deduplication automatically
    async fn insert_node(&self, hash: Blake3Hash, node: &AstNode) -> Result<()>;

    /// Insert multiple nodes in a single transaction
    /// More efficient than calling insert_node multiple times
    async fn insert_many(&self, nodes: &HashMap<Blake3Hash, AstNode>) -> Result<()>;

    /// Get a single node by hash
    /// Returns None if the node doesn't exist
    async fn get_node(&self, hash: Blake3Hash) -> Result<Option<AstNode>>;

    /// Get multiple nodes by hash in a single query
    /// Returns a map of hash -> node for all found nodes
    /// Missing nodes are simply not included in the result
    async fn get_many(&self, hashes: &[Blake3Hash]) -> Result<HashMap<Blake3Hash, AstNode>>;
}

/// PostgreSQL implementation of NodeStore
pub struct PostgresNodeStore {
    pool: sqlx::PgPool,
}

impl PostgresNodeStore {
    /// Create a new PostgresNodeStore from a connection pool
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}

#[async_trait]
impl NodeStore for PostgresNodeStore {
    async fn insert_node(&self, hash: Blake3Hash, node: &AstNode) -> Result<()> {
        let node_json = serde_json::to_value(node)
            .map_err(|e| crate::error::AppError::Serialization(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO ast_nodes (hash, node_data, ref_count)
            VALUES ($1, $2, 0)
            ON CONFLICT (hash) DO NOTHING
            "#,
        )
        .bind(hash.as_bytes() as &[u8])
        .bind(node_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert_many(&self, nodes: &HashMap<Blake3Hash, AstNode>) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        // Build arrays for bulk insert
        let mut hashes = Vec::with_capacity(nodes.len());
        let mut node_data = Vec::with_capacity(nodes.len());

        for (hash, node) in nodes {
            hashes.push(hash.as_bytes().to_vec());
            let json = serde_json::to_value(node)
                .map_err(|e| crate::error::AppError::Serialization(e.to_string()))?;
            node_data.push(json);
        }

        // Use UNNEST to insert all rows in a single query
        sqlx::query(
            r#"
            INSERT INTO ast_nodes (hash, node_data, ref_count)
            SELECT * FROM UNNEST($1::BYTEA[], $2::JSONB[]) AS t(hash, node_data)
            ON CONFLICT (hash) DO NOTHING
            "#,
        )
        .bind(&hashes)
        .bind(&node_data)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_node(&self, hash: Blake3Hash) -> Result<Option<AstNode>> {
        let row: Option<(sqlx::types::JsonValue,)> = sqlx::query_as(
            r#"
            SELECT node_data
            FROM ast_nodes
            WHERE hash = $1
            "#,
        )
        .bind(hash.as_bytes() as &[u8])
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((node_data,)) => {
                let node: AstNode = serde_json::from_value(node_data)
                    .map_err(|e| crate::error::AppError::Serialization(e.to_string()))?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    async fn get_many(&self, hashes: &[Blake3Hash]) -> Result<HashMap<Blake3Hash, AstNode>> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }

        let hash_bytes: Vec<Vec<u8>> = hashes.iter().map(|h| h.as_bytes().to_vec()).collect();

        let rows: Vec<(Vec<u8>, sqlx::types::JsonValue)> = sqlx::query_as(
            r#"
            SELECT hash, node_data
            FROM ast_nodes
            WHERE hash = ANY($1)
            "#,
        )
        .bind(&hash_bytes)
        .fetch_all(&self.pool)
        .await?;

        let mut result = HashMap::with_capacity(rows.len());
        for (hash_vec, node_data) in rows {
            // Convert the hash bytes back to Blake3Hash
            let hash_bytes: [u8; 32] = hash_vec.try_into().map_err(|_| {
                crate::error::AppError::Internal("Invalid hash length in database".to_string())
            })?;
            let hash = Blake3Hash::new(hash_bytes);

            // Deserialize the node
            let node: AstNode = serde_json::from_value(node_data)
                .map_err(|e| crate::error::AppError::Serialization(e.to_string()))?;

            result.insert(hash, node);
        }

        Ok(result)
    }
}

// Tests for this module are in walker.rs, as the NodeStore trait
// is primarily exercised through tree walking operations.
