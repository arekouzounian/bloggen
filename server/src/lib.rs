pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod models;
pub mod render;
pub mod renderer;
pub mod storage;

// Re-export shared AST types
pub use bgc_ast::{AstNode, Blake3Hash, ReferenceKind};
