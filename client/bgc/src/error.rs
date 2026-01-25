//! Error types for the BlogGen client library
//!
//! This module provides unified error handling for all operations in the library.

use std::fmt;

/// Main error type for the BlogGen client library
#[derive(Debug)]
pub enum Error {
    /// Error parsing markdown content
    Parse(ParseError),
    /// Error during serialization or deserialization
    Serialization(SerializationError),
    /// I/O error
    Io(std::io::Error),
    /// Error during rendering
    Render(crate::render::RenderError),
    /// Missing node in the content-addressable store
    MissingNode(crate::ast::Blake3Hash),
    /// Invalid hash format
    InvalidHash(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(e) => write!(f, "Parse error: {}", e),
            Error::Serialization(e) => write!(f, "Serialization error: {}", e),
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Render(e) => write!(f, "Render error: {}", e),
            Error::MissingNode(hash) => write!(f, "Missing node in store: {}", hash.to_hex()),
            Error::InvalidHash(msg) => write!(f, "Invalid hash: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Parse(e) => Some(e),
            Error::Serialization(e) => Some(e),
            Error::Io(e) => Some(e),
            Error::Render(e) => Some(e),
            Error::MissingNode(_) => None,
            Error::InvalidHash(_) => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<crate::render::RenderError> for Error {
    fn from(err: crate::render::RenderError) -> Self {
        Error::Render(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Error::Parse(err)
    }
}

impl From<SerializationError> for Error {
    fn from(err: SerializationError) -> Self {
        Error::Serialization(err)
    }
}

/// Error that occurs during markdown parsing
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Error that occurs during serialization or deserialization
#[derive(Debug)]
pub enum SerializationError {
    /// JSON serialization/deserialization error
    Json(serde_json::Error),
    /// MessagePack serialization error
    MsgPackEncode(rmp_serde::encode::Error),
    /// MessagePack deserialization error
    MsgPackDecode(rmp_serde::decode::Error),
    /// Compression error
    Compression(std::io::Error),
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SerializationError::Json(e) => write!(f, "JSON error: {}", e),
            SerializationError::MsgPackEncode(e) => write!(f, "MessagePack encode error: {}", e),
            SerializationError::MsgPackDecode(e) => write!(f, "MessagePack decode error: {}", e),
            SerializationError::Compression(e) => write!(f, "Compression error: {}", e),
        }
    }
}

impl std::error::Error for SerializationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SerializationError::Json(e) => Some(e),
            SerializationError::MsgPackEncode(e) => Some(e),
            SerializationError::MsgPackDecode(e) => Some(e),
            SerializationError::Compression(e) => Some(e),
        }
    }
}

impl From<serde_json::Error> for SerializationError {
    fn from(err: serde_json::Error) -> Self {
        SerializationError::Json(err)
    }
}

impl From<rmp_serde::encode::Error> for SerializationError {
    fn from(err: rmp_serde::encode::Error) -> Self {
        SerializationError::MsgPackEncode(err)
    }
}

impl From<rmp_serde::decode::Error> for SerializationError {
    fn from(err: rmp_serde::decode::Error) -> Self {
        SerializationError::MsgPackDecode(err)
    }
}

/// Result type using the library's Error type
pub type Result<T> = std::result::Result<T, Error>;
