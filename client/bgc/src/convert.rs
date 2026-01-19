use crate::ast::{AstNode, Blake3Hash};
use crate::cas::NodeStore;
use markdown::mdast;

/// Convert a markdown-rs AST node to our owned AstNode representation.
///
/// This function recursively converts the markdown-rs AST to our content-addressable
/// AST format. Children are stored in the provided NodeStore and referenced by hash.
///
/// # Arguments
///
/// * `md_node` - The markdown-rs AST node to convert
/// * `store` - The NodeStore to store child nodes in
///
/// # Returns
///
/// The converted AstNode with child hashes populated from the store
pub fn convert_node(md_node: &mdast::Node, store: &mut NodeStore) -> AstNode {
    match md_node {
        mdast::Node::Root(root) => {
            let children = convert_children(&root.children, store);
            AstNode::Root { children }
        }

        mdast::Node::Heading(heading) => {
            let children = convert_children(&heading.children, store);
            AstNode::Heading {
                level: heading.depth,
                children,
            }
        }

        mdast::Node::Paragraph(para) => {
            let children = convert_children(&para.children, store);
            AstNode::Paragraph { children }
        }

        mdast::Node::List(list) => {
            let children = convert_children(&list.children, store);
            AstNode::List {
                ordered: list.ordered,
                start: list.start,
                children,
            }
        }

        mdast::Node::ListItem(item) => {
            let children = convert_children(&item.children, store);
            AstNode::ListItem {
                checked: item.checked,
                children,
            }
        }

        mdast::Node::Blockquote(quote) => {
            let children = convert_children(&quote.children, store);
            AstNode::Blockquote { children }
        }

        mdast::Node::Strong(strong) => {
            let children = convert_children(&strong.children, store);
            AstNode::Strong { children }
        }

        mdast::Node::Emphasis(em) => {
            let children = convert_children(&em.children, store);
            AstNode::Emphasis { children }
        }

        mdast::Node::Delete(del) => {
            let children = convert_children(&del.children, store);
            AstNode::Delete { children }
        }

        mdast::Node::Text(text) => AstNode::Text {
            value: text.value.clone(),
        },

        mdast::Node::Code(code) => AstNode::CodeBlock {
            lang: code.lang.clone(),
            value: code.value.clone(),
        },

        mdast::Node::InlineCode(code) => AstNode::InlineCode {
            value: code.value.clone(),
        },

        mdast::Node::Link(link) => {
            let children = convert_children(&link.children, store);
            AstNode::Link {
                url: link.url.clone(),
                title: link.title.clone(),
                children,
            }
        }

        mdast::Node::Image(img) => AstNode::Image {
            url: img.url.clone(),
            alt: img.alt.clone(),
            title: img.title.clone(),
        },

        mdast::Node::Break(_) => AstNode::Break,

        mdast::Node::ThematicBreak(_) => AstNode::ThematicBreak,

        // Tables (GFM)
        mdast::Node::Table(table) => {
            let children = convert_children(&table.children, store);
            AstNode::Table { children }
        }

        mdast::Node::TableRow(row) => {
            let children = convert_children(&row.children, store);
            AstNode::TableRow { children }
        }

        mdast::Node::TableCell(cell) => {
            let children = convert_children(&cell.children, store);
            AstNode::TableCell { children }
        }

        // HTML
        mdast::Node::Html(html) => AstNode::Html {
            value: html.value.clone(),
        },

        // References
        mdast::Node::Definition(def) => AstNode::Definition {
            identifier: def.identifier.clone(),
            label: def.label.clone(),
            url: def.url.clone(),
            title: def.title.clone(),
        },

        mdast::Node::LinkReference(link_ref) => {
            let children = convert_children(&link_ref.children, store);
            AstNode::LinkReference {
                reference_kind: convert_reference_kind(&link_ref.reference_kind),
                identifier: link_ref.identifier.clone(),
                label: link_ref.label.clone(),
                children,
            }
        }

        mdast::Node::ImageReference(img_ref) => AstNode::ImageReference {
            reference_kind: convert_reference_kind(&img_ref.reference_kind),
            identifier: img_ref.identifier.clone(),
            label: img_ref.label.clone(),
            alt: img_ref.alt.clone(),
        },

        // Frontmatter
        mdast::Node::Yaml(yaml) => AstNode::Yaml {
            value: yaml.value.clone(),
        },

        mdast::Node::Toml(toml) => AstNode::Toml {
            value: toml.value.clone(),
        },

        // Footnotes
        mdast::Node::FootnoteDefinition(footnote_def) => {
            let children = convert_children(&footnote_def.children, store);
            AstNode::FootnoteDefinition {
                identifier: footnote_def.identifier.clone(),
                label: footnote_def.label.clone(),
                children,
            }
        }

        mdast::Node::FootnoteReference(footnote_ref) => AstNode::FootnoteReference {
            identifier: footnote_ref.identifier.clone(),
            label: footnote_ref.label.clone(),
        },

        // Math
        mdast::Node::Math(math) => AstNode::Math {
            value: math.value.clone(),
        },

        mdast::Node::InlineMath(inline_math) => AstNode::InlineMath {
            value: inline_math.value.clone(),
        },

        // MDX
        mdast::Node::MdxjsEsm(esm) => AstNode::MdxjsEsm {
            value: esm.value.clone(),
        },

        mdast::Node::MdxFlowExpression(expr) => AstNode::MdxFlowExpression {
            value: expr.value.clone(),
        },

        mdast::Node::MdxTextExpression(expr) => AstNode::MdxTextExpression {
            value: expr.value.clone(),
        },

        mdast::Node::MdxJsxFlowElement(element) => {
            let children = convert_children(&element.children, store);
            AstNode::MdxJsxFlowElement {
                name: element.name.clone(),
                children,
            }
        }

        mdast::Node::MdxJsxTextElement(element) => {
            let children = convert_children(&element.children, store);
            AstNode::MdxJsxTextElement {
                name: element.name.clone(),
                children,
            }
        }
    }
}

/// Convert markdown ReferenceKind to our ReferenceKind
fn convert_reference_kind(kind: &markdown::mdast::ReferenceKind) -> crate::ast::ReferenceKind {
    match kind {
        markdown::mdast::ReferenceKind::Full => crate::ast::ReferenceKind::Full,
        markdown::mdast::ReferenceKind::Collapsed => crate::ast::ReferenceKind::Collapsed,
        markdown::mdast::ReferenceKind::Shortcut => crate::ast::ReferenceKind::Shortcut,
    }
}

/// Convert a list of markdown-rs child nodes to a vector of hashes.
///
/// Each child is recursively converted and stored in the NodeStore,
/// and its hash is returned.
fn convert_children(children: &[mdast::Node], store: &mut NodeStore) -> Vec<Blake3Hash> {
    children
        .iter()
        .map(|child| {
            let node = convert_node(child, store);
            store.store(node)
        })
        .collect()
}

/// Parse markdown source and convert to content-addressable AST.
///
/// This is the main entry point for parsing markdown documents.
///
/// # Arguments
///
/// * `source` - The markdown source text to parse
/// * `store` - The NodeStore to store all nodes in
///
/// # Returns
///
/// The hash of the root node, or an error if parsing fails
pub fn parse_markdown(source: &str, store: &mut NodeStore) -> Result<Blake3Hash, String> {
    // Create ParseOptions with GFM + frontmatter + math support
    let mut options = markdown::ParseOptions::gfm();
    options.constructs.frontmatter = true;
    options.constructs.math_flow = true;
    options.constructs.math_text = true;
    options.constructs.gfm_footnote_definition = true;
    options.constructs.gfm_label_start_footnote = true;

    // Parse markdown to markdown AST
    let md_ast = markdown::to_mdast(source, &options)
        .map_err(|e| format!("Failed to parse markdown: {}", e))?;

    // Convert to our owned AST and store in the NodeStore
    let root_node = convert_node(&md_ast, store);
    let root_hash = store.store(root_node);

    Ok(root_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cas::CasDocument;

    #[test]
    fn test_parse_simple_text() {
        let mut store = NodeStore::new();
        let root_hash = parse_markdown("Hello, world!", &mut store).unwrap();

        let root = store.get(&root_hash).unwrap();
        assert!(matches!(root, AstNode::Root { .. }));

        // Should have root + paragraph + text = 3 nodes
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn test_parse_heading() {
        let mut store = NodeStore::new();
        let root_hash = parse_markdown("# Hello\n\nWorld", &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Find the heading node
        let has_heading = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::Heading { level: 1, .. })
        });
        assert!(has_heading);
    }

    #[test]
    fn test_parse_complex_document() {
        let markdown = r#"
# Title

This is a **bold** and *italic* text.

- Item 1
- Item 2
- Item 3

```rust
fn main() {
    println!("Hello!");
}
```
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        // Verify we can create a document
        let doc = CasDocument::new(&store, root_hash).unwrap();
        assert!(!doc.nodes.is_empty());

        // Verify root exists
        assert!(doc.root().is_some());
    }

    #[test]
    fn test_deduplication() {
        let markdown = r#"
Hello

Hello
"#;

        let mut store = NodeStore::new();
        parse_markdown(markdown, &mut store).unwrap();

        // The two "Hello" text nodes should be deduplicated
        // We should have: Root, Paragraph (deduplicated since both are empty), Text (deduplicated)
        // Actually, if both paragraphs have the same child (Hello text), they hash to the same value!
        // So we have: Root, 1x Paragraph (deduplicated), 1x Text (deduplicated) = 3 nodes
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn test_parse_list() {
        let markdown = r#"
1. First
2. Second
3. Third
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Find the list node
        let has_ordered_list = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::List { ordered: true, .. })
        });
        assert!(has_ordered_list);
    }

    #[test]
    fn test_parse_link() {
        let markdown = "[Click here](https://example.com)";

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Find the link node
        let has_link = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::Link { url, .. } if url == "https://example.com")
        });
        assert!(has_link);
    }

    #[test]
    fn test_round_trip_serialization() {
        let markdown = "# Hello\n\nThis is **bold** text.";

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        // Create document and serialize
        let doc = CasDocument::new(&store, root_hash).unwrap();
        let json = doc.to_json_pretty().unwrap();

        // Deserialize
        let doc2 = CasDocument::from_json(&json).unwrap();

        // Should have same structure
        assert_eq!(doc.root_hash, doc2.root_hash);
        assert_eq!(doc.nodes.len(), doc2.nodes.len());
    }

    // ========== Tests for new node types ==========

    #[test]
    fn test_parse_table() {
        let markdown = r#"
| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |
| Cell 3   | Cell 4   |
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Should have table, rows, and cells
        let has_table = nodes.iter().any(|(_, node)| matches!(node, AstNode::Table { .. }));
        let has_row = nodes.iter().any(|(_, node)| matches!(node, AstNode::TableRow { .. }));
        let has_cell = nodes.iter().any(|(_, node)| matches!(node, AstNode::TableCell { .. }));

        assert!(has_table, "Should have Table node");
        assert!(has_row, "Should have TableRow node");
        assert!(has_cell, "Should have TableCell node");
    }

    #[test]
    fn test_parse_html() {
        let markdown = r#"
This is a paragraph.

<div class="custom">
  <p>Raw HTML content</p>
</div>

Another paragraph.
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Find HTML node
        let has_html = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::Html { value } if value.contains("<div"))
        });

        assert!(has_html, "Should have Html node");
    }

    #[test]
    fn test_parse_link_reference() {
        let markdown = r#"
[Link text][ref]

[ref]: https://example.com "Title"
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Should have LinkReference and Definition
        let has_link_ref = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::LinkReference { identifier, .. } if identifier == "ref")
        });
        let has_definition = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::Definition { identifier, .. } if identifier == "ref")
        });

        assert!(has_link_ref, "Should have LinkReference node");
        assert!(has_definition, "Should have Definition node");
    }

    #[test]
    fn test_parse_image_reference() {
        let markdown = r#"
![Alt text][img-ref]

[img-ref]: /path/to/image.jpg "Image Title"
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Should have ImageReference and Definition
        let has_img_ref = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::ImageReference { identifier, alt, .. }
                if identifier == "img-ref" && alt == "Alt text")
        });
        let has_definition = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::Definition { identifier, .. } if identifier == "img-ref")
        });

        assert!(has_img_ref, "Should have ImageReference node");
        assert!(has_definition, "Should have Definition node");
    }

    #[test]
    fn test_parse_yaml_frontmatter() {
        let markdown = r#"---
title: My Post
author: John Doe
tags:
  - rust
  - markdown
---

# Content

This is the post content.
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Should have YAML node
        let has_yaml = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::Yaml { value } if value.contains("title: My Post"))
        });

        assert!(has_yaml, "Should have Yaml frontmatter node");
    }

    #[test]
    fn test_parse_toml_frontmatter() {
        let markdown = r#"+++
title = "My Post"
author = "John Doe"
+++

# Content

This is the post content.
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Should have TOML node
        let has_toml = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::Toml { value } if value.contains("title = \"My Post\""))
        });

        assert!(has_toml, "Should have Toml frontmatter node");
    }

    #[test]
    fn test_parse_footnotes() {
        let markdown = r#"
Here is a sentence with a footnote[^1].

[^1]: This is the footnote content.
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Should have FootnoteReference and FootnoteDefinition
        let has_footnote_ref = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::FootnoteReference { identifier, .. } if identifier == "1")
        });
        let has_footnote_def = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::FootnoteDefinition { identifier, .. } if identifier == "1")
        });

        assert!(has_footnote_ref, "Should have FootnoteReference node");
        assert!(has_footnote_def, "Should have FootnoteDefinition node");
    }

    #[test]
    fn test_parse_math() {
        let markdown = r#"
Block math:

$$
E = mc^2
$$

Inline math: $a^2 + b^2 = c^2$
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Should have Math and InlineMath nodes
        let has_block_math = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::Math { value } if value.contains("E = mc^2"))
        });
        let has_inline_math = nodes.iter().any(|(_, node)| {
            matches!(node, AstNode::InlineMath { value } if value.contains("a^2 + b^2"))
        });

        assert!(has_block_math, "Should have block Math node");
        assert!(has_inline_math, "Should have inline InlineMath node");
    }

    #[test]
    fn test_parse_thematic_break() {
        let markdown = r#"
Before

---

After
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        let has_thematic_break = nodes
            .iter()
            .any(|(_, node)| matches!(node, AstNode::ThematicBreak));

        assert!(has_thematic_break, "Should have ThematicBreak node");
    }

    #[test]
    fn test_parse_blockquote() {
        let markdown = r#"
> This is a blockquote.
> It can span multiple lines.
>
> And have multiple paragraphs.
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        let has_blockquote = nodes
            .iter()
            .any(|(_, node)| matches!(node, AstNode::Blockquote { .. }));

        assert!(has_blockquote, "Should have Blockquote node");
    }

    #[test]
    fn test_parse_strikethrough() {
        let markdown = "This is ~~deleted~~ text.";

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        let has_delete = nodes
            .iter()
            .any(|(_, node)| matches!(node, AstNode::Delete { .. }));

        assert!(has_delete, "Should have Delete node for strikethrough");
    }

    #[test]
    fn test_comprehensive_document() {
        let markdown = r#"---
title: Comprehensive Test
author: Test Suite
---

# Main Title

This is a paragraph with **bold**, *italic*, and ~~strikethrough~~ text.

## Lists and Links

- Item 1
- Item 2 with [a link](https://example.com)
- Item 3

### Ordered List

1. First
2. Second
3. Third

## Code

Inline `code` and a code block:

```rust
fn main() {
    println!("Hello!");
}
```

## Tables

| Column 1 | Column 2 |
|----------|----------|
| A        | B        |

## Blockquote

> This is a quote.

## References

[Link][ref] and ![Image][img]

[ref]: https://example.com
[img]: /image.jpg

---

That's all!
"#;

        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();

        let nodes = store.walk_tree(&root_hash).unwrap();

        // Verify we have a diverse set of node types
        let node_types: std::collections::HashSet<_> =
            nodes.iter().map(|(_, node)| node.node_type()).collect();

        // Check for key node types
        assert!(node_types.contains("Yaml"), "Should have YAML frontmatter");
        assert!(node_types.contains("Heading"), "Should have headings");
        assert!(node_types.contains("Paragraph"), "Should have paragraphs");
        assert!(node_types.contains("Strong"), "Should have bold text");
        assert!(node_types.contains("Emphasis"), "Should have italic text");
        assert!(node_types.contains("Delete"), "Should have strikethrough");
        assert!(node_types.contains("List"), "Should have lists");
        assert!(node_types.contains("Link"), "Should have links");
        assert!(node_types.contains("CodeBlock"), "Should have code blocks");
        assert!(node_types.contains("InlineCode"), "Should have inline code");
        assert!(node_types.contains("Table"), "Should have tables");
        assert!(node_types.contains("Blockquote"), "Should have blockquotes");
        assert!(node_types.contains("LinkReference"), "Should have link references");
        assert!(node_types.contains("ImageReference"), "Should have image references");
        assert!(node_types.contains("Definition"), "Should have definitions");
        assert!(node_types.contains("ThematicBreak"), "Should have thematic break");

        // Verify serialization works
        let doc = CasDocument::new(&store, root_hash).unwrap();
        let json = doc.to_json_pretty().unwrap();
        assert!(!json.is_empty());

        // Verify round-trip
        let doc2 = CasDocument::from_json(&json).unwrap();
        assert_eq!(doc.root_hash, doc2.root_hash);
        assert_eq!(doc.nodes.len(), doc2.nodes.len());
    }
}
