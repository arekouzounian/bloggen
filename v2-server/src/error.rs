use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Post not found: {0}")]
    PostNotFound(String),

    #[error("Invalid slug: {0}")]
    InvalidSlug(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(ref e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            }
            AppError::Serialization(ref e) => {
                tracing::error!("Serialization error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error")
            }
            AppError::Compression(ref e) => {
                tracing::error!("Compression error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Compression error")
            }
            AppError::PostNotFound(_) => (StatusCode::NOT_FOUND, "Post not found"),
            AppError::InvalidSlug(_) => (StatusCode::BAD_REQUEST, "Invalid slug"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "Conflict detected"),
            AppError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "Invalid request"),
            AppError::Internal(ref e) => {
                tracing::error!("Internal error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = Json(json!({
            "error": error_message,
            "details": self.to_string(),
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AppError::PostNotFound("test-post".to_string());
        assert_eq!(err.to_string(), "Post not found: test-post");

        let err = AppError::InvalidSlug("bad slug!".to_string());
        assert_eq!(err.to_string(), "Invalid slug: bad slug!");

        let err = AppError::Conflict("version mismatch".to_string());
        assert_eq!(err.to_string(), "Conflict: version mismatch");
    }

    #[test]
    fn test_error_into_response() {
        let err = AppError::PostNotFound("test-post".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let err = AppError::InvalidSlug("bad".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let err = AppError::Conflict("conflict".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let err = AppError::Internal("oops".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_from_sqlx() {
        let sql_err = sqlx::Error::RowNotFound;
        let app_err: AppError = sql_err.into();
        assert!(matches!(app_err, AppError::Database(_)));
    }

    #[test]
    fn test_serialization_error() {
        let err = AppError::Serialization("bad data".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_compression_error() {
        let err = AppError::Compression("zstd failed".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_invalid_request_error() {
        let err = AppError::InvalidRequest("missing field".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
