#!/usr/bin/env bash
#
# Full End-to-End Test Script for BlogGen v2
#
# Tests the complete workflow:
# 1. Client: Parse markdown -> CAS document
# 2. Client: Serialize and deserialize
# 3. Server: Start PostgreSQL and v2-server
# 4. Client->Server: Upload post
# 5. Client->Server: Download and verify
# 6. Client: Make changes and compute delta
# 7. Client->Server: Upload delta
# 8. Client->Server: Download updated post
# 9. Verify round-trip fidelity
#
# Usage:
#   ./e2e-full.sh [markdown_file]
#
# Environment variables:
#   SKIP_DB_SETUP=1    - Skip database setup (assumes DB is running)
#   KEEP_DB_RUNNING=1  - Don't stop DB after tests
#   SERVER_PORT=3000   - Server port (default: 3000)

set -e  # Exit on error
set -u  # Exit on undefined variable

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SERVER_PORT=${SERVER_PORT:-3000}
SERVER_URL="http://localhost:$SERVER_PORT"
DB_URL="postgres://postgres:postgres@localhost:5432/bloggen"
SKIP_DB_SETUP=${SKIP_DB_SETUP:-0}
KEEP_DB_RUNNING=${KEEP_DB_RUNNING:-0}

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Temp directory for test files
TEST_DIR=$(mktemp -d)
SERVER_PID=""

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"

    # Kill server if running
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "Sending SIGTERM to server (PID $SERVER_PID)..."
        kill -TERM "$SERVER_PID" 2>/dev/null || true

        # Wait up to 5 seconds for graceful shutdown
        for i in {1..5}; do
            if ! kill -0 "$SERVER_PID" 2>/dev/null; then
                echo "Server stopped gracefully"
                break
            fi
            sleep 1
        done

        # Force kill if still running
        if kill -0 "$SERVER_PID" 2>/dev/null; then
            echo "Server didn't stop gracefully, sending SIGKILL..."
            kill -9 "$SERVER_PID" 2>/dev/null || true
            sleep 1
        fi

        # Wait for port to be released
        # Give process a moment to fully exit and release the socket (typical TIME_WAIT < 1s)
        sleep 1

        # Verify port is released (use ss instead of lsof for better compatibility)
        for i in {1..10}; do
            if ss -tlnp 2>/dev/null | grep -q ":3000 "; then
                if [ $i -eq 10 ]; then
                    echo "Warning: Port 3000 still in use after 5 seconds, force killing..."
                    fuser -k 3000/tcp 2>/dev/null || pkill -9 -f v2-server || true
                    sleep 2
                fi
                sleep 0.5
            else
                # Port released
                break
            fi
        done
    fi

    # Stop database unless KEEP_DB_RUNNING is set
    if [ "$KEEP_DB_RUNNING" != "1" ] && [ "$SKIP_DB_SETUP" != "1" ]; then
        echo "Stopping database..."
        if [ -d "$REPO_ROOT/v2-server" ]; then
            (cd "$REPO_ROOT/v2-server" && docker compose -f docker-compose.dev.yml down -v >/dev/null 2>&1)
        fi
    fi

    # Clean temp directory
    rm -rf "$TEST_DIR"

    echo -e "${GREEN}Cleanup complete${NC}"
}

trap cleanup EXIT INT TERM

# Helper functions
log_section() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

log_step() {
    echo -e "${YELLOW}▸${NC} $1"
}

pass() {
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "  ${GREEN}✓${NC} $1"
}

fail() {
    TESTS_RUN=$((TESTS_RUN + 1))
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "  ${RED}✗${NC} $1"
    if [ $# -gt 1 ]; then
        echo -e "    ${RED}Error: $2${NC}"
    fi
}

# Check if we're in the bloggen root directory
if [ ! -d "client/bgc" ] || [ ! -d "v2-server" ]; then
    echo -e "${RED}Error: Must run from bloggen repository root${NC}"
    echo "Expected directory structure:"
    echo "  ./client/bgc/     - BlogGen client"
    echo "  ./v2-server/      - BlogGen v2 server"
    exit 1
fi

# Save repository root
REPO_ROOT="$(pwd)"

# ============================================================================
# Phase 1: Build Projects
# ============================================================================

log_section "Phase 1: Building Projects"

log_step "Building client (bgc)..."
if (cd "$REPO_ROOT/client/bgc" && cargo build --quiet 2>&1); then
    pass "Client built successfully"
else
    fail "Client build failed"
    exit 1
fi
BGC_BIN="$REPO_ROOT/client/bgc/target/debug/bgc"

log_step "Building server (v2-server)..."
if (cd "$REPO_ROOT/v2-server" && DATABASE_URL="$DB_URL" cargo build --quiet 2>&1); then
    pass "Server built successfully"
else
    fail "Server build failed"
    exit 1
fi
SERVER_BIN="$REPO_ROOT/v2-server/target/debug/v2-server"

# ============================================================================
# Phase 2: Setup Database
# ============================================================================

log_section "Phase 2: Database Setup"

if [ "$SKIP_DB_SETUP" = "1" ]; then
    log_step "Skipping database setup (SKIP_DB_SETUP=1)"
    pass "Using existing database"
else
    log_step "Ensuring clean database state..."
    # Stop and remove any existing database
    (cd "$REPO_ROOT/v2-server" && docker compose -f docker-compose.dev.yml down -v) >/dev/null 2>&1 || true

    log_step "Starting PostgreSQL database..."
    (cd "$REPO_ROOT/v2-server" && docker compose -f docker-compose.dev.yml up -d) >/dev/null 2>&1

    # Wait for database to be ready with proper health check
    log_step "Waiting for database to be ready..."
    for i in {1..30}; do
        if psql "$DB_URL" -c "SELECT 1" >/dev/null 2>&1; then
            break
        fi
        sleep 1
        if [ $i -eq 30 ]; then
            fail "Database failed to become ready"
            exit 1
        fi
    done
    sleep 1  # Extra buffer
    pass "Database is ready"

    log_step "Running migrations..."
    (cd "$REPO_ROOT/v2-server" && DATABASE_URL="$DB_URL" cargo sqlx migrate run) >/dev/null 2>&1
    if [ $? -eq 0 ]; then
        pass "Migrations applied"
    else
        fail "Migration failed"
        (cd "$REPO_ROOT/v2-server" && DATABASE_URL="$DB_URL" cargo sqlx migrate run 2>&1)
        exit 1
    fi
fi

# ============================================================================
# Phase 3: Start Server
# ============================================================================

log_section "Phase 3: Starting Server"

# Ensure no server is running on port 3000
log_step "Checking if port $SERVER_PORT is available..."
for i in {1..20}; do
    if ss -tlnp 2>/dev/null | grep -q ":$SERVER_PORT "; then
        if [ $i -eq 20 ]; then
            fail "Port $SERVER_PORT still in use after 20 attempts"
            exit 1
        fi
        sleep 0.5
    else
        pass "Port $SERVER_PORT is available"
        break
    fi
done

log_step "Starting v2-server on port $SERVER_PORT..."
(cd "$REPO_ROOT/v2-server" && DATABASE_URL="$DB_URL" "$SERVER_BIN" > "$TEST_DIR/server.log" 2>&1) &
SERVER_PID=$!

# Wait for server to be ready
sleep 2
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    fail "Server failed to start"
    echo "Server log:"
    cat "$TEST_DIR/server.log"
    exit 1
fi

# Test server health endpoint
if curl -s "$SERVER_URL/health" | grep -q -i "ok"; then
    pass "Server is running and healthy"
else
    fail "Server health check failed"
    exit 1
fi

# ============================================================================
# Phase 4: Prepare Test Markdown
# ============================================================================

log_section "Phase 4: Preparing Test Data"

# Use provided file or create test markdown
if [ $# -gt 0 ] && [ -f "$1" ]; then
    TEST_MD="$1"
    log_step "Using provided markdown file: $TEST_MD"
    pass "Test file loaded"
else
    TEST_MD="$TEST_DIR/test-post.md"
    log_step "Creating test markdown file..."
    cat > "$TEST_MD" <<'EOF'
# Test Blog Post

This is a test post for the end-to-end test suite.

## Features Tested

- Content-addressable storage with Blake3 hashing
- Server upload and download
- Delta updates
- Round-trip fidelity

## Code Example

```rust
fn main() {
    println!("Hello, BlogGen!");
}
```

## List

1. First item
2. Second item
3. Third item

### Nested Section

This is **bold** and this is *italic*.

[Link to example](https://example.com)
EOF
    pass "Test markdown created"
fi

# ============================================================================
# Phase 5: Client-Only Tests
# ============================================================================

log_section "Phase 5: Client Parsing & Serialization"

log_step "Parsing markdown to CAS..."
if "$BGC_BIN" parse "$TEST_MD" --output "$TEST_DIR/parsed.json" --pretty --stats 2>&1 | grep -q "✓ Parsed successfully"; then
    pass "Markdown parsed to CAS"
else
    fail "Parsing failed"
fi

log_step "Testing JSON round-trip..."
if "$BGC_BIN" render "$TEST_DIR/parsed.json" --output "$TEST_DIR/rendered-json.md" 2>&1; then
    pass "JSON deserialized and rendered"
else
    fail "JSON rendering failed"
fi

log_step "Testing MessagePack round-trip..."
"$BGC_BIN" parse "$TEST_MD" --output "$TEST_DIR/parsed.msgpack" --msgpack
if "$BGC_BIN" render "$TEST_DIR/parsed.msgpack" --msgpack --output "$TEST_DIR/rendered-msgpack.md" 2>&1; then
    pass "MessagePack round-trip successful"
else
    fail "MessagePack round-trip failed"
fi

# ============================================================================
# Phase 6: Server Upload
# ============================================================================

log_section "Phase 6: Uploading to Server"

TEST_SLUG="e2e-test-post-$RANDOM"

log_step "Uploading post '$TEST_SLUG' to server..."
if "$BGC_BIN" upload "$TEST_MD" --slug "$TEST_SLUG" --title "E2E Test Post" --server "$SERVER_URL" --stats 2>&1 | grep -q "✓ Upload successful"; then
    pass "Post uploaded to server"
else
    fail "Upload failed"
fi

# ============================================================================
# Phase 7: Server Download
# ============================================================================

log_section "Phase 7: Downloading from Server"

log_step "Downloading post from server..."
DOWNLOAD_OUTPUT_P7=$("$BGC_BIN" download "$TEST_SLUG" --server "$SERVER_URL" --output "$TEST_DIR/downloaded.md" --stats 2>&1)
if echo "$DOWNLOAD_OUTPUT_P7" | grep -q "✓ Download successful" && [ -f "$TEST_DIR/downloaded.md" ]; then
    pass "Post downloaded from server"
else
    fail "Download failed"
    echo "Download output:"
    echo "$DOWNLOAD_OUTPUT_P7"
fi

log_step "Verifying downloaded content matches original..."
# Normalize function
normalize_md() {
    if [ ! -f "$1" ]; then
        echo "Error: File $1 does not exist" >&2
        return 1
    fi
    sed 's/[[:space:]]*$//' "$1" | grep -v '^$'
}

if ! normalize_md "$TEST_MD" > "$TEST_DIR/original-norm.md"; then
    fail "Failed to normalize original file"
    exit 1
fi

if ! normalize_md "$TEST_DIR/downloaded.md" > "$TEST_DIR/downloaded-norm.md"; then
    fail "Failed to normalize downloaded file (file may not exist)"
    exit 1
fi

if diff -q "$TEST_DIR/original-norm.md" "$TEST_DIR/downloaded-norm.md" >/dev/null 2>&1; then
    pass "Downloaded content matches original"
else
    fail "Downloaded content differs from original"
    diff -u "$TEST_DIR/original-norm.md" "$TEST_DIR/downloaded-norm.md" | head -20
fi

# ============================================================================
# Phase 8: Delta Update
# ============================================================================

log_section "Phase 8: Testing Delta Updates"

log_step "Creating modified version of post..."
TEST_MD_MODIFIED="$TEST_DIR/test-post-modified.md"
cat > "$TEST_MD_MODIFIED" <<'EOF'
# Test Blog Post (Updated)

This is an UPDATED test post for the end-to-end test suite.

## Features Tested

- Content-addressable storage with Blake3 hashing
- Server upload and download
- **Delta updates (NEW!)**
- Round-trip fidelity

## Code Example

```rust
fn main() {
    println!("Hello, BlogGen v2!");  // Updated!
}
```

## New Section

This section was added in the update!

## List

1. First item
2. Second item (modified)
3. Third item
4. Fourth item (NEW!)

### Nested Section

This is **bold** and this is *italic* and this is ~~strikethrough~~.

[Link to example](https://example.com)
EOF
pass "Modified markdown created"

log_step "Computing and uploading delta..."
if "$BGC_BIN" update "$TEST_MD" "$TEST_MD_MODIFIED" --slug "$TEST_SLUG" --server "$SERVER_URL" --stats 2>&1 | grep -q "✓ Update successful"; then
    pass "Delta update uploaded successfully"
else
    fail "Delta update failed"
fi

# ============================================================================
# Phase 9: Verify Delta Update
# ============================================================================

log_section "Phase 9: Verifying Delta Update"

log_step "Downloading updated post from server..."
DOWNLOAD_OUTPUT=$("$BGC_BIN" download "$TEST_SLUG" --server "$SERVER_URL" --output "$TEST_DIR/updated-download.md" --stats 2>&1)
if echo "$DOWNLOAD_OUTPUT" | grep -q "✓ Download successful" && [ -f "$TEST_DIR/updated-download.md" ]; then
    pass "Updated post downloaded"
else
    fail "Download of updated post failed"
    echo "Download output:"
    echo "$DOWNLOAD_OUTPUT"
fi

log_step "Verifying updated content..."
if ! normalize_md "$TEST_MD_MODIFIED" > "$TEST_DIR/modified-norm.md"; then
    fail "Failed to normalize modified file"
    exit 1
fi

if ! normalize_md "$TEST_DIR/updated-download.md" > "$TEST_DIR/updated-norm.md"; then
    fail "Failed to normalize updated download file (file may not exist)"
    exit 1
fi

if diff -q "$TEST_DIR/modified-norm.md" "$TEST_DIR/updated-norm.md" >/dev/null 2>&1; then
    pass "Updated content matches expected"
else
    fail "Updated content differs from expected"
    diff -u "$TEST_DIR/modified-norm.md" "$TEST_DIR/updated-norm.md" | head -20
fi

# ============================================================================
# Phase 10: Cleanup Test Post
# ============================================================================

log_section "Phase 10: Cleanup"

log_step "Deleting test post from server..."
DELETE_STATUS=$(curl -s -w "%{http_code}" -X DELETE "$SERVER_URL/posts/$TEST_SLUG" -o /dev/null)
if [ "$DELETE_STATUS" = "204" ] || [ "$DELETE_STATUS" = "404" ]; then
    pass "Test post deleted from server (status: $DELETE_STATUS)"
else
    fail "Delete request failed with status $DELETE_STATUS"
fi

# ============================================================================
# Final Report
# ============================================================================

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Test Results${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "Total tests: $TESTS_RUN"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    exit 1
else
    echo -e "${GREEN}Failed: 0${NC}"
    echo ""
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
fi
