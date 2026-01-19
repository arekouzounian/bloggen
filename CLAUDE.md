# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BlogGen is a simplified blogging framework consisting of three main components:
1. **Rust CLI client** (`client/bgc/`) - Parses markdown posts into content-addressable AST payloads
2. **Rust server** (`server/`) - Handles post uploads and storage (currently SFTP-based, migrating to database)
3. **Next.js frontend** (`bloggen-frontend/`) - Web interface for displaying blog posts

## Architecture

### Version 2 Migration (In Progress)

The project is currently transitioning from v1 to v2 architecture. See `design/v2-design.md` for the full specification.

**Key v2 changes:**
- Client uses content-addressable storage with Blake3 hashing
- Multiple serialization formats: JSON (debugging), MessagePack (binary), zstd compression
- Deduplication of identical subtrees (e.g., repeated headers, code snippets)
- Server will use PostgreSQL with `jsonb` storage (not yet implemented)
- Future: Delta updates for efficient network transmission, FUSE driver for filesystem-like interaction

**v1 limitations being addressed:**
- No sync capability between local and server state
- Naive file storage without database
- Inefficient full post re-upload for updates
- Limited SFTP implementation coupled to specific client

### Client Architecture (`client/bgc/`)

The v2 client uses a content-addressable storage (CAS) system:

1. **Markdown parsing** (`convert.rs`): Uses `markdown-rs` crate, converts to owned `AstNode` enum
2. **Content-addressable storage** (`cas.rs`):
   - Each node identified by Blake3 hash of its content
   - Children stored as hash references, not inline content
   - `NodeStore` provides in-memory hash→node mapping
   - Automatic deduplication of identical subtrees
3. **Serialization** (`cas.rs`, `ast.rs`):
   - Custom `Serialize` implementations for hex-encoded hashes
   - Position metadata excluded to reduce payload size
   - Multiple output formats: JSON, MessagePack, zstd-compressed
4. **Rendering** (`render.rs`): Experimental AST→Markdown round-trip (work in progress)

**Key data flow:**
```
Markdown → markdown-rs → AstNode → NodeStore → CasDocument → JSON/MessagePack/zstd
```

### Server Architecture (`server/`)

Currently implements a custom SFTP server over SSH (v1). Configuration via JSON files documented in `src/config.rs`.

**Note**: Server is in transition to v2 design with database storage.

## Development Commands

### Client (Rust - `client/bgc/`)

```bash
# Build
cd client/bgc
cargo build

# Run all tests (includes CommonMark compliance tests)
cargo test

# Run specific test
cargo test test_msgpack_roundtrip
cargo test test_parse_html_v0_31_2

# Run tests with output
cargo test -- --nocapture

# Parse markdown to JSON (compact, to stdout)
cargo run -- parse document.md

# Parse with pretty-printing and output file
cargo run -- parse document.md --pretty --output output.json

# Parse with statistics
cargo run -- parse document.md --stats

# Parse to MessagePack binary format
cargo run -- parse document.md --msgpack --output doc.msgpack

# Parse to compressed format (best for network transmission)
cargo run -- parse document.md --msgpack --compress --output doc.msgpack.zst

# Render AST back to markdown (experimental, WIP)
cargo run -- render document.md

# Lint
cargo clippy

# Format
cargo fmt
```

### Server (Rust - `server/`)

```bash
# Build
cd server
cargo build

# Run (requires config JSON)
cargo run

# Format
cargo fmt

# Lint
cargo clippy
```

### Frontend (Next.js - `bloggen-frontend/`)

```bash
cd bloggen-frontend

# Development server
npm run dev

# Production build
npm run build

# Start production server
npm start

# Lint
npm run lint
```

### Docker Deployment

The project uses `compose.yaml` for Docker deployment:

```bash
# Build containers
docker compose build

# Start services
docker compose up -d

# Stop services
docker compose down

# View logs
docker compose logs -f
```

### Nix Development Environment

```bash
# Enter default dev shell (includes Rust, Go, Node.js)
nix develop

# Rust-specific shell
nix develop .#rust

# Go-specific shell (for v1 client)
nix develop .#go

# Build packages
nix build .#server
nix build .#go_client
```

## Code Organization

### Client Code Structure (`client/bgc/src/`)

- `ast.rs` - `AstNode` enum definitions for all CommonMark + GFM node types, custom Blake3Hash serialization
- `cas.rs` - Content-addressable storage: `CasNode`, `NodeStore`, `CasDocument`, hash computation
- `convert.rs` - Conversion from `markdown::mdast::Node` to owned `AstNode` (~200 lines)
- `render.rs` - AST→Markdown rendering (experimental, work in progress)
- `lib.rs` - Public API exports
- `main.rs` - CLI tool implementation with clap

### Adding New AST Node Types

When adding support for new markdown node types:
1. Add variant to `AstNode` enum in `ast.rs`
2. Implement conversion logic in `convert.rs` (in `convert_node()` match statement)
3. If rendering is needed, update `render.rs`
4. Add test cases to verify parsing and serialization
5. Ensure `Serialize`/`Deserialize` derive works correctly

### Testing

The client includes comprehensive test coverage:
- **CommonMark compliance**: Test suites v0.30 and v0.31.2 in `assets/`
- **Round-trip tests**: Serialization→Deserialization for all formats
- **Compression tests**: zstd compression/decompression with bomb protection
- **Hash determinism**: Same content always produces same hash

Key test files:
```bash
cargo test test_parse_html_v0_31_2       # CommonMark compliance
cargo test test_msgpack_roundtrip        # Binary serialization
cargo test test_compression_roundtrip    # Compression
cargo test test_hash_determinism         # CAS correctness
```

## Configuration

### Server Configuration

Configured via JSON file (see `server/src/config.rs` for schema). Docker deployment uses `server/docker.json`.

Default SSH port: 2222 (configurable)

### Environment Variables (v1 Client)

- `BLOGGEN_SERVER`: Server target in `ip:port` format (e.g., `localhost:2222`)

## Important Notes

### Version Status
- **Current branch**: `v2` - Active v2 client development
- **Main branch**: `main` - Contains v1 implementation
- The v1 Go client in `cli/` is deprecated in favor of the v2 Rust client in `client/bgc/`
- Server v2 database implementation is not yet complete; server is still v1 SFTP-based

### Design Philosophy
- **Content-addressable storage**: Enables deduplication, efficient updates, and version history
- **Multiple serialization formats**: JSON for debugging/databases, MessagePack+zstd for network transmission
- **Position data excluded**: Reduces payload size; position info from markdown-rs AST is intentionally omitted
- **Hash determinism**: Same content always produces same Blake3 hash for reliable deduplication
- **Planned delta updates**: Only changed nodes will be transmitted for efficient post updates

### Key Documentation
- `design/v2-design.md` - Overall v2 architecture and goals
- `design/cas-ast-spec.md` - Content-addressable storage specification
- `design/ast-serialization-spec.md` - Serialization format details
- `client/bgc/IMPLEMENTATION-TRACKER.md` - Feature implementation status and roadmap
- `client/bgc/docs/serialization-size.md` - Format analysis and optimization
- `client/bgc/README.md` - Client-specific documentation
