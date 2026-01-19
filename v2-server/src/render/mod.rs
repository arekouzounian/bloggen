//! AST rendering modules for converting AST nodes to various output formats.
//!
//! This module provides renderers for converting content-addressed AST nodes
//! into different output formats:
//! - `markdown`: Renders AST back to markdown text (for FUSE reads, round-trip compatibility)
//! - `html`: Renders AST to semantic HTML (for web frontend, cached output)
//! - `html_extensions`: Extension system for customizing HTML rendering (Tailwind, syntax highlighting)

pub mod html;
pub mod html_extensions;
pub mod markdown;
