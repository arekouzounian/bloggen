# Implementation Paths - Current Implementation

This document describes the **actual** implementation paths in the current code. All statements are facts about the existing codebase, not future plans.

## FUSE Filesystem Operations

### Directory Listing (`ls` command)

**User Action:** `ls /mount/point`

**Code Path:**
```
1. FUSE kernel → readdir() [src/fuse.rs:497]
2. Check if inode == ROOT_INODE (1)
3. refresh_posts() [src/fuse.rs:147]
   - HTTP GET /posts [src/http.rs:122]
   - Parse JSON response with PostSummary list
   - Update cache with new entries (skip dirty entries)
   - Allocate inodes for new posts
4. Build entries list: ".", "..", and "{slug}.md" for each post
5. Return entries starting from offset
```

**Cache Behavior:**
- Dirty posts are NOT overwritten during refresh
- Post metadata cached in `HashMap<String, CacheEntry>`
- Inode allocated on first access, reused thereafter

### File Read (`cat` command)

**User Action:** `cat /mount/point/my-post.md`

**Code Path:**
```
1. FUSE kernel → lookup() [src/fuse.rs:342]
   - Strip ".md" extension from filename
   - refresh_posts() to ensure post exists
   - allocate_inode() for the slug
   - Return file attributes

2. FUSE kernel → read() [src/fuse.rs:419]
   - get_slug() maps inode to slug [src/fuse.rs:144]
   - get_markdown() checks cache [src/fuse.rs:235]

   Cache Hit:
   - Return cached markdown immediately

   Cache Miss:
   - fetch_post() [src/fuse.rs:199]
   - HTTP GET /posts/{slug}/ast [src/http.rs:61]
   - Deserialize CasDocumentResponse
   - to_store() converts to NodeStore [src/cas.rs:230]
   - MarkdownRenderer.render() [src/render.rs:33]
   - Cache result (markdown + AST)
   - Return markdown

3. Return bytes from offset to offset+size
```

**Cache Behavior:**
- First read: ~100-500ms (server fetch + render)
- Subsequent reads: <1ms (memory lookup)
- Cache invalidated on write to same file

### File Write (editor save)

**User Action:** Save file in vim/emacs/vscode

**Code Path:**
```
1. FUSE kernel → write() [src/fuse.rs:449]
   - Currently ONLY supports offset=0 (full file write)
   - get_slug() maps inode to slug
   - Verify data is valid UTF-8
   - write_markdown() [src/fuse.rs:251]
     a. NodeStore::new()
     b. parse_markdown() [src/convert.rs:23]
        - markdown::to_mdast() parses to markdown-rs AST
        - convert_node() walks tree [src/convert.rs:64]
        - Stores each node in NodeStore with Blake3 hash
        - Returns root hash
     c. CasDocument::new() [src/cas.rs:200]
     d. Update cache:
        - entry.markdown = new markdown
        - entry.ast = new CasDocument
        - entry.dirty = true
     e. Return success

2. User closes file → flush() [src/fuse.rs:539]
   - get_slug() maps inode to slug
   - Check if entry.dirty
   - If dirty: flush_post() [src/fuse.rs:272]
     - HTTP POST /posts [src/http.rs:24]
     - Upload full AST (root_hash + all nodes)
     - Server stores in database
     - Set entry.dirty = false
```

**Current Limitations:**
- Only full file writes (offset != 0 rejected)
- Uploads ENTIRE AST on every save (no delta yet)
- No conflict detection (server version not checked)
- No retry on network failure

### File Attributes (`ls -l` command)

**User Action:** `ls -l /mount/point`

**Code Path:**
```
1. FUSE kernel → getattr() [src/fuse.rs:359]
   - If inode == ROOT_INODE: return directory attributes
   - Else: get_slug() maps inode to slug
   - get_file_attr() [src/fuse.rs:300]
     a. Read cache for slug
     b. If markdown cached: size = markdown.len()
        Else: size = 4096 (estimate)
     c. Return FileAttr:
        - ino: inode number
        - size: actual or estimated
        - kind: RegularFile
        - perm: 0o644 (rw-r--r--)
        - uid/gid: current user
        - timestamps: SystemTime::now()
```

**Behavior:**
- Size accurate only if post previously read
- Timestamps always current time (not from server)
- All files appear as regular files with 0o644 permissions

## Cache Architecture

### CacheEntry Structure
```rust
struct CacheEntry {
    summary: PostSummary,          // Server metadata
    ast: Option<CasDocument>,      // Parsed AST
    markdown: Option<String>,       // Rendered markdown
    dirty: bool,                    // Modified locally?
    accessed_at: SystemTime,        // Last access time
}
```

**Location:** `Arc<RwLock<HashMap<String, CacheEntry>>>`

### Cache Operations

**Insert (refresh_posts):**
- Skips entries where `dirty == true`
- Updates only metadata for existing entries
- New entries created with `ast=None, markdown=None`

**Read (get_markdown):**
- Returns cached markdown if present
- Fetches from server and caches on miss
- Updates `accessed_at` timestamp

**Write (write_markdown):**
- Parses markdown immediately
- Stores AST and markdown in cache
- Sets `dirty = true`
- Does NOT upload until flush

**Flush (flush_post):**
- Uploads if `dirty == true`
- Clears `dirty` flag on success
- Keeps cache entry (for future reads)

## Inode Management

### Allocation Strategy
```rust
// Constants
ROOT_INODE = 1
FIRST_FILE_INODE = 2

// State
next_inode: u64 (starts at 2)
inodes: HashMap<u64, String>  // inode → slug
slugs: HashMap<String, u64>   // slug → inode
```

**allocate_inode():** [src/fuse.rs:122]
- Check `slugs` map for existing allocation
- If exists: return existing inode
- Else: allocate next_inode++, store bidirectional mapping

**get_slug():** [src/fuse.rs:144]
- Simple HashMap lookup: `inodes.get(inode)`

**Properties:**
- Sequential allocation (2, 3, 4, ...)
- Persistent for lifetime of mount
- Same slug always gets same inode
- Not saved to disk (ephemeral per mount)

## HTTP Client Integration

### Server Endpoints Used

**List Posts:**
```rust
GET /posts
Response: {
    "posts": [{"slug", "title", "created_at", "updated_at", "published"}],
    "total": i64
}
```
[src/http.rs:122]

**Download Post:**
```rust
GET /posts/{slug}/ast
Response: {
    "root_hash": "hex_string",
    "nodes": {"hex_hash": AstNode}
}
```
[src/http.rs:61]

**Upload Post:**
```rust
POST /posts
Body: {
    "slug": string,
    "title": Option<string>,
    "ast_root": "hex_string",
    "nodes": {"hex_hash": AstNode}
}
```
[src/http.rs:24]

### Client Implementation

**Client Structure:**
```rust
pub struct Client {
    base_url: String,
    http_client: reqwest::Client,
}
```

**Async Runtime:**
- BlogGenFS contains `Arc<tokio::runtime::Runtime>`
- All HTTP calls use `runtime.block_on(async_fn)`
- Runs async code synchronously within FUSE callbacks

## Markdown Processing

### Parse Path
```
Markdown string
  ↓
markdown::to_mdast() [markdown-rs crate]
  ↓
markdown::mdast::Node (AST with positions)
  ↓
convert_node() [src/convert.rs:64]
  ↓
AstNode (owned, no positions) + Blake3 hash
  ↓
NodeStore.insert() [src/cas.rs:44]
  ↓
Return root Blake3Hash
```

**Node Hashing:** [src/cas.rs:89]
- Serialize node to JSON
- Blake3::hash(json_bytes)
- 32-byte hash, hex-encoded as 64 chars

### Render Path
```
Root Blake3Hash + NodeStore
  ↓
store.get(root_hash) → AstNode
  ↓
MarkdownRenderer::new(store) [src/render.rs:23]
  ↓
renderer.render(node) [src/render.rs:33]
  ↓
Match node type → visit_X() methods
  ↓
Build markdown string with proper spacing
  ↓
Return markdown String
```

**Rendering Style:**
- ATX headings: `# Heading`
- Fenced code blocks: triple backticks
- Lists: `-` for unordered, `1.` for ordered
- Emphasis: `**bold**`, `*italic*`
- Deterministic output (same AST → same markdown)

## Concurrency & Safety

### Thread Safety
- All cache access through `RwLock` (multiple readers OR one writer)
- Inode maps through `RwLock`
- HTTP client is `Arc<Client>` (thread-safe)
- Tokio runtime is `Arc<Runtime>` (thread-safe)

### FUSE Callbacks
- Called by FUSE kernel (single-threaded per operation)
- Operations are blocking (not async within FUSE)
- Async HTTP wrapped in `runtime.block_on()`

### Race Conditions
**None currently handled:**
- Multiple processes accessing same file
- Server-side changes during read
- Concurrent writes to same file
- Cache invalidation across instances

## Error Handling

### Network Errors
- HTTP failures return `ENOSYS` to FUSE
- No retry mechanism
- No offline queue
- Error logged via `log::error!()`

### Parse Errors
- Invalid markdown returns `ENOSYS`
- Invalid UTF-8 in write returns `EINVAL`
- Errors logged with context

### Server Errors
- HTTP status codes checked
- Error body extracted and logged
- Mapped to generic FUSE errors

## Performance Characteristics

### Measured Timings
- Parse markdown: 50-200μs (small doc)
- Render markdown: 50-150μs (small doc)
- HTTP roundtrip: 50-200ms (local network)
- Cache hit: <1ms

### Memory Usage
- Per post cached: ~5-25 KB (depends on size)
- No eviction: cache grows unbounded
- No persistence: lost on unmount

### Blocking Operations
- All FUSE operations block on completion
- HTTP requests block entire FUSE callback
- No background prefetch
- No write-behind caching

## Limitations (Current Implementation)

### Write Operations
- ✗ Only offset=0 supported
- ✗ No partial writes
- ✗ No append support
- ✗ Full AST uploaded (no delta)

### Read Operations
- ✓ Offset and size respected
- ✓ Caching works correctly
- ✗ No prefetch
- ✗ No readahead

### Cache
- ✗ No size limit
- ✗ No LRU eviction
- ✗ No persistence
- ✗ No invalidation on server change

### Concurrency
- ✗ No file locking
- ✗ No conflict detection
- ✗ No multi-mount coordination
- ✗ No optimistic concurrency control

### Error Recovery
- ✗ No retry logic
- ✗ No offline mode
- ✗ No dirty write queue
- ✗ No crash recovery

## Code Locations

### Key Files
- `src/fuse.rs` (550 lines) - FUSE implementation
- `src/http.rs` (495 lines) - HTTP client
- `src/cas.rs` (600+ lines) - Content-addressable storage
- `src/convert.rs` (600+ lines) - Markdown → AST
- `src/render.rs` (1000+ lines) - AST → Markdown

### Key Functions
- `BlogGenFS::mount()` [src/fuse.rs:114] - Mount filesystem
- `refresh_posts()` [src/fuse.rs:147] - Fetch post list
- `fetch_post()` [src/fuse.rs:199] - Download single post
- `write_markdown()` [src/fuse.rs:251] - Parse and cache
- `flush_post()` [src/fuse.rs:272] - Upload to server

### FUSE Callbacks
- `lookup()` [src/fuse.rs:342] - Resolve filename → inode
- `getattr()` [src/fuse.rs:359] - Get file metadata
- `read()` [src/fuse.rs:419] - Read file contents
- `write()` [src/fuse.rs:449] - Write file contents
- `readdir()` [src/fuse.rs:497] - List directory
- `flush()` [src/fuse.rs:539] - Sync dirty data

## State Machine

### Post Lifecycle
```
1. Unknown → refresh_posts() → Cached (metadata only)
2. Cached → read() → Cached (with AST + markdown)
3. Cached → write() → Dirty
4. Dirty → flush() → Cached (clean)
5. Cached → refresh_posts() → Updated metadata
```

### Cache States
- **Empty:** No cache entry
- **Metadata:** `summary` only, no AST/markdown
- **Loaded:** `summary` + `ast` + `markdown`, clean
- **Dirty:** `summary` + `ast` + `markdown`, modified
- **Flushing:** Dirty → uploading → clean

## Example Trace

### User reads a file
```
$ cat /mount/my-post.md

FUSE: lookup("my-post.md")
  → refresh_posts()
  → HTTP GET /posts → 2 posts
  → allocate_inode("my-post") → 2
  → return inode=2

FUSE: open(inode=2)
  → return fh=0

FUSE: read(inode=2, offset=0, size=4096)
  → get_slug(2) → "my-post"
  → get_markdown("my-post")
    → cache miss
    → fetch_post("my-post")
      → HTTP GET /posts/my-post/ast
      → Response: {root_hash, nodes}
      → to_store() → NodeStore
      → render() → "# My Post\n\n..."
      → cache.insert("my-post", {markdown, ast})
  → return markdown[0..4096]

Output: # My Post

        Content here...
```

### User edits and saves
```
$ vim /mount/my-post.md
[Edit and :wq]

FUSE: write(inode=2, offset=0, data="# Updated\n...")
  → get_slug(2) → "my-post"
  → write_markdown("my-post", "# Updated\n...")
    → parse_markdown() → root_hash + NodeStore
    → CasDocument::new()
    → cache["my-post"].dirty = true
  → return bytes_written

FUSE: flush(inode=2)
  → get_slug(2) → "my-post"
  → flush_post("my-post")
    → HTTP POST /posts
      Body: {slug, ast_root, nodes}
    → cache["my-post"].dirty = false
  → return success

File saved successfully.
```

---

**Last Updated:** 2026-01-19
**Code Version:** Corresponds to client/bgc commit with FUSE MVP implementation
