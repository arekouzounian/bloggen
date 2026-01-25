//! AST → Markdown renderer
//!
//! This module provides a wrapper around the shared bgc-ast markdown renderer,
//! adapting it to work with the server's NodeStore trait and async interfaces.

use crate::error::AppError;
use crate::storage::NodeStore;
use bgc_ast::Blake3Hash;

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

    // Convert to the format expected by bgc-ast renderer
    let root_node = nodes
        .get(&root_hash)
        .ok_or_else(|| AppError::Internal(format!("Missing root node: {}", root_hash.to_hex())))?;

    // Use the shared renderer from bgc-ast
    let mut renderer = bgc_ast::render::MarkdownRenderer::new(&nodes);
    renderer
        .render(root_node)
        .map_err(|e| AppError::Internal(format!("Render error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bgc_ast::AstNode;
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

    #[tokio::test]
    async fn test_render_table() {
        let mut store = MockNodeStore::new();

        // Header cells
        let h1_text = AstNode::Text {
            value: "Name".to_string(),
        };
        let h1_text_hash = hash_node(&h1_text);
        store.insert(h1_text_hash, h1_text);

        let h1_cell = AstNode::TableCell {
            children: vec![h1_text_hash],
        };
        let h1_cell_hash = hash_node(&h1_cell);
        store.insert(h1_cell_hash, h1_cell);

        let h2_text = AstNode::Text {
            value: "Age".to_string(),
        };
        let h2_text_hash = hash_node(&h2_text);
        store.insert(h2_text_hash, h2_text);

        let h2_cell = AstNode::TableCell {
            children: vec![h2_text_hash],
        };
        let h2_cell_hash = hash_node(&h2_cell);
        store.insert(h2_cell_hash, h2_cell);

        // Header row
        let header_row = AstNode::TableRow {
            children: vec![h1_cell_hash, h2_cell_hash],
        };
        let header_row_hash = hash_node(&header_row);
        store.insert(header_row_hash, header_row);

        // Data cells
        let d1_text = AstNode::Text {
            value: "Alice".to_string(),
        };
        let d1_text_hash = hash_node(&d1_text);
        store.insert(d1_text_hash, d1_text);

        let d1_cell = AstNode::TableCell {
            children: vec![d1_text_hash],
        };
        let d1_cell_hash = hash_node(&d1_cell);
        store.insert(d1_cell_hash, d1_cell);

        let d2_text = AstNode::Text {
            value: "30".to_string(),
        };
        let d2_text_hash = hash_node(&d2_text);
        store.insert(d2_text_hash, d2_text);

        let d2_cell = AstNode::TableCell {
            children: vec![d2_text_hash],
        };
        let d2_cell_hash = hash_node(&d2_cell);
        store.insert(d2_cell_hash, d2_cell);

        // Data row
        let data_row = AstNode::TableRow {
            children: vec![d1_cell_hash, d2_cell_hash],
        };
        let data_row_hash = hash_node(&data_row);
        store.insert(data_row_hash, data_row);

        // Table
        let table = AstNode::Table {
            children: vec![header_row_hash, data_row_hash],
        };
        let table_hash = hash_node(&table);
        store.insert(table_hash, table);

        let root = AstNode::Root {
            children: vec![table_hash],
        };
        let root_hash = hash_node(&root);
        store.insert(root_hash, root);

        let markdown = render_to_markdown(root_hash, &store).await.unwrap();

        // Verify table separator row is present (this was broken in the old server implementation)
        assert!(markdown.contains("| Name | Age |"));
        assert!(markdown.contains("| --- | --- |"));
        assert!(markdown.contains("| Alice | 30 |"));
    }

    #[tokio::test]
    async fn test_render_quote_escaping() {
        let mut store = MockNodeStore::new();

        let text = AstNode::Text {
            value: "link text".to_string(),
        };
        let text_hash = hash_node(&text);
        store.insert(text_hash, text);

        let link = AstNode::Link {
            url: "https://example.com".to_string(),
            title: Some("Title with \"quotes\"".to_string()),
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

        let markdown = render_to_markdown(root_hash, &store).await.unwrap();

        // Verify quotes are properly escaped (this was missing in the old server implementation)
        assert!(markdown.contains(r#"Title with \"quotes\""#));
    }
}
