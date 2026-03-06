//! Markdown rendering for content-addressable AST nodes
//!
//! This module provides rendering of AST back to markdown format. The renderer
//! aims for deterministic output - the same AST should always produce the same
//! markdown. However, it may not produce byte-identical output to the original
//! source (e.g., different heading styles, spacing).
//!
//! The goal is semantic equivalence: markdown → AST → markdown should produce
//! functionally equivalent documents.

use crate::{AstNode, Blake3Hash};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

/// Renders an AST back to markdown format
pub struct MarkdownRenderer<'a> {
    /// Map of hash to node for resolving references
    nodes: &'a HashMap<Blake3Hash, AstNode>,
    /// Output buffer
    output: String,
    /// Current list depth (for proper indentation)
    list_depth: usize,
}

impl<'a> MarkdownRenderer<'a> {
    /// Create a new renderer with a node map
    pub fn new(nodes: &'a HashMap<Blake3Hash, AstNode>) -> Self {
        Self {
            nodes,
            output: String::new(),
            list_depth: 0,
        }
    }

    /// Render a node and return the markdown string
    pub fn render(&mut self, node: &AstNode) -> Result<String, RenderError> {
        self.output.clear();
        self.render_node(node)?;
        Ok(self.output.clone())
    }

    /// Internal recursive rendering function
    fn render_node(&mut self, node: &AstNode) -> Result<(), RenderError> {
        match node {
            AstNode::Root { children } => {
                self.render_block_children(children)?;
            }
            AstNode::Heading { level, children } => {
                // Use ATX-style headings (##)
                for _ in 0..*level {
                    self.output.push('#');
                }
                self.output.push(' ');
                self.render_children(children)?;
                self.output.push('\n');
            }
            AstNode::Paragraph { children } => {
                self.render_children(children)?;
                self.output.push('\n');
            }
            AstNode::ThematicBreak => {
                self.output.push_str("---\n");
            }
            AstNode::CodeBlock { lang, value } => {
                self.output.push_str("```");
                if let Some(language) = lang {
                    self.output.push_str(language);
                }
                self.output.push('\n');
                self.output.push_str(value);
                if !value.ends_with('\n') {
                    self.output.push('\n');
                }
                self.output.push_str("```\n");
            }
            AstNode::Blockquote { children } => {
                // Save current output and render children to a temporary buffer
                let saved_output = std::mem::take(&mut self.output);
                self.render_block_children(children)?;
                let inner_content = std::mem::replace(&mut self.output, saved_output);

                // Prefix each line with "> "
                for line in inner_content.lines() {
                    self.output.push_str("> ");
                    self.output.push_str(line);
                    self.output.push('\n');
                }
            }
            AstNode::List {
                ordered,
                start,
                children,
            } => {
                self.list_depth += 1;
                for (i, child_hash) in children.iter().enumerate() {
                    let child = self
                        .nodes
                        .get(child_hash)
                        .ok_or(RenderError::MissingNode(*child_hash))?;

                    // Render list marker with proper indentation
                    let indent = "  ".repeat(self.list_depth.saturating_sub(1));
                    self.output.push_str(&indent);

                    if *ordered {
                        let num = start.unwrap_or(1) + i as u32;
                        write!(&mut self.output, "{}. ", num).unwrap();
                    } else {
                        self.output.push_str("- ");
                    }

                    // Render list item content
                    if let AstNode::ListItem { checked, children } = child {
                        // Task list checkbox if present
                        if let Some(is_checked) = checked {
                            if *is_checked {
                                self.output.push_str("[x] ");
                            } else {
                                self.output.push_str("[ ] ");
                            }
                        }
                        self.render_list_item_children(children)?;
                    } else {
                        return Err(RenderError::UnsupportedNode(
                            "List children must be ListItems".to_string(),
                        ));
                    }

                    self.output.push('\n');
                }
                self.list_depth -= 1;
            }
            AstNode::ListItem { .. } => {
                // ListItems are handled by their parent List node
                return Err(RenderError::UnsupportedNode(
                    "ListItem must be rendered as part of a List".to_string(),
                ));
            }
            AstNode::Text { value } => {
                self.output.push_str(value);
            }
            AstNode::InlineCode { value } => {
                write!(&mut self.output, "`{}`", value).unwrap();
            }
            AstNode::Break => {
                self.output.push_str("  \n");
            }
            AstNode::Strong { children } => {
                self.output.push_str("**");
                self.render_children(children)?;
                self.output.push_str("**");
            }
            AstNode::Emphasis { children } => {
                self.output.push('*');
                self.render_children(children)?;
                self.output.push('*');
            }
            AstNode::Delete { children } => {
                self.output.push_str("~~");
                self.render_children(children)?;
                self.output.push_str("~~");
            }
            AstNode::Link {
                url,
                title,
                children,
            } => {
                self.output.push('[');
                self.render_children(children)?;
                self.output.push_str("](");
                self.output.push_str(url);
                if let Some(t) = title {
                    write!(&mut self.output, " \"{}\"", escape_quotes(t)).unwrap();
                }
                self.output.push(')');
            }
            AstNode::Image { url, alt, title } => {
                self.output.push_str("![");
                self.output.push_str(alt);
                self.output.push_str("](");
                self.output.push_str(url);
                if let Some(t) = title {
                    write!(&mut self.output, " \"{}\"", escape_quotes(t)).unwrap();
                }
                self.output.push(')');
            }
            AstNode::Table { children } => {
                for (i, row_hash) in children.iter().enumerate() {
                    let row = self
                        .nodes
                        .get(row_hash)
                        .ok_or(RenderError::MissingNode(*row_hash))?;

                    if let AstNode::TableRow { children: cells } = row {
                        self.output.push('|');
                        for cell_hash in cells {
                            let cell = self
                                .nodes
                                .get(cell_hash)
                                .ok_or(RenderError::MissingNode(*cell_hash))?;

                            if let AstNode::TableCell { children } = cell {
                                self.output.push(' ');
                                self.render_children(children)?;
                                self.output.push_str(" |");
                            }
                        }
                        self.output.push('\n');

                        // Add separator row after header (first row)
                        if i == 0 {
                            self.output.push('|');
                            for _ in 0..cells.len() {
                                self.output.push_str(" --- |");
                            }
                            self.output.push('\n');
                        }
                    }
                }
            }
            AstNode::TableRow { .. } | AstNode::TableCell { .. } => {
                // These are handled by Table rendering
                return Err(RenderError::UnsupportedNode(
                    "TableRow/TableCell must be rendered as part of a Table".to_string(),
                ));
            }
            // Raw HTML pass-through
            AstNode::Html { value } => {
                self.output.push_str(value);
            }
            // Frontmatter
            AstNode::Yaml { value } => {
                self.output.push_str("---\n");
                self.output.push_str(value);
                if !value.ends_with('\n') {
                    self.output.push('\n');
                }
                self.output.push_str("---\n");
            }
            AstNode::Toml { value } => {
                self.output.push_str("+++\n");
                self.output.push_str(value);
                if !value.ends_with('\n') {
                    self.output.push('\n');
                }
                self.output.push_str("+++\n");
            }
            // Math
            AstNode::Math { value } => {
                self.output.push_str("$$\n");
                self.output.push_str(value);
                if !value.ends_with('\n') {
                    self.output.push('\n');
                }
                self.output.push_str("$$\n");
            }
            AstNode::InlineMath { value } => {
                write!(&mut self.output, "${}$", value).unwrap();
            }
            // Footnotes
            AstNode::FootnoteReference { identifier, label } => {
                write!(
                    &mut self.output,
                    "[^{}]",
                    label.as_ref().unwrap_or(identifier)
                )
                .unwrap();
            }
            AstNode::FootnoteDefinition {
                identifier,
                label,
                children,
            } => {
                write!(
                    &mut self.output,
                    "[^{}]: ",
                    label.as_ref().unwrap_or(identifier)
                )
                .unwrap();
                self.render_children(children)?;
                self.output.push('\n');
            }
            // Link/Image References
            AstNode::Definition {
                identifier,
                url,
                title,
                ..
            } => {
                write!(&mut self.output, "[{}]: {}", identifier, url).unwrap();
                if let Some(t) = title {
                    write!(&mut self.output, " \"{}\"", escape_quotes(t)).unwrap();
                }
                self.output.push('\n');
            }
            AstNode::LinkReference {
                reference_kind,
                identifier,
                label,
                children,
            } => {
                self.output.push('[');
                self.render_children(children)?;
                self.output.push(']');

                match reference_kind {
                    crate::ReferenceKind::Full => {
                        self.output.push('[');
                        if let Some(l) = label {
                            self.output.push_str(l);
                        } else {
                            self.output.push_str(identifier);
                        }
                        self.output.push(']');
                    }
                    crate::ReferenceKind::Collapsed => {
                        self.output.push_str("[]");
                    }
                    crate::ReferenceKind::Shortcut => {
                        // No additional syntax needed
                    }
                }
            }
            AstNode::ImageReference {
                reference_kind,
                identifier,
                label,
                alt,
            } => {
                self.output.push_str("![");
                self.output.push_str(alt);
                self.output.push(']');

                match reference_kind {
                    crate::ReferenceKind::Full => {
                        self.output.push('[');
                        if let Some(l) = label {
                            self.output.push_str(l);
                        } else {
                            self.output.push_str(identifier);
                        }
                        self.output.push(']');
                    }
                    crate::ReferenceKind::Collapsed => {
                        self.output.push_str("[]");
                    }
                    crate::ReferenceKind::Shortcut => {
                        // No additional syntax needed
                    }
                }
            }
            // MDX
            AstNode::MdxjsEsm { value } => {
                self.output.push_str(value);
            }
            AstNode::MdxFlowExpression { value } => {
                self.output.push('{');
                self.output.push_str(value);
                self.output.push('}');
                self.output.push('\n');
            }
            AstNode::MdxTextExpression { value } => {
                self.output.push('{');
                self.output.push_str(value);
                self.output.push('}');
            }
            AstNode::MdxJsxFlowElement { name, children } => {
                if let Some(tag_name) = name {
                    self.output.push('<');
                    self.output.push_str(tag_name);
                    self.output.push('>');
                    self.render_children(children)?;
                    self.output.push_str("</");
                    self.output.push_str(tag_name);
                    self.output.push('>');
                } else {
                    // Fragment
                    self.output.push_str("<>");
                    self.render_children(children)?;
                    self.output.push_str("</>");
                }
                self.output.push('\n');
            }
            AstNode::MdxJsxTextElement { name, children } => {
                if let Some(tag_name) = name {
                    self.output.push('<');
                    self.output.push_str(tag_name);
                    self.output.push('>');
                    self.render_children(children)?;
                    self.output.push_str("</");
                    self.output.push_str(tag_name);
                    self.output.push('>');
                } else {
                    // Fragment
                    self.output.push_str("<>");
                    self.render_children(children)?;
                    self.output.push_str("</>");
                }
            }
        }
        Ok(())
    }

    /// Render a list of child node hashes
    fn render_children(&mut self, children: &[Blake3Hash]) -> Result<(), RenderError> {
        for child_hash in children {
            let child = self
                .nodes
                .get(child_hash)
                .ok_or(RenderError::MissingNode(*child_hash))?;
            self.render_node(child)?;
        }
        Ok(())
    }

    /// Render block-level children with proper spacing (blank line between blocks)
    fn render_block_children(&mut self, children: &[Blake3Hash]) -> Result<(), RenderError> {
        for (i, child_hash) in children.iter().enumerate() {
            let child = self
                .nodes
                .get(child_hash)
                .ok_or(RenderError::MissingNode(*child_hash))?;

            self.render_node(child)?;

            // Add blank line between block elements, but not after the last one
            if i < children.len() - 1 && is_block_node(child) {
                self.output.push('\n');
            }
        }
        Ok(())
    }

    /// Render list item children (can include nested lists or paragraphs)
    fn render_list_item_children(&mut self, children: &[Blake3Hash]) -> Result<(), RenderError> {
        for child_hash in children.iter() {
            let child = self
                .nodes
                .get(child_hash)
                .ok_or(RenderError::MissingNode(*child_hash))?;

            match child {
                // Nested list - render on next line with proper indentation
                AstNode::List { .. } => {
                    // Start nested list on a new line
                    self.output.push('\n');
                    self.render_node(child)?;
                    // Pop the last newline added by the child (parent will add its own)
                    if self.output.ends_with('\n') {
                        self.output.pop();
                    }
                }
                // Paragraph in list item - just render content without extra newline
                AstNode::Paragraph { children } => {
                    self.render_children(children)?;
                }
                // Inline content
                _ => {
                    self.render_node(child)?;
                }
            }
        }
        Ok(())
    }
}

/// Check if a node is a block-level element that needs spacing
fn is_block_node(node: &AstNode) -> bool {
    matches!(
        node,
        AstNode::Heading { .. }
            | AstNode::Paragraph { .. }
            | AstNode::List { .. }
            | AstNode::CodeBlock { .. }
            | AstNode::Blockquote { .. }
            | AstNode::ThematicBreak
            | AstNode::Table { .. }
            | AstNode::Yaml { .. }
            | AstNode::Toml { .. }
    )
}

/// Escape double quotes in title attributes
fn escape_quotes(s: &str) -> String {
    s.replace('"', "\\\"")
}

/// Errors that can occur during rendering
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// A node hash was referenced but not found in the node map
    MissingNode(Blake3Hash),
    /// A node type is not yet supported by the renderer
    UnsupportedNode(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::MissingNode(hash) => {
                write!(f, "Missing node in store: {}", hash.to_hex())
            }
            RenderError::UnsupportedNode(node_type) => {
                write!(f, "Unsupported node type: {}", node_type)
            }
        }
    }
}

impl std::error::Error for RenderError {}
