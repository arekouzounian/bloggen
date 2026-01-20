#!/usr/bin/env bash
#
# Debug script to manually test FUSE flush behavior
#

set -e

# Setup
REPO_ROOT="/home/arek/code/bloggen"
BGC_BIN="$REPO_ROOT/client/bgc/target/debug/bgc"
MOUNT_POINT="/tmp/fuse-debug-mount"
SERVER_URL="http://localhost:3000"
TEST_SLUG="debug-test-$$"

# Cleanup function
cleanup() {
    echo "Cleaning up..."
    fusermount -u "$MOUNT_POINT" 2>/dev/null || true
    rm -rf "$MOUNT_POINT"
}
trap cleanup EXIT

# Create mount point
rm -rf "$MOUNT_POINT"
mkdir -p "$MOUNT_POINT"

echo "=== Starting FUSE with DEBUG logging ==="
RUST_LOG=debug "$BGC_BIN" mount "$MOUNT_POINT" --server "$SERVER_URL" > /tmp/fuse-debug.log 2>&1 &
FUSE_PID=$!
sleep 2

echo "FUSE PID: $FUSE_PID"
echo ""

echo "=== Creating and writing file ==="
TEST_FILE="$MOUNT_POINT/$TEST_SLUG.md"
echo "# Test Post" > "$TEST_FILE"
echo ""

echo "=== Checking FUSE log for callbacks ==="
echo "Create calls:"
grep -c "create(" /tmp/fuse-debug.log || echo "0"
echo "Write calls:"
grep -c "write(" /tmp/fuse-debug.log || echo "0"
echo "Flush calls:"
grep -c "flush(" /tmp/fuse-debug.log || echo "0"
echo ""

echo "=== Reading file back ==="
cat "$TEST_FILE"
echo ""

echo "=== Checking server for post ==="
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$SERVER_URL/posts/$TEST_SLUG")
echo "HTTP response: $HTTP_CODE"
if [ "$HTTP_CODE" = "200" ]; then
    echo "✓ Post found on server!"
else
    echo "✗ Post NOT found on server"
fi
echo ""

echo "=== Full FUSE log ==="
cat /tmp/fuse-debug.log
echo ""

echo "Press Enter to unmount and exit..."
read
