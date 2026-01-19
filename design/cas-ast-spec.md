# Content-Addressable AST Specification for BlogGen v2

## Core Concept

Blog posts are stored as **content-addressed Abstract Syntax Trees** where each node is identified by a cryptographic hash of its content. This enables:

- **Deduplication**: Identical subtrees (common headers, footers, code snippets) stored once
- **Efficient updates**: Only changed nodes transmitted over network
- **Version history**: Free (just store root hash snapshots)
- **FUSE compatibility**: Random access and partial updates with minimal overhead

The AST serves as the **canonical representation**, with bidirectional conversion to markdown and extensible HTML rendering.

## Leveraging markdown-rs: Analysis

### What markdown-rs Provides

The `markdown` crate offers:
- **Parsing**: `markdown::to_mdast(source) → Node` - Full CommonMark + GFM support
- **HTML rendering**: `markdown::to_html(source) → String` - Basic HTML output
- **AST structure**: `markdown::mdast::Node` enum with all node types

### What We Must Implement Ourselves

**1. AST → Markdown Conversion** (REQUIRED)
- markdown-rs is a **parser**, not a renderer
- It provides markdown → AST → HTML, but not AST → markdown
- We need markdown reconstruction for:
  - FUSE driver (users edit markdown in virtual filesystem)
  - Exporting posts back to markdown files
  - Client-side editing workflows

**Conclusion**: Custom markdown renderer is unavoidable.

**2. Content-Addressed Storage** (REQUIRED)
- markdown-rs has no concept of content addressing
- Hashing must exclude position fields (line/column data) to ensure identical content → identical hash
- Need custom logic to walk tree, compute hashes, manage references

**Conclusion**: Custom storage layer is unavoidable.

**3. Extensible HTML Rendering** (REQUIRED)
- markdown-rs `to_html()` produces basic HTML
- No support for:
  - Injecting Tailwind classes
  - Custom attributes (IDs, data-* attrs)
  - Syntax highlighting integration
  - Table of contents generation
  - Custom element rendering

**Conclusion**: Custom HTML renderer needed for server-side manipulation.

### Recommended Approach: Hybrid

**Use markdown-rs for:**
- Parsing markdown source to AST ✓
- Reference AST structure (well-tested, maintained) ✓

**Implement ourselves:**
- Owned AST types (simplified, only fields we need)
- Content-addressed hashing and storage
- AST → Markdown renderer
- Extensible AST → HTML renderer

**Why not use markdown-rs AST directly?**

| Aspect | Using markdown::mdast::Node | Custom AstNode |
|--------|---------------------------|----------------|
| Parsing effort | None (use as-is) | Simple conversion (pattern matching) |
| Storage bloat | Position fields in every node (~30% overhead) | Only semantic data |
| Content hashing | Need custom Serialize to exclude position | Hash exactly what matters |
| Extensibility | Can't add custom node types | Easy to add Metadata, Asset, etc. |
| Maintainability | Coupled to markdown-rs internals | We control the schema |
| Markdown rendering | Still need custom code | Still need custom code (same work) |

**Decision**: Use a **conversion layer** from `markdown::mdast::Node` → custom `AstNode`. This is ~200 lines of straightforward pattern matching but gives us:
- Clean storage representation
- Stable hashing (not affected by markdown-rs internal changes)
- Ability to extend with custom nodes
- Full control over serialization

The conversion is simple because it's just copying data:
```rust
markdown::mdast::Node::Heading(h) => AstNode::Heading {
    level: h.depth,
    children: h.children.iter().map(convert).collect(),
}
```

## Architecture Overview

```
┌──────────────┐
│   Markdown   │
│    Source    │
└──────┬───────┘
       │
       ├─────────────────────────────────────────┐
       │                                         │
       │ markdown-rs                             │
       │ (parsing)                               │
       ▼                                         │
┌──────────────┐                                 │
│ markdown::   │                                 │
│ mdast::Node  │                                 │
└──────┬───────┘                                 │
       │                                         │
       │ Convert (our code, ~200 lines)          │
       ▼                                         │
┌──────────────┐                                 │
│   AstNode    │◄────────────────────────────────┤
│  (owned)     │                                 │
└──────┬───────┘                                 │
       │                                         │
       ├──────────┬──────────┬──────────────┐    │
       │          │          │              │    │
       │          │          │              │    │
       ▼          ▼          ▼              ▼    │
┌──────────┐ ┌─────────┐ ┌─────────┐  ┌────────┴─────┐
│ Content  │ │Markdown │ │  HTML   │  │   Storage    │
│  Hash    │ │Renderer │ │Renderer │  │  (Postgres)  │
│ (Blake3) │ │         │ │ +Extensions│ │   (jsonb)   │
└──────────┘ └─────────┘ └─────────┘  └──────────────┘
```

## Data Model

### Core AST Structure

```rust
// Simplified owned AST (no position fields, no metadata we don't need)
pub enum AstNode {
    // Containers
    Root { children: Vec<Blake3Hash> },
    Heading { level: u8, children: Vec<Blake3Hash> },
    Paragraph { children: Vec<Blake3Hash> },
    List { ordered: bool, start: Option<u64>, children: Vec<Blake3Hash> },
    ListItem { checked: Option<bool>, children: Vec<Blake3Hash> },
    Blockquote { children: Vec<Blake3Hash> },

    // Inline formatting
    Strong { children: Vec<Blake3Hash> },
    Emphasis { children: Vec<Blake3Hash> },
    Delete { children: Vec<Blake3Hash> },

    // Leaf nodes
    Text { value: String },
    CodeBlock { lang: Option<String>, value: String },
    InlineCode { value: String },
    Link { url: String, title: Option<String>, children: Vec<Blake3Hash> },
    Image { url: String, alt: String, title: Option<String> },

    // ... other node types (table, break, etc.)

    // BlogGen extensions (can't do this with markdown-rs types!)
    Metadata { fields: HashMap<String, Value> }, // Front matter
    Asset { id: String, content_type: String }, // Uploaded images/files
}

pub struct CasNode {
    pub hash: Blake3Hash,  // Content-addressable identifier
    pub node: AstNode,
}
```

**Key insight**: Children are stored as **hashes**, not inline. This is what makes content-addressing work.

### Content Hashing

Hash computed from:
- Node type discriminant
- Node attributes (level, url, value, etc.)
- **Child hashes** (not child content)

```
Example:
  Paragraph
     ├─ Text("Hello ")
     └─ Strong
           └─ Text("world")

Hash(Paragraph) = Blake3(
    [discriminant=Paragraph] +
    [hash(Text("Hello "))] +
    [hash(Strong(...))]
)

Hash(Strong(...)) = Blake3(
    [discriminant=Strong] +
    [hash(Text("world"))]
)
```

**Result**: Identical subtrees anywhere in any document → same hash → single storage.

### Why Blake3?

- **Fast**: 3-10x faster than SHA-256 (~1-2 GB/s)
- **Secure**: Cryptographically strong (collision resistance)
- **Fixed size**: 32 bytes (reasonable for UUIDs)
- **Incremental**: Can hash in chunks for large trees

## Database Schema

### Conceptual Model

```sql
-- Individual AST nodes (content-addressed)
CREATE TABLE ast_nodes (
    hash BYTEA PRIMARY KEY,           -- Blake3 hash (32 bytes)
    node_data JSONB NOT NULL,         -- Serialized AstNode
    ref_count INTEGER DEFAULT 0,      -- For garbage collection
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Blog posts (root references)
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(255) UNIQUE NOT NULL,

    ast_root BYTEA REFERENCES ast_nodes(hash),      -- Content tree
    ast_metadata BYTEA REFERENCES ast_nodes(hash),  -- Front matter

    html_cache TEXT,                  -- Rendered HTML (invalidated on update)

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Version history (free with content-addressing!)
CREATE TABLE post_versions (
    id SERIAL PRIMARY KEY,
    post_id INTEGER REFERENCES posts(id),
    version_number INTEGER NOT NULL,
    ast_root BYTEA REFERENCES ast_nodes(hash),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Storage efficiency example**:
- 100 blog posts, each with standard header/footer
- Traditional: Store header 100 times
- Content-addressed: Store header once (shared hash)
- Savings: ~20-40% depending on commonality

### Reference Counting & GC

When a post is created:
1. Walk entire AST tree
2. Increment `ref_count` for all nodes
3. Store post with root hash

When a post is updated:
1. Walk new tree, increment refs for new nodes
2. Walk old tree, decrement refs for old nodes
3. Update post with new root hash

Garbage collection:
```sql
DELETE FROM ast_nodes
WHERE ref_count = 0
AND created_at < NOW() - INTERVAL '30 days';
```

## Bidirectional Conversion

### Markdown → AST

**Phase 1**: Parse using markdown-rs
```rust
let md_ast = markdown::to_mdast(source, &ParseOptions::gfm())?;
```

**Phase 2**: Convert to our types (our code, ~200 lines)
```rust
fn convert_node(md_node: &markdown::mdast::Node) -> Blake3Hash {
    let ast_node = match md_node {
        markdown::mdast::Node::Heading(h) => AstNode::Heading {
            level: h.depth,
            children: h.children.iter().map(convert_node).collect(),
        },
        markdown::mdast::Node::Text(t) => AstNode::Text {
            value: t.value.clone(),
        },
        // ... pattern match all types
    };

    compute_hash_and_store(ast_node)
}
```

**Phase 3**: Extract front matter (if any)
```rust
// Parse YAML/TOML header
if source.starts_with("---\n") {
    let (metadata, content) = split_frontmatter(source);
    let metadata_node = AstNode::Metadata {
        fields: parse_yaml(metadata)
    };
    // Parse content separately
}
```

### AST → Markdown

**Challenge**: Reconstruct markdown syntax from semantic AST.

**Approach**: Visitor pattern that walks tree and emits markdown.

**High-level algorithm**:
- Walk tree depth-first
- For each node type, emit appropriate markdown syntax
- Track context (list depth, ordered/unordered, etc.)
- Insert appropriate spacing between blocks

**Design decisions**:
- Prefer ATX headings (`## Heading`) over Setext
- Use `**bold**` and `*italic*` consistently
- 2-space list indentation
- Blank line between block elements

**Result**: Deterministic markdown output (same AST → same markdown).

### AST → HTML (Extensible)

**Core renderer**: Basic HTML generation

**Extension system**: Plugins that inject behavior

**High-level design**:
```rust
trait HtmlExtension {
    fn before_render(&self, node: &AstNode, ctx: &mut RenderContext);
    fn after_render(&self, node: &AstNode, ctx: &mut RenderContext);
    fn render_override(&self, node: &AstNode) -> Option<String>;
}
```

**Extension examples**:
- **Tailwind**: Inject CSS classes based on node type
- **Syntax highlighting**: Override code block rendering with highlighted HTML
- **Table of contents**: Collect headings, generate ID attributes
- **Custom attributes**: Add data-* attributes for JavaScript hooks

**Why not use markdown-rs `to_html()`?**
- No way to inject Tailwind classes
- No custom attributes
- No syntax highlighting hooks
- Would need to post-process HTML (fragile, error-prone)

## Update Strategy: Delta-Based

### Problem

User edits paragraph 5 of 50-paragraph document via FUSE.

**Naive approach**: Re-parse entire document, upload entire AST (~100KB).

**Delta approach**: Upload only changed nodes.

### Algorithm

**Client side**:
1. Maintain in-memory cache of current AST (root hash + all nodes)
2. On file write, re-parse markdown → new AST
3. Walk both trees simultaneously:
   - Nodes with same hash: unchanged (skip)
   - Nodes with different hash: changed (include in delta)
4. Send delta: `{ new_nodes: HashMap<Hash, Node>, new_root: Hash }`

**Server side**:
1. Receive delta
2. Insert new nodes (ignore duplicates via `ON CONFLICT DO NOTHING`)
3. Update post root hash
4. Increment refs for new tree, decrement for old tree

### Performance

**Typical edit** (1 paragraph in 50):
- Changed nodes: ~5 (paragraph + path to root)
- Payload: ~1-2KB
- Traditional: ~100KB
- **Speedup: 50-100x**

**Worst case** (rewrite entire document):
- Changed nodes: all
- Payload: same as traditional
- **No regression**

## FUSE Integration

### High-Level Architecture

```
User edits file in ~/blog-posts/my-post.md
                 ↓
        FUSE driver intercepts write
                 ↓
    Buffered in memory (no immediate network call)
                 ↓
        User saves (flush event)
                 ↓
    Re-parse markdown → new AST
                 ↓
    Compute delta (changed nodes only)
                 ↓
    HTTP POST /posts/:slug/update { delta }
                 ↓
        Server applies delta
                 ↓
    Response: success + new root hash
                 ↓
    FUSE cache updated
```

### Key Operations

**Read** (lazy load):
1. Check local cache for AST
2. If miss, fetch from server: `GET /posts/:slug/ast`
3. Render AST → markdown
4. Cache locally
5. Return markdown to FUSE

**Write** (buffered):
1. Update in-memory markdown buffer
2. Mark as dirty
3. Return immediately (no network)

**Flush** (sync):
1. Parse dirty markdown → AST
2. Compute delta vs cached AST
3. Send delta to server
4. Update cache with new root hash
5. Clear dirty flag

**Latency targets**:
- Read (cached): <1ms
- Read (cache miss): 50-200ms (network + render)
- Write: <1ms (buffered)
- Flush: 50-500ms (parse + delta + network)

## Implementation Strategy

### Phase 1: Proof of Concept (1 week)

Goal: Validate core concepts

Tasks:
- Define `AstNode` enum (10 most common node types)
- Implement Blake3 hashing
- Write markdown-rs → AstNode converter (for those 10 types)
- Write AstNode → markdown renderer (for those 10 types)
- Test round-trip: markdown → AST → markdown → compare
- Measure hash stability (same input → same hash)

**Success criteria**:
- Round-trip produces equivalent markdown
- Hash stability: 100% deterministic
- Performance: <10ms for typical blog post

### Phase 2: Storage Layer (1 week)

Goal: Implement content-addressed storage

Tasks:
- PostgreSQL schema
- `NodeStore` API (get, store_many, walk_tree)
- Reference counting logic
- Test: store post, update post, verify deduplication
- Test: garbage collection removes orphaned nodes

**Success criteria**:
- Can store and retrieve posts via root hash
- Updates only store changed nodes
- Measure storage savings (target: 20-40% on 10 posts)

### Phase 3: HTML Rendering (1 week)

Goal: Server can render HTML with extensions

Tasks:
- Basic HTML renderer (no extensions)
- Extension system (trait + hooks)
- Tailwind extension (inject classes)
- Test: verify HTML output is valid
- Test: extensions applied correctly

**Success criteria**:
- HTML matches markdown semantics
- Tailwind classes injected correctly
- Can swap/combine extensions easily

### Phase 4: Complete AST Support (1 week)

Goal: Handle all CommonMark + GFM nodes

Tasks:
- Extend `AstNode` to all node types (tables, task lists, etc.)
- Update converter (markdown-rs → AstNode)
- Update markdown renderer
- Update HTML renderer
- Run CommonMark test suite

**Success criteria**:
- Pass 95%+ of CommonMark tests
- Support GFM extensions (tables, strikethrough, task lists)

### Phase 5: Server API (1 week)

Goal: REST/gRPC API for post operations

Tasks:
- POST /posts (create)
- GET /posts/:slug/ast (retrieve)
- POST /posts/:slug/update (delta update)
- GET /posts/:slug/html (rendered HTML)
- Version history endpoints

**Success criteria**:
- Can create/update/retrieve posts via API
- Delta updates work correctly
- HTML caching reduces server load

### Phase 6: FUSE Driver (2 weeks)

Goal: Edit posts as files in filesystem

Tasks:
- Basic FUSE implementation (read/write)
- Local AST cache
- Delta computation
- Conflict handling
- Integration tests

**Success criteria**:
- Can mount ~/blog-posts as FUSE
- Editing files syncs to server
- Latency: read <50ms, flush <500ms
- No data loss on network failures

### Total: ~7-8 weeks

## Open Questions

### 1. Serialization Format

**Options**:
- **JSON** (jsonb in Postgres): Human-readable, queryable, ~2x larger
- **Binary** (postcard/bincode): Compact, fast, opaque
- **Hybrid**: JSON in DB (for queries), binary on wire

**Recommendation**: Start with JSON for debugging, add binary optimization later.

### 2. Conflict Resolution

**Scenario**: FUSE client edits while server version updated elsewhere.

**Options**:
- **Last-write-wins**: Simple, can lose data
- **Version checking**: Reject if root hash changed, force user to pull
- **Operational transforms**: Complex, merges changes automatically

**Recommendation**: Version checking initially (fail-fast), OT if needed later.

### 3. Asset Handling

**Options**:
- **Embed in AST**: Base64 encode images, bloats payload
- **Separate storage**: Content-addressed blob store, AST references by hash
- **External URLs**: Link to CDN, no local storage

**Recommendation**: Separate content-addressed blob store (consistent with AST approach).

### 4. Schema Evolution

**Challenge**: Adding new node types or fields to `AstNode`.

**Options**:
- **Version field**: Each node tagged with schema version
- **Graceful degradation**: Unknown nodes → fallback rendering
- **Migration**: Background job to upgrade old nodes

**Recommendation**: Version field in `ast_nodes` table, graceful fallback for unknown types.

### 5. Caching Strategy

**Server-side**:
- Cache rendered HTML in `posts.html_cache`
- Invalidate on update
- TTL: indefinite (only invalidate on write)

**Client-side (FUSE)**:
- LRU cache for AST nodes (e.g., 1000 nodes ≈ 200KB)
- Invalidate on flush
- Fetch missing nodes on demand

**Recommendation**: Implement both, tune cache sizes based on usage patterns.

## Why This Approach Wins

### vs. Traditional Full-Document Storage

| Metric | Traditional | Content-Addressed |
|--------|-------------|-------------------|
| Update payload | 100KB (full doc) | 1-5KB (delta) |
| Storage (100 posts) | 500KB | 300-400KB (dedupe) |
| Version history | 500KB per version | ~50KB per version (shared nodes) |
| FUSE performance | Re-upload entire doc | Upload changed nodes only |

### vs. Direct markdown-rs Usage

| Aspect | Using markdown-rs AST | Custom AstNode |
|--------|----------------------|----------------|
| Storage overhead | +30% (position fields) | Minimal |
| Extensibility | Can't add custom nodes | Easy (Metadata, Asset) |
| Content hashing | Fragile (depends on internals) | Stable |
| Markdown rendering | Still need custom code | Still need custom code |

### vs. HTML as Source

| Aspect | HTML as source | AST as source |
|--------|----------------|---------------|
| Editing | Parse HTML (lossy) | Edit markdown (clean) |
| Styling | Baked into HTML | Applied at render time |
| Search | Parse HTML to extract text | Walk AST for text nodes |
| Future formats | Stuck with HTML | Can render to anything |

## Conclusion

**Core decision**: Implement a **lightweight owned AST** with content-addressed storage.

**Leverage markdown-rs for**: Parsing markdown → AST (well-tested, maintained).

**Implement ourselves**:
- Owned `AstNode` types (~100 lines)
- Conversion from markdown-rs (~200 lines)
- Markdown renderer (~500 lines)
- HTML renderer with extensions (~500 lines)
- Storage layer (~300 lines)

**Total custom code**: ~1600 lines (manageable).

**Benefits**:
- Clean, minimal storage representation
- Efficient updates (delta-based)
- Flexible HTML rendering (Tailwind, syntax highlighting, etc.)
- Version history essentially free
- Full control over schema evolution

**Tradeoffs**:
- ~1600 lines of custom code vs. 0 if we used markdown-rs directly
- But markdown-rs doesn't provide AST→markdown or extensible HTML, so we'd write similar code anyway
- Our approach gives us storage efficiency, clean content-addressing, and extensibility

**Next step**: Start Phase 1 POC to validate assumptions (1 week).
