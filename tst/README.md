# BlogGen v2 Test Suite

This directory contains end-to-end integration tests for BlogGen v2, testing both client and server components.

## Test Scripts

### `e2e-fuse.sh` - FUSE Filesystem Integration Test

**TRUE end-to-end test** that validates the complete FUSE workflow:

**What it tests:**
1. ✅ Database and server startup
2. ✅ FUSE filesystem mounting
3. ✅ Creating new files through FUSE (virtual files, not local)
4. ✅ Writing content through FUSE
5. ✅ Reading content through FUSE
6. ✅ Directory listing shows FUSE files
7. ✅ Unmounting FUSE
8. ✅ Verifying files are gone locally (only existed through FUSE)
9. ✅ Re-mounting FUSE
10. ✅ Verifying files reappear with same content
11. ✅ Full persistence through mount/unmount cycle

**Requirements:**
- Docker (for PostgreSQL)
- Rust toolchain
- FUSE support (libfuse3)
- Both `client/bgc` and `v2-server` must be buildable

**Usage:**
```bash
# Run from repository root
./tst/e2e-fuse.sh

# Skip database setup (use existing DB)
SKIP_DB_SETUP=1 ./tst/e2e-fuse.sh

# Keep database running after tests
KEEP_DB_RUNNING=1 ./tst/e2e-fuse.sh

# Use custom server port
SERVER_PORT=8080 ./tst/e2e-fuse.sh
```

**Environment Variables:**
- `SKIP_DB_SETUP=1` - Skip database setup (assumes DB is already running)
- `KEEP_DB_RUNNING=1` - Don't stop database after tests complete
- `SERVER_PORT=3000` - Server port (default: 3000)

**Expected Output:**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Test Results
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total tests: 20
Passed: 20
Failed: 0

✓ All FUSE E2E tests passed!
```

---

### `e2e-full.sh` - Full Client + Server Integration Test

Complete end-to-end test that validates the entire BlogGen v2 **CLI workflow** (not FUSE):

**What it tests:**
1. ✅ Client markdown parsing and CAS generation
2. ✅ JSON and MessagePack serialization round-trips
3. ✅ PostgreSQL database setup
4. ✅ v2-server startup and health checks
5. ✅ Post upload to server
6. ✅ Post download from server
7. ✅ Round-trip fidelity (original → server → downloaded)
8. ✅ Delta computation for updates
9. ✅ Delta upload to server
10. ✅ Verification of updated content

**Requirements:**
- Docker (for PostgreSQL)
- Rust toolchain
- Both `client/bgc` and `v2-server` must be buildable

**Usage:**
```bash
# Run with default test markdown
./e2e-full.sh

# Run with custom markdown file
./e2e-full.sh path/to/your/post.md

# Skip database setup (use existing DB)
SKIP_DB_SETUP=1 ./e2e-full.sh

# Keep database running after tests
KEEP_DB_RUNNING=1 ./e2e-full.sh

# Use custom server port
SERVER_PORT=8080 ./e2e-full.sh
```

**Environment Variables:**
- `SKIP_DB_SETUP=1` - Skip database setup (assumes DB is already running)
- `KEEP_DB_RUNNING=1` - Don't stop database after tests complete
- `SERVER_PORT=3000` - Server port (default: 3000)

**Expected Output:**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Phase 1: Building Projects
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
▸ Building client (bgc)...
  ✓ Client built successfully
▸ Building server (v2-server)...
  ✓ Server built successfully

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Test Results
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total tests: 18
Passed: 18
Failed: 0

✓ All tests passed!
```

---

### `e2e-client.sh` - Client-Only Test

Tests the client (`bgc`) in isolation without requiring the server:

**What it tests:**
1. ✅ Markdown parsing to CAS
2. ✅ JSON serialization/deserialization
3. ✅ MessagePack serialization/deserialization
4. ✅ Compressed formats (JSON+zstd, MessagePack+zstd)
5. ✅ Round-trip rendering (markdown → CAS → markdown)
6. ✅ Delta computation between two markdown files
7. ✅ Delta application
8. ✅ Edge cases (empty files, large files, unicode)

**Requirements:**
- Rust toolchain
- Only `client/bgc` needs to be buildable

**Usage:**
```bash
# Run from repository root with default test markdown
./tst/e2e-client.sh

# Run with custom markdown file
./tst/e2e-client.sh path/to/your/file.md

# Can also run from tst/ directory
cd tst
./e2e-client.sh
```

**Expected Output:**
```
━━━ Phase 1: Parsing and Serialization ━━━
✓ Test 1/23: Parse markdown to JSON
✓ Test 2/23: Parse to MessagePack
✓ Test 3/23: Parse to compressed JSON
...
✓ Test 23/23: Unicode handling

Summary: 23/23 tests passed (100.0%)
```

---

## Running All Tests

To run the complete test suite:

```bash
# From repository root

# 1. Run client-only tests first (fast, ~2-5 seconds)
./tst/e2e-client.sh

# 2. Run CLI integration tests (~15-30 seconds)
./tst/e2e-full.sh

# 3. Run FUSE integration tests (~20-40 seconds)
./tst/e2e-fuse.sh
```

**Back-to-back testing:** All scripts are idempotent and can be run multiple times in a row without issues.

**Quick full suite:**
```bash
./tst/e2e-client.sh && ./tst/e2e-full.sh && ./tst/e2e-fuse.sh
```

## Test Workflow Diagrams

### e2e-fuse.sh - FUSE Integration

```
┌─────────────────────────────────────────────┐
│          e2e-fuse.sh Workflow               │
└─────────────────────────────────────────────┘

1. Build Projects
   ├─ cargo build (client/bgc)
   └─ cargo build (v2-server)

2. Setup Infrastructure
   ├─ docker compose up (PostgreSQL)
   └─ cargo sqlx migrate run

3. Start Server
   └─ v2-server (background process)

4. Mount FUSE (First Time)
   └─ bgc mount /tmp/mount --server http://localhost:3000

5. Create File Through FUSE
   └─ echo "content" > /tmp/mount/new-post.md

6. Read File Through FUSE
   └─ cat /tmp/mount/new-post.md

7. Verify Directory Listing
   └─ ls /tmp/mount (shows new-post.md)

8. Unmount FUSE
   └─ fusermount -u /tmp/mount

9. Verify File Gone Locally
   └─ ls /tmp/mount (empty, file only existed through FUSE)

10. Re-mount FUSE
    └─ bgc mount /tmp/mount --server http://localhost:3000

11. Verify File Reappears
    ├─ ls /tmp/mount (shows new-post.md again)
    ├─ cat /tmp/mount/new-post.md
    └─ content matches original

12. Cleanup
    ├─ DELETE /posts/new-post
    ├─ unmount FUSE
    ├─ kill server
    └─ docker compose down
```

### e2e-full.sh - CLI Integration

```
┌─────────────────────────────────────────────┐
│          e2e-full.sh Workflow               │
└─────────────────────────────────────────────┘

1. Build Projects
   ├─ cargo build (client/bgc)
   └─ cargo build (v2-server)

2. Setup Infrastructure
   ├─ docker compose up (PostgreSQL)
   └─ cargo sqlx migrate run

3. Start Server
   └─ v2-server (background process)

4. Client Operations
   ├─ bgc parse test.md
   ├─ bgc render (JSON round-trip)
   └─ bgc render (MessagePack round-trip)

5. Upload to Server
   └─ bgc upload test.md --slug test-post

6. Download from Server
   └─ bgc download test-post > downloaded.md

7. Verify Round-Trip
   └─ diff test.md downloaded.md

8. Delta Update
   ├─ Edit test.md → test-modified.md
   └─ bgc update test.md test-modified.md --slug test-post

9. Verify Update
   ├─ bgc download test-post > updated.md
   └─ diff test-modified.md updated.md

10. Cleanup
    ├─ DELETE /posts/test-post
    ├─ kill server
    └─ docker compose down
```

## Continuous Integration

These tests are designed to run in CI environments:

```yaml
# Example GitHub Actions workflow
name: E2E Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Run client tests
        run: cd client/bgc && ../../tst/e2e-client.sh

      - name: Run full integration tests
        run: ./tst/e2e-full.sh
```

## Troubleshooting

### Database Connection Errors

```bash
# Make sure PostgreSQL container is running
docker ps | grep bloggen-dev-db

# If not, start it manually
cd v2-server
docker compose -f docker-compose.dev.yml up -d
cd ..

# Then run tests with SKIP_DB_SETUP=1
SKIP_DB_SETUP=1 ./tst/e2e-full.sh
```

### Server Port Already in Use

```bash
# Use a different port
SERVER_PORT=8080 ./tst/e2e-full.sh
```

### Build Failures

```bash
# Build projects manually to see detailed errors
cd client/bgc && cargo build
cd ../../v2-server && cargo build
```

### Test Failures

Check the detailed logs:
```bash
# Server logs are in $TEST_DIR/server.log during test execution
# The script will show the path in error messages
```

## Adding New Tests

To add a new test to `e2e-full.sh`:

1. Add a new phase section:
```bash
log_section "Phase X: Your Test Name"
```

2. Use `log_step` for individual steps:
```bash
log_step "Doing something..."
if your_command; then
    pass "Success message"
else
    fail "Failure message"
fi
```

3. Use the `pass()` and `fail()` helpers to track test results

## Test Data

The tests use temporary directories created with `mktemp -d` and automatically cleaned up on exit.

Default test markdown includes:
- Headings (H1-H3)
- Bold, italic, strikethrough
- Code blocks
- Lists (ordered and unordered)
- Links
- Multiple paragraphs

You can provide your own markdown files to test specific edge cases.

## Performance

**e2e-client.sh**: ~2-5 seconds (no external dependencies)
**e2e-full.sh**: ~15-30 seconds (includes Docker startup, server compilation)
**e2e-fuse.sh**: ~20-40 seconds (includes FUSE mount/unmount cycles)

---

**Last Updated**: 2026-01-19
**Status**: ✅ All three test suites are functional and verified to work back-to-back
**Verified**: All scripts work from repository root and pass when run consecutively
