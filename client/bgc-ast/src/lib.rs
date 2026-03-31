pub mod render;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A Blake3 hash used as a content-addressable identifier for AST nodes.
/// This is a 32-byte hash that uniquely identifies node content.
///
/// Serializes as a hex string (64 characters) rather than a byte array to reduce
/// JSON size by ~30% per hash reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blake3Hash([u8; 32]);

impl Blake3Hash {
    /// Create a new Blake3Hash from a 32-byte array
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes of the hash
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to a byte array
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Encode as a hex string
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Decode from a hex string
    pub fn from_hex(s: &str) -> Result<Self, String> {
        if s.len() != 64 {
            return Err(format!("Expected 64 hex characters, got {}", s.len()));
        }

        let mut bytes = [0u8; 32];
        for i in 0..32 {
            let byte_str = &s[i * 2..i * 2 + 2];
            bytes[i] = u8::from_str_radix(byte_str, 16)
                .map_err(|e| format!("Invalid hex at position {}: {}", i * 2, e))?;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for Blake3Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Blake3Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// Owned AST node representation for BlogGen.
///
/// This is a simplified, owned version of the markdown-rs AST that:
/// - Excludes position/location metadata (line, column, offset)
/// - Stores children as hashes (not inline) for content-addressable storage
/// - Only includes semantic information needed for rendering
///
/// Children are stored as Blake3 hashes to enable:
/// - Deduplication: identical subtrees stored once
/// - Efficient updates: only changed nodes transmitted
/// - Version history: free (just store root hash snapshots)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AstNode {
    /// Root node of the document
    Root {
        children: Vec<Blake3Hash>,
    },

    /// Heading with level 1-6
    Heading {
        level: u8,
        children: Vec<Blake3Hash>,
    },

    /// Paragraph containing inline content
    Paragraph {
        children: Vec<Blake3Hash>,
    },

    /// Ordered or unordered list
    List {
        ordered: bool,
        start: Option<u32>,
        children: Vec<Blake3Hash>,
    },

    /// List item (may contain checkbox for task lists)
    ListItem {
        checked: Option<bool>,
        children: Vec<Blake3Hash>,
    },

    /// Blockquote
    Blockquote {
        children: Vec<Blake3Hash>,
    },

    /// Strong emphasis (bold)
    Strong {
        children: Vec<Blake3Hash>,
    },

    /// Emphasis (italic)
    Emphasis {
        children: Vec<Blake3Hash>,
    },

    /// Strikethrough text (GFM extension)
    Delete {
        children: Vec<Blake3Hash>,
    },

    /// Plain text content (leaf node)
    Text {
        value: String,
    },

    /// Code block with optional language
    CodeBlock {
        lang: Option<String>,
        value: String,
    },

    /// Inline code
    InlineCode {
        value: String,
    },

    /// Link with URL, optional title, and text content
    Link {
        url: String,
        title: Option<String>,
        children: Vec<Blake3Hash>,
    },

    /// Image with URL, alt text, and optional title
    Image {
        url: String,
        alt: String,
        title: Option<String>,
    },

    /// Hard line break
    Break,

    /// Thematic break (horizontal rule)
    ThematicBreak,

    // GFM Tables
    /// Table
    Table {
        children: Vec<Blake3Hash>,
    },

    /// Table row
    TableRow {
        children: Vec<Blake3Hash>,
    },

    /// Table cell
    TableCell {
        children: Vec<Blake3Hash>,
    },

    // HTML and References
    /// Raw HTML content
    Html {
        value: String,
    },

    /// Link or image reference definition
    Definition {
        identifier: String,
        label: Option<String>,
        url: String,
        title: Option<String>,
    },

    /// Reference to a link definition
    LinkReference {
        reference_kind: ReferenceKind,
        identifier: String,
        label: Option<String>,
        children: Vec<Blake3Hash>,
    },

    /// Reference to an image definition
    ImageReference {
        reference_kind: ReferenceKind,
        identifier: String,
        label: Option<String>,
        alt: String,
    },

    // Frontmatter
    /// YAML frontmatter
    Yaml {
        value: String,
    },

    /// TOML frontmatter
    Toml {
        value: String,
    },

    // Footnotes
    /// Footnote definition
    FootnoteDefinition {
        identifier: String,
        label: Option<String>,
        children: Vec<Blake3Hash>,
    },

    /// Footnote reference
    FootnoteReference {
        identifier: String,
        label: Option<String>,
    },

    // Math (optional extension)
    /// Block math (LaTeX)
    Math {
        value: String,
    },

    /// Inline math (LaTeX)
    InlineMath {
        value: String,
    },

    // MDX Extensions (for MDX support)
    /// MDX ESM import/export
    MdxjsEsm {
        value: String,
    },

    /// MDX flow expression
    MdxFlowExpression {
        value: String,
    },

    /// MDX text expression
    MdxTextExpression {
        value: String,
    },

    /// MDX JSX flow element
    MdxJsxFlowElement {
        name: Option<String>,
        children: Vec<Blake3Hash>,
    },

    /// MDX JSX text element
    MdxJsxTextElement {
        name: Option<String>,
        children: Vec<Blake3Hash>,
    },
}

/// Reference kind for link and image references
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    /// `[text][label]`
    Full,
    /// `[text][]` or `[text]`
    Collapsed,
    /// `[text]`
    Shortcut,
}

impl AstNode {
    /// Returns true if this node is a leaf node (has no children)
    pub fn is_leaf(&self) -> bool {
        matches!(
            self,
            AstNode::Text { .. }
                | AstNode::CodeBlock { .. }
                | AstNode::InlineCode { .. }
                | AstNode::Image { .. }
                | AstNode::Break
                | AstNode::ThematicBreak
                | AstNode::Html { .. }
                | AstNode::Definition { .. }
                | AstNode::ImageReference { .. }
                | AstNode::Yaml { .. }
                | AstNode::Toml { .. }
                | AstNode::FootnoteReference { .. }
                | AstNode::Math { .. }
                | AstNode::InlineMath { .. }
                | AstNode::MdxjsEsm { .. }
                | AstNode::MdxFlowExpression { .. }
                | AstNode::MdxTextExpression { .. }
        )
    }

    /// Returns the child hashes for this node, or an empty slice for leaf nodes
    pub fn children(&self) -> &[Blake3Hash] {
        match self {
            AstNode::Root { children }
            | AstNode::Heading { children, .. }
            | AstNode::Paragraph { children }
            | AstNode::List { children, .. }
            | AstNode::ListItem { children, .. }
            | AstNode::Blockquote { children }
            | AstNode::Strong { children }
            | AstNode::Emphasis { children }
            | AstNode::Delete { children }
            | AstNode::Link { children, .. }
            | AstNode::Table { children }
            | AstNode::TableRow { children }
            | AstNode::TableCell { children }
            | AstNode::LinkReference { children, .. }
            | AstNode::FootnoteDefinition { children, .. }
            | AstNode::MdxJsxFlowElement { children, .. }
            | AstNode::MdxJsxTextElement { children, .. } => children,
            _ => &[],
        }
    }

    /// Returns the discriminant name for debugging
    pub fn node_type(&self) -> &'static str {
        match self {
            AstNode::Root { .. } => "Root",
            AstNode::Heading { .. } => "Heading",
            AstNode::Paragraph { .. } => "Paragraph",
            AstNode::List { .. } => "List",
            AstNode::ListItem { .. } => "ListItem",
            AstNode::Blockquote { .. } => "Blockquote",
            AstNode::Strong { .. } => "Strong",
            AstNode::Emphasis { .. } => "Emphasis",
            AstNode::Delete { .. } => "Delete",
            AstNode::Text { .. } => "Text",
            AstNode::CodeBlock { .. } => "CodeBlock",
            AstNode::InlineCode { .. } => "InlineCode",
            AstNode::Link { .. } => "Link",
            AstNode::Image { .. } => "Image",
            AstNode::Break => "Break",
            AstNode::ThematicBreak => "ThematicBreak",
            AstNode::Table { .. } => "Table",
            AstNode::TableRow { .. } => "TableRow",
            AstNode::TableCell { .. } => "TableCell",
            AstNode::Html { .. } => "Html",
            AstNode::Definition { .. } => "Definition",
            AstNode::LinkReference { .. } => "LinkReference",
            AstNode::ImageReference { .. } => "ImageReference",
            AstNode::Yaml { .. } => "Yaml",
            AstNode::Toml { .. } => "Toml",
            AstNode::FootnoteDefinition { .. } => "FootnoteDefinition",
            AstNode::FootnoteReference { .. } => "FootnoteReference",
            AstNode::Math { .. } => "Math",
            AstNode::InlineMath { .. } => "InlineMath",
            AstNode::MdxjsEsm { .. } => "MdxjsEsm",
            AstNode::MdxFlowExpression { .. } => "MdxFlowExpression",
            AstNode::MdxTextExpression { .. } => "MdxTextExpression",
            AstNode::MdxJsxFlowElement { .. } => "MdxJsxFlowElement",
            AstNode::MdxJsxTextElement { .. } => "MdxJsxTextElement",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_leaf() {
        let text = AstNode::Text {
            value: "hello".to_string(),
        };
        assert!(text.is_leaf());

        let para = AstNode::Paragraph { children: vec![] };
        assert!(!para.is_leaf());
    }

    #[test]
    fn test_children() {
        let hash = Blake3Hash::new([0; 32]);
        let para = AstNode::Paragraph {
            children: vec![hash],
        };
        assert_eq!(para.children().len(), 1);

        let text = AstNode::Text {
            value: "hello".to_string(),
        };
        assert_eq!(text.children().len(), 0);
    }

    #[test]
    fn test_serialization() {
        let node = AstNode::Text {
            value: "hello world".to_string(),
        };
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"value\":\"hello world\""));
    }

    #[test]
    fn test_leaf_variants() {
        // All leaf nodes should return true for is_leaf()
        assert!(AstNode::InlineCode { value: "x".into() }.is_leaf());
        assert!(AstNode::Break.is_leaf());
        assert!(AstNode::ThematicBreak.is_leaf());
        assert!(AstNode::Html { value: "<hr>".into() }.is_leaf());
        assert!(AstNode::Math { value: "x^2".into() }.is_leaf());
        assert!(AstNode::InlineMath { value: "y".into() }.is_leaf());
        assert!(AstNode::Yaml { value: "key: val".into() }.is_leaf());
        assert!(AstNode::Toml { value: "[meta]".into() }.is_leaf());
        assert!(AstNode::MdxjsEsm { value: "import x".into() }.is_leaf());
        assert!(AstNode::MdxFlowExpression { value: "{a}".into() }.is_leaf());
        assert!(AstNode::MdxTextExpression { value: "{b}".into() }.is_leaf());

        // Non-leaf nodes
        assert!(!AstNode::Root { children: vec![] }.is_leaf());
        assert!(!AstNode::Heading { level: 1, children: vec![] }.is_leaf());
        assert!(!AstNode::List { ordered: false, children: vec![] }.is_leaf());
        assert!(!AstNode::Blockquote { children: vec![] }.is_leaf());
        assert!(!AstNode::Strong { children: vec![] }.is_leaf());
        assert!(!AstNode::Emphasis { children: vec![] }.is_leaf());
    }

    #[test]
    fn test_node_type_strings() {
        assert_eq!(AstNode::Root { children: vec![] }.node_type(), "Root");
        assert_eq!(
            AstNode::Heading {
                level: 1,
                children: vec![]
            }
            .node_type(),
            "Heading"
        );
        assert_eq!(
            AstNode::Paragraph { children: vec![] }.node_type(),
            "Paragraph"
        );
        assert_eq!(
            AstNode::Text { value: "t".into() }.node_type(),
            "Text"
        );
        assert_eq!(
            AstNode::InlineCode { value: "c".into() }.node_type(),
            "InlineCode"
        );
        assert_eq!(
            AstNode::CodeBlock {
                value: "x".into(),
                lang: None
            }
            .node_type(),
            "CodeBlock"
        );
        assert_eq!(AstNode::Break.node_type(), "Break");
        assert_eq!(AstNode::ThematicBreak.node_type(), "ThematicBreak");
        assert_eq!(
            AstNode::Strong { children: vec![] }.node_type(),
            "Strong"
        );
        assert_eq!(
            AstNode::Emphasis { children: vec![] }.node_type(),
            "Emphasis"
        );
        assert_eq!(
            AstNode::Delete { children: vec![] }.node_type(),
            "Delete"
        );
        assert_eq!(
            AstNode::Html { value: "".into() }.node_type(),
            "Html"
        );
        assert_eq!(
            AstNode::Math { value: "".into() }.node_type(),
            "Math"
        );
        assert_eq!(
            AstNode::InlineMath { value: "".into() }.node_type(),
            "InlineMath"
        );
        assert_eq!(AstNode::Yaml { value: "".into() }.node_type(), "Yaml");
        assert_eq!(AstNode::Toml { value: "".into() }.node_type(), "Toml");
        assert_eq!(
            AstNode::List {
                ordered: true,
                children: vec![]
            }
            .node_type(),
            "List"
        );
        assert_eq!(
            AstNode::ListItem {
                spread: false,
                children: vec![],
                checked: None
            }
            .node_type(),
            "ListItem"
        );
        assert_eq!(
            AstNode::Blockquote { children: vec![] }.node_type(),
            "Blockquote"
        );
        assert_eq!(
            AstNode::Table {
                align: vec![],
                children: vec![]
            }
            .node_type(),
            "Table"
        );
        assert_eq!(
            AstNode::TableRow { children: vec![] }.node_type(),
            "TableRow"
        );
        assert_eq!(
            AstNode::TableCell { children: vec![] }.node_type(),
            "TableCell"
        );
        assert_eq!(
            AstNode::Link {
                url: "".into(),
                title: None,
                children: vec![]
            }
            .node_type(),
            "Link"
        );
        assert_eq!(
            AstNode::Image {
                url: "".into(),
                alt: "".into(),
                title: None
            }
            .node_type(),
            "Image"
        );
    }

    #[test]
    fn test_blake3hash_roundtrip() {
        let original = Blake3Hash::new([42u8; 32]);
        let hex = original.to_hex();
        let recovered = Blake3Hash::from_hex(&hex).unwrap();
        assert_eq!(original.as_bytes(), recovered.as_bytes());
    }

    #[test]
    fn test_blake3hash_from_hex_invalid() {
        // Wrong length
        assert!(Blake3Hash::from_hex("abc").is_err());
        // Non-hex characters
        assert!(Blake3Hash::from_hex(&"zz".repeat(32)).is_err());
        // Empty string
        assert!(Blake3Hash::from_hex("").is_err());
    }

    #[test]
    fn test_blake3hash_serialization() {
        let hash = Blake3Hash::new([1u8; 32]);
        let json = serde_json::to_string(&hash).unwrap();
        // Should serialize as a hex string
        assert!(json.starts_with('"'));
        let recovered: Blake3Hash = serde_json::from_str(&json).unwrap();
        assert_eq!(hash.as_bytes(), recovered.as_bytes());
    }
}
