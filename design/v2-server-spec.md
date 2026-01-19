# BlogGen v2 Server Implementation Specification

## Executive Summary

This specification defines the BlogGen v2 server architecture using an SSH tunnel + HTTP API approach, optimized for:
1. **SSH-based authentication** (no custom auth flows)
2. **Fast read/write operations** (optimized for FUSE workloads)
3. **Single-user simplicity** (minimal operational overhead)
4. **Content-addressable storage** (deduplication + delta updates)

The design leverages the existing content-addressable AST client implementation (Blake3 hashing, MessagePack+zstd serialization, delta updates).

---

## Design Goals & Constraints

### Primary Goals
1. **Zero custom authentication**: Use SSH keys exclusively
2. **FUSE-optimized performance**: <50ms reads (cached), <500ms writes (flush)
3. **Simple mental model**: "Just edit markdown files" experience
4. **Efficient updates**: Delta-based, only transmit changed nodes

### Non-Goals
1. Multi-user coordination (future consideration)
2. Planet-scale performance
3. Complex conflict resolution (fail-fast initially)

### Performance Targets

| Operation | Target Latency | Notes |
|-----------|----------------|-------|
| Read (cached) | <1ms | Client-side cache hit |
| Read (miss) | 50-200ms | Network + DB + AST→Markdown |
| Write (buffer) | <1ms | In-memory, no network |
| Flush (sync) | 50-500ms | Parse + Delta + Network + DB |
| List posts | <100ms | Metadata query only |

---

## Architecture: SSH Tunnel + HTTP API

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                        User's Machine                       │
│                                                             │
│  ┌──────────────┐         ┌─────────────────────────────┐  │
│  │              │         │                             │  │
│  │  Text Editor │◄────────┤  FUSE Driver (bgc-fuse)     │  │
│  │  (vim/emacs) │         │  - Mounts ~/blog-posts/     │  │
│  │              │         │  - Caches AST nodes         │  │
│  └──────────────┘         │  - Computes deltas          │  │
│                           │  - HTTP client              │  │
│                           └────────────┬────────────────┘  │
│                                        │                    │
│                                        │ HTTP/1.1           │
│                                        │ (via SSH tunnel)   │
└────────────────────────────────────────┼────────────────────┘
                                         │
                              ┌──────────▼──────────┐
                              │   SSH Tunnel        │
                              │   localhost:8080 -> │
                              │   server:3000       │
                              └──────────┬──────────┘
                                         │
┌────────────────────────────────────────┼────────────────────┐
│                        Server                                │
│                                        │                     │
│  ┌─────────────────────────────────────▼──────────────────┐ │
│  │          HTTP API Server (Axum)                        │ │
│  │          Listens on localhost:3000 ONLY                │ │
│  │          (no TLS, no auth - SSH tunnel handles it)     │ │
│  │                                                         │ │
│  │  Endpoints:                                            │ │
│  │  - POST   /posts                   (create)           │ │
│  │  - GET    /posts/:slug/ast         (fetch AST)        │ │
│  │  - POST   /posts/:slug/delta       (delta update)     │ │
│  │  - GET    /posts/:slug/markdown    (render markdown)  │ │
│  │  - GET    /posts/:slug/html        (render HTML)      │ │
│  │  - GET    /posts                   (list all)         │ │
│  │  - DELETE /posts/:slug             (delete)           │ │
│  └─────────────────────┬───────────────────────────────────┘ │
│                        │                                     │
│  ┌─────────────────────▼───────────────────────────────────┐ │
│  │           Content-Addressable Storage Layer            │ │
│  │           - Insert nodes (dedupe via hash)             │ │
│  │           - Increment/decrement ref counts             │ │
│  │           - Walk trees                                 │ │
│  │           - Render AST → Markdown/HTML                 │ │
│  └─────────────────────┬───────────────────────────────────┘ │
│                        │                                     │
│  ┌─────────────────────▼───────────────────────────────────┐ │
│  │              PostgreSQL Database                       │ │
│  │              - ast_nodes (hash -> jsonb)               │ │
│  │              - posts (metadata + root hash)            │ │
│  │              - post_versions (version history)         │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### Design Rationale

**Why SSH Tunnel + HTTP:**
1. **Zero custom auth**: SSH tunnel provides authentication + encryption
2. **No TLS complexity**: Server binds to localhost only, SSH handles encryption
3. **Debuggable**: Can test API directly with curl over tunnel
4. **Fast**: HTTP/1.1 keepalive reuses connections, low overhead
5. **Stateless server**: Each request independent, easy to reason about
6. **Standard tooling**: Axum is battle-tested, well-documented
7. **Flexible**: Same API serves FUSE, web frontend, CLI tools, future mobile apps

**Trade-offs:**
- User must establish SSH tunnel (automated via systemd/launchd)
- Two processes to manage (SSH tunnel + FUSE driver)

---

## Technology Stack

- **Web framework**: `axum` (lightweight, fast, async)
- **Database**: PostgreSQL 14+ with `jsonb` support
- **ORM**: `sqlx` (compile-time checked queries)
- **Serialization**: `serde_json` for DB, `rmp-serde` + `zstd` for wire format
- **Hashing**: `blake3` crate (already used by client)
- **Rendering**: Custom AST→Markdown and AST→HTML renderers

---

## Database Schema

```sql
-- Content-addressed AST nodes
CREATE TABLE ast_nodes (
    hash BYTEA PRIMARY KEY,              -- Blake3 hash (32 bytes)
    node_data JSONB NOT NULL,            -- Serialized AstNode
    ref_count INTEGER DEFAULT 0,         -- Reference counting for GC
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CHECK (length(hash) = 32)
);
CREATE INDEX idx_ast_nodes_ref_count ON ast_nodes(ref_count);

-- Blog posts
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(255) UNIQUE NOT NULL,

    -- Content
    ast_root BYTEA NOT NULL REFERENCES ast_nodes(hash),

    -- Metadata
    title VARCHAR(500),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    published BOOLEAN DEFAULT false,

    -- Caching
    html_cache TEXT,
    html_cache_updated_at TIMESTAMPTZ,

    CHECK (length(ast_root) = 32)
);
CREATE INDEX idx_posts_published ON posts(published);
CREATE INDEX idx_posts_updated_at ON posts(updated_at DESC);

-- Version history (free with content-addressing!)
CREATE TABLE post_versions (
    id SERIAL PRIMARY KEY,
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    ast_root BYTEA NOT NULL REFERENCES ast_nodes(hash),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(post_id, version_number),
    CHECK (length(ast_root) = 32)
);
CREATE INDEX idx_post_versions_post_id ON post_versions(post_id);
```

---

## API Endpoints

- `POST /posts` - Create new post (slug, AST root, nodes)
- `GET /posts/:slug/ast` - Fetch AST (returns MessagePack+zstd)
- `POST /posts/:slug/delta` - Delta update (old_root, new_root, new_nodes)
- `GET /posts/:slug/markdown` - Render AST to markdown (for FUSE reads)
- `GET /posts/:slug/html` - Render AST to HTML (cached)
- `GET /posts` - List posts (with filtering)
- `DELETE /posts/:slug` - Delete post

---

## FUSE Driver Architecture

**Mount operation**:
- Lists all posts from server
- Creates virtual directory with `.md` files
- Starts FUSE event loop

**Read operation** (user opens file):
- Check local cache for markdown
- If miss: fetch from server via `GET /posts/:slug/markdown`
- Cache locally and return to FUSE

**Write operation** (user edits file):
- Update in-memory buffer
- Mark file as dirty
- Return immediately (no network call)

**Flush operation** (user saves file):
- Parse markdown → AST using bgc library
- Compute delta vs cached AST
- Send delta to server via `POST /posts/:slug/delta`
- Update cache with new root hash
- Clear dirty flag

---

## Performance Characteristics

| Operation | Target Latency | Breakdown |
|-----------|----------------|-----------|
| Read (cached) | <1ms | In-memory cache hit |
| Read (miss) | 50-200ms | Network RTT (20-50ms) + DB query (10-50ms) + AST→Markdown (20-100ms) |
| Write (buffer) | <1ms | In-memory only, no network |
| Flush (sync) | 50-500ms | Parse (50-200ms) + Delta (10-50ms) + Network (20-100ms) + DB (20-100ms) |
| List posts | <100ms | Metadata query only |

---

## Implementation Considerations

**Conflict Resolution**:
- Fail-fast approach: Check `old_root` hash in delta request, reject if mismatch
- Force client to refetch and retry on conflict
- Single-user system minimizes conflict likelihood

**Asset Handling**:
- Separate asset upload endpoint: `POST /assets`
- Returns content-addressed hash
- Markdown references: `![screenshot](asset://abc123...)`
- Stored in separate blob table with content-addressed keys

**Garbage Collection**:
- Background job deletes nodes with `ref_count = 0` older than 30 days
- Grace period prevents accidental deletion
- Admin endpoint for manual GC trigger

**Database Sizing** (100 posts):
- Raw data: ~1MB
- With deduplication: ~700KB
- With version history (10 versions/post): ~7MB
- With HTML cache: ~8MB total
- No special tuning needed for PostgreSQL

---

## Client-Server Protocol Details

**Create Post**:
```
POST /posts
Body: { slug, ast_root (hash), nodes: { hash → AstNode } }
Response: { id, slug }
```

**Delta Update**:
```
POST /posts/:slug/delta
Body: { old_root (hash), new_root (hash), new_nodes: { hash → AstNode } }
Response: { version, new_root }

Server validation:
1. Check current post.ast_root == old_root (conflict detection)
2. Insert new_nodes into ast_nodes (ON CONFLICT DO NOTHING)
3. Walk new tree, increment ref_counts
4. Walk old tree, decrement ref_counts
5. Update post.ast_root = new_root
6. Create version snapshot
7. Invalidate HTML cache
```

**Fetch AST**:
```
GET /posts/:slug/ast
Response: { root (hash), nodes: { hash → AstNode } }

Server process:
1. Query post by slug, get ast_root
2. Walk tree from root, collect all nodes
3. Serialize to MessagePack+zstd
4. Return compressed payload
```

**Render Markdown**:
```
GET /posts/:slug/markdown
Response: Raw markdown text

Server process:
1. Query post by slug, get ast_root
2. Walk tree, load all nodes
3. AST→Markdown visitor pattern
4. Return text/plain
```

---

## AST Rendering

### AST → Markdown Renderer

**Design**: Visitor pattern that walks the AST tree depth-first

**Key decisions**:
- ATX headings (`## Heading`) over Setext
- `**bold**` and `*italic*` consistently
- 2-space list indentation
- Blank line between block elements

**Requirements**:
- Deterministic output (same AST → same markdown)
- Round-trip compatibility (markdown → AST → markdown ≈ equivalent)
- Performance: <50ms for typical blog post

### AST → HTML Renderer

**Design**: Visitor pattern with extension system

**Extension system**:
- Base renderer produces semantic HTML
- Extensions inject classes, attributes, custom rendering
- Examples: Tailwind classes, syntax highlighting, table of contents

**Caching**:
- Store rendered HTML in `posts.html_cache`
- Invalidate on any delta update
- Reduces server load for read-heavy workloads
