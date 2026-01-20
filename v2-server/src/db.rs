use anyhow::{anyhow, Context, Result};
use bgc_ast::{AstNode, Blake3Hash};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};

use crate::models::{Post, PostSummary, PostVersion};

/// Database operations for content-addressable storage
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert an AST node into the database (idempotent)
    pub async fn insert_node(&self, hash: &Blake3Hash, node: &AstNode) -> Result<()> {
        let hash_bytes = hash.as_bytes();
        let node_json = serde_json::to_value(node)?;

        sqlx::query(
            "INSERT INTO ast_nodes (hash, node_data, ref_count)
             VALUES ($1, $2, 0)
             ON CONFLICT (hash) DO NOTHING",
        )
        .bind(hash_bytes.as_slice())
        .bind(&node_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert multiple nodes in a transaction
    pub async fn insert_nodes(&self, nodes: &HashMap<Blake3Hash, AstNode>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for (hash, node) in nodes {
            let hash_bytes = hash.as_bytes();
            let node_json = serde_json::to_value(node)?;

            sqlx::query(
                "INSERT INTO ast_nodes (hash, node_data, ref_count)
                 VALUES ($1, $2, 0)
                 ON CONFLICT (hash) DO NOTHING",
            )
            .bind(hash_bytes.as_slice())
            .bind(&node_json)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Retrieve an AST node by hash
    pub async fn get_node(&self, hash: &Blake3Hash) -> Result<Option<AstNode>> {
        let hash_bytes = hash.as_bytes();

        let row: Option<PgRow> = sqlx::query("SELECT node_data FROM ast_nodes WHERE hash = $1")
            .bind(hash_bytes.as_slice())
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let node_json: serde_json::Value = row.get("node_data");
                let node: AstNode = serde_json::from_value(node_json)?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    /// Walk the tree from a root hash and collect all nodes
    ///
    /// Uses batched fetching to avoid the N+1 query problem.
    /// Fetches nodes in waves: fetch current level, extract child hashes,
    /// batch-fetch all children, repeat. For a 1000-node tree with depth 10,
    /// this reduces from 1000 queries to ~10 queries.
    pub async fn walk_tree(&self, root_hash: &Blake3Hash) -> Result<HashMap<Blake3Hash, AstNode>> {
        let mut nodes = HashMap::new();
        let mut visited = HashSet::new();
        let mut current_level = vec![*root_hash];

        while !current_level.is_empty() {
            // Batch fetch all nodes at current level
            let fetched = self.get_nodes_batch(&current_level).await?;

            let mut next_level = Vec::new();

            for (hash, node) in fetched {
                if visited.contains(&hash) {
                    continue;
                }
                visited.insert(hash);

                // Collect children for next level
                for child_hash in node.children() {
                    if !visited.contains(child_hash) {
                        next_level.push(*child_hash);
                    }
                }

                nodes.insert(hash, node);
            }

            current_level = next_level;
        }

        if nodes.is_empty() {
            return Err(anyhow!("Root node not found: {}", root_hash.to_hex()));
        }

        Ok(nodes)
    }

    /// Batch fetch multiple nodes by their hashes
    async fn get_nodes_batch(&self, hashes: &[Blake3Hash]) -> Result<Vec<(Blake3Hash, AstNode)>> {
        if hashes.is_empty() {
            return Ok(Vec::new());
        }

        // Convert hashes to byte arrays for query
        let hash_bytes: Vec<Vec<u8>> = hashes
            .iter()
            .map(|h| h.as_bytes().to_vec())
            .collect();

        let rows: Vec<PgRow> = sqlx::query(
            "SELECT hash, node_data FROM ast_nodes WHERE hash = ANY($1)"
        )
        .bind(&hash_bytes)
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            let hash_bytes: Vec<u8> = row.get("hash");
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&hash_bytes);
            let hash = Blake3Hash::new(hash_array);

            let node_json: serde_json::Value = row.get("node_data");
            let node: AstNode = serde_json::from_value(node_json)?;

            result.push((hash, node));
        }

        Ok(result)
    }

    /// Increment reference count for a node
    pub async fn increment_ref_count(&self, hash: &Blake3Hash) -> Result<()> {
        let hash_bytes = hash.as_bytes();

        sqlx::query("UPDATE ast_nodes SET ref_count = ref_count + 1 WHERE hash = $1")
            .bind(hash_bytes.as_slice())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Decrement reference count for a node
    pub async fn decrement_ref_count(&self, hash: &Blake3Hash) -> Result<()> {
        let hash_bytes = hash.as_bytes();

        sqlx::query("UPDATE ast_nodes SET ref_count = ref_count - 1 WHERE hash = $1")
            .bind(hash_bytes.as_slice())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Increment ref counts for all nodes in a tree
    ///
    /// Uses a single batched UPDATE to modify all nodes at once.
    /// For a 1000-node tree, this reduces from 1000 UPDATEs to 1 UPDATE.
    pub async fn increment_tree_refs(&self, root_hash: &Blake3Hash) -> Result<()> {
        let nodes = self.walk_tree(root_hash).await?;

        if nodes.is_empty() {
            return Ok(());
        }

        // Batch update all ref counts in one query
        let hash_bytes: Vec<Vec<u8>> = nodes
            .keys()
            .map(|h| h.as_bytes().to_vec())
            .collect();

        sqlx::query("UPDATE ast_nodes SET ref_count = ref_count + 1 WHERE hash = ANY($1)")
            .bind(&hash_bytes)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Decrement ref counts for all nodes in a tree
    ///
    /// Uses a single batched UPDATE to modify all nodes at once.
    /// For a 1000-node tree, this reduces from 1000 UPDATEs to 1 UPDATE.
    pub async fn decrement_tree_refs(&self, root_hash: &Blake3Hash) -> Result<()> {
        let nodes = self.walk_tree(root_hash).await?;

        if nodes.is_empty() {
            return Ok(());
        }

        // Batch update all ref counts in one query
        let hash_bytes: Vec<Vec<u8>> = nodes
            .keys()
            .map(|h| h.as_bytes().to_vec())
            .collect();

        sqlx::query("UPDATE ast_nodes SET ref_count = ref_count - 1 WHERE hash = ANY($1)")
            .bind(&hash_bytes)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update reference counts when transitioning from old_root to new_root
    ///
    /// This is much more efficient than decrement_tree_refs(old) + increment_tree_refs(new)
    /// because it only updates nodes that actually changed. For a typical edit where
    /// 95% of nodes are shared, this reduces from 2000 UPDATEs to ~100 UPDATEs.
    pub async fn update_tree_refs_delta(
        &self,
        old_root: &Blake3Hash,
        new_root: &Blake3Hash,
    ) -> Result<()> {
        // If roots are the same, no changes needed
        if old_root == new_root {
            return Ok(());
        }

        // Walk both trees
        let old_nodes = self.walk_tree(old_root).await?;
        let new_nodes = self.walk_tree(new_root).await?;

        let old_hashes: HashSet<_> = old_nodes.keys().copied().collect();
        let new_hashes: HashSet<_> = new_nodes.keys().copied().collect();

        // Compute delta
        let added: Vec<_> = new_hashes.difference(&old_hashes).copied().collect();
        let removed: Vec<_> = old_hashes.difference(&new_hashes).copied().collect();

        // Batch increment new nodes
        if !added.is_empty() {
            let hash_bytes: Vec<Vec<u8>> = added
                .iter()
                .map(|h| h.as_bytes().to_vec())
                .collect();

            sqlx::query("UPDATE ast_nodes SET ref_count = ref_count + 1 WHERE hash = ANY($1)")
                .bind(&hash_bytes)
                .execute(&self.pool)
                .await?;
        }

        // Batch decrement removed nodes
        if !removed.is_empty() {
            let hash_bytes: Vec<Vec<u8>> = removed
                .iter()
                .map(|h| h.as_bytes().to_vec())
                .collect();

            sqlx::query("UPDATE ast_nodes SET ref_count = ref_count - 1 WHERE hash = ANY($1)")
                .bind(&hash_bytes)
                .execute(&self.pool)
                .await?;
        }

        tracing::debug!(
            "Delta refcount update: {} added, {} removed, {} shared",
            added.len(),
            removed.len(),
            old_hashes.intersection(&new_hashes).count()
        );

        Ok(())
    }

    /// Create a new post
    pub async fn create_post(
        &self,
        slug: &str,
        title: Option<&str>,
        ast_root: &Blake3Hash,
    ) -> Result<Post> {
        let root_bytes = ast_root.as_bytes();

        let row: PgRow = sqlx::query(
            "INSERT INTO posts (slug, title, ast_root)
             VALUES ($1, $2, $3)
             RETURNING id, slug, ast_root, title, created_at, updated_at, published",
        )
        .bind(slug)
        .bind(title)
        .bind(root_bytes.as_slice())
        .fetch_one(&self.pool)
        .await
        .context("Failed to create post")?;

        row_to_post(row)
    }

    /// Get a post by slug
    pub async fn get_post(&self, slug: &str) -> Result<Option<Post>> {
        let row: Option<PgRow> = sqlx::query(
            "SELECT id, slug, ast_root, title, created_at, updated_at, published
             FROM posts WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_post(row)?)),
            None => Ok(None),
        }
    }

    /// Update a post's root hash
    pub async fn update_post_root(
        &self,
        slug: &str,
        old_root: &Blake3Hash,
        new_root: &Blake3Hash,
    ) -> Result<bool> {
        let old_bytes = old_root.as_bytes();
        let new_bytes = new_root.as_bytes();

        let result = sqlx::query(
            "UPDATE posts
             SET ast_root = $1, html_cache = NULL, html_cache_updated_at = NULL
             WHERE slug = $2 AND ast_root = $3",
        )
        .bind(new_bytes.as_slice())
        .bind(slug)
        .bind(old_bytes.as_slice())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List all posts
    pub async fn list_posts(&self) -> Result<Vec<PostSummary>> {
        let rows: Vec<PgRow> = sqlx::query(
            "SELECT slug, title, created_at, updated_at, published
             FROM posts
             ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut posts = Vec::new();
        for row in rows {
            posts.push(PostSummary {
                slug: row.get("slug"),
                title: row.get("title"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                published: row.get("published"),
            });
        }

        Ok(posts)
    }

    /// Create a version snapshot
    pub async fn create_version(&self, post_id: i32, ast_root: &Blake3Hash) -> Result<PostVersion> {
        let root_bytes = ast_root.as_bytes();

        // Get next version number
        let next_version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) + 1
             FROM post_versions WHERE post_id = $1",
        )
        .bind(post_id)
        .fetch_one(&self.pool)
        .await?;

        let row: PgRow = sqlx::query(
            "INSERT INTO post_versions (post_id, version_number, ast_root)
             VALUES ($1, $2, $3)
             RETURNING id, post_id, version_number, ast_root, created_at",
        )
        .bind(post_id)
        .bind(next_version)
        .bind(root_bytes.as_slice())
        .fetch_one(&self.pool)
        .await?;

        row_to_version(row)
    }

    /// Delete a post and its versions
    pub async fn delete_post(&self, slug: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM posts WHERE slug = $1")
            .bind(slug)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

fn row_to_post(row: PgRow) -> Result<Post> {
    let root_bytes: Vec<u8> = row.get("ast_root");
    let mut root_array = [0u8; 32];
    root_array.copy_from_slice(&root_bytes);

    Ok(Post {
        id: row.get("id"),
        slug: row.get("slug"),
        ast_root: Blake3Hash::new(root_array),
        title: row.get("title"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        published: row.get("published"),
    })
}

fn row_to_version(row: PgRow) -> Result<PostVersion> {
    let root_bytes: Vec<u8> = row.get("ast_root");
    let mut root_array = [0u8; 32];
    root_array.copy_from_slice(&root_bytes);

    Ok(PostVersion {
        id: row.get("id"),
        post_id: row.get("post_id"),
        version_number: row.get("version_number"),
        ast_root: Blake3Hash::new(root_array),
        created_at: row.get("created_at"),
    })
}
