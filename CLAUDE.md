# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BlogGen is a simplified blogging framework consisting of three main components:
1. **Rust CLI client** (`client/bgc/`) - Parses markdown posts into JSON AST payloads
2. **Rust server** (`server/`) - Handles post uploads and storage (currently SFTP-based, migrating to database)
3. **Next.js frontend** (`bloggen-frontend/`) - Web interface for displaying blog posts

## Architecture

### Version 2 Migration (In Progress)

The project is currently transitioning from v1 to v2 architecture. The current branch `v2-client` contains work on the new client implementation.

**Key v2 changes:**
- Client now parses markdown into JSON AST instead of HTML
- Server will use PostgreSQL with `jsonb` storage (not yet implemented)
- Posts sent as single JSON payload rather than directory uploads
- Future: FUSE driver for filesystem-like server interaction

**v1 limitations being addressed:**
- No sync capability between local and server state
- Naive file storage without database
- Inefficient full post re-upload for updates
- Limited SFTP implementation coupled to specific client

### Client Architecture (`client/bgc/`)

The v2 client parses markdown files into a custom JSON AST representation:

1. **Markdown parsing** (`parse.rs`): Uses the `markdown` crate to parse markdown into an AST
2. **AST wrapping** (`ast_wrapper.rs`): Wraps markdown AST nodes to control JSON serialization
   - Uses newtype pattern to avoid duplicating the entire tree in memory
   - Custom `Serialize` implementations exclude unnecessary fields (e.g., position data)
   - Snake_case conversion for field names
   - Macro `serialize_wrapped_type!` for consistent serialization
3. **Output**: JSON payload suitable for database storage and server-side processing

### Server Architecture (`server/`)

Currently implements a custom SFTP server over SSH (v1). Configuration via JSON files documented in `src/config.rs`.

**Note**: Server is in transition to v2 design with database storage.

## Development Commands

### Client (Rust - `client/bgc/`)

```bash
# Build
cd client/bgc
cargo build

# Run tests (includes CommonMark compliance tests)
cargo test

# Parse markdown to JSON
cargo run -- parse <file.md> --pretty --outfile output.json

# Debug mode (shows AST and intermediate steps)
cargo run -- --debug parse <file.md>

# Render (experimental)
cargo run -- render <file.md>

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

```bash
# Build containers
docker compose build

# Start services
docker compose up -d

# Stop services
docker compose down
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

### Client AST Wrapper Pattern

When modifying AST serialization:
- Each markdown node type has a corresponding wrapper type in `ast_wrapper.rs`
- Add new wrapper types to the `NodeWrapper` enum
- Implement `Serialize` using the `serialize_wrapped_type!` macro
- The macro automatically adds a "type" field with snake_case conversion
- Use `wrap_vec()` to wrap children nodes

### Testing

Client includes CommonMark compliance tests in `parse.rs`:
- Test suite v0.30: `assets/common_mark_test_suite_v0_30.json`
- Test suite v0.31.2: `assets/common_mark_test_suite_v0_31_2.json`

Run specific tests:
```bash
cargo test test_parse_html_v0_31_2
cargo test test_nonexistent_file_parse
```

## Configuration

### Server Configuration

Configured via JSON file (see `server/src/config.rs` for schema). Docker deployment uses `server/docker.json`.

Default SSH port: 2222 (configurable)

### Environment Variables (v1 Client)

- `BLOGGEN_SERVER`: Server target in `ip:port` format (e.g., `localhost:2222`)

## Important Notes

- The v1 Go client in `cli/` is deprecated in favor of the v2 Rust client in `client/bgc/`
- Server v2 database implementation is not yet complete
- Position information from markdown AST is intentionally excluded from JSON serialization to reduce payload size
- Custom serialization avoids the `markdown` crate's default format to have full control over output structure
