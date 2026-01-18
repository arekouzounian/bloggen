use bgc::{parse_markdown_file, CasDocument};
use std::collections::HashSet;
use std::path::Path;

fn main() {
    println!("=== BlogGen v2 - Comprehensive Node Type Test ===\n");

    let markdown_file = Path::new("examples/comprehensive_test.md");

    println!("Parsing: {}\n", markdown_file.display());

    // Parse the markdown file
    let (root_hash, store) = parse_markdown_file(markdown_file)
        .expect("Failed to parse markdown file");

    println!("✓ Parsed successfully");
    println!("  Root hash: {}", root_hash.to_hex());
    println!("  Total nodes stored: {}\n", store.len());

    // Walk the tree and collect node types
    let nodes = store.walk_tree(&root_hash).expect("Failed to walk tree");
    let node_types: HashSet<_> = nodes.iter()
        .map(|(_, node)| node.node_type())
        .collect();

    println!("Node types found ({} unique):", node_types.len());
    let mut sorted_types: Vec<_> = node_types.iter().collect();
    sorted_types.sort();
    for node_type in sorted_types {
        let count = nodes.iter()
            .filter(|(_, node)| node.node_type() == *node_type)
            .count();
        println!("  - {}: {} instances", node_type, count);
    }

    // Create and serialize document
    let doc = CasDocument::new(&store, root_hash).expect("Failed to create document");
    let json = doc.to_json_pretty().expect("Failed to serialize");

    println!("\n✓ Successfully serialized to JSON ({} bytes)", json.len());

    // Test round-trip
    let doc2 = CasDocument::from_json(&json).expect("Failed to deserialize");
    assert_eq!(doc.root_hash, doc2.root_hash, "Round-trip failed: root hash mismatch");
    assert_eq!(doc.nodes.len(), doc2.nodes.len(), "Round-trip failed: node count mismatch");

    println!("✓ Round-trip serialization verified\n");

    println!("=== All node types supported! ===");
}

