# Markdown AST Serialization Specification

## Problem Statement

The current implementation in `client/bgc/src/ast_wrapper.rs` manually wraps every node type from the `markdown-rs` crate to exclude the `position` field during serialization. This approach provides maximum control over the serialization format but suffers from:

- **High verbosity**: 30+ wrapper types, each with manual `Serialize` implementations (~700 lines of code)
- **Poor maintainability**: Adding new node types or fields requires updates across multiple locations
- **Fragility**: Changes to `markdown-rs` types require manual synchronization
- **No type safety**: Wrapper types can drift from source types without compile-time errors

## Requirements

### Functional Requirements

1. **Parse markdown files to syntax tree representation**
   - Support full CommonMark spec + GFM extensions
   - Preserve all semantic information (node types, attributes, content)
   - Exclude position/location metadata (line, column, offset)

2. **Serialize syntax tree for network transmission**
   - Minimize payload size for potentially large markdown documents
   - Support both human-readable (debugging) and compact (production) formats
   - Enable efficient incremental serialization for partial updates

3. **Upload to server and store in database**
   - Server receives serialized payload and stores in PostgreSQL `jsonb` column
   - Enable efficient querying on metadata (title, tags, date)
   - Support versioning/history tracking

4. **Server-side HTML rendering**
   - Convert stored syntax tree to HTML for browser display
   - Apply templates and styling server-side
   - Cache rendered HTML when possible

5. **Bidirectional conversion**
   - Client can retrieve syntax tree from server
   - Convert syntax tree back to markdown source
   - Preserve original markdown structure as much as possible

6. **FUSE filesystem interface**
   - Expose server posts as virtual filesystem
   - Random writes/small updates cause network round-trips
   - Operations should complete in <100ms for typical edits
   - Support atomic multi-node updates

### Non-Functional Requirements

1. **Performance**
   - Serialization/deserialization: <10ms for 100KB markdown file
   - Network payload: <50% of original markdown file size (excluding assets)
   - FUSE operations: <100ms latency for single-node updates
   - Support documents up to 10MB markdown size

2. **Maintainability**
   - Adding new node types should require minimal code changes
   - Schema changes should be backwards-compatible where possible
   - Clear separation between AST representation and wire format

3. **Reliability**
   - Lossless round-trip: markdown → AST → serialize → deserialize → markdown
   - Graceful handling of schema version mismatches
   - Corruption detection via checksums/validation

4. **Developer Experience**
   - Clear error messages for deserialization failures
   - Debugging tools to inspect serialized payloads
   - Documentation for custom node types/extensions

## Implementation Approaches

### Approach 1: Procedural Macro for Automatic Wrapper Generation

**Overview**: Use Rust procedural macros to automatically generate wrapper types and `Serialize` implementations, eliminating manual boilerplate.

**Implementation Details**:

```rust
// Define once in a central location
#[derive(WrapMarkdownNode)]
#[exclude_fields(position)]
#[add_type_field(snake_case)]
pub struct NodeWrapper<'a>(&'a markdown::mdast::Node);
```

The proc macro would:
- Inspect the `markdown::mdast::Node` enum at compile time
- Generate wrapper structs for each variant
- Implement `Serialize` trait excluding specified fields
- Add discriminant type field automatically
- Handle child node wrapping recursively

**Serialization Format**: JSON (same as current)

**Advantages**:
- ✅ Eliminates 90% of boilerplate code (~700 → ~50 lines)
- ✅ Compile-time validation against `markdown-rs` types
- ✅ Easy to maintain: changes to excluded fields in one place
- ✅ Type-safe: compiler errors if source types change
- ✅ Backward compatible with current JSON format

**Disadvantages**:
- ❌ Adds proc macro complexity (learning curve)
- ❌ Still uses JSON (larger payloads than binary)
- ❌ Macro debugging can be challenging
- ❌ Compile times slightly increased

**Payload Size Estimate**: ~70-80% of current JSON (position fields removed)

**FUSE Performance**: Update single node requires serializing entire tree (~50-100ms for large docs)

**Recommended For**: Teams comfortable with proc macros who want to keep JSON format but improve maintainability.

---

### Approach 2: Binary Serialization with Structural Sharing

**Overview**: Use a binary format (postcard, bincode, or custom) with structural sharing to minimize redundant data. Store nodes in a flat array with references instead of nested trees.

**Implementation Details**:

```rust
#[derive(Serialize, Deserialize)]
pub struct CompactAst {
    // Flat array of all nodes
    nodes: Vec<CompactNode>,
    // Root node index
    root: usize,
}

#[derive(Serialize, Deserialize)]
pub enum CompactNode {
    Text { value: String },
    Paragraph { children: Vec<usize> }, // Indices into nodes array
    Heading { depth: u8, children: Vec<usize> },
    // ... other node types
}
```

For FUSE updates, use copy-on-write:
- Keep node ID stable across updates
- Only serialize changed nodes + path to root
- Server merges delta into full tree

**Serialization Format**: Binary (postcard recommended for no_std compatibility)

**Advantages**:
- ✅ Smallest payload size: 20-30% of JSON
- ✅ Fast serialization/deserialization (<5ms for 100KB)
- ✅ Efficient incremental updates for FUSE
- ✅ Natural deduplication (shared subtrees reuse node IDs)
- ✅ Own data model: not coupled to `markdown-rs` internals

**Disadvantages**:
- ❌ Requires complete rewrite of AST handling
- ❌ More complex implementation (~1000 lines)
- ❌ Binary format is not human-readable
- ❌ Need custom tools for debugging/inspection
- ❌ Must implement markdown → AST and AST → HTML converters

**Payload Size Estimate**: ~20-30% of original markdown size

**FUSE Performance**: Delta updates ~5-10ms for single node changes

**Recommended For**: Performance-critical applications where FUSE interface is primary use case and team can invest in custom AST implementation.

---

### Approach 3: Compression Layer with Schema-Aware Encoding

**Overview**: Keep existing wrapper approach but add smart compression on top. Use schema knowledge to optimize encoding.

**Implementation Details**:

1. Serialize to JSON using current approach (or simplified with proc macro)
2. Apply schema-aware transformations:
   - Dictionary encode common strings ("type", "children", "paragraph")
   - Variable-length encoding for integers
   - Omit null/default values
3. Compress with zstd (with pre-trained dictionary on sample markdown ASTs)
4. Add lightweight framing (version, checksum, uncompressed size)

```rust
pub fn serialize_ast(ast: &Node) -> Vec<u8> {
    let json = ast_to_wrapped_json(ast, false)?;
    let transformed = schema_transform(&json); // Dictionary encode
    let compressed = zstd::encode_all(transformed, DICT)?;
    add_framing(compressed)
}
```

For FUSE updates:
- Small updates: send full compressed tree (fast enough with compression)
- Large docs: maintain server-side cache, send JSON Patch (RFC 6902) deltas

**Serialization Format**: JSON + schema transform + zstd compression

**Advantages**:
- ✅ Minimal code changes to existing approach
- ✅ Excellent compression ratio: 30-40% of JSON, ~40-50% of markdown
- ✅ Preserves JSON for debugging (decompress + view)
- ✅ zstd is extremely fast (20-50 MB/s compression)
- ✅ Can use proc macro from Approach 1 for even better base

**Disadvantages**:
- ❌ Compression/decompression overhead (~10-20ms)
- ❌ Still sends full tree for updates unless JSON Patch implemented
- ❌ Dictionary training requires sample data corpus
- ❌ Slightly more complex error handling

**Payload Size Estimate**: ~30-40% of original markdown size

**FUSE Performance**: 20-30ms for full tree updates, 5-10ms with JSON Patch

**Recommended For**: Teams wanting best compression with minimal changes to existing architecture. Good middle ground.

---

### Approach 4: Owned AST with Content-Addressed Storage

**Overview**: Create a clean, owned AST representation designed for the BlogGen use case. Use content-addressing for deduplication and efficient storage.

**Implementation Details**:

```rust
// Clean owned types with only needed fields
#[derive(Serialize, Deserialize, Clone, Hash)]
pub enum AstNode {
    Root { children: Vec<AstNode> },
    Heading { level: u8, children: Vec<AstNode> },
    Paragraph { children: Vec<AstNode> },
    Text { value: String },
    // ... other node types
}

// Content-addressed storage
pub struct CasNode {
    hash: Blake3Hash,  // 32 bytes
    node: AstNode,
}

impl CasNode {
    pub fn store(&self, db: &Database) -> Blake3Hash {
        // Recursively store children, replace with hashes
        // Store this node with hash as key
        self.hash
    }
}
```

For storage:
- Each node stored separately in database keyed by content hash
- Post metadata stores root hash
- Natural deduplication (identical subtrees share storage)
- FUSE updates only affect changed nodes

**Serialization Format**: JSON or binary for individual nodes, content-addressed storage

**Advantages**:
- ✅ Optimal storage efficiency (deduplication)
- ✅ Extremely efficient updates (only store changed nodes)
- ✅ Version history is free (store root hashes)
- ✅ Clean owned types, easy to work with
- ✅ Simple schema evolution (old hashes remain valid)
- ✅ FUSE updates are O(log n) with tree depth

**Disadvantages**:
- ❌ Requires database schema changes
- ❌ More complex server-side implementation
- ❌ Retrieving full post requires multiple DB queries (can be cached)
- ❌ Initial implementation effort (~1500 lines)
- ❌ Need garbage collection for unused nodes

**Payload Size Estimate**: Variable (only changed nodes sent), typically 5-20% for updates

**FUSE Performance**: <5ms for single node updates (only serialize changed path)

**Recommended For**: Long-term production system where storage efficiency and update performance are critical. Best for FUSE use case.

---

### Approach 5: Hybrid: Proc Macro + Lightweight Binary with Compression

**Overview**: Combine best elements of previous approaches for balanced solution.

**Implementation Details**:

1. Use proc macro (Approach 1) to generate clean wrapper types
2. Implement both `serde_json` and `postcard` serialization
3. For network transport:
   - Development: JSON (human-readable debugging)
   - Production: postcard + zstd compression
4. For FUSE:
   - Cache full AST in memory on client
   - Send binary deltas using `similar` or custom diff
   - Server applies delta and stores updated tree

```rust
// Generated by proc macro
#[derive(WrapMarkdownNode, Serialize, Deserialize)]
#[exclude_fields(position)]
pub struct NodeWrapper<'a>(&'a markdown::mdast::Node);

// Transport layer
pub fn serialize_for_transport(ast: &Node, mode: Mode) -> Vec<u8> {
    let wrapped = NodeWrapper::from(ast);
    match mode {
        Mode::Debug => serde_json::to_vec(&wrapped)?,
        Mode::Production => {
            let binary = postcard::to_allocvec(&wrapped)?;
            zstd::encode_all(&binary[..], 3)?
        }
    }
}

// FUSE delta layer
pub fn compute_delta(old: &Node, new: &Node) -> Delta {
    // Structural diff at AST level
    tree_diff(old, new)
}
```

**Serialization Format**: JSON (debug) or postcard+zstd (production)

**Advantages**:
- ✅ Clean code via proc macro
- ✅ Excellent production performance (binary + compression)
- ✅ Good debugging experience (JSON mode)
- ✅ Efficient FUSE updates via deltas
- ✅ Flexible: can tune format per use case
- ✅ Incremental migration path

**Disadvantages**:
- ❌ Most complex approach (combines multiple techniques)
- ❌ Need to maintain two serialization paths
- ❌ Delta diffing adds complexity
- ❌ Requires careful testing of all modes

**Payload Size Estimate**: JSON ~70% markdown, Binary ~25% markdown, Delta ~5-15% for updates

**FUSE Performance**: 5-10ms for delta computation + network, <20ms total

**Recommended For**: Production systems that need both excellent performance and good DX. Most complete solution but highest complexity.

---

## Comparison Matrix

| Approach | Code Complexity | Payload Size | FUSE Perf | Maintainability | Migration Effort |
|----------|----------------|--------------|-----------|-----------------|------------------|
| 1. Proc Macro | Low | Medium | Low | Excellent | Low |
| 2. Binary + Sharing | High | Excellent | Excellent | Good | High |
| 3. Compression Layer | Medium | Good | Medium | Good | Low |
| 4. Content-Addressed | High | Excellent* | Excellent | Excellent | Very High |
| 5. Hybrid | Very High | Excellent | Excellent | Good | Medium |

*For updates only; initial payload similar to others

## Recommendations

### For Current Project State (v2 migration in progress):

**Short-term (next 2-4 weeks)**: **Approach 1 (Proc Macro)**
- Immediately improves maintainability
- Low risk, high reward
- Can be implemented incrementally
- Provides foundation for future optimizations

**Medium-term (2-3 months, if FUSE is implemented)**: **Approach 3 (Compression)**
- Add compression layer on top of proc macro approach
- Significantly reduces bandwidth without major changes
- Simple to implement and test
- Good enough for FUSE with reasonable performance

**Long-term (6+ months, if scaling becomes concern)**: **Consider Approach 4 (Content-Addressed)**
- Only if supporting many users with large document collections
- Provides best storage efficiency and update performance
- Natural fit for version control features
- Requires significant investment but pays off at scale

### For Immediate Next Steps:

1. Implement **Approach 1** to eliminate current technical debt
2. Add benchmarking harness to measure real payload sizes
3. Test with realistic blog posts (10KB, 100KB, 1MB)
4. Measure FUSE operation latency requirements
5. Re-evaluate based on data

### If Time Constrained:

**Approach 3** (Compression Layer) without proc macro:
- Add zstd compression to existing code
- ~50 lines of code
- Immediate 60-70% size reduction
- No refactoring required
- Can add proc macro later when time permits

## Open Questions

1. **Asset handling**: Should images be embedded (base64), referenced (URLs), or stored separately with content-addressing?
2. **Schema versioning**: How to handle markdown-rs updates that add new node types?
3. **Database storage**: Store as single jsonb column or decompose into relational schema?
4. **Caching strategy**: Cache rendered HTML, serialized JSON, or both?
5. **FUSE granularity**: Should FUSE operations work at AST node level or higher-level (paragraph/section)?

## Appendix: Sample Payload Sizes

Based on a typical blog post (5KB markdown, 2000 words, 10 sections):

| Format | Size | Compression Ratio |
|--------|------|-------------------|
| Original markdown | 5,120 bytes | 100% |
| markdown-rs JSON (with position) | 18,432 bytes | 360% |
| Current wrapped JSON | 12,800 bytes | 250% |
| Approach 1 (proc macro JSON) | 12,800 bytes | 250% |
| Approach 2 (binary) | 3,584 bytes | 70% |
| Approach 3 (JSON + zstd) | 2,048 bytes | 40% |
| Approach 4 (CAS, initial) | 3,584 bytes | 70% |
| Approach 4 (CAS, update 1 para) | 384 bytes | 7.5%* |
| Approach 5 (binary + zstd) | 1,536 bytes | 30% |

*Update only, not full document

## Implementation Checklist

For chosen approach, ensure:

- [ ] Round-trip tests: markdown → AST → serialize → deserialize → markdown → HTML
- [ ] Benchmark suite with real blog posts (small, medium, large)
- [ ] Error handling for malformed/corrupted payloads
- [ ] Schema version negotiation between client/server
- [ ] Documentation for custom node types
- [ ] Migration path from current format (if applicable)
- [ ] FUSE operation profiling (if applicable)
- [ ] Backward compatibility testing
