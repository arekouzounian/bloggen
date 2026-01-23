//! BlogGen Client (bgc) - Content-Addressable Storage for Markdown
//!
//! This library provides a content-addressable storage system for markdown documents.
//! It parses markdown into an AST, stores nodes using Blake3 hashing, and enables
//! efficient deduplication and incremental updates.
//!
//! # Overview
//!
//! The workflow is:
//! 1. Parse markdown source using `markdown-rs`
//! 2. Convert to owned `AstNode` representation (excludes position data)
//! 3. Store nodes in a `NodeStore` with Blake3 content addressing
//! 4. Reference nodes by hash instead of storing them inline
//!
//! # Example
//!
//! ```
//! use bgc::{parse_markdown, NodeStore, CasDocument};
//!
//! let mut store = NodeStore::new();
//! let root_hash = parse_markdown("# Hello, world!", &mut store).unwrap();
//!
//! // Create a serializable document
//! let doc = CasDocument::new(&store, root_hash).unwrap();
//!
//! // Serialize to JSON
//! let json = doc.to_json_pretty().unwrap();
//! println!("{}", json);
//! ```

pub mod ast;
pub mod cas;
pub mod convert;
pub mod fuse;
pub mod http;
pub mod render;

// Re-export main types for convenience
pub use ast::{AstNode, Blake3Hash};
pub use cas::{CasDocument, CasNode, DeltaDocument, DeltaStats, NodeStore};
pub use convert::parse_markdown;
pub use http::Client;
pub use render::{MarkdownRenderer, RenderError};

/// Parse markdown from a file and return the root hash and populated NodeStore.
///
/// # Arguments
///
/// * `path` - Path to the markdown file
///
/// # Returns
///
/// A tuple of (root_hash, node_store) or an error
pub fn parse_markdown_file(
    path: &std::path::Path,
) -> Result<(Blake3Hash, NodeStore), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(path)?;
    let mut store = NodeStore::new();
    let root_hash = parse_markdown(&source, &mut store)
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
    Ok((root_hash, store))
}

/// Serialize a document to a JSON file.
///
/// # Arguments
///
/// * `doc` - The CasDocument to serialize
/// * `path` - Path to write the JSON file to
/// * `pretty` - Whether to use pretty-printing
pub fn write_json_file(
    doc: &CasDocument,
    path: &std::path::Path,
    pretty: bool,
) -> std::io::Result<()> {
    let json = if pretty {
        doc.to_json_pretty()
    } else {
        doc.to_json()
    }
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    std::fs::write(path, json)
}

pub fn write_msgpack_file(
    doc: &CasDocument,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let serialized = match doc.to_msgpack() {
        Ok(ser) => ser,
        Err(e) => return Err(Box::new(e)),
    };

    if let Err(e) = std::fs::write(path, serialized) {
        return Err(Box::new(e));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_markdown_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# Test\n\nHello, world!").unwrap();

        let (root_hash, store) = parse_markdown_file(file.path()).unwrap();

        assert!(store.contains(&root_hash));
        assert!(!store.is_empty());
    }

    #[test]
    fn test_write_json_file() {
        let mut store = NodeStore::new();
        let root_hash = parse_markdown("# Test", &mut store).unwrap();
        let doc = CasDocument::new(&store, root_hash).unwrap();

        let file = NamedTempFile::new().unwrap();
        write_json_file(&doc, file.path(), true).unwrap();

        // Verify file was written
        let content = std::fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("root_hash"));
        assert!(content.contains("nodes"));
    }
}
