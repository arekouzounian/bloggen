//! HTML Extension System
//!
//! Provides a trait-based extension system for customizing HTML rendering.
//! Extensions can:
//! - Add CSS classes to elements
//! - Inject attributes
//! - Override rendering for specific node types
//! - Add wrapper elements
//!
//! Example extensions:
//! - TailwindExtension: Injects Tailwind CSS classes
//! - SyntaxHighlightExtension: Adds syntax highlighting to code blocks

use bgc_ast::AstNode;
use std::collections::HashMap;

/// Extension hook timing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookTiming {
    /// Before rendering the node's opening tag
    BeforeOpen,
    /// After rendering the node's opening tag (inside the element)
    AfterOpen,
    /// Before rendering the node's closing tag (inside the element)
    BeforeClose,
    /// After rendering the node's closing tag
    AfterClose,
}

/// Extension context provided to hooks
#[derive(Debug, Clone)]
pub struct ExtensionContext {
    /// The node being rendered
    pub node: AstNode,
    /// Current rendering depth
    pub depth: usize,
    /// Whether we're in an inline context
    pub inline: bool,
}

/// HTML rendering extension trait
pub trait HtmlExtension: Send + Sync {
    /// Get the name of this extension (for debugging)
    fn name(&self) -> &str;

    /// Add CSS classes to a node
    ///
    /// Returns a list of CSS class names to add to the element.
    fn add_classes(&self, _ctx: &ExtensionContext) -> Vec<String> {
        vec![]
    }

    /// Add HTML attributes to a node
    ///
    /// Returns a map of attribute name → value to add to the element.
    fn add_attributes(&self, _ctx: &ExtensionContext) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Override rendering for a specific node
    ///
    /// If this returns Some(html), the default rendering is replaced entirely.
    /// Return None to use default rendering.
    fn render_override(&self, _ctx: &ExtensionContext) -> Option<String> {
        None
    }

    /// Inject content at specific hook points
    ///
    /// Returns HTML to inject at the given timing.
    fn inject_content(&self, _ctx: &ExtensionContext, _timing: HookTiming) -> Option<String> {
        None
    }
}

/// Tailwind CSS extension
///
/// Injects Tailwind utility classes based on node type.
pub struct TailwindExtension {
    /// Custom class mappings (override defaults)
    custom_classes: HashMap<String, Vec<String>>,
}

impl TailwindExtension {
    /// Create a new Tailwind extension with default classes
    pub fn new() -> Self {
        Self {
            custom_classes: HashMap::new(),
        }
    }

    /// Add custom classes for a specific node type
    pub fn with_custom_classes(mut self, node_type: &str, classes: Vec<String>) -> Self {
        self.custom_classes.insert(node_type.to_string(), classes);
        self
    }
}

impl Default for TailwindExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlExtension for TailwindExtension {
    fn name(&self) -> &str {
        "tailwind"
    }

    fn add_classes(&self, ctx: &ExtensionContext) -> Vec<String> {
        let node_type = ctx.node.node_type();

        // Check custom classes first
        if let Some(classes) = self.custom_classes.get(node_type) {
            return classes.clone();
        }

        // Default Tailwind classes
        match &ctx.node {
            AstNode::Heading { level, .. } => {
                let mut classes = vec![
                    "font-bold".to_string(),
                    "mt-6".to_string(),
                    "mb-4".to_string(),
                ];
                let size_class = match level {
                    1 => "text-4xl",
                    2 => "text-3xl",
                    3 => "text-2xl",
                    4 => "text-xl",
                    _ => "text-lg",
                };
                classes.push(size_class.to_string());
                classes
            }
            AstNode::Paragraph { .. } => {
                vec![
                    "mb-4".to_string(),
                    "text-gray-800".to_string(),
                    "dark:text-gray-200".to_string(),
                ]
            }
            AstNode::CodeBlock { .. } => {
                vec![
                    "bg-gray-900".to_string(),
                    "text-gray-100".to_string(),
                    "p-4".to_string(),
                    "rounded".to_string(),
                    "overflow-x-auto".to_string(),
                    "my-4".to_string(),
                ]
            }
            AstNode::InlineCode { .. } => {
                vec![
                    "bg-gray-100".to_string(),
                    "dark:bg-gray-800".to_string(),
                    "px-1".to_string(),
                    "py-0.5".to_string(),
                    "rounded".to_string(),
                    "text-sm".to_string(),
                    "font-mono".to_string(),
                ]
            }
            AstNode::Blockquote { .. } => {
                vec![
                    "border-l-4".to_string(),
                    "border-gray-300".to_string(),
                    "dark:border-gray-600".to_string(),
                    "pl-4".to_string(),
                    "my-4".to_string(),
                    "italic".to_string(),
                    "text-gray-700".to_string(),
                    "dark:text-gray-300".to_string(),
                ]
            }
            AstNode::List { ordered, .. } => {
                if *ordered {
                    vec![
                        "list-decimal".to_string(),
                        "ml-6".to_string(),
                        "my-4".to_string(),
                    ]
                } else {
                    vec![
                        "list-disc".to_string(),
                        "ml-6".to_string(),
                        "my-4".to_string(),
                    ]
                }
            }
            AstNode::ListItem { .. } => {
                vec!["mb-2".to_string()]
            }
            AstNode::Link { .. } => {
                vec![
                    "text-blue-600".to_string(),
                    "dark:text-blue-400".to_string(),
                    "hover:underline".to_string(),
                ]
            }
            AstNode::Table { .. } => {
                vec![
                    "table-auto".to_string(),
                    "border-collapse".to_string(),
                    "my-4".to_string(),
                    "w-full".to_string(),
                ]
            }
            AstNode::TableCell { .. } => {
                vec![
                    "border".to_string(),
                    "border-gray-300".to_string(),
                    "px-4".to_string(),
                    "py-2".to_string(),
                ]
            }
            AstNode::ThematicBreak => {
                vec![
                    "my-8".to_string(),
                    "border-gray-300".to_string(),
                    "dark:border-gray-600".to_string(),
                ]
            }
            _ => vec![],
        }
    }
}

/// Apply extensions to HTML output
///
/// This helper function applies a list of extensions to generate
/// CSS classes and attributes for an element.
pub fn apply_extensions(
    node: &AstNode,
    extensions: &[Box<dyn HtmlExtension>],
    depth: usize,
    inline: bool,
) -> (Vec<String>, HashMap<String, String>) {
    let ctx = ExtensionContext {
        node: node.clone(),
        depth,
        inline,
    };

    let mut all_classes = Vec::new();
    let mut all_attrs = HashMap::new();

    for ext in extensions {
        all_classes.extend(ext.add_classes(&ctx));
        all_attrs.extend(ext.add_attributes(&ctx));
    }

    (all_classes, all_attrs)
}

/// Render opening tag with classes and attributes
pub fn render_opening_tag(
    tag: &str,
    classes: &[String],
    attributes: &HashMap<String, String>,
) -> String {
    let mut output = String::from("<");
    output.push_str(tag);

    if !classes.is_empty() {
        output.push_str(" class=\"");
        output.push_str(&classes.join(" "));
        output.push('"');
    }

    for (key, value) in attributes {
        output.push(' ');
        output.push_str(key);
        output.push_str("=\"");
        output.push_str(&escape_attr(value));
        output.push('"');
    }

    output.push('>');
    output
}

/// Escape HTML attribute values
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tailwind_extension_heading() {
        let ext = TailwindExtension::new();
        let node = AstNode::Heading {
            level: 2,
            children: vec![],
        };
        let ctx = ExtensionContext {
            node,
            depth: 0,
            inline: false,
        };

        let classes = ext.add_classes(&ctx);
        assert!(classes.contains(&"text-3xl".to_string()));
        assert!(classes.contains(&"font-bold".to_string()));
    }

    #[test]
    fn test_tailwind_extension_paragraph() {
        let ext = TailwindExtension::new();
        let node = AstNode::Paragraph { children: vec![] };
        let ctx = ExtensionContext {
            node,
            depth: 0,
            inline: false,
        };

        let classes = ext.add_classes(&ctx);
        assert!(classes.contains(&"mb-4".to_string()));
        assert!(classes.contains(&"text-gray-800".to_string()));
    }

    #[test]
    fn test_tailwind_extension_custom_classes() {
        let ext = TailwindExtension::new()
            .with_custom_classes("Paragraph", vec!["my-custom-class".to_string()]);

        let node = AstNode::Paragraph { children: vec![] };
        let ctx = ExtensionContext {
            node,
            depth: 0,
            inline: false,
        };

        let classes = ext.add_classes(&ctx);
        assert_eq!(classes, vec!["my-custom-class".to_string()]);
    }

    #[test]
    fn test_render_opening_tag() {
        let classes = vec!["foo".to_string(), "bar".to_string()];
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "test".to_string());

        let tag = render_opening_tag("div", &classes, &attrs);
        assert!(tag.contains("<div"));
        assert!(tag.contains("class=\"foo bar\""));
        assert!(tag.contains("id=\"test\""));
        assert!(tag.ends_with('>'));
    }

    #[test]
    fn test_apply_extensions() {
        let ext: Box<dyn HtmlExtension> = Box::new(TailwindExtension::new());
        let extensions = vec![ext];

        let node = AstNode::Paragraph { children: vec![] };
        let (classes, attrs) = apply_extensions(&node, &extensions, 0, false);

        assert!(!classes.is_empty());
        assert!(classes.contains(&"mb-4".to_string()));
        // TailwindExtension doesn't add attributes, so attrs should be empty
        assert!(attrs.is_empty());
    }
}
