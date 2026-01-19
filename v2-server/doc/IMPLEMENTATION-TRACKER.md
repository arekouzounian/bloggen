# BlogGen v2 Server - Implementation Tracker

This document tracks the implementation status of the Rust server for BlogGen v2's HTTP API + content-addressable storage system.

See `design/v2-server-spec.md` for the complete specification.

---

## Phase 1: Core Infrastructure & Database Layer ✅ COMPLETE

### ✅ Project Setup
- [x] Create new Rust project in `v2-server/`
- [x] Setup Cargo.toml with dependencies:
  - `axum` (web framework)
  - `tokio` (async runtime)
  - `sqlx` with PostgreSQL and compile-time query checking
  - `serde`, `serde_json` (JSON serialization)
  - `rmp-serde` (MessagePack serialization)
  - `zstd` (compression)
  - `blake3` (hashing, match client version)
  - `tower-http` (middleware for logging, CORS)
- [x] Setup logging with `tracing` and `tracing-subscriber`
- [x] Create basic project structure:
  - `main.rs` - Entry point, server startup with graceful shutdown
  - `lib.rs` - Public API exports
  - `config.rs` - Configuration loading with defaults and tests
  - `error.rs` - Error types and handling with Axum integration
- [x] Create configuration file format (JSON with example: `config.example.json`)

### ✅ Database Setup & Schema
- [x] Write SQL schema in migration files:
  - `ast_nodes` table with hash, node_data, ref_count
  - `posts` table with slug, ast_root, metadata, caching fields
  - `post_versions` table for version history
  - All indexes and constraints
- [x] Create database migration system (using `sqlx migrate`)
- [x] Write migration scripts:
  - `001_create_ast_nodes.sql`
  - `002_create_posts.sql`
  - `003_create_post_versions.sql`
- [x] Add database URL configuration
- [x] Automatic migration runner in `main.rs`

### ✅ Shared AST Types
- [x] Create shared `bgc-ast` crate in `client/bgc-ast/`
- [x] Move `AstNode` enum to shared crate
- [x] Move `Blake3Hash` type and serialization to shared crate
- [x] Update client `bgc` to use `bgc-ast` dependency
- [x] Update server `v2-server` to use `bgc-ast` dependency
- [x] Ensure `Serialize`/`Deserialize` implementations match exactly
- [x] Test: Unit tests for AST types (3 tests passing)

---

## Phase 2: Content-Addressable Storage Layer ✅ COMPLETE

### ✅ Node Storage Module (`src/storage/mod.rs`)
- [x] Define `NodeStore` trait:
  - `async fn insert_node(&self, hash, node) -> Result<()>`
  - `async fn insert_many(&self, nodes) -> Result<()>`
  - `async fn get_node(&self, hash) -> Result<Option<AstNode>>`
  - `async fn get_many(&self, hashes) -> Result<HashMap<Hash, AstNode>>`
- [x] Implement `PostgresNodeStore`:
  - Connection pool management
  - Insert with `ON CONFLICT DO NOTHING` for deduplication
  - Batch inserts for efficiency using UNNEST
- [x] Test: Insert node, fetch node, verify deduplication (compile-time tests)

### ✅ Reference Counting (`src/storage/refcount.rs`)
- [x] Implement `async fn increment_refs(&self, hashes) -> Result<()>`
  - Batch update all hashes in single query
  - Uses ANY($1) for efficient batch operations
- [x] Implement `async fn decrement_refs(&self, hashes) -> Result<()>`
  - Batch update all hashes in single query
  - Uses GREATEST to prevent negative counts
- [x] Add helper functions: `increment_tree_refs` and `decrement_tree_refs`
  - Combines tree walking with ref counting
  - Designed for use within transactions
- [x] Test: Compile-time tests verify API correctness

### ✅ Tree Walking (`src/storage/walker.rs`)
- [x] Implement `walk_tree(root_hash, store) -> HashMap<Hash, AstNode>`:
  - Breadth-first traversal (more efficient for batch loading)
  - Load nodes in batches (up to 100 at a time)
  - Recursively walk children (by hash)
  - Return all nodes in subtree
- [x] Optimize with visited set (avoid re-fetching shared nodes)
- [x] Handle malformed trees (missing nodes return error)
- [x] Test: Walk small tree, verify all nodes collected (5 comprehensive tests)
- [x] Test: Walk tree with shared subtrees (deduplication verified)
- [x] Test: Error handling for missing nodes

### ✅ Garbage Collection (`src/storage/gc.rs`)
- [x] Implement `async fn garbage_collect(&self, grace_period_days) -> Result<u64>`:
  - Query nodes with `ref_count = 0` older than grace period
  - Delete matching nodes
  - Return count of deleted nodes
- [x] Implement `async fn count_orphaned(&self, grace_period_days) -> Result<u64>`
  - Count nodes eligible for GC without deleting
- [x] Add background task scheduler: `start_gc_background_task`
  - Uses `tokio::spawn` for background execution
  - Configurable interval and grace period
- [x] Test: Background task starts and runs correctly

---

## Phase 3: AST Rendering ✅ COMPLETE

### ✅ AST → Markdown Renderer (`src/render/markdown.rs`)
- [x] Define `render_to_markdown(root_hash, store) -> Result<String>`
- [x] Implement visitor pattern for all `AstNode` variants:
  - Root → join children
  - Heading → `## ` prefix + children
  - Paragraph → children + blank line
  - List → ordered/unordered, proper indentation
  - Strong → `**`children`**`
  - Emphasis → `*`children`*`
  - Text → raw value
  - Code block → triple backticks + lang
  - Link → `[`children`](`url`)`
  - Image → `![`alt`](`url`)`
  - All 33 node types fully implemented
- [x] Handle context (list depth, block vs inline)
- [x] Add proper spacing between blocks
- [x] Test: 5 comprehensive unit tests covering all major node types
- [ ] Test: Round-trip 100 CommonMark examples (future work)
- [ ] Benchmark: <50ms for 5KB post (future work)

### ✅ AST → HTML Renderer (`src/render/html.rs`)
- [x] Define base `render_to_html(root_hash, store) -> Result<String>`
- [x] Implement visitor pattern for all `AstNode` variants:
  - Root → `<div class="markdown-content">`children`</div>`
  - Heading → `<h{level}>`children`</h{level}>`
  - Paragraph → `<p>`children`</p>`
  - Strong → `<strong>`children`</strong>`
  - Code block → `<pre><code class="language-{lang}">`value`</code></pre>`
  - All 33 node types with semantic HTML5
- [x] XSS protection via HTML entity escaping
- [x] Test: 6 comprehensive unit tests
- [x] Test: XSS vulnerability protection verified

### ✅ HTML Renderer Extension System (`src/render/html_extensions.rs`)
- [x] Define `HtmlExtension` trait:
  - `fn add_classes(&self, ctx) -> Vec<String>`
  - `fn add_attributes(&self, ctx) -> HashMap<String, String>`
  - `fn render_override(&self, ctx) -> Option<String>`
  - `fn inject_content(&self, ctx, timing) -> Option<String>`
- [x] Implement extension application helpers
- [x] Create `TailwindExtension`:
  - Inject CSS classes based on node type
  - Support for custom class mappings
  - Dark mode support (e.g., `dark:text-gray-200`)
  - Comprehensive coverage: headings, paragraphs, code, lists, tables, etc.
- [x] Test: 5 unit tests for extension system
- [ ] Create `SyntaxHighlightExtension` (future work, requires syntect dependency)

---

## Phase 4: HTTP API Server

### ✅ / ⬜ Server Setup (`src/server.rs`)
- [ ] Create Axum app with routes
- [ ] Setup middleware:
  - Request logging (tracing)
  - Error handling
  - Compression (gzip/br for responses)
- [ ] Configure to bind to `localhost:3000` only
- [ ] Add graceful shutdown handler
- [ ] Test: Start server, verify responds to health check

### ✅ / ⬜ POST /posts - Create Post (`src/handlers/create_post.rs`)
- [ ] Define request struct: `CreatePostRequest { slug, ast_root, nodes }`
- [ ] Validate slug (alphanumeric + hyphens, unique)
- [ ] Insert all nodes via `NodeStore::insert_many`
- [ ] Increment ref counts for all nodes
- [ ] Insert post record with ast_root
- [ ] Return `{ id, slug }`
- [ ] Test: Create post via curl, verify in database
- [ ] Test: Duplicate slug returns error
- [ ] Test: Invalid slug returns error

### ✅ / ⬜ GET /posts/:slug/ast - Fetch AST (`src/handlers/get_ast.rs`)
- [ ] Query post by slug, get ast_root
- [ ] Walk tree via `walk_tree(ast_root)`
- [ ] Serialize to MessagePack
- [ ] Compress with zstd
- [ ] Set `Content-Type: application/msgpack`
- [ ] Set `Content-Encoding: zstd`
- [ ] Return compressed payload
- [ ] Test: Fetch AST, decompress, deserialize, verify matches original
- [ ] Test: Non-existent slug returns 404

### ✅ / ⬜ POST /posts/:slug/delta - Delta Update (`src/handlers/delta_update.rs`)
- [ ] Define request struct: `DeltaRequest { old_root, new_root, new_nodes }`
- [ ] Query current post ast_root
- [ ] Validate `old_root == current_root` (conflict detection)
- [ ] Start database transaction
- [ ] Insert new_nodes
- [ ] Increment ref_counts for new tree
- [ ] Decrement ref_counts for old tree
- [ ] Update post.ast_root = new_root
- [ ] Increment post.version_number
- [ ] Insert version snapshot
- [ ] Invalidate HTML cache (set html_cache = NULL)
- [ ] Commit transaction
- [ ] Return `{ version, new_root }`
- [ ] Test: Delta update, verify new nodes stored
- [ ] Test: Ref counts updated correctly
- [ ] Test: Conflict (old_root mismatch) returns error
- [ ] Test: Version history created

### ✅ / ⬜ GET /posts/:slug/markdown - Render Markdown (`src/handlers/get_markdown.rs`)
- [ ] Query post by slug, get ast_root
- [ ] Render via `render_to_markdown(ast_root, store)`
- [ ] Set `Content-Type: text/plain; charset=utf-8`
- [ ] Return markdown text
- [ ] Test: Fetch markdown, verify correct rendering
- [ ] Test: Benchmark latency <200ms (cache miss)

### ✅ / ⬜ GET /posts/:slug/html - Render HTML (`src/handlers/get_html.rs`)
- [ ] Query post by slug
- [ ] Check if html_cache is populated and fresh
- [ ] If cached: Return html_cache
- [ ] If not cached:
  - Render via `render_to_html(ast_root, store, extensions)`
  - Store in html_cache
  - Update html_cache_updated_at
- [ ] Set `Content-Type: text/html; charset=utf-8`
- [ ] Return HTML
- [ ] Test: First request renders and caches
- [ ] Test: Second request returns cached version (faster)
- [ ] Test: After delta update, cache is invalidated
- [ ] Test: Measure cache hit rate >80%

### ✅ / ⬜ GET /posts - List Posts (`src/handlers/list_posts.rs`)
- [ ] Support query params: `?published=true&limit=20&offset=0`
- [ ] Query posts table with filters
- [ ] Return `[{ slug, title, created_at, updated_at, published }]`
- [ ] Order by updated_at DESC
- [ ] Test: List all posts
- [ ] Test: Filter by published
- [ ] Test: Pagination works

### ✅ / ⬜ DELETE /posts/:slug - Delete Post (`src/handlers/delete_post.rs`)
- [ ] Query post by slug
- [ ] Start transaction
- [ ] Decrement ref_counts for entire tree
- [ ] Delete post_versions entries
- [ ] Delete post entry
- [ ] Commit transaction
- [ ] Return `{ deleted: true }`
- [ ] Test: Delete post, verify removed from database
- [ ] Test: Ref counts decremented
- [ ] Test: Orphaned nodes have ref_count = 0

---

## Phase 5: Performance Optimization

### ✅ / ⬜ Connection Pooling
- [ ] Configure sqlx connection pool size (based on expected load)
- [ ] Add connection pool metrics (idle, active, max)
- [ ] Test: Concurrent requests use pool efficiently
- [ ] Monitor: No connection starvation under load

### ✅ / ⬜ Batch Operations
- [ ] Optimize `insert_many` to use single SQL statement
- [ ] Optimize ref count updates to batch in single transaction
- [ ] Use `UNNEST` or `VALUES` for bulk inserts
- [ ] Test: Measure speedup vs individual inserts

### ✅ / ⬜ Caching Layer
- [ ] Implement in-memory LRU cache for frequently accessed nodes
- [ ] Cache rendered markdown (separate from HTML cache)
- [ ] Add cache metrics (hit rate, size)
- [ ] Test: Measure cache hit rate
- [ ] Test: Memory usage stays bounded

### ✅ / ⬜ Compression
- [ ] Implement zstd compression for API responses
- [ ] Add `Accept-Encoding` / `Content-Encoding` header handling
- [ ] Compress HTML responses (gzip/br via middleware)
- [ ] Test: Verify compression reduces payload size
- [ ] Benchmark: Compression overhead vs transfer time

### ✅ / ⬜ Benchmarking & Profiling
- [ ] Create benchmark suite using `criterion`
- [ ] Benchmark: Parse and store 100-node post
- [ ] Benchmark: Delta update with 5 changed nodes
- [ ] Benchmark: Render markdown (cold cache)
- [ ] Benchmark: Render HTML (cold cache)
- [ ] Profile hot paths with `flamegraph`
- [ ] Optimize bottlenecks
- [ ] Verify: Flush latency <300ms (90th percentile)

---

## Phase 6: FUSE Driver (Client-Side)

### ✅ / ⬜ FUSE Driver Project Setup
- [ ] Create `client/bgc-fuse` project (or add feature to bgc)
- [ ] Add dependencies:
  - `fuse3` or `fuser` (FUSE bindings)
  - `reqwest` (HTTP client)
  - `tokio` (async runtime)
  - `bgc` library (for parsing/rendering)
- [ ] Create basic project structure

### ✅ / ⬜ FUSE Filesystem Implementation (`src/fuse_fs.rs`)
- [ ] Implement `Filesystem` trait (from FUSE library)
- [ ] Implement core operations:
  - `init()` - Initialize, fetch post list from server
  - `readdir()` - List all `.md` files
  - `getattr()` - Return file metadata
  - `open()` - Open file for reading/writing
  - `read()` - Fetch markdown from cache or server
  - `write()` - Update in-memory buffer
  - `flush()` - Sync changes to server
  - `release()` - Close file
- [ ] Test: Mount filesystem, verify posts appear

### ✅ / ⬜ Local Caching (`src/cache.rs`)
- [ ] Implement LRU cache for AST nodes
- [ ] Cache markdown renderings
- [ ] Track dirty files
- [ ] Implement cache invalidation logic
- [ ] Test: Read file twice, second read from cache

### ✅ / ⬜ Delta Computation (`src/delta.rs`)
- [ ] Implement `compute_delta(old_doc, new_doc) -> Delta`:
  - Walk both trees simultaneously
  - Compare node hashes
  - Collect changed nodes (different hash)
  - Return `{ old_root, new_root, new_nodes }`
- [ ] Test: Edit one paragraph in 50-paragraph post, verify delta ~5 nodes
- [ ] Benchmark: Delta computation <50ms

### ✅ / ⬜ HTTP Client (`src/client.rs`)
- [ ] Implement API client:
  - `async fn list_posts() -> Result<Vec<PostMeta>>`
  - `async fn get_markdown(slug) -> Result<String>`
  - `async fn send_delta(slug, delta) -> Result<()>`
  - `async fn create_post(slug, doc) -> Result<()>`
- [ ] Add retry logic (transient network failures)
- [ ] Add request timeout configuration
- [ ] Test: All API operations work correctly

### ✅ / ⬜ Error Handling & Recovery
- [ ] Handle network failures gracefully
- [ ] Handle conflict errors (refetch and retry)
- [ ] Implement offline mode (buffer writes, sync when online)
- [ ] Test: Network failure during flush, verify retry
- [ ] Test: Conflict error, verify user prompted to refetch

### ✅ / ⬜ Integration Testing
- [ ] End-to-end test: Mount → Read → Edit → Save → Verify
- [ ] Test: Create new post via FUSE
- [ ] Test: Delete post via FUSE
- [ ] Test: Multiple concurrent edits
- [ ] Measure: Read latency <200ms, flush latency <500ms

---

## Phase 7: Frontend Integration

### ✅ / ⬜ Update Next.js Frontend
- [ ] Replace file-based post loading with API calls
- [ ] Implement SSR (Server-Side Rendering):
  - Fetch post list via `GET /posts`
  - Fetch HTML via `GET /posts/:slug/html`
  - Render HTML in Next.js page
- [ ] Implement ISR (Incremental Static Regeneration):
  - Cache rendered pages
  - Revalidate on webhook or time-based
- [ ] Add webhook endpoint for cache invalidation
- [ ] Test: View post, verify correct rendering
- [ ] Test: Update post via FUSE, reload page, verify changes

### ✅ / ⬜ API Configuration
- [ ] Configure Next.js to connect to server API
- [ ] Handle SSH tunnel or internal network routing
- [ ] Add API authentication if needed (for frontend)
- [ ] Test: Production deployment, verify frontend fetches posts

---

## Phase 8: Deployment & Operations

### ✅ / ⬜ Server Deployment
- [ ] Create systemd service file
- [ ] Write deployment script
- [ ] Setup PostgreSQL database on server
- [ ] Run database migrations
- [ ] Configure server to start on boot
- [ ] Test: Deploy to server, verify running

### ✅ / ⬜ Client Deployment
- [ ] Write SSH tunnel setup script (systemd/launchd)
- [ ] Create FUSE mount script
- [ ] Write user documentation
- [ ] Test: Fresh install on new machine

### ✅ / ⬜ Monitoring & Logging
- [ ] Setup structured logging (JSON logs)
- [ ] Add metrics endpoint (`/metrics` for Prometheus)
- [ ] Monitor key metrics:
  - Request latency (p50, p90, p99)
  - Database query time
  - Cache hit rate
  - Error rate
- [ ] Setup alerting for errors

### ✅ / ⬜ Backup & Recovery
- [ ] Implement database backup script
- [ ] Test: Restore from backup
- [ ] Document recovery procedures

---

## Additional Features (Future)

### ⬜ Asset Storage
- [ ] Create `assets` table (content-addressed blob storage)
- [ ] Implement `POST /assets` endpoint
- [ ] Support image uploads
- [ ] Add asset references in markdown (`asset://hash`)

### ⬜ Full-Text Search
- [ ] Add full-text search index to PostgreSQL
- [ ] Implement `GET /posts/search?q=query`
- [ ] Index markdown content for search

### ⬜ Multi-User Support
- [ ] Add user authentication
- [ ] Add authorization (who can edit what)
- [ ] Implement conflict resolution beyond fail-fast

### ⬜ Version History UI
- [ ] Add endpoints to browse version history
- [ ] Implement diff view
- [ ] Add rollback functionality

---

## Success Criteria

**Phase 1-2: Core Infrastructure**
- ✓ Can create post with all nodes stored
- ✓ Deduplication works (same node stored once)
- ✓ Ref counting correct (increments/decrements)
- ✓ GC deletes orphaned nodes

**Phase 3: Rendering**
- ✓ Round-trip markdown → AST → markdown works
- ✓ HTML rendering produces valid semantic HTML
- ✓ Extensions inject classes/attributes correctly

**Phase 4: HTTP API**
- ✓ All endpoints functional
- ✓ Delta updates work correctly
- ✓ HTML caching reduces server load
- ✓ Conflict detection works

**Phase 5: Performance**
- ✓ Flush latency <300ms (90th percentile)
- ✓ Read latency <200ms (cache miss)
- ✓ HTML cache hit rate >80%

**Phase 6: FUSE Driver**
- ✓ Can mount ~/blog-posts
- ✓ Files appear and can be edited
- ✓ Saving syncs to server
- ✓ Delta computation reduces payload size

**Phase 7: Frontend**
- ✓ Blog displays posts correctly
- ✓ Updates via FUSE reflected on website
- ✓ Page load time <500ms

**Phase 8: Deployment**
- ✓ Server runs on production
- ✓ Client configured and working
- ✓ Monitoring in place

---

## Timeline Estimate

- **Phase 1: Core Infrastructure** - 3-5 days
- **Phase 2: Storage Layer** - 5-7 days
- **Phase 3: Rendering** - 5-7 days
- **Phase 4: HTTP API** - 7-10 days
- **Phase 5: Optimization** - 3-5 days
- **Phase 6: FUSE Driver** - 10-14 days
- **Phase 7: Frontend Integration** - 5-7 days
- **Phase 8: Deployment** - 3-5 days

**Total: 41-60 days (~6-9 weeks)**

---

## Current Status

**Phase 1: COMPLETE** ✅ (2026-01-18)
- Project infrastructure set up with all dependencies
- Database schema and migrations created
- Shared `bgc-ast` crate created and integrated
- Configuration system with JSON support and defaults
- Error handling with HTTP status code mapping
- Comprehensive unit tests (11 tests in v2-server, 3 tests in bgc-ast)
- Server builds and tests pass successfully

**Phase 2: COMPLETE** ✅ (2026-01-18)
- NodeStore trait and PostgresNodeStore implementation
- Batch insert/fetch operations with deduplication
- Tree walking with shared subtree optimization
- Reference counting with helper functions for tree operations
- Garbage collection with background task support
- Comprehensive unit tests (20 tests total, all passing)
- All modules compile without errors

**Phase 3: COMPLETE** ✅ (2026-01-18)
- AST → Markdown renderer with full support for 33 node types
- AST → HTML renderer with semantic HTML5 output
- XSS protection via HTML entity escaping
- HTML extension system with trait-based architecture
- TailwindExtension with comprehensive CSS class mappings
- 16 new unit tests (5 markdown + 6 HTML + 5 extensions)
- Total: 36 tests passing, all modules compile cleanly

**Ready to begin Phase 4: HTTP API Server**
