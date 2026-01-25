//! Markdown rendering wrapper for the bgc client
//!
//! This module re-exports and wraps the shared rendering functionality from bgc-ast,
//! adapting it to work with the client's NodeStore.

use crate::ast::AstNode;
use crate::cas::NodeStore;

// Re-export error type from bgc-ast
pub use bgc_ast::render::RenderError;

/// Renders an AST back to markdown format.
///
/// This is a wrapper around bgc_ast::render::MarkdownRenderer that adapts
/// the NodeStore interface to the HashMap-based API.
pub struct MarkdownRenderer<'a> {
    /// Reference to the node store for resolving hash references
    store: &'a NodeStore,
}

impl<'a> MarkdownRenderer<'a> {
    /// Create a new renderer with a reference to the node store
    pub fn new(store: &'a NodeStore) -> Self {
        Self { store }
    }

    /// Render a node by its hash and return the markdown string
    pub fn render(&mut self, node: &AstNode) -> Result<String, RenderError> {
        // Convert NodeStore to HashMap for bgc-ast renderer
        let nodes = self.store.to_map();
        let mut renderer = bgc_ast::render::MarkdownRenderer::new(&nodes);
        renderer.render(node)
    }
}

// Re-export tests from the original implementation
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstNode;
    use crate::cas::NodeStore;

    #[test]
    fn test_render_text() {
        let store = NodeStore::new();
        let text = AstNode::Text {
            value: "Hello, world!".to_string(),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&text).unwrap();

        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_render_inline_code() {
        let store = NodeStore::new();
        let code = AstNode::InlineCode {
            value: "println!(\"test\")".to_string(),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&code).unwrap();

        assert_eq!(result, "`println!(\"test\")`");
    }

    #[test]
    fn test_render_break() {
        let store = NodeStore::new();
        let br = AstNode::Break;

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&br).unwrap();

        assert_eq!(result, "  \n");
    }

    #[test]
    fn test_render_strong() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "bold text".to_string(),
        };
        let text_hash = store.store(text);

        let strong = AstNode::Strong {
            children: vec![text_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&strong).unwrap();

        assert_eq!(result, "**bold text**");
    }

    #[test]
    fn test_render_emphasis() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "italic text".to_string(),
        };
        let text_hash = store.store(text);

        let emphasis = AstNode::Emphasis {
            children: vec![text_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&emphasis).unwrap();

        assert_eq!(result, "*italic text*");
    }

    #[test]
    fn test_render_strikethrough() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "deleted text".to_string(),
        };
        let text_hash = store.store(text);

        let delete = AstNode::Delete {
            children: vec![text_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&delete).unwrap();

        assert_eq!(result, "~~deleted text~~");
    }

    #[test]
    fn test_render_nested_formatting() {
        let mut store = NodeStore::new();

        // Create "bold and italic" with nested formatting
        let text = AstNode::Text {
            value: "bold and italic".to_string(),
        };
        let text_hash = store.store(text);

        let emphasis = AstNode::Emphasis {
            children: vec![text_hash],
        };
        let emphasis_hash = store.store(emphasis);

        let strong = AstNode::Strong {
            children: vec![emphasis_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&strong).unwrap();

        assert_eq!(result, "***bold and italic***");
    }

    #[test]
    fn test_render_link_without_title() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "click here".to_string(),
        };
        let text_hash = store.store(text);

        let link = AstNode::Link {
            url: "https://example.com".to_string(),
            title: None,
            children: vec![text_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&link).unwrap();

        assert_eq!(result, "[click here](https://example.com)");
    }

    #[test]
    fn test_render_link_with_title() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "click here".to_string(),
        };
        let text_hash = store.store(text);

        let link = AstNode::Link {
            url: "https://example.com".to_string(),
            title: Some("Example Site".to_string()),
            children: vec![text_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&link).unwrap();

        assert_eq!(
            result,
            "[click here](https://example.com \"Example Site\")"
        );
    }

    #[test]
    fn test_render_image_without_title() {
        let store = NodeStore::new();
        let image = AstNode::Image {
            url: "https://example.com/image.png".to_string(),
            alt: "An example image".to_string(),
            title: None,
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&image).unwrap();

        assert_eq!(result, "![An example image](https://example.com/image.png)");
    }

    #[test]
    fn test_render_image_with_title() {
        let store = NodeStore::new();
        let image = AstNode::Image {
            url: "https://example.com/image.png".to_string(),
            alt: "An example image".to_string(),
            title: Some("Hover text".to_string()),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&image).unwrap();

        assert_eq!(
            result,
            "![An example image](https://example.com/image.png \"Hover text\")"
        );
    }

    #[test]
    fn test_render_heading() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "Hello World".to_string(),
        };
        let text_hash = store.store(text);

        let heading = AstNode::Heading {
            level: 2,
            children: vec![text_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&heading).unwrap();

        assert_eq!(result, "## Hello World\n");
    }

    #[test]
    fn test_render_heading_levels() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "Test".to_string(),
        };
        let text_hash = store.store(text.clone());

        for level in 1..=6 {
            let heading = AstNode::Heading {
                level,
                children: vec![text_hash],
            };

            let mut renderer = MarkdownRenderer::new(&store);
            let result = renderer.render(&heading).unwrap();

            let expected = format!("{} Test\n", "#".repeat(level as usize));
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_render_paragraph() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "This is a paragraph.".to_string(),
        };
        let text_hash = store.store(text);

        let para = AstNode::Paragraph {
            children: vec![text_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&para).unwrap();

        assert_eq!(result, "This is a paragraph.\n");
    }

    #[test]
    fn test_render_paragraph_with_formatting() {
        let mut store = NodeStore::new();

        let text1 = AstNode::Text {
            value: "This is ".to_string(),
        };
        let text1_hash = store.store(text1);

        let text2 = AstNode::Text {
            value: "bold".to_string(),
        };
        let text2_hash = store.store(text2);

        let strong = AstNode::Strong {
            children: vec![text2_hash],
        };
        let strong_hash = store.store(strong);

        let text3 = AstNode::Text {
            value: " text.".to_string(),
        };
        let text3_hash = store.store(text3);

        let para = AstNode::Paragraph {
            children: vec![text1_hash, strong_hash, text3_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&para).unwrap();

        assert_eq!(result, "This is **bold** text.\n");
    }

    #[test]
    fn test_render_thematic_break() {
        let store = NodeStore::new();
        let thematic_break = AstNode::ThematicBreak;

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&thematic_break).unwrap();

        assert_eq!(result, "---\n");
    }

    #[test]
    fn test_render_document_with_spacing() {
        let mut store = NodeStore::new();

        // Create heading
        let h_text = AstNode::Text {
            value: "Title".to_string(),
        };
        let h_text_hash = store.store(h_text);
        let heading = AstNode::Heading {
            level: 1,
            children: vec![h_text_hash],
        };
        let heading_hash = store.store(heading);

        // Create paragraph
        let p_text = AstNode::Text {
            value: "First paragraph.".to_string(),
        };
        let p_text_hash = store.store(p_text);
        let para = AstNode::Paragraph {
            children: vec![p_text_hash],
        };
        let para_hash = store.store(para);

        // Create root with both
        let root = AstNode::Root {
            children: vec![heading_hash, para_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&root).unwrap();

        // Should have blank line between heading and paragraph
        assert_eq!(result, "# Title\n\nFirst paragraph.\n");
    }

    #[test]
    fn test_render_unordered_list() {
        let mut store = NodeStore::new();

        // Create list items
        let text1 = AstNode::Text {
            value: "First item".to_string(),
        };
        let text1_hash = store.store(text1);
        let para1 = AstNode::Paragraph {
            children: vec![text1_hash],
        };
        let para1_hash = store.store(para1);
        let item1 = AstNode::ListItem {
            checked: None,
            children: vec![para1_hash],
        };
        let item1_hash = store.store(item1);

        let text2 = AstNode::Text {
            value: "Second item".to_string(),
        };
        let text2_hash = store.store(text2);
        let para2 = AstNode::Paragraph {
            children: vec![text2_hash],
        };
        let para2_hash = store.store(para2);
        let item2 = AstNode::ListItem {
            checked: None,
            children: vec![para2_hash],
        };
        let item2_hash = store.store(item2);

        let list = AstNode::List {
            ordered: false,
            start: None,
            children: vec![item1_hash, item2_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&list).unwrap();

        assert_eq!(result, "- First item\n- Second item\n");
    }

    #[test]
    fn test_render_ordered_list() {
        let mut store = NodeStore::new();

        let text1 = AstNode::Text {
            value: "First".to_string(),
        };
        let text1_hash = store.store(text1);
        let para1 = AstNode::Paragraph {
            children: vec![text1_hash],
        };
        let para1_hash = store.store(para1);
        let item1 = AstNode::ListItem {
            checked: None,
            children: vec![para1_hash],
        };
        let item1_hash = store.store(item1);

        let text2 = AstNode::Text {
            value: "Second".to_string(),
        };
        let text2_hash = store.store(text2);
        let para2 = AstNode::Paragraph {
            children: vec![text2_hash],
        };
        let para2_hash = store.store(para2);
        let item2 = AstNode::ListItem {
            checked: None,
            children: vec![para2_hash],
        };
        let item2_hash = store.store(item2);

        let list = AstNode::List {
            ordered: true,
            start: None,
            children: vec![item1_hash, item2_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&list).unwrap();

        assert_eq!(result, "1. First\n2. Second\n");
    }

    #[test]
    fn test_render_task_list() {
        let mut store = NodeStore::new();

        let text1 = AstNode::Text {
            value: "Done task".to_string(),
        };
        let text1_hash = store.store(text1);
        let para1 = AstNode::Paragraph {
            children: vec![text1_hash],
        };
        let para1_hash = store.store(para1);
        let item1 = AstNode::ListItem {
            checked: Some(true),
            children: vec![para1_hash],
        };
        let item1_hash = store.store(item1);

        let text2 = AstNode::Text {
            value: "Todo task".to_string(),
        };
        let text2_hash = store.store(text2);
        let para2 = AstNode::Paragraph {
            children: vec![text2_hash],
        };
        let para2_hash = store.store(para2);
        let item2 = AstNode::ListItem {
            checked: Some(false),
            children: vec![para2_hash],
        };
        let item2_hash = store.store(item2);

        let list = AstNode::List {
            ordered: false,
            start: None,
            children: vec![item1_hash, item2_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&list).unwrap();

        assert_eq!(result, "- [x] Done task\n- [ ] Todo task\n");
    }

    #[test]
    fn test_render_nested_list() {
        let mut store = NodeStore::new();

        // Inner list item
        let inner_text = AstNode::Text {
            value: "Nested item".to_string(),
        };
        let inner_text_hash = store.store(inner_text);
        let inner_para = AstNode::Paragraph {
            children: vec![inner_text_hash],
        };
        let inner_para_hash = store.store(inner_para);
        let inner_item = AstNode::ListItem {
            checked: None,
            children: vec![inner_para_hash],
        };
        let inner_item_hash = store.store(inner_item);

        // Inner list
        let inner_list = AstNode::List {
            ordered: false,
            start: None,
            children: vec![inner_item_hash],
        };
        let inner_list_hash = store.store(inner_list);

        // Outer list item with nested list
        let outer_text = AstNode::Text {
            value: "Parent item".to_string(),
        };
        let outer_text_hash = store.store(outer_text);
        let outer_para = AstNode::Paragraph {
            children: vec![outer_text_hash],
        };
        let outer_para_hash = store.store(outer_para);
        let outer_item = AstNode::ListItem {
            checked: None,
            children: vec![outer_para_hash, inner_list_hash],
        };
        let outer_item_hash = store.store(outer_item);

        // Outer list
        let outer_list = AstNode::List {
            ordered: false,
            start: None,
            children: vec![outer_item_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&outer_list).unwrap();

        assert_eq!(result, "- Parent item\n  - Nested item\n");
    }

    #[test]
    fn test_render_code_block_without_language() {
        let store = NodeStore::new();
        let code = AstNode::CodeBlock {
            lang: None,
            value: "fn main() {\n    println!(\"Hello!\");\n}".to_string(),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&code).unwrap();

        assert_eq!(
            result,
            "```\nfn main() {\n    println!(\"Hello!\");\n}\n```\n"
        );
    }

    #[test]
    fn test_render_code_block_with_language() {
        let store = NodeStore::new();
        let code = AstNode::CodeBlock {
            lang: Some("rust".to_string()),
            value: "fn main() {}".to_string(),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&code).unwrap();

        assert_eq!(result, "```rust\nfn main() {}\n```\n");
    }

    #[test]
    fn test_render_blockquote() {
        let mut store = NodeStore::new();

        let text = AstNode::Text {
            value: "This is a quote.".to_string(),
        };
        let text_hash = store.store(text);

        let para = AstNode::Paragraph {
            children: vec![text_hash],
        };
        let para_hash = store.store(para);

        let blockquote = AstNode::Blockquote {
            children: vec![para_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&blockquote).unwrap();

        assert_eq!(result, "> This is a quote.\n");
    }

    #[test]
    fn test_render_table() {
        let mut store = NodeStore::new();

        // Header cells
        let h1_text = AstNode::Text {
            value: "Name".to_string(),
        };
        let h1_text_hash = store.store(h1_text);
        let h1_cell = AstNode::TableCell {
            children: vec![h1_text_hash],
        };
        let h1_cell_hash = store.store(h1_cell);

        let h2_text = AstNode::Text {
            value: "Age".to_string(),
        };
        let h2_text_hash = store.store(h2_text);
        let h2_cell = AstNode::TableCell {
            children: vec![h2_text_hash],
        };
        let h2_cell_hash = store.store(h2_cell);

        // Header row
        let header_row = AstNode::TableRow {
            children: vec![h1_cell_hash, h2_cell_hash],
        };
        let header_row_hash = store.store(header_row);

        // Data cells
        let d1_text = AstNode::Text {
            value: "Alice".to_string(),
        };
        let d1_text_hash = store.store(d1_text);
        let d1_cell = AstNode::TableCell {
            children: vec![d1_text_hash],
        };
        let d1_cell_hash = store.store(d1_cell);

        let d2_text = AstNode::Text {
            value: "30".to_string(),
        };
        let d2_text_hash = store.store(d2_text);
        let d2_cell = AstNode::TableCell {
            children: vec![d2_text_hash],
        };
        let d2_cell_hash = store.store(d2_cell);

        // Data row
        let data_row = AstNode::TableRow {
            children: vec![d1_cell_hash, d2_cell_hash],
        };
        let data_row_hash = store.store(data_row);

        // Table
        let table = AstNode::Table {
            children: vec![header_row_hash, data_row_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&table).unwrap();

        let expected = "| Name | Age |\n| --- | --- |\n| Alice | 30 |\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_render_yaml_frontmatter() {
        let store = NodeStore::new();
        let yaml = AstNode::Yaml {
            value: "title: Test\nauthor: Alice".to_string(),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&yaml).unwrap();

        assert_eq!(result, "---\ntitle: Test\nauthor: Alice\n---\n");
    }

    #[test]
    fn test_render_toml_frontmatter() {
        let store = NodeStore::new();
        let toml = AstNode::Toml {
            value: "title = \"Test\"".to_string(),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&toml).unwrap();

        assert_eq!(result, "+++\ntitle = \"Test\"\n+++\n");
    }

    #[test]
    fn test_render_math_block() {
        let store = NodeStore::new();
        let math = AstNode::Math {
            value: "E = mc^2".to_string(),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&math).unwrap();

        assert_eq!(result, "$$\nE = mc^2\n$$\n");
    }

    #[test]
    fn test_render_inline_math() {
        let store = NodeStore::new();
        let math = AstNode::InlineMath {
            value: "x^2 + y^2 = z^2".to_string(),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&math).unwrap();

        assert_eq!(result, "$x^2 + y^2 = z^2$");
    }

    #[test]
    fn test_render_footnote_reference() {
        let store = NodeStore::new();
        let footnote = AstNode::FootnoteReference {
            identifier: "1".to_string(),
            label: None,
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&footnote).unwrap();

        assert_eq!(result, "[^1]");
    }

    #[test]
    fn test_render_footnote_definition() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "This is a footnote.".to_string(),
        };
        let text_hash = store.store(text);

        let footnote = AstNode::FootnoteDefinition {
            identifier: "1".to_string(),
            label: None,
            children: vec![text_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&footnote).unwrap();

        assert_eq!(result, "[^1]: This is a footnote.\n");
    }

    #[test]
    fn test_render_link_reference() {
        let mut store = NodeStore::new();
        let text = AstNode::Text {
            value: "click here".to_string(),
        };
        let text_hash = store.store(text);

        let link_ref = AstNode::LinkReference {
            reference_kind: crate::ast::ReferenceKind::Full,
            identifier: "link1".to_string(),
            label: None,
            children: vec![text_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&link_ref).unwrap();

        assert_eq!(result, "[click here][link1]");
    }

    #[test]
    fn test_render_definition() {
        let store = NodeStore::new();
        let definition = AstNode::Definition {
            identifier: "link1".to_string(),
            label: None,
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&definition).unwrap();

        assert_eq!(result, "[link1]: https://example.com \"Example\"\n");
    }

    #[test]
    fn test_render_html() {
        let store = NodeStore::new();
        let html = AstNode::Html {
            value: "<div>Custom HTML</div>".to_string(),
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&html).unwrap();

        assert_eq!(result, "<div>Custom HTML</div>");
    }

    #[test]
    fn test_missing_node_error() {
        let store = NodeStore::new();
        let fake_hash = crate::ast::Blake3Hash::new([0u8; 32]);

        let strong = AstNode::Strong {
            children: vec![fake_hash],
        };

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(&strong);

        assert!(matches!(result, Err(RenderError::MissingNode(_))));
    }

    // Round-trip tests
    #[test]
    fn test_round_trip_simple_text() {
        let markdown = "Hello, world!";
        let mut store = NodeStore::new();
        let root_hash = crate::convert::parse_markdown(markdown, &mut store).unwrap();
        let root = store.get(&root_hash).unwrap();

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(root).unwrap();

        // Should have paragraph wrapper
        assert_eq!(result, "Hello, world!\n");
    }

    #[test]
    fn test_round_trip_heading() {
        let markdown = "# Hello World";
        let mut store = NodeStore::new();
        let root_hash = crate::convert::parse_markdown(markdown, &mut store).unwrap();
        let root = store.get(&root_hash).unwrap();

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(root).unwrap();

        assert_eq!(result, "# Hello World\n");
    }

    #[test]
    fn test_round_trip_formatting() {
        let markdown = "This is **bold** and *italic* text.";
        let mut store = NodeStore::new();
        let root_hash = crate::convert::parse_markdown(markdown, &mut store).unwrap();
        let root = store.get(&root_hash).unwrap();

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(root).unwrap();

        assert_eq!(result, "This is **bold** and *italic* text.\n");
    }

    #[test]
    fn test_round_trip_list() {
        let markdown = "- Item 1\n- Item 2\n- Item 3";
        let mut store = NodeStore::new();
        let root_hash = crate::convert::parse_markdown(markdown, &mut store).unwrap();
        let root = store.get(&root_hash).unwrap();

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(root).unwrap();

        assert_eq!(result, "- Item 1\n- Item 2\n- Item 3\n");
    }

    #[test]
    fn test_round_trip_code_block() {
        let markdown = "```rust\nfn main() {}\n```";
        let mut store = NodeStore::new();
        let root_hash = crate::convert::parse_markdown(markdown, &mut store).unwrap();
        let root = store.get(&root_hash).unwrap();

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(root).unwrap();

        assert_eq!(result, "```rust\nfn main() {}\n```\n");
    }

    #[test]
    fn test_round_trip_complex_document() {
        let markdown = r#"# My Document

This is a paragraph with **bold** and *italic* text.

## Features

- Feature 1
- Feature 2
- Feature 3

```rust
fn main() {
    println!("Hello!");
}
```

> A wise quote
> from someone

---

The end.
"#;
        let mut store = NodeStore::new();
        let root_hash = crate::convert::parse_markdown(markdown, &mut store).unwrap();
        let root = store.get(&root_hash).unwrap();

        let mut renderer = MarkdownRenderer::new(&store);
        let result = renderer.render(root).unwrap();

        // Parse the result again to verify it's valid
        let mut store2 = NodeStore::new();
        let root_hash2 = crate::convert::parse_markdown(&result, &mut store2);

        // Should parse successfully
        assert!(root_hash2.is_ok(), "Rendered markdown should be parseable");
    }
}
