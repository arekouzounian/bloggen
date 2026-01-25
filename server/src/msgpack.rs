//! MessagePack extractor and response types for Axum
//!
//! This module provides MessagePack support for the server, similar to Axum's built-in JSON support.
//! It includes extractors for request bodies and response types for MessagePack-encoded data.

use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{de::DeserializeOwned, Serialize};

/// MessagePack extractor for request bodies
///
/// Similar to Axum's `Json<T>`, this extracts and deserializes MessagePack data from request bodies.
///
/// # Example
/// ```ignore
/// async fn handler(MsgPack(payload): MsgPack<MyRequest>) -> MsgPack<MyResponse> {
///     // Use payload...
///     MsgPack(response)
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct MsgPack<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for MsgPack<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = MsgPackRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|err| MsgPackRejection::BytesRejection(err.to_string()))?;

        let value = rmp_serde::from_slice(&bytes).map_err(|err| {
            MsgPackRejection::DeserializationError(format!("Failed to deserialize MessagePack: {}", err))
        })?;

        Ok(MsgPack(value))
    }
}

impl<T> IntoResponse for MsgPack<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match rmp_serde::to_vec_named(&self.0) {
            Ok(bytes) => (
                [(header::CONTENT_TYPE, "application/msgpack")],
                bytes,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize MessagePack: {}", err),
            )
                .into_response(),
        }
    }
}

/// Rejection type for MessagePack extraction errors
#[derive(Debug)]
pub enum MsgPackRejection {
    BytesRejection(String),
    DeserializationError(String),
}

impl IntoResponse for MsgPackRejection {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            MsgPackRejection::BytesRejection(err) => {
                (StatusCode::BAD_REQUEST, format!("Failed to read request body: {}", err))
            }
            MsgPackRejection::DeserializationError(err) => {
                (StatusCode::BAD_REQUEST, format!("Invalid MessagePack: {}", err))
            }
        };

        (status, message).into_response()
    }
}

impl std::fmt::Display for MsgPackRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MsgPackRejection::BytesRejection(err) => write!(f, "Failed to read request body: {}", err),
            MsgPackRejection::DeserializationError(err) => write!(f, "Failed to deserialize MessagePack: {}", err),
        }
    }
}

impl std::error::Error for MsgPackRejection {}

/// Helper function to create a MessagePack response from any serializable value
///
/// This is useful when you want to return a MessagePack response without wrapping in `MsgPack<T>`
pub fn msgpack_response<T: Serialize>(value: &T) -> Result<Response, String> {
    match rmp_serde::to_vec_named(value) {
        Ok(bytes) => Ok((
            [(header::CONTENT_TYPE, "application/msgpack")],
            bytes,
        )
            .into_response()),
        Err(err) => Err(format!("Failed to serialize MessagePack: {}", err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        message: String,
        count: i32,
    }

    #[tokio::test]
    async fn test_msgpack_extraction() {
        let data = TestData {
            message: "Hello".to_string(),
            count: 42,
        };

        let bytes = rmp_serde::to_vec(&data).unwrap();
        let req = Request::builder()
            .header("content-type", "application/msgpack")
            .body(Body::from(bytes))
            .unwrap();

        let result = MsgPack::<TestData>::from_request(req, &()).await;
        assert!(result.is_ok());
        let MsgPack(extracted) = result.unwrap();
        assert_eq!(extracted, data);
    }

    #[tokio::test]
    async fn test_msgpack_response() {
        let data = TestData {
            message: "World".to_string(),
            count: 99,
        };

        let response = MsgPack(data.clone()).into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response.headers().get(header::CONTENT_TYPE);
        assert_eq!(content_type.unwrap(), "application/msgpack");
    }

    #[tokio::test]
    async fn test_invalid_msgpack() {
        let invalid_bytes = vec![0xFF, 0xFF, 0xFF]; // Invalid MessagePack
        let req = Request::builder()
            .header("content-type", "application/msgpack")
            .body(Body::from(invalid_bytes))
            .unwrap();

        let result = MsgPack::<TestData>::from_request(req, &()).await;
        assert!(result.is_err());
    }
}
