# bgc-ast

Shared AST (Abstract Syntax Tree) types for the BlogGen project.

## Purpose

This crate contains the core AST node definitions and Blake3 hash types used by both the BlogGen client (`bgc`) and server (`server`). By extracting these types into a separate crate, we ensure:

1. **Single source of truth**: AST types are defined once and used everywhere
2. **Consistency**: Client and server always use compatible types
3. **Easy updates**: Changes to AST structure only need to be made in one place

## Types

### `Blake3Hash`

A 32-byte Blake3 hash used as a content-addressable identifier for AST nodes. Serializes as a hex string (64 characters) for efficient JSON representation.

### `AstNode`

An enum representing all possible markdown AST node types, including:
- Block elements: Heading, Paragraph, List, Blockquote, CodeBlock, etc.
- Inline elements: Text, Strong, Emphasis, Link, InlineCode, etc.
- GFM extensions: Tables, Strikethrough
- Additional features: Frontmatter (YAML/TOML), Footnotes, Math, MDX

Children are stored as `Blake3Hash` references rather than inline content, enabling:
- **Deduplication**: Identical subtrees stored once
- **Efficient updates**: Only changed nodes transmitted
- **Version history**: Free (just store root hash snapshots)

### `ReferenceKind`

An enum for link and image reference types (Full, Collapsed, Shortcut).

## Dependencies

- `serde` with derive feature for serialization/deserialization

## Usage

In your `Cargo.toml`:

```toml
[dependencies]
bgc-ast = { path = "../bgc-ast" }
```

In your code:

```rust
use bgc_ast::{AstNode, Blake3Hash};

let hash = Blake3Hash::new([0u8; 32]);
let node = AstNode::Text {
    value: "Hello, world!".to_string(),
};
```
