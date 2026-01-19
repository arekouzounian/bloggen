//! AST → Markdown renderer
//!
//! Converts content-addressed AST nodes back to markdown text using a visitor pattern.
//!
//! Design principles:
//! - ATX headings (`## Heading`) over Setext style
//! - Consistent `**bold**` and `*italic*` formatting
//! - 2-space list indentation
//! - Blank lines between block elements
//! - Deterministic output (same AST always produces same markdown)

use crate::error::AppError;
use crate::storage::NodeStore;
use bgc_ast::{AstNode, Blake3Hash, ReferenceKind};
use std::collections::HashMap;

/// Rendering context to track state during traversal
#[derive(Debug, Clone)]
struct RenderContext {
    /// Current list nesting depth (for proper indentation)
    list_depth: usize,
    /// Whether we're inside an inline context (no newlines)
    /// Currently unused but reserved for future inline/block handling
    _inline: bool,
    /// Whether we're at the start of a line (affects spacing)
    /// Currently unused but reserved for future whitespace handling
    _at_line_start: bool,
}

impl Default for RenderContext {
    fn default() -> Self {
        Self {
            list_depth: 0,
            _inline: false,
            _at_line_start: true,
        }
    }
}

/// Render a content-addressed AST to markdown text.
///
/// # Arguments
/// * `root_hash` - The Blake3 hash of the root node
/// * `store` - The node store to fetch nodes from
///
/// # Returns
/// The rendered markdown as a String
///
/// # Errors
/// Returns an error if:
/// - The root node or any child nodes are missing from the store
/// - The tree structure is malformed
pub async fn render_to_markdown<S: NodeStore>(
    root_hash: Blake3Hash,
    store: &S,
) -> Result<String, AppError> {
    // Fetch the entire tree upfront
    let nodes = crate::storage::walker::walk_tree(root_hash, store).await?;

    // Start rendering from the root
    let mut output = String::new();
    let ctx = RenderContext::default();
    render_node(root_hash, &nodes, &ctx, &mut output)?;

    Ok(output)
}

/// Recursively render a single node and its children
fn render_node(
    hash: Blake3Hash,
    nodes: &HashMap<Blake3Hash, AstNode>,
    ctx: &RenderContext,
    output: &mut String,
) -> Result<(), AppError> {
    let node = nodes.get(&hash).ok_or_else(|| {
        AppError::Internal(format!("Missing node during rendering: {}", hash.to_hex()))
    })?;

    match node {
        AstNode::Root { children } => {
            render_children(children, nodes, ctx, output)?;
        }

        AstNode::Heading { level, children } => {
            ensure_blank_line_before(output);
            let level = (*level).clamp(1, 6);
            output.push_str(&"#".repeat(level as usize));
            output.push(' ');

            let inline_ctx = RenderContext {
                _inline: true,
                _at_line_start: false,
                ..*ctx
            };
            render_children(children, nodes, &inline_ctx, output)?;
            output.push('\n');
        }

        AstNode::Paragraph { children } => {
            ensure_blank_line_before(output);
            let inline_ctx = RenderContext {
                _inline: true,
                _at_line_start: true,
                ..*ctx
            };
            render_children(children, nodes, &inline_ctx, output)?;
            output.push('\n');
        }

        AstNode::List {
            ordered,
            start,
            children,
        } => {
            ensure_blank_line_before(output);
            let list_ctx = RenderContext {
                list_depth: ctx.list_depth + 1,
                ..*ctx
            };

            for (i, child_hash) in children.iter().enumerate() {
                let indent = "  ".repeat(ctx.list_depth);
                output.push_str(&indent);

                if *ordered {
                    let num = start.unwrap_or(1) + i as u32;
                    output.push_str(&format!("{}. ", num));
                } else {
                    output.push_str("- ");
                }

                render_node(*child_hash, nodes, &list_ctx, output)?;
            }
        }

        AstNode::ListItem { checked, children } => {
            // Checkbox for task lists
            if let Some(is_checked) = checked {
                if *is_checked {
                    output.push_str("[x] ");
                } else {
                    output.push_str("[ ] ");
                }
            }

            let item_ctx = RenderContext {
                _inline: true,
                _at_line_start: false,
                ..*ctx
            };
            render_children(children, nodes, &item_ctx, output)?;
            output.push('\n');
        }

        AstNode::Blockquote { children } => {
            ensure_blank_line_before(output);
            // Render children to a temporary buffer
            let mut inner = String::new();
            render_children(children, nodes, ctx, &mut inner)?;

            // Prefix each line with "> "
            for line in inner.lines() {
                output.push_str("> ");
                output.push_str(line);
                output.push('\n');
            }
        }

        AstNode::Strong { children } => {
            output.push_str("**");
            render_children(children, nodes, ctx, output)?;
            output.push_str("**");
        }

        AstNode::Emphasis { children } => {
            output.push('*');
            render_children(children, nodes, ctx, output)?;
            output.push('*');
        }

        AstNode::Delete { children } => {
            output.push_str("~~");
            render_children(children, nodes, ctx, output)?;
            output.push_str("~~");
        }

        AstNode::Text { value } => {
            output.push_str(value);
        }

        AstNode::CodeBlock { lang, value } => {
            ensure_blank_line_before(output);
            output.push_str("```");
            if let Some(language) = lang {
                output.push_str(language);
            }
            output.push('\n');
            output.push_str(value);
            if !value.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("```\n");
        }

        AstNode::InlineCode { value } => {
            output.push('`');
            output.push_str(value);
            output.push('`');
        }

        AstNode::Link {
            url,
            title,
            children,
        } => {
            output.push('[');
            render_children(children, nodes, ctx, output)?;
            output.push_str("](");
            output.push_str(url);
            if let Some(t) = title {
                output.push_str(" \"");
                output.push_str(&escape_quotes(t));
                output.push('"');
            }
            output.push(')');
        }

        AstNode::Image { url, alt, title } => {
            output.push_str("![");
            output.push_str(alt);
            output.push_str("](");
            output.push_str(url);
            if let Some(t) = title {
                output.push_str(" \"");
                output.push_str(&escape_quotes(t));
                output.push('"');
            }
            output.push(')');
        }

        AstNode::Break => {
            output.push_str("  \n");
        }

        AstNode::ThematicBreak => {
            ensure_blank_line_before(output);
            output.push_str("---\n");
        }

        AstNode::Table { children } => {
            ensure_blank_line_before(output);
            render_children(children, nodes, ctx, output)?;
        }

        AstNode::TableRow { children } => {
            output.push('|');
            for child_hash in children {
                output.push(' ');
                render_node(*child_hash, nodes, ctx, output)?;
                output.push_str(" |");
            }
            output.push('\n');
        }

        AstNode::TableCell { children } => {
            let inline_ctx = RenderContext {
                _inline: true,
                ..*ctx
            };
            render_children(children, nodes, &inline_ctx, output)?;
        }

        AstNode::Html { value } => {
            ensure_blank_line_before(output);
            output.push_str(value);
            output.push('\n');
        }

        AstNode::Definition {
            identifier,
            url,
            title,
            ..
        } => {
            ensure_blank_line_before(output);
            output.push('[');
            output.push_str(identifier);
            output.push_str("]: ");
            output.push_str(url);
            if let Some(t) = title {
                output.push_str(" \"");
                output.push_str(&escape_quotes(t));
                output.push('"');
            }
            output.push('\n');
        }

        AstNode::LinkReference {
            reference_kind,
            identifier,
            label,
            children,
        } => {
            output.push('[');
            render_children(children, nodes, ctx, output)?;
            output.push(']');

            match reference_kind {
                ReferenceKind::Full => {
                    output.push('[');
                    if let Some(l) = label {
                        output.push_str(l);
                    } else {
                        output.push_str(identifier);
                    }
                    output.push(']');
                }
                ReferenceKind::Collapsed => {
                    output.push_str("[]");
                }
                ReferenceKind::Shortcut => {
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
            output.push_str("![");
            output.push_str(alt);
            output.push(']');

            match reference_kind {
                ReferenceKind::Full => {
                    output.push('[');
                    if let Some(l) = label {
                        output.push_str(l);
                    } else {
                        output.push_str(identifier);
                    }
                    output.push(']');
                }
                ReferenceKind::Collapsed => {
                    output.push_str("[]");
                }
                ReferenceKind::Shortcut => {
                    // No additional syntax needed
                }
            }
        }

        AstNode::Yaml { value } => {
            output.push_str("---\n");
            output.push_str(value);
            if !value.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("---\n");
        }

        AstNode::Toml { value } => {
            output.push_str("+++\n");
            output.push_str(value);
            if !value.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("+++\n");
        }

        AstNode::FootnoteDefinition {
            identifier,
            children,
            ..
        } => {
            ensure_blank_line_before(output);
            output.push_str("[^");
            output.push_str(identifier);
            output.push_str("]: ");
            render_children(children, nodes, ctx, output)?;
            output.push('\n');
        }

        AstNode::FootnoteReference { identifier, .. } => {
            output.push_str("[^");
            output.push_str(identifier);
            output.push(']');
        }

        AstNode::Math { value } => {
            ensure_blank_line_before(output);
            output.push_str("$$\n");
            output.push_str(value);
            if !value.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("$$\n");
        }

        AstNode::InlineMath { value } => {
            output.push('$');
            output.push_str(value);
            output.push('$');
        }

        AstNode::MdxjsEsm { value } => {
            ensure_blank_line_before(output);
            output.push_str(value);
            output.push('\n');
        }

        AstNode::MdxFlowExpression { value } => {
            ensure_blank_line_before(output);
            output.push('{');
            output.push_str(value);
            output.push('}');
            output.push('\n');
        }

        AstNode::MdxTextExpression { value } => {
            output.push('{');
            output.push_str(value);
            output.push('}');
        }

        AstNode::MdxJsxFlowElement { name, children } => {
            ensure_blank_line_before(output);
            if let Some(tag_name) = name {
                output.push('<');
                output.push_str(tag_name);
                output.push('>');
                render_children(children, nodes, ctx, output)?;
                output.push_str("</");
                output.push_str(tag_name);
                output.push('>');
            } else {
                // Fragment
                output.push_str("<>");
                render_children(children, nodes, ctx, output)?;
                output.push_str("</>");
            }
            output.push('\n');
        }

        AstNode::MdxJsxTextElement { name, children } => {
            if let Some(tag_name) = name {
                output.push('<');
                output.push_str(tag_name);
                output.push('>');
                render_children(children, nodes, ctx, output)?;
                output.push_str("</");
                output.push_str(tag_name);
                output.push('>');
            } else {
                // Fragment
                output.push_str("<>");
                render_children(children, nodes, ctx, output)?;
                output.push_str("</>");
            }
        }
    }

    Ok(())
}

/// Render multiple child nodes in sequence
fn render_children(
    children: &[Blake3Hash],
    nodes: &HashMap<Blake3Hash, AstNode>,
    ctx: &RenderContext,
    output: &mut String,
) -> Result<(), AppError> {
    for child_hash in children {
        render_node(*child_hash, nodes, ctx, output)?;
    }
    Ok(())
}

/// Ensure there's a blank line before block-level elements (if output is not empty)
fn ensure_blank_line_before(output: &mut String) {
    if output.is_empty() {
        return;
    }

    // Check if we already have a blank line
    if output.ends_with("\n\n") {
        return;
    }

    // Add appropriate newlines
    if output.ends_with('\n') {
        output.push('\n');
    } else {
        output.push_str("\n\n");
    }
}

/// Escape double quotes in title attributes
fn escape_quotes(s: &str) -> String {
    s.replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Mock in-memory node store for testing
    struct MockNodeStore {
        nodes: HashMap<Blake3Hash, AstNode>,
    }

    impl MockNodeStore {
        fn new() -> Self {
            Self {
                nodes: HashMap::new(),
            }
        }

        fn insert(&mut self, hash: Blake3Hash, node: AstNode) {
            self.nodes.insert(hash, node);
        }
    }

    #[async_trait::async_trait]
    impl NodeStore for MockNodeStore {
        async fn insert_node(&self, _hash: Blake3Hash, _node: &AstNode) -> Result<(), AppError> {
            Ok(())
        }

        async fn insert_many(&self, _nodes: &HashMap<Blake3Hash, AstNode>) -> Result<(), AppError> {
            Ok(())
        }

        async fn get_node(&self, hash: Blake3Hash) -> Result<Option<AstNode>, AppError> {
            Ok(self.nodes.get(&hash).cloned())
        }

        async fn get_many(
            &self,
            hashes: &[Blake3Hash],
        ) -> Result<HashMap<Blake3Hash, AstNode>, AppError> {
            Ok(hashes
                .iter()
                .filter_map(|h| self.nodes.get(h).map(|n| (*h, n.clone())))
                .collect())
        }
    }

    fn hash_node(node: &AstNode) -> Blake3Hash {
        let json = serde_json::to_string(node).unwrap();
        let hash = blake3::hash(json.as_bytes());
        Blake3Hash::new(*hash.as_bytes())
    }

    #[tokio::test]
    async fn test_render_simple_paragraph() {
        let mut store = MockNodeStore::new();

        let text = AstNode::Text {
            value: "Hello, world!".to_string(),
        };
        let text_hash = hash_node(&text);
        store.insert(text_hash, text);

        let para = AstNode::Paragraph {
            children: vec![text_hash],
        };
        let para_hash = hash_node(&para);
        store.insert(para_hash, para.clone());

        let root = AstNode::Root {
            children: vec![para_hash],
        };
        let root_hash = hash_node(&root);
        store.insert(root_hash, root);

        let markdown = render_to_markdown(root_hash, &store).await.unwrap();
        assert_eq!(markdown, "Hello, world!\n");
    }

    #[tokio::test]
    async fn test_render_heading() {
        let mut store = MockNodeStore::new();

        let text = AstNode::Text {
            value: "My Heading".to_string(),
        };
        let text_hash = hash_node(&text);
        store.insert(text_hash, text);

        let heading = AstNode::Heading {
            level: 2,
            children: vec![text_hash],
        };
        let heading_hash = hash_node(&heading);
        store.insert(heading_hash, heading);

        let root = AstNode::Root {
            children: vec![heading_hash],
        };
        let root_hash = hash_node(&root);
        store.insert(root_hash, root);

        let markdown = render_to_markdown(root_hash, &store).await.unwrap();
        assert_eq!(markdown, "## My Heading\n");
    }

    #[tokio::test]
    async fn test_render_strong_emphasis() {
        let mut store = MockNodeStore::new();

        let text = AstNode::Text {
            value: "bold text".to_string(),
        };
        let text_hash = hash_node(&text);
        store.insert(text_hash, text);

        let strong = AstNode::Strong {
            children: vec![text_hash],
        };
        let strong_hash = hash_node(&strong);
        store.insert(strong_hash, strong);

        let para = AstNode::Paragraph {
            children: vec![strong_hash],
        };
        let para_hash = hash_node(&para);
        store.insert(para_hash, para);

        let root = AstNode::Root {
            children: vec![para_hash],
        };
        let root_hash = hash_node(&root);
        store.insert(root_hash, root);

        let markdown = render_to_markdown(root_hash, &store).await.unwrap();
        assert_eq!(markdown, "**bold text**\n");
    }

    #[tokio::test]
    async fn test_render_code_block() {
        let mut store = MockNodeStore::new();

        let code = AstNode::CodeBlock {
            lang: Some("rust".to_string()),
            value: "fn main() {\n    println!(\"Hello!\");\n}".to_string(),
        };
        let code_hash = hash_node(&code);
        store.insert(code_hash, code);

        let root = AstNode::Root {
            children: vec![code_hash],
        };
        let root_hash = hash_node(&root);
        store.insert(root_hash, root);

        let markdown = render_to_markdown(root_hash, &store).await.unwrap();
        assert!(markdown.starts_with("```rust\n"));
        assert!(markdown.contains("fn main()"));
        assert!(markdown.ends_with("```\n"));
    }

    #[tokio::test]
    async fn test_render_list() {
        let mut store = MockNodeStore::new();

        let text1 = AstNode::Text {
            value: "First item".to_string(),
        };
        let text1_hash = hash_node(&text1);
        store.insert(text1_hash, text1);

        let item1 = AstNode::ListItem {
            checked: None,
            children: vec![text1_hash],
        };
        let item1_hash = hash_node(&item1);
        store.insert(item1_hash, item1);

        let text2 = AstNode::Text {
            value: "Second item".to_string(),
        };
        let text2_hash = hash_node(&text2);
        store.insert(text2_hash, text2);

        let item2 = AstNode::ListItem {
            checked: None,
            children: vec![text2_hash],
        };
        let item2_hash = hash_node(&item2);
        store.insert(item2_hash, item2);

        let list = AstNode::List {
            ordered: false,
            start: None,
            children: vec![item1_hash, item2_hash],
        };
        let list_hash = hash_node(&list);
        store.insert(list_hash, list);

        let root = AstNode::Root {
            children: vec![list_hash],
        };
        let root_hash = hash_node(&root);
        store.insert(root_hash, root);

        let markdown = render_to_markdown(root_hash, &store).await.unwrap();
        assert!(markdown.contains("- First item"));
        assert!(markdown.contains("- Second item"));
    }
}
