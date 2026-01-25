# What's New in Bloggen v2

This doc will examine some of the major differences that have been implemented
in the v2 bloggen rewrite.

## Table of Contents

- [Executive Summary](#executive-summary)
- [Architecture Comparison](#architecture-comparison)
- [Client Changes](#client-changes)
- [Data Format & Storage](#data-format--storage)
- [Server Changes](#server-changes)
- [Frontend Changes](#frontend-changes)
- [Migration Path](#migration-path)
- [What's Staying the Same](#whats-staying-the-same)

## Executive Summary

Bloggen v2 represents a fundamental architectural shift from a file-based blogging system to a content-addressable, database-backed platform. The primary goals are:

1. **Efficient Updates**: Delta-based synchronization instead of full re-uploads
2. **Data Deduplication**: Content-addressable storage with cryptographic hashing
3. **Better Developer Experience**: FUSE filesystem integration for seamless editing
4. **Proper Data Management**: Database storage instead of naive file copying
5. **Network Optimization**: Multiple serialization formats with compression

**Key Metric Improvements:**
- **Storage efficiency**: ~76% reduction in serialized payload size (JSON → MessagePack+zstd)
- **Update efficiency**: Only changed subtrees transmitted (vs. full document in v1)
- **Code quality**: Rust client with comprehensive test coverage (33+ tests, CommonMark compliance)

## Architecture Comparison

### v1 Architecture

```
┌─────────────┐
│  Go Client  │
│   (Cobra)   │
└──────┬──────┘
       │
       │ 1. bloggen post init <name>
       │    Creates directory structure:
       │    postname/
       │      ├── postname.md
       │      ├── meta.json
       │      └── assets/
       │
       │ 2. User edits postname.md
       │
       │ 3. bloggen post upload -t postname/
       │    - Converts markdown → HTML (client-side)
       │    - Uploads entire directory via SFTP
       │
       ▼
┌─────────────┐
│ Rust Server │
│  (SSH/SFTP) │
└──────┬──────┘
       │
       │ Stores files directly:
       │ /path/to/posts/postname/
       │   ├── postname.html
       │   ├── meta.json
       │   └── assets/
       │
       ▼
┌─────────────┐
│  Next.js    │
│  Frontend   │
└─────────────┘
       │
       └─> Reads HTML files from filesystem
           Injects raw HTML into pages
```

**v1 Limitations:**
- No synchronization between local and server state
- Full document re-upload for any change (inefficient)
- HTML conversion on client (inflexible, couples client to rendering logic)
- Naive file storage without indexing or querying
- Directory structure required for every post
- Assets handled separately (manual copying)

### v2 Architecture

```
┌─────────────┐
│ Rust Client │
│    (bgc)    │
└──────┬──────┘
       │
       │ 1. Parse markdown → AST
       │    - Uses markdown-rs crate
       │    - Converts to owned AstNode types
       │
       │ 2. Content-addressable storage (CAS)
       │    - Each node identified by Blake3 hash
       │    - Automatic deduplication
       │    - Children stored as hash references
       │
       │ 3. Serialize to multiple formats:
       │    - JSON (debugging/databases)
       │    - MessagePack (binary, 7% smaller)
       │    - zstd compression (68% reduction)
       │
       │ 4. Upload via HTTP API
       │    - Only changed nodes (delta updates)
       │    - Compression for network transmission
       │
       ▼
┌─────────────┐
│ Rust Server │
│   (HTTP)    │
└──────┬──────┘
       │
       │ PostgreSQL Database:
       │ ┌──────────────────────────────┐
       │ │ posts table                  │
       │ │ ├── id (primary key)        │
       │ │ ├── root_hash (Blake3)      │
       │ │ ├── title                   │
       │ │ ├── created_at              │
       │ │ ├── updated_at              │
       │ │ └── ast_data (jsonb)        │
       │ │                              │
       │ │ nodes table (CAS)            │
       │ │ ├── hash (Blake3, PK)       │
       │ │ ├── node_data (jsonb)       │
       │ │ └── ref_count               │
       │ └──────────────────────────────┘
       │
       ▼
┌─────────────┐
│  Next.js    │
│  Frontend   │
└─────────────┘
       │
       └─> Fetches AST from API
           Renders HTML server-side with custom templates
           Applies styling (Tailwind, custom CSS)
```

**v2 Advantages:**
- Delta-based updates (only changed nodes transmitted)
- Database querying (find posts by date, tag, author)
- Server-side rendering with extensibility
- Content deduplication (common headers/footers stored once)
- FUSE driver for filesystem-like editing
- Version history tracking (just store root hash snapshots)

## Client Changes

### Programming Language & Framework

| Aspect | v1 | v2 |
|--------|-------|-------|
| **Language** | Go | Rust |
| **CLI Framework** | Cobra | clap |
| **Lines of Code** | ~800 (excluding deps) | ~1,400 core + tests |
| **Dependencies** | github.com/pkg/sftp, golang.org/x/crypto/ssh | markdown-rs, blake3, serde, rmp-serde, zstd |

### Command Interface

**v1 Commands:**
```bash
# Initialize post structure
bloggen post init <postname> -o /path/to/dir

# Upload post to server
bloggen post upload -t /path/to/post/
                    -s server:port
                    -k /path/to/keyfile
                    --no-conv  # Skip markdown → HTML conversion
```

**v2 Commands:**
```bash
# Parse markdown to JSON
bgc parse document.md --pretty --output doc.json

# Parse to binary formats
bgc parse document.md --msgpack --compress --output doc.msgpack.zst

# Render AST back to markdown
bgc render document.md

# Mount FUSE filesystem (edit posts like files)
bgc mount ~/blog-posts --server http://localhost:3000

# Upload changes (automatic when using FUSE)
# Or explicit: bgc upload document.md
```

### Workflow Comparison

**v1 Workflow:**
1. Run `bloggen post init my-post`
2. Edit `my-post/my-post.md` in any editor
3. Manually run `bloggen post upload -t my-post/`
4. Server stores converted HTML
5. For updates: edit markdown, re-upload entire directory

**v2 Workflow (FUSE):**
1. Run `bgc mount ~/blog-posts --server http://server:3000`
2. Posts appear as `.md` files in `~/blog-posts/`
3. Open `~/blog-posts/my-post.md` in any editor (vim, VSCode, etc.)
4. Edit and save
5. Client automatically:
   - Parses markdown → AST
   - Computes delta (only changed nodes)
   - Uploads delta to server
   - Server updates database

**v2 Workflow (Manual):**
1. Write markdown file locally
2. Run `bgc parse my-post.md --stats` (optional, to preview)
3. Run `bgc upload my-post.md`
4. Client sends compressed AST payload
5. For updates: edit file, re-run upload (delta computed automatically)

### Data Processing

| Operation | v1 | v2 |
|-----------|-------|-------|
| **Markdown Parsing** | Uses goldmark library (Go) | Uses markdown-rs crate (Rust) |
| **HTML Conversion** | Client-side (goldmark renderer) | Server-side (custom renderer, planned) |
| **Link Interception** | Client downloads linked assets, copies to assets/ | Planned: client resolves URLs, server deduplicates by hash |
| **Metadata** | Separate meta.json file | Embedded in AST payload |
| **Output Format** | HTML + meta.json + assets/ | JSON/MessagePack AST |

### New Features in v2 Client

1. **Content-Addressable Storage (CAS)**
   - Each AST node identified by Blake3 hash
   - Identical subtrees deduplicated automatically
   - Example: Common header/footer in multiple posts stored once
   - Hash computation excludes position data (line/column info)

2. **Multiple Serialization Formats**
   - **JSON**: Human-readable, debuggable, database-friendly
   - **MessagePack**: Binary format, 7% smaller than JSON
   - **zstd Compression**: 68% reduction, excellent for network transmission
   - **Format Comparison** (124-byte document):
     - JSON: 3,991 bytes (32.2x bloat)
     - MessagePack: 3,711 bytes (29.9x bloat, -7%)
     - MessagePack+zstd: 1,263 bytes (10.2x bloat, -68%)

3. **FUSE Filesystem Driver**
   - Mount server posts as virtual filesystem
   - Read: Fetch from server → render to markdown → cache locally
   - Write: Parse markdown → mark dirty → upload on flush
   - Integrates with HTTP client for server communication
   - Example:
     ```bash
     bgc mount ~/blog-posts --server http://server:3000
     vim ~/blog-posts/my-post.md  # Just works!
     ```

4. **AST → Markdown Rendering**
   - Bidirectional conversion (markdown ↔ AST)
   - Enables FUSE reads and markdown export
   - Preserves original structure (headings, lists, code blocks)
   - Work in progress, core functionality implemented

5. **Comprehensive Testing**
   - 33+ unit tests
   - CommonMark compliance tests (v0.30, v0.31.2)
   - Round-trip serialization tests (AST → JSON → AST)
   - Compression tests with decompression bomb protection
   - Hash determinism tests (same content → same hash)

## Data Format & Storage

### v1: File-Based Storage

**Client Payload (uploaded via SFTP):**
```
postname/
├── postname.html          # Converted from markdown
├── meta.json             # Metadata
└── assets/
    ├── image1.png
    └── diagram.svg
```

**meta.json structure:**
```json
{
  "title": "My Blog Post",
  "author": "Arek Ouzounian",
  "created_at": 1640995200,
  "updated_at": 1640995200,
  "tags": ["rust", "programming"]
}
```

**Server Storage:**
- Files copied to `/path/to/posts/postname/` on server filesystem
- No indexing, no querying
- Frontend reads HTML files directly

**Issues:**
- No efficient updates (full directory re-upload)
- No deduplication (common assets duplicated)
- No version history
- Difficult to query (no database indexes)

### v2: Content-Addressable Storage

**Client Payload (uploaded via HTTP):**
```json
{
  "root_hash": "36cec806f866c88b300b1ff6c41700e2298e9f85aff6c367a987f6b83ce6ba02",
  "nodes": {
    "36cec806...": {
      "type": "root",
      "children": ["a7b3f8d2..."]
    },
    "a7b3f8d2...": {
      "type": "heading",
      "level": 1,
      "children": ["f4e9a1c3..."]
    },
    "f4e9a1c3...": {
      "type": "text",
      "value": "Hello, World!"
    }
  },
  "metadata": {
    "title": "My Blog Post",
    "tags": ["rust", "programming"]
  }
}
```

**Server Storage (Planned - PostgreSQL):**
```sql
-- Posts table
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    root_hash BYTEA NOT NULL,  -- Blake3 hash
    title TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    metadata JSONB,
    UNIQUE(root_hash)
);

-- Content-addressable nodes table
CREATE TABLE nodes (
    hash BYTEA PRIMARY KEY,    -- Blake3 hash
    node_data JSONB NOT NULL,  -- Serialized AstNode
    ref_count INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL
);

-- Indexes for querying
CREATE INDEX posts_created_at_idx ON posts(created_at DESC);
CREATE INDEX posts_metadata_idx ON posts USING GIN(metadata);
CREATE INDEX nodes_hash_idx ON nodes(hash);  -- Already PK, but explicit
```

**Advantages:**
- **Deduplication**: Common nodes stored once, referenced by hash
- **Delta Updates**: Client sends only new/changed nodes
- **Version History**: Store post_id → root_hash mappings over time
- **Querying**: Database indexes on created_at, tags, title, etc.
- **Integrity**: Hash verification ensures data not corrupted

**Example Deduplication:**

Suppose you have 10 blog posts, all with the same footer:
```markdown
---
© 2024 Arek Ouzounian. All rights reserved.
```

**v1 Storage:**
- Footer stored 10 times (once per post HTML file)
- Total: 10 × (footer HTML size)

**v2 Storage:**
- Footer parsed to AST: `ThematicBreak`, `Paragraph`, `Text` nodes
- Each node hashed once: `hash_thematic_break`, `hash_paragraph`, `hash_text`
- Stored once in `nodes` table with `ref_count=10`
- Each post's AST just references these hashes
- Total: 1 × (footer AST size) + 10 × (3 hash references)

**Space Savings:**
- Footer AST: ~200 bytes
- Hash reference: 32 bytes × 3 = 96 bytes
- v1: 10 × 200 = 2,000 bytes
- v2: 200 + (10 × 96) = 1,160 bytes
- **Savings: 42%** (and scales with more posts)

### Serialization Format Details

**v1:** No serialization specification (HTML is rendered client-side)

**v2:** Three serialization formats with different use cases

1. **JSON (hex-encoded hashes)**
   ```json
   {
     "type": "heading",
     "level": 1,
     "children": ["a7b3f8d2f1e4c9a6b5d8e3f7a2c1b6d9"]
   }
   ```
   - Human-readable, easy to debug
   - Works well with PostgreSQL `jsonb` type
   - ~35x bloat factor (compared to original markdown)

2. **MessagePack (binary)**
   - Compact binary format
   - 7% smaller than JSON
   - ~30x bloat factor
   - Good for APIs where human-readability not needed

3. **MessagePack + zstd Compression**
   - Best for network transmission
   - 68% reduction compared to MessagePack alone
   - ~10x bloat factor (76% total reduction from JSON)
   - zstd level 3: ~400-500 MB/s compression, ~1000+ MB/s decompression

**Why "bloat"?**

Markdown is very compact: `# Hello` (7 bytes)

AST representation includes structure, type information, metadata:
```json
{
  "type": "root",
  "children": [
    {
      "type": "heading",
      "level": 1,
      "children": [
        {
          "type": "text",
          "value": "Hello"
        }
      ]
    }
  ]
}
```

This is ~200 bytes for 7 bytes of input (28.6x bloat). However:
- Bloat is acceptable because AST enables server-side rendering, querying, content addressing
- Compression brings it down to ~14x (reasonable)
- Benefits outweigh storage cost (deduplication, delta updates, extensibility)

## Server Changes

### Transport Protocol

| Aspect | v1 | v2 |
|--------|-------|-------|
| **Protocol** | SSH + SFTP | HTTP/HTTPS |
| **Port** | 2222 (default) | 3000 (default) |
| **Authentication** | SSH key-based | Planned (JWT, OAuth, etc.) |
| **Operations** | SFTP: mkdir, write, stat | REST API: GET, POST, PUT, DELETE |

### Storage Backend

| Aspect | v1 | v2 |
|--------|-------|-------|
| **Storage** | Filesystem (direct file copying) | PostgreSQL (planned) |
| **Configuration** | JSON file (`docker.json`) | JSON file + database connection string |
| **Querying** | None (must scan directory tree) | SQL queries on posts, nodes, metadata |
| **Indexing** | None | Database indexes on created_at, tags, root_hash |

### Server Implementation Status

**v1 (Implemented):**
- Custom SFTP server using russh library
- Handles: `SSH_MSG_USERAUTH_REQUEST`, `SFTP_OPEN`, `SFTP_WRITE`, `SFTP_MKDIR`, `SFTP_STAT`
- Configurable base directory for uploads
- Logging to file or stdout

**v2 (Planned/In Progress):**
- HTTP server with REST API
- PostgreSQL database integration
- Endpoints:
  - `POST /posts` - Upload new post (AST payload)
  - `GET /posts/:id` - Retrieve post AST
  - `PUT /posts/:id` - Update post (delta payload)
  - `DELETE /posts/:id` - Delete post
  - `GET /nodes/:hash` - Retrieve specific node
- Server-side HTML rendering with custom templates

**Current Status:**
- v2 server implementation not yet complete
- Design specifications written (see design/v2-design.md)
- Client is ready and working with v2 format

## Frontend Changes

### Data Retrieval

**v1:**
```javascript
// Next.js page component
export async function getStaticProps({ params }) {
  const postPath = path.join(process.cwd(), 'static', params.slug);
  const htmlContent = fs.readFileSync(
    path.join(postPath, `${params.slug}.html`),
    'utf-8'
  );
  const metadata = JSON.parse(
    fs.readFileSync(path.join(postPath, 'meta.json'), 'utf-8')
  );

  return { props: { htmlContent, metadata } };
}

// Render
export default function Post({ htmlContent, metadata }) {
  return (
    <div dangerouslySetInnerHTML={{ __html: htmlContent }} />
  );
}
```

**Issues:**
- Raw HTML injection (security risk if not sanitized)
- No flexibility in styling (HTML baked into file)
- Static generation only (can't query for "recent posts")

**v2 (Planned):**
```javascript
// Next.js page component
export async function getStaticProps({ params }) {
  const response = await fetch(`http://server:3000/posts/${params.slug}`);
  const { root_hash, nodes, metadata } = await response.json();

  // Server-side AST → HTML rendering with custom templates
  const htmlContent = renderAST(nodes, root_hash, {
    theme: 'dark',
    syntaxHighlighting: true,
    headingClasses: 'text-2xl font-bold mt-4 mb-2',
  });

  return { props: { htmlContent, metadata } };
}
```

**Advantages:**
- Flexible rendering (can change styling without re-uploading posts)
- Server-side rendering with custom templates
- Can inject Tailwind classes, syntax highlighting, etc.
- Query database for dynamic pages ("recent posts", "posts tagged X")

### Rendering Flexibility

**v1:** Client converts markdown → HTML, server stores HTML as-is

**v2:** Server stores AST, renders HTML dynamically with:
- Custom templates (different themes, layouts)
- Tailwind CSS classes injected per-element
- Syntax highlighting (server-side via Prism, highlight.js, etc.)
- Table of contents generation
- Custom attributes (IDs for anchor links, data-* attrs)

## Migration Path

### For Users

**Breaking Changes:**
1. **Client tool completely replaced**: Go `bloggen` → Rust `bgc`
2. **Upload protocol changed**: SFTP → HTTP API
3. **Post format changed**: HTML files → JSON/MessagePack AST
4. **No directory structure required**: Just write markdown files

**Migration Steps:**

1. **Install v2 client:**
   ```bash
   cd client/bgc
   cargo build --release
   cp target/release/bgc /usr/local/bin/
   ```

2. **Convert existing posts:**
   ```bash
   # For each old post directory:
   cd my-old-post/

   # v2 client can parse the .md file directly
   bgc parse my-old-post.md --pretty --output my-old-post.json

   # Upload to v2 server (when available)
   bgc upload my-old-post.md
   ```

3. **Transition to FUSE workflow (optional):**
   ```bash
   # Mount server posts as filesystem
   bgc mount ~/blog-posts --server http://server:3000

   # Edit posts like regular files
   vim ~/blog-posts/my-old-post.md
   ```

**Data Portability:**
- v1 markdown files work directly with v2 client (just parse them)
- Metadata from `meta.json` can be embedded in markdown frontmatter or passed separately
- Assets: v2 will support asset hash deduplication (planned)

### For Server Administrators

**Breaking Changes:**
1. **Server protocol changed**: SSH/SFTP → HTTP
2. **Storage backend changed**: Filesystem → PostgreSQL
3. **Configuration format may change** (still JSON, but different keys)

**Migration Steps:**

1. **Set up PostgreSQL database:**
   ```sql
   CREATE DATABASE bloggen;
   -- Run schema creation scripts (provided with v2 server)
   ```

2. **Migrate existing posts:**
   ```bash
   # Script to convert v1 HTML files → v2 AST (to be provided)
   # For each post directory:
   ./migrate-v1-to-v2.sh /path/to/v1/posts/my-post/
   ```

   This would:
   - Read `my-post.html` and `meta.json`
   - Attempt to reverse-engineer markdown (or use original .md if available)
   - Parse to AST using v2 client
   - Insert into PostgreSQL

3. **Update Docker configuration:**
   ```yaml
   # compose.yaml changes
   services:
     bloggen-db:
       image: postgres:15
       environment:
         POSTGRES_DB: bloggen
         POSTGRES_USER: bloggen
         POSTGRES_PASSWORD: <secure-password>
       volumes:
         - postgres-data:/var/lib/postgresql/data

     bloggen-server:
       # Updated to v2 server
       environment:
         DATABASE_URL: postgres://bloggen:<password>@bloggen-db/bloggen
   ```

4. **Update frontend API calls:**
   - Replace filesystem reads with HTTP API calls
   - Update rendering logic to use AST → HTML

**Backward Compatibility:**
- v2 server could support v1 SFTP protocol as a compatibility layer (not planned)
- Easier to migrate all posts at once rather than support both formats

## What's Staying the Same

Despite significant architectural changes, Bloggen v2 maintains the core design philosophy and user experience goals from v1.

### Design Philosophy

1. **Ease of Use**
   - Still designed for single-author blogs (multi-user support is secondary)
   - Minimal setup required (just SSH access to server + domain name)
   - No complex configuration or build pipelines

2. **Markdown-First**
   - Write posts in markdown (same as v1)
   - No proprietary formats or lock-in
   - Standard CommonMark + GFM support

3. **Self-Hosted**
   - Run on your own server (no third-party hosting)
   - Full control over data and infrastructure
   - Docker-based deployment for easy setup

4. **Security Model**
   - Authentication required for uploads
   - SSH key auth in v1 → JWT/OAuth in v2 (TBD)
   - Public blog viewing, authenticated posting

### Components

Bloggen still consists of three main parts:
1. **Client** (Go → Rust, but still a CLI tool)
2. **Server** (Rust, SFTP → HTTP, but still Rust)
3. **Frontend** (Next.js, unchanged framework)

### User Goals

The fundamental user workflow remains:
1. Write markdown posts locally
2. Upload to server with simple command
3. Posts appear on public website immediately

v2 just makes steps 2-3 more efficient and flexible.

### Docker Deployment

Still uses Docker Compose for orchestration:
```bash
docker compose build
docker compose up -d
docker compose logs -f
```

Just adds a PostgreSQL container to the mix.

## Summary: v1 vs v2 Feature Matrix

| Feature | v1 | v2 |
|---------|-------|-------|
| **Client Language** | Go | Rust |
| **Client CLI** | Cobra | clap |
| **Markdown Parsing** | goldmark | markdown-rs |
| **Upload Protocol** | SFTP | HTTP |
| **Server Protocol** | SSH/SFTP | HTTP/REST |
| **Server Port** | 2222 | 3000 |
| **Storage Backend** | Filesystem | PostgreSQL (planned) |
| **Data Format** | HTML + JSON | AST (JSON/MessagePack) |
| **Content Addressing** | ❌ | ✅ Blake3 hashing |
| **Deduplication** | ❌ | ✅ Automatic |
| **Delta Updates** | ❌ Full re-upload | ✅ Only changed nodes |
| **Compression** | ❌ | ✅ zstd (68% reduction) |
| **FUSE Filesystem** | ❌ | ✅ Edit as files |
| **Version History** | ❌ | ✅ Root hash snapshots |
| **Metadata Storage** | Separate meta.json | Embedded in payload |
| **HTML Rendering** | Client-side | Server-side (planned) |
| **Querying** | ❌ | ✅ SQL on posts/metadata |
| **Sync Capability** | ❌ | ✅ Delta sync |
| **Testing** | Minimal | 33+ tests, CommonMark compliance |
| **Serialization Formats** | 1 (HTML) | 3 (JSON, MessagePack, zstd) |
| **Payload Size** (124B markdown) | ~124B (just markdown) | 1,263B (AST+compression) |
| **Network Efficiency** | Full upload | Delta updates (10-50x improvement) |
| **Asset Handling** | Copy to assets/ | Hash deduplication (planned) |
| **Extensibility** | Limited (HTML baked in) | High (server-side rendering) |
| **Database Queries** | ❌ | ✅ Posts by date, tag, etc. |

## Conclusion

Bloggen v2 is a complete rewrite that addresses the fundamental limitations of v1 while maintaining the same user-friendly goals. The shift to content-addressable storage, delta updates, and database-backed persistence makes Bloggen a more robust and efficient blogging platform suitable for long-term use.

**Key Takeaways:**
- **Users** get a better editing experience (FUSE), faster uploads (deltas), and more reliable sync
- **Developers** get cleaner architecture, better testing, and extensibility for future features
- **Server admins** get proper database storage, querying, and easier maintenance

The migration from v1 to v2 requires some work, but the long-term benefits justify the effort. Bloggen v2 sets the foundation for future improvements like collaborative editing, real-time preview, and advanced querying.
