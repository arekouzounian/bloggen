//! AST → HTML renderer
//!
//! Converts content-addressed AST nodes to semantic HTML using a visitor pattern.
//!
//! Design principles:
//! - Semantic HTML5 elements
//! - XSS protection via HTML entity escaping
//! - Clean, readable output
//! - Support for extension system (classes, attributes, custom rendering)

use crate::error::AppError;
use crate::storage::NodeStore;
use bgc_ast::{AstNode, Blake3Hash, ReferenceKind};
use std::collections::HashMap;

/// HTML escaping to prevent XSS attacks
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Escape HTML attribute values
fn escape_attr(s: &str) -> String {
    escape_html(s)
}

/// Rendering context to track state during traversal
#[derive(Debug, Clone, Default)]
struct RenderContext {
    /// Whether we're inside an inline context (affects element choice)
    /// Currently unused but reserved for future inline/block handling
    _inline: bool,
    /// Table alignment information (if inside a table)
    /// Currently unused but reserved for GFM table alignment support
    _table_aligns: Vec<Option<String>>,
}

/// Render a content-addressed AST to HTML.
///
/// # Arguments
/// * `root_hash` - The Blake3 hash of the root node
/// * `store` - The node store to fetch nodes from
///
/// # Returns
/// The rendered HTML as a String
///
/// # Errors
/// Returns an error if:
/// - The root node or any child nodes are missing from the store
/// - The tree structure is malformed
pub async fn render_to_html<S: NodeStore>(
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
            output.push_str("<div class=\"markdown-content\">");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</div>");
        }

        AstNode::Heading { level, children } => {
            let level = (*level).clamp(1, 6);
            output.push_str(&format!("<h{}>", level));
            render_children(children, nodes, ctx, output)?;
            output.push_str(&format!("</h{}>", level));
        }

        AstNode::Paragraph { children } => {
            output.push_str("<p>");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</p>");
        }

        AstNode::List {
            ordered,
            start,
            children,
        } => {
            if *ordered {
                if let Some(start_num) = start {
                    if *start_num != 1 {
                        output.push_str(&format!("<ol start=\"{}\">", start_num));
                    } else {
                        output.push_str("<ol>");
                    }
                } else {
                    output.push_str("<ol>");
                }
                render_children(children, nodes, ctx, output)?;
                output.push_str("</ol>");
            } else {
                output.push_str("<ul>");
                render_children(children, nodes, ctx, output)?;
                output.push_str("</ul>");
            }
        }

        AstNode::ListItem { checked, children } => {
            output.push_str("<li>");

            // Task list checkbox
            if let Some(is_checked) = checked {
                if *is_checked {
                    output.push_str("<input type=\"checkbox\" checked disabled> ");
                } else {
                    output.push_str("<input type=\"checkbox\" disabled> ");
                }
            }

            render_children(children, nodes, ctx, output)?;
            output.push_str("</li>");
        }

        AstNode::Blockquote { children } => {
            output.push_str("<blockquote>");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</blockquote>");
        }

        AstNode::Strong { children } => {
            output.push_str("<strong>");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</strong>");
        }

        AstNode::Emphasis { children } => {
            output.push_str("<em>");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</em>");
        }

        AstNode::Delete { children } => {
            output.push_str("<del>");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</del>");
        }

        AstNode::Text { value } => {
            output.push_str(&escape_html(value));
        }

        AstNode::CodeBlock { lang, value } => {
            output.push_str("<pre><code");
            if let Some(language) = lang {
                output.push_str(&format!(" class=\"language-{}\"", escape_attr(language)));
            }
            output.push('>');
            output.push_str(&escape_html(value));
            output.push_str("</code></pre>");
        }

        AstNode::InlineCode { value } => {
            output.push_str("<code>");
            output.push_str(&escape_html(value));
            output.push_str("</code>");
        }

        AstNode::Link {
            url,
            title,
            children,
        } => {
            output.push_str("<a href=\"");
            output.push_str(&escape_attr(url));
            output.push('"');
            if let Some(t) = title {
                output.push_str(" title=\"");
                output.push_str(&escape_attr(t));
                output.push('"');
            }
            output.push('>');
            render_children(children, nodes, ctx, output)?;
            output.push_str("</a>");
        }

        AstNode::Image { url, alt, title } => {
            output.push_str("<img src=\"");
            output.push_str(&escape_attr(url));
            output.push_str("\" alt=\"");
            output.push_str(&escape_attr(alt));
            output.push('"');
            if let Some(t) = title {
                output.push_str(" title=\"");
                output.push_str(&escape_attr(t));
                output.push('"');
            }
            output.push_str(" />");
        }

        AstNode::Break => {
            output.push_str("<br />");
        }

        AstNode::ThematicBreak => {
            output.push_str("<hr />");
        }

        AstNode::Table { children } => {
            output.push_str("<table>");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</table>");
        }

        AstNode::TableRow { children } => {
            output.push_str("<tr>");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</tr>");
        }

        AstNode::TableCell { children } => {
            // Use <th> for header cells, <td> for body cells
            // For now, we'll use <td> - proper detection would require tracking row position
            output.push_str("<td>");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</td>");
        }

        AstNode::Html { value } => {
            // WARNING: Raw HTML is inserted without escaping
            // This is intentional to support embedded HTML in markdown
            // But it means user input MUST be trusted
            output.push_str(value);
        }

        AstNode::Definition { .. } => {
            // Link definitions don't render to visible HTML
            // They're used for reference resolution
        }

        AstNode::LinkReference {
            reference_kind,
            identifier,
            label,
            children,
        } => {
            // For now, render as a link with the identifier as href
            // A full implementation would resolve the reference from definitions
            output.push_str("<a href=\"#");
            output.push_str(&escape_attr(identifier));
            output.push_str("\" data-reference=\"");
            output.push_str(match reference_kind {
                ReferenceKind::Full => "full",
                ReferenceKind::Collapsed => "collapsed",
                ReferenceKind::Shortcut => "shortcut",
            });
            output.push('"');
            if let Some(l) = label {
                output.push_str(" data-label=\"");
                output.push_str(&escape_attr(l));
                output.push('"');
            }
            output.push('>');
            render_children(children, nodes, ctx, output)?;
            output.push_str("</a>");
        }

        AstNode::ImageReference {
            reference_kind,
            identifier,
            label,
            alt,
        } => {
            // Similar to link references - would need full reference resolution
            output.push_str("<img src=\"#");
            output.push_str(&escape_attr(identifier));
            output.push_str("\" alt=\"");
            output.push_str(&escape_attr(alt));
            output.push_str("\" data-reference=\"");
            output.push_str(match reference_kind {
                ReferenceKind::Full => "full",
                ReferenceKind::Collapsed => "collapsed",
                ReferenceKind::Shortcut => "shortcut",
            });
            output.push('"');
            if let Some(l) = label {
                output.push_str(" data-label=\"");
                output.push_str(&escape_attr(l));
                output.push('"');
            }
            output.push_str(" />");
        }

        AstNode::Yaml { value } => {
            // Frontmatter doesn't render to HTML
            // It's metadata that would be processed separately
            output.push_str("<!-- YAML frontmatter:\n");
            output.push_str(&escape_html(value));
            output.push_str("\n-->");
        }

        AstNode::Toml { value } => {
            output.push_str("<!-- TOML frontmatter:\n");
            output.push_str(&escape_html(value));
            output.push_str("\n-->");
        }

        AstNode::FootnoteDefinition {
            identifier,
            children,
            ..
        } => {
            output.push_str("<div class=\"footnote\" id=\"fn-");
            output.push_str(&escape_attr(identifier));
            output.push_str("\"><sup>");
            output.push_str(&escape_html(identifier));
            output.push_str("</sup> ");
            render_children(children, nodes, ctx, output)?;
            output.push_str("</div>");
        }

        AstNode::FootnoteReference { identifier, .. } => {
            output.push_str("<sup><a href=\"#fn-");
            output.push_str(&escape_attr(identifier));
            output.push_str("\" class=\"footnote-ref\">");
            output.push_str(&escape_html(identifier));
            output.push_str("</a></sup>");
        }

        AstNode::Math { value } => {
            // Block math - would typically be rendered with KaTeX or MathJax
            output.push_str("<div class=\"math-block\">");
            output.push_str(&escape_html(value));
            output.push_str("</div>");
        }

        AstNode::InlineMath { value } => {
            output.push_str("<span class=\"math-inline\">");
            output.push_str(&escape_html(value));
            output.push_str("</span>");
        }

        AstNode::MdxjsEsm { value } => {
            output.push_str("<!-- MDX ESM:\n");
            output.push_str(&escape_html(value));
            output.push_str("\n-->");
        }

        AstNode::MdxFlowExpression { value } => {
            output.push_str("<div class=\"mdx-expression\">");
            output.push_str(&escape_html(value));
            output.push_str("</div>");
        }

        AstNode::MdxTextExpression { value } => {
            output.push_str("<span class=\"mdx-expression\">");
            output.push_str(&escape_html(value));
            output.push_str("</span>");
        }

        AstNode::MdxJsxFlowElement { name, children } => {
            if let Some(tag_name) = name {
                output.push('<');
                output.push_str(&escape_html(tag_name));
                output.push('>');
                render_children(children, nodes, ctx, output)?;
                output.push_str("</");
                output.push_str(&escape_html(tag_name));
                output.push('>');
            } else {
                // Fragment
                render_children(children, nodes, ctx, output)?;
            }
        }

        AstNode::MdxJsxTextElement { name, children } => {
            if let Some(tag_name) = name {
                output.push('<');
                output.push_str(&escape_html(tag_name));
                output.push('>');
                render_children(children, nodes, ctx, output)?;
                output.push_str("</");
                output.push_str(&escape_html(tag_name));
                output.push('>');
            } else {
                // Fragment
                render_children(children, nodes, ctx, output)?;
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

        let html = render_to_html(root_hash, &store).await.unwrap();
        assert!(html.contains("<p>Hello, world!</p>"));
        assert!(html.contains("markdown-content"));
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

        let html = render_to_html(root_hash, &store).await.unwrap();
        assert!(html.contains("<h2>My Heading</h2>"));
    }

    #[tokio::test]
    async fn test_xss_protection() {
        let mut store = MockNodeStore::new();

        let text = AstNode::Text {
            value: "<script>alert('xss')</script>".to_string(),
        };
        let text_hash = hash_node(&text);
        store.insert(text_hash, text);

        let para = AstNode::Paragraph {
            children: vec![text_hash],
        };
        let para_hash = hash_node(&para);
        store.insert(para_hash, para);

        let root = AstNode::Root {
            children: vec![para_hash],
        };
        let root_hash = hash_node(&root);
        store.insert(root_hash, root);

        let html = render_to_html(root_hash, &store).await.unwrap();
        // Should escape HTML entities
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&lt;/script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[tokio::test]
    async fn test_render_link() {
        let mut store = MockNodeStore::new();

        let text = AstNode::Text {
            value: "Click here".to_string(),
        };
        let text_hash = hash_node(&text);
        store.insert(text_hash, text);

        let link = AstNode::Link {
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            children: vec![text_hash],
        };
        let link_hash = hash_node(&link);
        store.insert(link_hash, link);

        let para = AstNode::Paragraph {
            children: vec![link_hash],
        };
        let para_hash = hash_node(&para);
        store.insert(para_hash, para);

        let root = AstNode::Root {
            children: vec![para_hash],
        };
        let root_hash = hash_node(&root);
        store.insert(root_hash, root);

        let html = render_to_html(root_hash, &store).await.unwrap();
        assert!(html.contains("<a href=\"https://example.com\""));
        assert!(html.contains("title=\"Example\""));
        assert!(html.contains("Click here</a>"));
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

        let html = render_to_html(root_hash, &store).await.unwrap();
        assert!(html.contains("<pre><code class=\"language-rust\">"));
        assert!(html.contains("fn main()"));
        assert!(html.contains("</code></pre>"));
    }

    #[tokio::test]
    async fn test_render_list() {
        let mut store = MockNodeStore::new();

        let text1 = AstNode::Text {
            value: "First".to_string(),
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
            value: "Second".to_string(),
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

        let html = render_to_html(root_hash, &store).await.unwrap();
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>First</li>"));
        assert!(html.contains("<li>Second</li>"));
        assert!(html.contains("</ul>"));
    }
}
