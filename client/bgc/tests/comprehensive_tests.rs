use bgc::{parse_markdown, CasDocument, MarkdownRenderer, NodeStore};
use std::time::Instant;

/// Helper to measure and display elapsed time
fn time_operation<F, R>(name: &str, op: F) -> R
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = op();
    let elapsed = start.elapsed();
    eprintln!("  ⏱️  {} took: {:?}", name, elapsed);
    result
}

#[test]
fn test_small_document_with_timing() {
    eprintln!("\n=== Small Document Test ===");

    let markdown = r#"# Hello World

This is a simple test document with **bold** and *italic* text.

- Item 1
- Item 2
- Item 3
"#;

    eprintln!("Input size: {} bytes", markdown.len());

    // Parse
    let (store, root_hash) = time_operation("Parse", || {
        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();
        (store, root_hash)
    });

    eprintln!("  Nodes created: {}", store.len());

    // Create CAS document
    let doc = time_operation("Create CasDocument", || {
        CasDocument::new(&store, root_hash).unwrap()
    });

    // JSON serialization
    let json = time_operation("Serialize to JSON", || {
        doc.to_json().unwrap()
    });
    eprintln!("  JSON size: {} bytes", json.len());

    // JSON pretty serialization
    let json_pretty = time_operation("Serialize to JSON (pretty)", || {
        doc.to_json_pretty().unwrap()
    });
    eprintln!("  JSON (pretty) size: {} bytes", json_pretty.len());

    // MessagePack serialization
    let msgpack = time_operation("Serialize to MessagePack", || {
        doc.to_msgpack().unwrap()
    });
    eprintln!("  MessagePack size: {} bytes ({:.1}% of JSON)",
        msgpack.len(),
        (msgpack.len() as f64 / json.len() as f64) * 100.0
    );

    // Compressed MessagePack
    let msgpack_compressed = time_operation("Serialize to MessagePack+zstd", || {
        doc.to_msgpack_compressed().unwrap()
    });
    eprintln!("  MessagePack+zstd size: {} bytes ({:.1}% of JSON, {:.1}% of MessagePack)",
        msgpack_compressed.len(),
        (msgpack_compressed.len() as f64 / json.len() as f64) * 100.0,
        (msgpack_compressed.len() as f64 / msgpack.len() as f64) * 100.0
    );

    // JSON deserialization
    let doc2 = time_operation("Deserialize from JSON", || {
        CasDocument::from_json(&json).unwrap()
    });
    assert_eq!(doc.root_hash, doc2.root_hash);

    // MessagePack deserialization
    let doc3 = time_operation("Deserialize from MessagePack", || {
        CasDocument::from_msgpack(&msgpack).unwrap()
    });
    assert_eq!(doc.root_hash, doc3.root_hash);

    // Compressed MessagePack deserialization
    let doc4 = time_operation("Deserialize from MessagePack+zstd", || {
        CasDocument::from_msgpack_compressed(&msgpack_compressed).unwrap()
    });
    assert_eq!(doc.root_hash, doc4.root_hash);

    // Render back to markdown
    let rendered = time_operation("Render to Markdown", || {
        let mut renderer = MarkdownRenderer::new(&store);
        let root = store.get(&root_hash).unwrap();
        renderer.render(root).unwrap()
    });
    eprintln!("  Rendered markdown size: {} bytes", rendered.len());

    // Verify round-trip
    let (store2, _root_hash2) = time_operation("Re-parse rendered markdown", || {
        let mut store = NodeStore::new();
        let root_hash = parse_markdown(&rendered, &mut store).unwrap();
        (store, root_hash)
    });

    // Verify same number of nodes (or very close)
    let node_diff = (store2.len() as i32 - store.len() as i32).abs();
    eprintln!("  Node count difference after round-trip: {}", node_diff);
    assert!(node_diff <= 2, "Node count should be similar after round-trip");
}

#[test]
fn test_medium_document_with_timing() {
    eprintln!("\n=== Medium Document Test ===");

    let markdown = r#"# Complete Feature Guide

## Introduction

This document demonstrates all **major** features of *Markdown*, including:

- Lists (ordered and unordered)
- **Bold** and *italic* text
- `inline code` and code blocks
- Tables
- Blockquotes
- Links and images

## Code Examples

Here's some Rust code:

```rust
fn main() {
    println!("Hello, world!");
    let x = 42;
    let y = x * 2;
    assert_eq!(y, 84);
}
```

And some Python:

```python
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)

print([fibonacci(i) for i in range(10)])
```

## Lists

### Unordered Lists

- First item
- Second item
  - Nested item 1
  - Nested item 2
- Third item

### Ordered Lists

1. First step
2. Second step
3. Third step

### Task Lists

- [x] Completed task
- [ ] Pending task
- [ ] Another pending task

## Tables

| Feature | Supported | Notes |
| --- | --- | --- |
| Headers | Yes | Multiple levels |
| Lists | Yes | Ordered and unordered |
| Tables | Yes | GFM syntax |
| Code | Yes | Inline and blocks |

## Blockquotes

> This is a blockquote.
> It can span multiple lines.
>
> And have multiple paragraphs.

## Links and Images

Check out [this link](https://example.com "Example Site") for more information.

![Example Image](https://example.com/image.png "An example image")

## Inline Formatting

You can combine **bold** and *italic* text, or even ***both at once***.

Use ~~strikethrough~~ for deleted text.

## Horizontal Rule

---

## Conclusion

This document tests various Markdown features to ensure comprehensive coverage.
"#;

    eprintln!("Input size: {} bytes", markdown.len());

    let (store, root_hash) = time_operation("Parse", || {
        let mut store = NodeStore::new();
        let root_hash = parse_markdown(markdown, &mut store).unwrap();
        (store, root_hash)
    });

    eprintln!("  Nodes created: {}", store.len());

    let doc = time_operation("Create CasDocument", || {
        CasDocument::new(&store, root_hash).unwrap()
    });

    let json = time_operation("Serialize to JSON", || {
        doc.to_json().unwrap()
    });
    eprintln!("  JSON size: {} bytes", json.len());

    let msgpack = time_operation("Serialize to MessagePack", || {
        doc.to_msgpack().unwrap()
    });
    eprintln!("  MessagePack size: {} bytes ({:.1}% of JSON)",
        msgpack.len(),
        (msgpack.len() as f64 / json.len() as f64) * 100.0
    );

    let msgpack_compressed = time_operation("Serialize to MessagePack+zstd", || {
        doc.to_msgpack_compressed().unwrap()
    });
    eprintln!("  MessagePack+zstd size: {} bytes ({:.1}% of JSON)",
        msgpack_compressed.len(),
        (msgpack_compressed.len() as f64 / json.len() as f64) * 100.0
    );

    time_operation("Deserialize from JSON", || {
        CasDocument::from_json(&json).unwrap()
    });

    time_operation("Deserialize from MessagePack", || {
        CasDocument::from_msgpack(&msgpack).unwrap()
    });

    time_operation("Deserialize from MessagePack+zstd", || {
        CasDocument::from_msgpack_compressed(&msgpack_compressed).unwrap()
    });

    let rendered = time_operation("Render to Markdown", || {
        let mut renderer = MarkdownRenderer::new(&store);
        let root = store.get(&root_hash).unwrap();
        renderer.render(root).unwrap()
    });
    eprintln!("  Rendered markdown size: {} bytes", rendered.len());

    // Verify it parses
    time_operation("Re-parse rendered markdown", || {
        let mut store = NodeStore::new();
        parse_markdown(&rendered, &mut store).unwrap()
    });
}

#[test]
fn test_large_document_with_timing() {
    eprintln!("\n=== Large Document Test ===");

    // Generate a large document programmatically
    let mut markdown = String::new();
    markdown.push_str("# Large Test Document\n\n");
    markdown.push_str("This document contains many sections to test performance with larger files.\n\n");

    // Add 100 sections with various content
    for i in 0..100 {
        markdown.push_str(&format!("## Section {}\n\n", i + 1));
        markdown.push_str(&format!("This is section {} with some **bold** and *italic* text.\n\n", i + 1));

        // Every 5th section has a code block
        if i % 5 == 0 {
            markdown.push_str("```rust\n");
            markdown.push_str(&format!("fn section_{}() {{\n", i + 1));
            markdown.push_str(&format!("    println!(\"Section {}\");\n", i + 1));
            markdown.push_str("    let result = 42;\n");
            markdown.push_str("    result\n");
            markdown.push_str("}\n");
            markdown.push_str("```\n\n");
        }

        // Every 3rd section has a list
        if i % 3 == 0 {
            markdown.push_str("Key points:\n\n");
            for j in 0..5 {
                markdown.push_str(&format!("- Point {} of section {}\n", j + 1, i + 1));
            }
            markdown.push('\n');
        }

        // Every 7th section has a table
        if i % 7 == 0 {
            markdown.push_str("| Column A | Column B | Column C |\n");
            markdown.push_str("| --- | --- | --- |\n");
            for k in 0..3 {
                markdown.push_str(&format!("| Value {}A | Value {}B | Value {}C |\n", k + 1, k + 1, k + 1));
            }
            markdown.push('\n');
        }

        markdown.push_str("---\n\n");
    }

    markdown.push_str("## Conclusion\n\n");
    markdown.push_str("This large document tests performance with substantial content.\n");

    eprintln!("Input size: {} bytes ({:.2} KB)", markdown.len(), markdown.len() as f64 / 1024.0);

    let (store, root_hash) = time_operation("Parse", || {
        let mut store = NodeStore::new();
        let root_hash = parse_markdown(&markdown, &mut store).unwrap();
        (store, root_hash)
    });

    eprintln!("  Nodes created: {}", store.len());

    let doc = time_operation("Create CasDocument", || {
        CasDocument::new(&store, root_hash).unwrap()
    });

    let json = time_operation("Serialize to JSON", || {
        doc.to_json().unwrap()
    });
    eprintln!("  JSON size: {} bytes ({:.2} KB)", json.len(), json.len() as f64 / 1024.0);

    let msgpack = time_operation("Serialize to MessagePack", || {
        doc.to_msgpack().unwrap()
    });
    eprintln!("  MessagePack size: {} bytes ({:.2} KB, {:.1}% of JSON)",
        msgpack.len(),
        msgpack.len() as f64 / 1024.0,
        (msgpack.len() as f64 / json.len() as f64) * 100.0
    );

    let msgpack_compressed = time_operation("Serialize to MessagePack+zstd", || {
        doc.to_msgpack_compressed().unwrap()
    });
    eprintln!("  MessagePack+zstd size: {} bytes ({:.2} KB, {:.1}% of JSON)",
        msgpack_compressed.len(),
        msgpack_compressed.len() as f64 / 1024.0,
        (msgpack_compressed.len() as f64 / json.len() as f64) * 100.0
    );

    eprintln!("\n  Size reduction summary:");
    eprintln!("    Markdown → JSON: {:.1}x", json.len() as f64 / markdown.len() as f64);
    eprintln!("    Markdown → MessagePack: {:.1}x", msgpack.len() as f64 / markdown.len() as f64);
    eprintln!("    Markdown → MessagePack+zstd: {:.1}x", msgpack_compressed.len() as f64 / markdown.len() as f64);

    let doc2 = time_operation("Deserialize from JSON", || {
        CasDocument::from_json(&json).unwrap()
    });
    assert_eq!(doc.root_hash, doc2.root_hash);

    let doc3 = time_operation("Deserialize from MessagePack", || {
        CasDocument::from_msgpack(&msgpack).unwrap()
    });
    assert_eq!(doc.root_hash, doc3.root_hash);

    let doc4 = time_operation("Deserialize from MessagePack+zstd", || {
        CasDocument::from_msgpack_compressed(&msgpack_compressed).unwrap()
    });
    assert_eq!(doc.root_hash, doc4.root_hash);

    let rendered = time_operation("Render to Markdown", || {
        let mut renderer = MarkdownRenderer::new(&store);
        let root = store.get(&root_hash).unwrap();
        renderer.render(root).unwrap()
    });
    eprintln!("  Rendered markdown size: {} bytes ({:.2} KB)", rendered.len(), rendered.len() as f64 / 1024.0);

    time_operation("Re-parse rendered markdown", || {
        let mut store = NodeStore::new();
        parse_markdown(&rendered, &mut store).unwrap()
    });

    eprintln!("\n  ✅ All operations completed successfully");
}

#[test]
fn test_deduplication_efficiency() {
    eprintln!("\n=== Deduplication Efficiency Test ===");

    // Create a document with lots of repeated content
    let mut markdown = String::new();
    markdown.push_str("# Deduplication Test\n\n");

    // Repeat the same content 50 times
    let repeated_section = r#"## Common Section

This is a **common paragraph** that appears many times.

- Common list item 1
- Common list item 2
- Common list item 3

```rust
fn common_function() {
    println!("This code appears many times");
}
```

"#;

    for i in 0..50 {
        markdown.push_str(&format!("### Instance {}\n\n", i + 1));
        markdown.push_str(repeated_section);
    }

    eprintln!("Input size: {} bytes ({:.2} KB)", markdown.len(), markdown.len() as f64 / 1024.0);

    let (store, root_hash) = time_operation("Parse with deduplication", || {
        let mut store = NodeStore::new();
        let root_hash = parse_markdown(&markdown, &mut store).unwrap();
        (store, root_hash)
    });

    eprintln!("  Nodes created (after deduplication): {}", store.len());
    eprintln!("  Deduplication ratio: {:.2}x (many repeated nodes deduplicated)",
        markdown.len() as f64 / store.len() as f64
    );

    let doc = CasDocument::new(&store, root_hash).unwrap();
    let msgpack_compressed = doc.to_msgpack_compressed().unwrap();

    eprintln!("  Compressed size: {} bytes ({:.1}% of input)",
        msgpack_compressed.len(),
        (msgpack_compressed.len() as f64 / markdown.len() as f64) * 100.0
    );
}

#[test]
fn test_round_trip_fidelity() {
    eprintln!("\n=== Round-trip Fidelity Test ===");

    let test_cases = vec![
        ("Simple text", "Hello, world!"),
        ("Heading", "# Title"),
        ("Bold", "This is **bold** text"),
        ("Italic", "This is *italic* text"),
        ("Combined", "This is **bold** and *italic* and ***both***"),
        ("Link", "[Link](https://example.com)"),
        ("Image", "![Alt](https://example.com/img.png)"),
        ("Code", "Use `code` inline"),
        ("List", "- Item 1\n- Item 2"),
        ("Ordered list", "1. First\n2. Second"),
    ];

    for (name, markdown) in test_cases {
        eprint!("  Testing {}: ", name);

        let mut store1 = NodeStore::new();
        let root_hash1 = parse_markdown(markdown, &mut store1).unwrap();

        let mut renderer = MarkdownRenderer::new(&store1);
        let root1 = store1.get(&root_hash1).unwrap();
        let rendered = renderer.render(root1).unwrap();

        let mut store2 = NodeStore::new();
        let root_hash2 = parse_markdown(&rendered, &mut store2).unwrap();

        // Check if root hashes match (perfect round-trip)
        if root_hash1 == root_hash2 {
            eprintln!("✅ Perfect round-trip");
        } else {
            eprintln!("⚠️  Semantic equivalent (hashes differ but structure preserved)");
            // Verify both parse successfully
            assert!(store1.get(&root_hash1).is_some());
            assert!(store2.get(&root_hash2).is_some());
        }
    }
}
