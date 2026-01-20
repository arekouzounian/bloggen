# FUSE readdir Performance Investigation

**Date:** 2026-01-19
**Issue:** Multiple `readdir` calls and excessive server requests during directory listing

## Problem Statement

When mounting the BlogGen FUSE filesystem and running `ls` in the mounted directory, we observe at least 3 `readdir` FUSE operations being triggered, each resulting in a separate HTTP request to the server's `/posts` endpoint. This creates unnecessary network overhead and server load.

## Root Cause Analysis

### Current Implementation Behavior

The FUSE filesystem implementation in `client/bgc/src/fuse.rs` has the following behavior:

1. **readdir implementation** (lines 576-619):
   - Called by the kernel for each directory read operation
   - **Calls `refresh_posts()` on EVERY invocation** (line 592)
   - Returns all directory entries from the cache

2. **lookup implementation** (lines 384-419):
   - Called when resolving file names to inodes
   - **Also calls `refresh_posts()` on EVERY invocation** (line 404)
   - Used by `stat`, `ls -l`, and other operations that need file metadata

3. **refresh_posts function** (lines 176-208):
   - Makes HTTP GET request to server's `/posts` endpoint
   - Updates cache with post summaries
   - **No caching mechanism** - requests fresh data every time

### Why Multiple readdir Calls Occur

The Linux FUSE/VFS layer may invoke `readdir` multiple times for a single `ls` command due to:

1. **Offset-based iteration**: While our implementation returns all entries at once, the kernel may still call `readdir` with different offsets to ensure all entries are retrieved
2. **Attribute lookups**: After getting the directory entries, `ls` (especially `ls -l`) performs `lookup` or `getattr` on each file to get detailed metadata
3. **Directory entry validation**: The kernel may re-read the directory to verify consistency
4. **Multiple consumers**: Shell tab-completion, file managers, or concurrent accesses can trigger separate `readdir` operations

### Performance Impact

For a directory with N posts and M `readdir` calls per `ls`:
- **Network requests**: M × 1 (list all posts) per operation
- **Bandwidth**: M × (response size) bytes transferred
- **Server load**: M × database queries for post listing
- **Latency**: Each `readdir` blocks waiting for HTTP response

Example scenario:
- 10 posts on server
- User runs `ls`: 3 `readdir` calls + 10 `lookup` calls = 13 HTTP requests to `/posts`
- Each response ~1KB = 13KB transferred for a simple directory listing

## Proposed Solutions

### Solution 1: Time-based Cache (Recommended)

Implement a time-to-live (TTL) cache for the post list:

```rust
struct PostListCache {
    posts: Vec<PostSummary>,
    last_refresh: SystemTime,
    ttl: Duration,
}
```

- Default TTL: 5-10 seconds (configurable)
- `refresh_posts()` only fetches if cache is stale
- Balances freshness with performance
- **Pros**: Simple, effective, predictable behavior
- **Cons**: May show stale data for TTL duration

### Solution 2: Lazy Refresh with Manual Invalidation

Only refresh the post list when:
- First mount/access
- Explicit user action (e.g., sending SIGHUP signal)
- Cache is empty

Add a CLI command or signal handler for cache invalidation.

- **Pros**: Minimal network usage, user controls freshness
- **Cons**: May show stale data, requires user awareness

### Solution 3: Event-based Invalidation

Use server-side events (SSE/WebSockets) to notify client when post list changes:

- Server pushes updates when posts are added/modified/deleted
- Client invalidates cache only when necessary
- **Pros**: Perfect freshness, minimal overhead
- **Cons**: Complex implementation, requires server changes

## Recommendation

**Implement Solution 1 (Time-based Cache)** with the following parameters:

- Default TTL: 5 seconds
- Make TTL configurable via CLI flag or environment variable
- Add debug logging to track cache hits/misses

This provides a good balance of:
- Performance (reduces 13 requests → 1-2 requests per `ls`)
- Freshness (5s is acceptable for most blog workflows)
- Simplicity (minimal code changes, no server modifications)
- User control (configurable TTL)

## Implementation Notes

Changes needed in `src/fuse.rs`:

1. Add cache metadata to `BlogGenFS` struct:
   ```rust
   post_list_refreshed_at: Arc<RwLock<Option<SystemTime>>>,
   post_list_ttl: Duration,
   ```

2. Modify `refresh_posts()` to check cache age:
   ```rust
   fn refresh_posts(&self) -> Result<()> {
       let now = SystemTime::now();
       let should_refresh = {
           let last_refresh = self.post_list_refreshed_at.read().unwrap();
           match *last_refresh {
               None => true,
               Some(t) => now.duration_since(t).unwrap_or(self.post_list_ttl) >= self.post_list_ttl
           }
       };

       if !should_refresh {
           debug!("Post list cache still fresh, skipping refresh");
           return Ok(());
       }

       // ... existing fetch logic ...

       *self.post_list_refreshed_at.write().unwrap() = Some(now);
       Ok(())
   }
   ```

3. Add CLI parameter for TTL configuration

## Metrics to Track

After implementing the fix:
- Cache hit rate
- Number of `/posts` requests per `ls` operation
- Time to complete `ls` operation
- User-perceived latency

## Related Issues

- `lookup` function also calls `refresh_posts()` - should use same cache
- Consider caching individual post content as well (already partially implemented in `CacheEntry`)
- May want to implement cache eviction for large post counts
