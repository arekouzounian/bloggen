use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::{
    db::Database,
    models::{CasDocumentResponse, CreatePostRequest, DeltaUpdateRequest, ListPostsResponse},
    renderer,
};

pub type AppState = Arc<Database>;

/// Error response wrapper
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("Handler error: {:#}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal server error: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

/// POST /posts - Create a new post
pub async fn create_post(
    State(db): State<AppState>,
    Json(req): Json<CreatePostRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("Creating post: {}", req.slug);

    // Insert all nodes
    db.insert_nodes(&req.nodes).await?;

    // Increment ref counts for the tree
    db.increment_tree_refs(&req.ast_root).await?;

    // Create the post
    let post = db
        .create_post(&req.slug, req.title.as_deref(), &req.ast_root)
        .await?;

    // Create initial version
    db.create_version(post.id, &req.ast_root).await?;

    Ok(Json(serde_json::json!({
        "slug": post.slug,
        "ast_root": post.ast_root.to_hex(),
        "created_at": post.created_at.format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| post.created_at.to_string()),
    })))
}

/// GET /posts/:slug/ast - Get AST for a post
pub async fn get_post_ast(
    State(db): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<CasDocumentResponse>, AppError> {
    tracing::info!("Getting AST for post: {}", slug);

    let post = db
        .get_post(&slug)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    let nodes = db.walk_tree(&post.ast_root).await?;

    Ok(Json(CasDocumentResponse {
        root_hash: post.ast_root,
        nodes,
    }))
}

/// POST /posts/:slug/delta - Update post via delta
pub async fn update_post_delta(
    State(db): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<DeltaUpdateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!("Applying delta to post: {}", slug);

    // Get current post
    let post = db
        .get_post(&slug)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    // Verify old_root matches
    if post.ast_root != req.old_root {
        return Err(anyhow::anyhow!(
            "Conflict: old_root mismatch (expected {}, got {})",
            post.ast_root.to_hex(),
            req.old_root.to_hex()
        )
        .into());
    }

    // Insert new nodes
    db.insert_nodes(&req.added_nodes).await?;

    // Update reference counts
    db.increment_tree_refs(&req.new_root).await?;
    db.decrement_tree_refs(&req.old_root).await?;

    // Update post root
    let updated = db
        .update_post_root(&slug, &req.old_root, &req.new_root)
        .await?;

    if !updated {
        return Err(anyhow::anyhow!("Failed to update post (concurrent modification?)").into());
    }

    // Create version snapshot
    db.create_version(post.id, &req.new_root).await?;

    Ok(Json(serde_json::json!({
        "slug": slug,
        "old_root": req.old_root.to_hex(),
        "new_root": req.new_root.to_hex(),
        "nodes_added": req.added_nodes.len(),
        "nodes_removed": req.removed_hashes.len(),
    })))
}

/// GET /posts/:slug/markdown - Render post to markdown
pub async fn get_post_markdown(
    State(db): State<AppState>,
    Path(slug): Path<String>,
) -> Result<String, AppError> {
    tracing::info!("Rendering markdown for post: {}", slug);

    let post = db
        .get_post(&slug)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    let nodes = db.walk_tree(&post.ast_root).await?;

    let markdown = renderer::render_to_markdown(&post.ast_root, &nodes)?;

    Ok(markdown)
}

/// GET /posts - List all posts
pub async fn list_posts(State(db): State<AppState>) -> Result<Json<ListPostsResponse>, AppError> {
    tracing::info!("Listing all posts");

    let posts = db.list_posts().await?;
    let total = posts.len() as i64;

    Ok(Json(ListPostsResponse { posts, total }))
}

/// DELETE /posts/:slug - Delete a post
pub async fn delete_post(
    State(db): State<AppState>,
    Path(slug): Path<String>,
) -> Result<StatusCode, AppError> {
    tracing::info!("Deleting post: {}", slug);

    // Get post to decrement ref counts
    if let Some(post) = db.get_post(&slug).await? {
        db.decrement_tree_refs(&post.ast_root).await?;
    }

    let deleted = db.delete_post(&slug).await?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

/// Health check endpoint
pub async fn health() -> &'static str {
    "OK"
}
