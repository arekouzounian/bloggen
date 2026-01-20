# FUSE Filesystem Usage Guide

The BlogGen client provides a FUSE filesystem driver that allows you to interact with your blog posts as if they were regular markdown files in a directory.

## Overview

The FUSE driver:
- Mounts a virtual directory containing all your posts as `.md` files
- Automatically fetches posts from the server when you read them
- Caches posts locally for fast access
- Tracks changes and uploads them to the server on file close/flush
- Provides a native file-based workflow for editing blog posts

## Requirements

### System Dependencies

On NixOS (using the provided `flake.nix`):
```bash
nix develop .#rust
```

The flake includes:
- `fuse3` - FUSE library for filesystem operations
- `pkg-config` - Build configuration tool

### Server Requirements

- A running BlogGen v2 server
- Server URL (e.g., `http://localhost:3000`)

## Basic Usage

### 1. Mount the Filesystem

Create an empty directory and mount the FUSE filesystem:

```bash
# Create mount point
mkdir ~/blog-posts

# Mount the filesystem
cargo run -- mount ~/blog-posts --server http://localhost:3000

# Or with logging enabled
RUST_LOG=info cargo run -- mount ~/blog-posts --server http://localhost:3000
```

The filesystem will stay mounted until you press Ctrl+C.

### 2. Browse Posts

Once mounted, you can use standard filesystem commands:

```bash
# List all posts
ls ~/blog-posts/

# Output example:
# my-first-post.md
# getting-started.md
# tutorial-part-1.md
```

### 3. Read Posts

View post content with any text viewer:

```bash
# View with cat
cat ~/blog-posts/my-first-post.md

# Edit with your favorite editor
vim ~/blog-posts/my-first-post.md
nano ~/blog-posts/my-first-post.md
code ~/blog-posts/my-first-post.md
```

### 4. Edit Posts

Edit posts using any text editor. Changes are automatically tracked:

```bash
# Edit a post
vim ~/blog-posts/my-first-post.md

# Changes are marked as "dirty" in the cache
# When you save and close the file, changes are uploaded to the server
```

### 5. Unmount

Press `Ctrl+C` in the terminal where you ran the mount command, or:

```bash
# From another terminal
fusermount -u ~/blog-posts

# On macOS
umount ~/blog-posts
```

## Workflow Example

Here's a complete workflow for editing a blog post:

```bash
# 1. Mount the filesystem
mkdir -p ~/blog-mount
cd ~/blog-mount
cargo run --release -- mount . --server http://localhost:3000 &

# 2. Wait for mount to complete
sleep 2

# 3. List posts
ls -lh

# 4. Read a post
cat my-first-post.md

# 5. Edit the post
vim my-first-post.md
# Make your changes and save

# 6. The changes are automatically uploaded when you close the file

# 7. Unmount when done
fusermount -u .
```

## Architecture

### Cache Layer

The FUSE driver maintains an in-memory cache with three components:

1. **Post Metadata**: Slug, title, timestamps, published status
2. **AST**: Content-addressable storage representation
3. **Markdown**: Rendered markdown for fast reads

### Read Operation

1. User reads a file (e.g., `cat my-post.md`)
2. Check cache for markdown
3. If cache miss, fetch AST from server
4. Render AST to markdown
5. Cache the result
6. Return markdown to user

### Write Operation

1. User writes to a file (e.g., save in editor)
2. Parse markdown into AST
3. Store new AST in cache
4. Mark post as "dirty"
5. Return success

### Flush Operation

1. Triggered when file is closed
2. Check if post is dirty
3. If dirty, upload to server (currently full AST)
4. Clear dirty flag
5. Return success

## Logging

Enable logging to see what's happening:

```bash
# Info level (recommended)
RUST_LOG=info cargo run -- mount ~/blog-posts

# Debug level (verbose)
RUST_LOG=debug cargo run -- mount ~/blog-posts

# Specific module
RUST_LOG=bgc::fuse=debug cargo run -- mount ~/blog-posts
```

Log output includes:
- Mount/unmount events
- Post list refreshes
- Cache hits/misses
- File operations (read, write, flush)
- Server communication
- Errors and warnings

## Limitations & Future Work

### Current Limitations

1. **Full AST Upload**: Currently uploads the entire AST on write. Delta updates coming soon.
2. **In-Memory Cache**: Cache is lost on unmount. Persistent cache planned.
3. **No Conflict Detection**: If server version changes, conflicts aren't detected yet.
4. **No Offline Mode**: Requires server connection. Offline buffering planned.
5. **Write-Only Offset 0**: Only supports full file writes (no partial writes/appends).

### Planned Features

- **Delta Updates on Write**: Compute and send only changed nodes
- **Conflict Detection**: Detect when server version has changed
- **Persistent Cache**: Save cache to disk for faster remounts
- **Offline Mode**: Buffer writes when server is unavailable
- **Background Sync**: Periodically sync changes in the background
- **Partial Writes**: Support for text editor partial writes
- **LRU Eviction**: Limit cache size with least-recently-used eviction
- **Version History**: Browse previous versions of posts
- **Snapshots**: Create local checkpoints before making changes

## Troubleshooting

### "Mount point is not empty"

FUSE requires an empty directory:

```bash
# Check what's in the directory
ls -la ~/blog-posts/

# Remove contents or use a different directory
rm -rf ~/blog-posts/*
# OR
mkdir ~/empty-dir && cargo run -- mount ~/empty-dir
```

### "Transport endpoint is not connected"

The filesystem was forcibly unmounted or crashed:

```bash
# Unmount the stale mount
fusermount -u ~/blog-posts

# Remount
cargo run -- mount ~/blog-posts
```

### "Permission denied"

Ensure your user has FUSE permissions:

```bash
# Check if fusermount is available
which fusermount

# On NixOS with the flake, this should work automatically
```

### Server Connection Errors

Check server URL and ensure the server is running:

```bash
# Test server connectivity
curl http://localhost:3000/posts

# Check logs
RUST_LOG=info cargo run -- mount ~/blog-posts --server http://localhost:3000
```

## Performance Tips

1. **Use Release Builds**: `cargo run --release` is significantly faster
2. **Enable Logging Sparingly**: `RUST_LOG=debug` can slow things down
3. **Cache Warm-up**: First read of each post fetches from server (slower), subsequent reads are cached
4. **Batch Edits**: Make multiple changes before closing the file to minimize uploads

## Advanced Usage

### Custom Server URL

```bash
# Development server
cargo run -- mount ~/blog-posts --server http://localhost:3000

# Production server
cargo run -- mount ~/blog-posts --server https://blog.example.com

# Local server with different port
cargo run -- mount ~/blog-posts --server http://localhost:8080
```

### Integration with Git

Since posts appear as regular files, you can use version control:

```bash
cd ~/blog-posts
git init
git add *.md
git commit -m "Snapshot of all blog posts"

# Make edits
vim my-post.md

# See what changed
git diff my-post.md

# Commit changes
git add my-post.md
git commit -m "Updated my-post"
```

### Scripting

Automate post management with shell scripts:

```bash
#!/bin/bash
# Bulk edit all posts

MOUNT_DIR="$HOME/blog-posts"

# Mount
cargo run --release -- mount "$MOUNT_DIR" --server http://localhost:3000 &
MOUNT_PID=$!
sleep 2

# Process all posts
for post in "$MOUNT_DIR"/*.md; do
    echo "Processing $post..."
    # Add a footer to each post
    echo -e "\n\n---\n*Last updated: $(date)*" >> "$post"
done

# Unmount
kill $MOUNT_PID
```

## See Also

- [IMPLEMENTATION-TRACKER.md](../IMPLEMENTATION-TRACKER.md) - Implementation status
- [CLAUDE.md](../../CLAUDE.md) - Project overview
- [v2-design.md](../../design/v2-design.md) - Architecture design
