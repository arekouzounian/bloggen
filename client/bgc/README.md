# bgc - BlogGen Client

Rust client for BlogGen v2's content-addressable storage system.

## Overview

`bgc` parses markdown documents into a content-addressable Abstract Syntax Tree (AST) format using Blake3 hashing. This enables efficient storage, deduplication, and delta-based updates.

## Features

### ✅ Implemented
- **Content-Addressable Storage**: Blake3-based node identification with automatic deduplication
- **Multiple Serialization Formats**:
  - JSON with hex-encoded hashes (human-readable, debuggable)
  - MessagePack binary format (7% smaller than JSON)
  - zstd compression (68% reduction, 76% total from original format)
- **CommonMark + GFM Support**: Full compliance with CommonMark spec, plus GitHub Flavored Markdown extensions
- **CLI Tool**: Parse markdown files with flexible output options

### 🚧 Planned
- AST → Markdown rendering (for round-trip and FUSE reads)
- Delta update computation (network optimization)
- FUSE filesystem driver (edit posts as files)
- Server API integration

See [IMPLEMENTATION-TRACKER.md](./IMPLEMENTATION-TRACKER.md) for full roadmap.

## Installation

```bash
cargo build --release
```

## Usage

### Parse Markdown to JSON

```bash
# Compact JSON to stdout
cargo run -- parse document.md

# Pretty-printed JSON to file
cargo run -- parse document.md --pretty --output doc.json

# With statistics
cargo run -- parse document.md --stats
```

### Binary Formats

```bash
# MessagePack (smaller, faster)
cargo run -- parse document.md --msgpack --output doc.msgpack

# Compressed (best for network transmission)
cargo run -- parse document.md --msgpack --compress --output doc.msgpack.zst
```

### Example Output

```
✓ Parsed successfully
  Root hash: 36cec806f866c88b300b1ff6c41700e2298e9f85aff6c367a987f6b83ce6ba02
  Total nodes stored: 5
  Input size: 25 bytes
  Output size: 359 bytes (MessagePack+zstd)
  Size ratio: 14.4x
```

## Format Comparison

| Format | Size (25B input) | Bloat | Use Case |
|--------|-----------------|-------|----------|
| JSON (hex hashes) | 872 bytes | 34.9x | Debugging, database storage |
| MessagePack | 812 bytes | 32.5x | Binary APIs |
| JSON+zstd | 369 bytes | 14.8x | Compressed storage |
| **MessagePack+zstd** | **359 bytes** | **14.4x** | **Network transmission** ✓ |

For 124-byte documents:
- JSON: 3,991 bytes (32.2x)
- MessagePack+zstd: 1,263 bytes (10.2x) — **68% reduction**

## Architecture

```
Markdown Source
     ↓
 markdown-rs (parsing)
     ↓
markdown::mdast::Node
     ↓
Convert (~200 lines)
     ↓
AstNode (owned, no position data)
     ↓
┌────────────┬──────────────┬──────────────┐
│  Blake3    │   NodeStore  │ CasDocument  │
│  Hashing   │  (deduping)  │ (serialize)  │
└────────────┴──────────────┴──────────────┘
     ↓              ↓              ↓
   Hash        Reference      JSON/MessagePack
                 Tree         (+zstd optional)
```

### Key Concepts

- **Content-Addressable**: Each node identified by Blake3 hash of its content
- **Deduplication**: Identical subtrees (headers, footers, code snippets) stored once
- **Children as Hashes**: Nodes reference children by hash, not inline content
- **Efficient Updates**: Only changed nodes transmitted over network (planned)

## Documentation

- **[IMPLEMENTATION-TRACKER.md](./IMPLEMENTATION-TRACKER.md)**: Feature checklist and roadmap
- **[docs/serialization-size.md](./docs/serialization-size.md)**: Format analysis and optimization
- **[docs/network-compression.md](./docs/network-compression.md)**: Compression strategies

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Specific test
cargo test test_msgpack_roundtrip
```

Test coverage:
- 33+ unit tests
- CommonMark compliance tests (v0.30, v0.31.2)
- Round-trip serialization tests
- Compression/decompression tests

## Performance

### Serialization Sizes

Original format (byte array hashes): **42.6x bloat**
- Hex hashes: **34.9x** (-18%)
- MessagePack: **32.5x** (-7% more)
- zstd compression: **14.4x** (-68% more)
- **Total improvement: 76%**

### Compression Speed

zstd level 3:
- Compression: ~400-500 MB/s
- Decompression: ~1000+ MB/s

Good balance between speed and ratio for network transmission.

## Development

### Project Structure

```
bgc/
├── src/
│   ├── lib.rs           # Public API
│   ├── main.rs          # CLI tool
│   ├── ast.rs           # AstNode definitions
│   ├── cas.rs           # Content-addressable storage
│   └── convert.rs       # markdown-rs → AstNode
├── examples/            # Example usage
├── docs/               # Design documents
└── IMPLEMENTATION-TRACKER.md  # Feature tracker
```

### Adding a New AST Node Type

1. Add variant to `AstNode` enum in `src/ast.rs`
2. Implement conversion in `src/convert.rs`
3. Add test case
4. Update serialization tests

### Design Philosophy

- **Minimal overhead**: Only store semantic data, no position info
- **Deterministic hashing**: Same content → same hash, always
- **Standard libraries**: Leverage markdown-rs for parsing
- **Hybrid approach**: JSON for debugging, binary for production

## Roadmap

### Next Milestones

1. **AST → Markdown Renderer** (required for FUSE)
   - Deterministic markdown generation
   - Round-trip testing

2. **Delta Updates** (network optimization)
   - Compute diff between documents
   - 10-50x improvement for typical edits

3. **FUSE Driver** (filesystem integration)
   - Mount posts as files
   - Edit in any text editor
   - Auto-sync to server

4. **Server Integration**
   - API client
   - Delta-based updates
   - Conflict resolution

See [IMPLEMENTATION-TRACKER.md](./IMPLEMENTATION-TRACKER.md) for complete roadmap.

## Contributing

This is an active development project. Key areas needing work:
- AST → Markdown rendering
- Delta update computation
- FUSE driver implementation
- Performance optimization

## License

See parent project for license information.

## Related

- **Server**: `/server` (Rust, not yet migrated to v2)
- **Frontend**: `/bloggen-frontend` (Next.js)
- **Design Specs**: `/design` (architecture documentation)
