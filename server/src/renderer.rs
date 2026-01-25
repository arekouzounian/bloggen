use anyhow::Result;
use bgc_ast::{AstNode, Blake3Hash};
use std::collections::HashMap;

/// Render an AST tree to markdown
pub fn render_to_markdown(
    root_hash: &Blake3Hash,
    nodes: &HashMap<Blake3Hash, AstNode>,
) -> Result<String> {
    let root = nodes
        .get(root_hash)
        .ok_or_else(|| anyhow::anyhow!("Root node not found"))?;

    let mut output = String::new();
    render_node(root, nodes, &mut output, 0)?;
    Ok(output)
}

fn render_node(
    node: &AstNode,
    nodes: &HashMap<Blake3Hash, AstNode>,
    output: &mut String,
    depth: usize,
) -> Result<()> {
    match node {
        AstNode::Root { children } => {
            render_children(children, nodes, output, depth)?;
        }
        AstNode::Heading { level, children } => {
            output.push_str(&"#".repeat(*level as usize));
            output.push(' ');
            render_children(children, nodes, output, depth)?;
            output.push('\n');
        }
        AstNode::Paragraph { children } => {
            if depth > 0 {
                output.push('\n');
            }
            render_children(children, nodes, output, depth)?;
            output.push('\n');
        }
        AstNode::Text { value } => {
            output.push_str(value);
        }
        AstNode::Strong { children } => {
            output.push_str("**");
            render_children(children, nodes, output, depth)?;
            output.push_str("**");
        }
        AstNode::Emphasis { children } => {
            output.push('*');
            render_children(children, nodes, output, depth)?;
            output.push('*');
        }
        AstNode::Delete { children } => {
            output.push_str("~~");
            render_children(children, nodes, output, depth)?;
            output.push_str("~~");
        }
        AstNode::CodeBlock { lang, value } => {
            output.push('\n');
            output.push_str("```");
            if let Some(lang) = lang {
                output.push_str(lang);
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
            render_children(children, nodes, output, depth)?;
            output.push_str("](");
            output.push_str(url);
            if let Some(title) = title {
                output.push_str(" \"");
                output.push_str(title);
                output.push('"');
            }
            output.push(')');
        }
        AstNode::Image { url, alt, title } => {
            output.push_str("![");
            output.push_str(alt);
            output.push_str("](");
            output.push_str(url);
            if let Some(title) = title {
                output.push_str(" \"");
                output.push_str(title);
                output.push('"');
            }
            output.push(')');
        }
        AstNode::List {
            ordered,
            start,
            children,
        } => {
            output.push('\n');
            for (i, child_hash) in children.iter().enumerate() {
                if let Some(child) = nodes.get(child_hash) {
                    if *ordered {
                        let num = start.unwrap_or(1) + i as u32;
                        output.push_str(&format!("{}. ", num));
                    } else {
                        output.push_str("- ");
                    }
                    render_node(child, nodes, output, depth + 1)?;
                }
            }
        }
        AstNode::ListItem { checked, children } => {
            if let Some(checked) = checked {
                output.push_str(if *checked { "[x] " } else { "[ ] " });
            }
            render_children(children, nodes, output, depth)?;
            output.push('\n');
        }
        AstNode::Blockquote { children } => {
            output.push('\n');
            output.push_str("> ");
            render_children(children, nodes, output, depth)?;
            output.push('\n');
        }
        AstNode::Break => {
            output.push_str("  \n");
        }
        AstNode::ThematicBreak => {
            output.push('\n');
            output.push_str("---\n");
        }
        AstNode::Table { children } => {
            output.push('\n');
            render_children(children, nodes, output, depth)?;
        }
        AstNode::TableRow { children } => {
            output.push('|');
            for child_hash in children {
                output.push(' ');
                if let Some(child) = nodes.get(child_hash) {
                    render_node(child, nodes, output, depth)?;
                }
                output.push_str(" |");
            }
            output.push('\n');
        }
        AstNode::TableCell { children } => {
            render_children(children, nodes, output, depth)?;
        }
        AstNode::Html { value } => {
            output.push_str(value);
        }
        AstNode::Yaml { value } => {
            output.push_str("---\n");
            output.push_str(value);
            output.push_str("\n---\n");
        }
        AstNode::Toml { value } => {
            output.push_str("+++\n");
            output.push_str(value);
            output.push_str("\n+++\n");
        }
        _ => {
            // Handle other node types with default behavior
            let children = node.children();
            if !children.is_empty() {
                render_children(children, nodes, output, depth)?;
            }
        }
    }

    Ok(())
}

fn render_children(
    children: &[Blake3Hash],
    nodes: &HashMap<Blake3Hash, AstNode>,
    output: &mut String,
    depth: usize,
) -> Result<()> {
    for child_hash in children {
        if let Some(child) = nodes.get(child_hash) {
            render_node(child, nodes, output, depth)?;
        }
    }
    Ok(())
}
