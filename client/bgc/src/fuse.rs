//! FUSE filesystem implementation for BlogGen
//!
//! This module provides a FUSE filesystem that allows interacting with BlogGen posts
//! as if they were regular markdown files in a directory. Posts are fetched from the
//! server on read, cached locally, and pushed back using delta updates on write.
//!
//! ## Caching Strategy
//!
//! The filesystem uses a two-tier caching approach:
//!
//! - **Metadata cache** (unlimited): Stores post slugs, titles, and timestamps for all posts
//!   from the server. This enables fast directory listings (`ls`) without memory concerns.
//!
//! - **Content cache** (limited): Stores AST and rendered markdown only for recently accessed
//!   posts. Limited to 5 entries by default (configurable via `max_loaded_entries`).
//!   Uses LRU eviction when the limit is exceeded, but never evicts dirty (unsaved) content.
//!
//! This ensures memory usage scales with the number of actively edited files (~1-5) rather
//! than the total number of posts on the server (potentially hundreds).

use anyhow::{Context, Result};
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyWrite, Request,
};
use libc::{ENOENT, ENOSYS};
use log::{debug, error, info};
use nix::unistd::{Gid, Uid};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use crate::cas::{CasDocument, NodeStore};
use crate::convert::parse_markdown;
use crate::http::{Client, PostSummary};
use crate::render::MarkdownRenderer;

/// Inode number for the root directory
const ROOT_INODE: u64 = 1;

/// Starting inode for dynamically allocated files
const FIRST_FILE_INODE: u64 = 2;

/// Larger block size is better, surely
const BLOCK_SIZE: u32 = 4096;

/// Time-to-live for cached attributes (in seconds)
const ATTR_TTL: Duration = Duration::from_secs(60);

/// Time-to-live for post list refresh cache (in seconds)
/// Prevents redundant server calls when multiple FUSE operations happen in quick succession
const REFRESH_TTL: Duration = Duration::from_secs(3);

/// Default maximum number of posts to keep loaded with content (AST + markdown)
/// The metadata cache (slugs, titles, timestamps) is unlimited, but content is limited
/// to avoid excessive memory usage. Posts are evicted using LRU policy when this limit is exceeded.
const DEFAULT_MAX_LOADED_ENTRIES: usize = 5;

/// Check if a filename is a vim temporary file that should be ignored
fn is_vim_temp_file(name: &str) -> bool {
    // Swap files: .filename.swp, .filename.swo, .filename.swn, etc.
    if name.starts_with('.') && (name.contains(".sw") || name.ends_with(".un~")) {
        return true;
    }

    // Backup files: filename~, filename.md~
    if name.ends_with('~') {
        return true;
    }

    // Vim's numbered backup files (just numbers like "4913")
    if name.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    false
}

/// Cache entry for a post
///
/// The cache uses a two-tier strategy:
/// - **Metadata** (always present): `summary`, `dirty`, `exists_on_server`, timestamps
///   These are lightweight and kept for all posts to support directory listings.
/// - **Content** (optional): `ast`, `old_ast`, `markdown`
///   These are memory-intensive and only kept for recently accessed posts.
///   Limited by `max_loaded_entries` using LRU eviction.
///
/// An entry is considered "loaded" (has content) when `markdown.is_some()`.
#[derive(Debug, Clone)]
pub(crate) struct CacheEntry {
    /// Post metadata from server (lightweight, always present)
    summary: PostSummary,
    /// Original AST from server for delta computation (heavy, LRU-evicted)
    old_ast: Option<CasDocument>,
    /// Current AST - modified locally or fetched (heavy, LRU-evicted)
    ast: Option<CasDocument>,
    /// Cached markdown rendering (heavy, LRU-evicted)
    markdown: Option<String>,
    /// Whether the post has been modified locally (prevents eviction)
    dirty: bool,
    /// Whether the post exists on the server (vs being a local-only draft)
    exists_on_server: bool,
    /// Last access time (for LRU eviction)
    accessed_at: SystemTime,
    /// Last modification time
    modified_at: SystemTime,
}

impl CacheEntry {
    fn new(summary: PostSummary) -> Self {
        let now = SystemTime::now();
        Self {
            summary,
            old_ast: None,
            ast: None,
            markdown: None,
            dirty: false,
            exists_on_server: false,
            accessed_at: now,
            modified_at: now,
        }
    }

    fn mark_accessed(&mut self) {
        self.accessed_at = SystemTime::now();
    }

    fn mark_modified(&mut self) {
        let now = SystemTime::now();
        self.modified_at = now;
        self.accessed_at = now;
    }

    #[allow(dead_code)]
    fn is_loaded(&self) -> bool {
        self.markdown.is_some()
    }
}

/// FUSE filesystem for BlogGen
pub struct BlogGenFS {
    /// HTTP client for server communication
    client: Arc<Client>,
    /// Tokio runtime for async operations (kept alive)
    _runtime: Arc<tokio::runtime::Runtime>,
    /// Handle to the runtime for spawning tasks
    runtime_handle: tokio::runtime::Handle,
    /// Cache of posts: metadata (unlimited) + content (limited by max_loaded_entries)
    /// All posts have metadata entries, but only recently accessed posts have content loaded
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Inode mapping (inode -> slug)
    inodes: Arc<RwLock<HashMap<u64, String>>>,
    /// Reverse inode mapping (slug -> inode)
    slugs: Arc<RwLock<HashMap<String, u64>>>,
    /// Next available inode number
    next_inode: Arc<RwLock<u64>>,
    /// Maximum number of posts to keep loaded with content (AST + markdown)
    max_loaded_entries: usize,
    /// Last time the post list was refreshed from the server
    last_refresh: Arc<RwLock<Option<SystemTime>>>,
}

impl BlogGenFS {
    /// Create a new BlogGen FUSE filesystem
    pub fn new(server_url: String) -> Self {
        let client = Arc::new(Client::new(server_url));

        // Create a dedicated runtime for async operations in a background thread
        // This avoids "runtime within runtime" issues
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime"),
        );
        let runtime_handle = runtime.handle().clone();

        Self {
            client,
            _runtime: runtime,
            runtime_handle,
            cache: Arc::new(RwLock::new(HashMap::new())),
            inodes: Arc::new(RwLock::new(HashMap::new())),
            slugs: Arc::new(RwLock::new(HashMap::new())),
            next_inode: Arc::new(RwLock::new(FIRST_FILE_INODE)),
            max_loaded_entries: DEFAULT_MAX_LOADED_ENTRIES,
            last_refresh: Arc::new(RwLock::new(None)),
        }
    }

    /// Helper to run async code from sync context
    /// Uses a separate thread to avoid "runtime within runtime" issues
    fn force_block_on<F, T>(&self, future: F) -> Result<T>
    where
        F: std::future::Future<Output = anyhow::Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        self.runtime_handle.block_on(future)
    }

    /// Mount the filesystem at the given path
    pub fn mount(self, mount_point: &Path) -> Result<()> {
        info!("Mounting BlogGen filesystem at {:?}", mount_point);

        let options = vec![
            MountOption::FSName("bloggen".to_string()),
            MountOption::RW,
            // Note: AutoUnmount requires AllowOther which needs special permissions
            // Note: AllowOther requires user_allow_other in /etc/fuse.conf or root
            // Users will need to manually unmount with: fusermount -u <mount_point>
        ];

        fuser::mount2(self, mount_point, &options).context("Failed to mount FUSE filesystem")?;

        Ok(())
    }

    /// Allocate a new inode for a slug
    fn allocate_inode(&self, slug: String) -> u64 {
        let mut slugs = self.slugs.write().unwrap();

        // Check if already allocated
        if let Some(&inode) = slugs.get(&slug) {
            return inode;
        }

        // Allocate new inode
        let mut next = self.next_inode.write().unwrap();
        let inode = *next;
        *next += 1;

        // Store mappings
        slugs.insert(slug.clone(), inode);
        self.inodes.write().unwrap().insert(inode, slug);

        inode
    }

    /// Get the slug for an inode
    fn get_slug(&self, inode: u64) -> Option<String> {
        self.inodes.read().unwrap().get(&inode).cloned()
    }

    /// Evict content (AST + markdown) from least recently used loaded entries
    /// Metadata (slug, title, timestamps) is always preserved for directory listings
    /// This method MUST be called while NOT holding the cache write lock
    fn evict_lru_entries(&self) {
        let mut cache = self.cache.write().unwrap();

        // Count how many entries have content loaded
        let loaded_count = cache.iter().filter(|(_, entry)| entry.is_loaded()).count();

        if loaded_count <= self.max_loaded_entries {
            return;
        }

        let to_evict_count = loaded_count - self.max_loaded_entries;
        debug!(
            "Loaded entries {} exceeds limit {}, evicting content from {} entries",
            loaded_count, self.max_loaded_entries, to_evict_count
        );

        // Collect loaded entries sorted by access time (oldest first)
        // Filter out dirty entries - they cannot be evicted
        let mut eviction_candidates: Vec<(String, SystemTime)> = cache
            .iter()
            .filter(|(_, entry)| entry.is_loaded() && !entry.dirty)
            .map(|(slug, entry)| (slug.clone(), entry.accessed_at))
            .collect();

        eviction_candidates.sort_by_key(|(_, accessed_at)| *accessed_at);

        // Evict content from the oldest entries (preserve metadata)
        let to_evict: Vec<String> = eviction_candidates
            .into_iter()
            .take(to_evict_count)
            .map(|(slug, _)| slug)
            .collect();

        for slug in &to_evict {
            if let Some(entry) = cache.get_mut(slug) {
                entry.ast = None;
                entry.old_ast = None;
                entry.markdown = None;
                debug!("Evicted content for post: {} (metadata preserved)", slug);
            }
        }

        let loaded_after = cache.iter().filter(|(_, entry)| entry.is_loaded()).count();
        info!(
            "Evicted content from {} posts, loaded entries now: {}",
            to_evict.len(),
            loaded_after
        );
    }

    /// Refresh the post list from the server
    fn refresh_posts(&self) -> Result<()> {
        // Check if we refreshed recently to avoid redundant server calls
        {
            let last = self.last_refresh.read().unwrap();
            if let Some(last_time) = *last {
                let elapsed = SystemTime::now()
                    .duration_since(last_time)
                    .unwrap_or(Duration::from_secs(0));
                if elapsed < REFRESH_TTL {
                    debug!(
                        "Skipping refresh, last refresh was {:?} ago (TTL: {:?})",
                        elapsed, REFRESH_TTL
                    );
                    return Ok(());
                }
            }
        }

        debug!("Refreshing post list from server");

        let client = self.client.clone();
        let posts = self
            .force_block_on(async move { client.list_posts().await })
            .context("Failed to list posts from server")?;

        let mut cache = self.cache.write().unwrap();

        // Update cache with new posts
        for summary in posts.posts {
            let slug = &summary.slug;

            // If entry exists and is dirty, don't overwrite
            if let Some(entry) = cache.get(slug)
                && entry.dirty
            {
                debug!("Skipping update for dirty post: {}", slug);
                continue;
            }

            // Allocate inode for this post
            self.allocate_inode(slug.clone());

            // Update or insert cache entry
            let summary_clone = summary.clone();
            let entry = cache
                .entry(slug.clone())
                .or_insert_with(|| CacheEntry::new(summary.clone()));
            entry.summary = summary_clone;
            entry.exists_on_server = true;
        }

        info!("Refreshed {} posts from server", cache.len());
        drop(cache);

        // Update last refresh timestamp
        *self.last_refresh.write().unwrap() = Some(SystemTime::now());

        // Evict LRU entries if cache is over limit
        self.evict_lru_entries();

        Ok(())
    }

    /// Fetch a post from the server and cache it
    fn fetch_post(&self, slug: &str) -> Result<String> {
        debug!("Fetching post from server: {}", slug);

        // Download AST from server
        let client = self.client.clone();
        let slug_owned = slug.to_string();
        let response = self
            .force_block_on(async move { client.download_post(&slug_owned).await })
            .context("Failed to download post from server")?;

        // Convert to NodeStore
        let cas_doc = CasDocument {
            root_hash: response.root_hash,
            nodes: response.nodes,
        };

        let store = cas_doc.to_store();

        // Render to markdown
        let root_node = store
            .get(&response.root_hash)
            .context("Root node not found in store")?;
        let mut renderer = MarkdownRenderer::new(&store);
        let markdown = renderer
            .render(root_node)
            .context("Failed to render AST to markdown")?;

        // Update cache
        let mut cache = self.cache.write().unwrap();
        if let Some(entry) = cache.get_mut(slug) {
            // Store as both old and current AST since we just fetched from server
            entry.old_ast = Some(cas_doc.clone());
            entry.ast = Some(cas_doc);
            entry.markdown = Some(markdown.clone());
            entry.exists_on_server = true;
            entry.mark_accessed();
        }
        drop(cache);

        // Evict LRU entries if cache is over limit
        self.evict_lru_entries();

        Ok(markdown)
    }

    /// Get or fetch the markdown for a post
    fn get_markdown(&self, slug: &str) -> Result<String> {
        // Check cache first
        {
            let mut cache = self.cache.write().unwrap();
            let cache_size = cache.len();
            if let Some(entry) = cache.get_mut(slug) {
                if let Some(markdown) = &entry.markdown {
                    debug!("Cache hit for post: {} (cache size: {})", slug, cache_size);
                    let result = markdown.clone();
                    entry.mark_accessed();
                    return Ok(result);
                }
            }
        }

        // Cache miss - fetch from server
        info!("Cache miss for post: {}", slug);
        self.fetch_post(slug)
    }

    /// Write markdown for a post
    fn write_markdown(&self, slug: &str, markdown: String) -> Result<()> {
        debug!("Writing markdown for post: {}", slug);

        // Parse the new markdown
        let mut new_store = NodeStore::new();
        let new_root = parse_markdown(&markdown, &mut new_store)
            .map_err(|e| anyhow::anyhow!("Failed to parse markdown: {}", e))?;
        let new_doc =
            CasDocument::new(&new_store, new_root).context("Failed to create CasDocument")?;

        // Update cache with new content
        let mut cache = self.cache.write().unwrap();
        if let Some(entry) = cache.get_mut(slug) {
            // If old_ast is None (file was clean), preserve current ast for delta computation
            // This happens when a flushed file is edited again
            if entry.old_ast.is_none() && entry.ast.is_some() {
                debug!(
                    "Post '{}' was clean, saving current AST for delta computation",
                    slug
                );
                entry.old_ast = entry.ast.clone();
            }

            entry.markdown = Some(markdown);
            entry.ast = Some(new_doc);
            entry.dirty = true;
            entry.mark_modified();
        } else {
            // Entry doesn't exist - this shouldn't happen but handle it gracefully
            error!("Cache entry not found for slug '{}' during write", slug);
            return Err(anyhow::anyhow!("Cache entry not found for slug '{}'", slug));
        }

        Ok(())
    }

    /// Flush a dirty post to the server
    fn flush_post(&self, slug: &str) -> Result<()> {
        debug!("Flushing dirty post to server: {}", slug);

        // Clone all needed data while holding the lock, then drop it before network I/O
        let (new_doc, title, exists_on_server, old_doc) = {
            let cache = self.cache.read().unwrap();
            let entry = cache
                .get(slug)
                .context(format!("Post '{}' not found in cache during flush", slug))?;

            if !entry.dirty {
                debug!("Post '{}' is not dirty, skipping flush", slug);
                return Ok(());
            }

            debug!("Post '{}' is dirty, uploading to server...", slug);

            let new_doc = match entry.ast.as_ref() {
                Some(doc) => doc.clone(),
                None => {
                    // No AST yet - file was created but not written to
                    // This is normal for files created with touch or cat redirection
                    debug!("Post '{}' has no AST yet, skipping flush", slug);
                    return Ok(());
                }
            };

            let title = entry.summary.title.clone();
            let exists_on_server = entry.exists_on_server;
            let old_doc = entry.old_ast.clone();

            (new_doc, title, exists_on_server, old_doc)
        }; // Lock is dropped here

        let client = self.client.clone();
        let slug_owned = slug.to_string();

        // Check if this is a new post or an update
        if exists_on_server {
            // This is an update - use delta update
            debug!("Post '{}' exists on server, using delta update", slug);

            let old_doc = old_doc
                .ok_or_else(|| anyhow::anyhow!("Post exists on server but old_ast is None"))?;

            let old_root = old_doc.root_hash;
            let new_root = new_doc.root_hash;

            // Compute delta: nodes in new but not in old
            let mut added_nodes = HashMap::new();
            for (hash, node) in &new_doc.nodes {
                if !old_doc.nodes.contains_key(hash) {
                    added_nodes.insert(*hash, node.clone());
                }
            }

            // Compute removed hashes: nodes in old but not in new
            let mut removed_hashes = Vec::new();
            for hash in old_doc.nodes.keys() {
                if !new_doc.nodes.contains_key(hash) {
                    removed_hashes.push(*hash);
                }
            }

            debug!(
                "Delta: {} nodes added, {} nodes removed",
                added_nodes.len(),
                removed_hashes.len()
            );

            let response = self.force_block_on(async move {
                client
                    .update_post_delta(&slug_owned, old_root, new_root, added_nodes, removed_hashes)
                    .await
            })?;

            info!(
                "Post updated successfully: {} (delta: +{} nodes, -{} nodes)",
                response.slug, response.nodes_added, response.nodes_removed
            );
        } else {
            // This is a new post - use create
            debug!("Post '{}' is new, creating on server", slug);

            let root_hash = new_doc.root_hash;
            let nodes = new_doc.nodes.clone();

            let response = self.force_block_on(async move {
                client
                    .upload_post(&slug_owned, title.as_deref(), root_hash, nodes)
                    .await
            })?;

            info!("Post created successfully: {}", response.slug);

            // Mark post as existing on server
            let mut cache = self.cache.write().unwrap();
            if let Some(entry) = cache.get_mut(slug) {
                entry.exists_on_server = true;
            }
        }

        Ok(())
    }

    /// Get file attributes for a post
    fn get_file_attr(&self, inode: u64, slug: &str) -> FileAttr {
        let (size, mtime, atime) = {
            let cache = self.cache.read().unwrap();
            if let Some(entry) = cache.get(slug) {
                let size = if let Some(markdown) = &entry.markdown {
                    markdown.len() as u64
                } else {
                    // Estimate size if not loaded
                    4096
                };
                (size, entry.modified_at, entry.accessed_at)
            } else {
                let now = SystemTime::now();
                (4096, now, now)
            }
        }; // Lock is explicitly dropped here

        FileAttr {
            ino: inode,
            size,
            blocks: size.div_ceil(512),
            atime,
            mtime,
            ctime: mtime,
            crtime: mtime,
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: Uid::current().as_raw(),
            gid: Gid::current().as_raw(),
            rdev: 0,
            blksize: BLOCK_SIZE,
            flags: 0,
        }
    }
}

impl Drop for BlogGenFS {
    fn drop(&mut self) {
        // Explicitly shutdown the runtime to avoid panic on drop
        // We need to ensure this happens cleanly
        debug!("Cleaning up BlogGen filesystem");
    }
}

impl Filesystem for BlogGenFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        debug!("lookup(parent={}, name={:?})", parent, name);

        if parent != ROOT_INODE {
            reply.error(ENOENT);
            return;
        }

        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        // Filter out vim temporary files
        if is_vim_temp_file(name_str) {
            debug!("Ignoring vim temporary file: {}", name_str);
            reply.error(ENOENT);
            return;
        }

        // Remove .md extension if present
        let slug = name_str.strip_suffix(".md").unwrap_or(name_str);

        // Refresh posts to ensure we have latest
        if let Err(e) = self.refresh_posts() {
            error!("Failed to refresh posts: {}", e);
            reply.error(ENOSYS);
            return;
        }

        // Check if post exists
        let cache = self.cache.read().unwrap();
        if cache.contains_key(slug) {
            let inode = self.allocate_inode(slug.to_string());
            let attr = self.get_file_attr(inode, slug);
            reply.entry(&ATTR_TTL, &attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        debug!("getattr(ino={})", ino);

        if ino == ROOT_INODE {
            let attr = FileAttr {
                ino: ROOT_INODE,
                size: 0,
                blocks: 0,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: Uid::current().as_raw(),
                gid: Gid::current().as_raw(),
                rdev: 0,
                blksize: 512,
                flags: 0,
            };
            reply.attr(&ATTR_TTL, &attr);
            return;
        }

        // Get slug for inode
        let slug = match self.get_slug(ino) {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        let attr = self.get_file_attr(ino, &slug);
        reply.attr(&ATTR_TTL, &attr);
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        debug!("read(ino={}, offset={}, size={})", ino, offset, size);

        let slug = match self.get_slug(ino) {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        // Get or fetch markdown
        let markdown = match self.get_markdown(&slug) {
            Ok(md) => md,
            Err(e) => {
                error!("Failed to get markdown for {}: {}", slug, e);
                reply.error(ENOSYS);
                return;
            }
        };

        let bytes = markdown.as_bytes();
        let offset = offset as usize;

        if offset >= bytes.len() {
            reply.data(&[]);
            return;
        }

        let end = std::cmp::min(offset + size as usize, bytes.len());
        reply.data(&bytes[offset..end]);
    }

    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyWrite,
    ) {
        debug!("write(ino={}, offset={}, size={})", ino, offset, data.len());

        let slug = match self.get_slug(ino) {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        // Get current markdown content
        let mut current_content = {
            let cache = self.cache.read().unwrap();
            cache
                .get(&slug)
                .and_then(|entry| entry.markdown.clone())
                .unwrap_or_default()
        };

        // Convert new data to string
        let new_data = match std::str::from_utf8(data) {
            Ok(s) => s,
            Err(e) => {
                error!("Invalid UTF-8 in write data: {}", e);
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Handle the write based on offset
        let final_markdown = if offset == 0 {
            // Replace entire content
            new_data.to_string()
        } else {
            // Append or insert at offset
            let offset_usize = offset as usize;
            if offset_usize > current_content.len() {
                // Offset beyond current content - pad with zeros? Or error?
                // For now, just append
                current_content.push_str(new_data);
                current_content
            } else if offset_usize == current_content.len() {
                // Append at end
                current_content.push_str(new_data);
                current_content
            } else {
                // Insert in middle - replace from offset onwards
                current_content.truncate(offset_usize);
                current_content.push_str(new_data);
                current_content
            }
        };

        match self.write_markdown(&slug, final_markdown) {
            Ok(_) => reply.written(data.len() as u32),
            Err(e) => {
                error!("Failed to write markdown: {}", e);
                reply.error(ENOSYS);
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        debug!("readdir(ino={}, offset={})", ino, offset);

        if ino != ROOT_INODE {
            reply.error(ENOENT);
            return;
        }

        // Refresh posts from server
        if let Err(e) = self.refresh_posts() {
            error!("Failed to refresh posts: {}", e);
            reply.error(ENOSYS);
            return;
        }

        let mut entries = vec![
            (ROOT_INODE, FileType::Directory, ".".to_string()),
            (ROOT_INODE, FileType::Directory, "..".to_string()),
        ];

        // Add all posts as .md files
        let cache = self.cache.read().unwrap();
        for (slug, _entry) in cache.iter() {
            let inode = self.allocate_inode(slug.clone());
            let filename = format!("{}.md", slug);
            entries.push((inode, FileType::RegularFile, filename));
        }

        // Return entries starting from offset
        for (i, (inode, kind, name)) in entries.iter().enumerate().skip(offset as usize) {
            if reply.add(*inode, (i + 1) as i64, *kind, name) {
                break;
            }
        }

        reply.ok();
    }

    fn flush(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: fuser::ReplyEmpty,
    ) {
        debug!("flush(ino={})", ino);

        let slug = match self.get_slug(ino) {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        match self.flush_post(&slug) {
            Ok(_) => {
                // Clear dirty flag and drop old_ast to save memory
                let mut cache = self.cache.write().unwrap();
                if let Some(entry) = cache.get_mut(&slug) {
                    entry.dirty = false;
                    // Drop old_ast to save ~500KB-1MB per post
                    // We can restore it from current ast on next edit
                    entry.old_ast = None;
                    debug!(
                        "Post '{}' flushed and cleaned, dropped old_ast to save memory",
                        slug
                    );
                }
                reply.ok();
            }
            Err(e) => {
                error!("Failed to flush post: {}", e);
                reply.error(ENOSYS);
            }
        }
    }

    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        debug!("setattr(ino={}, size={:?})", ino, size);

        // We don't support most setattr operations, but we need to handle size truncation
        // for file writes to work properly

        if mode.is_some() || uid.is_some() || gid.is_some() {
            // Don't support chmod, chown, etc
            reply.error(ENOSYS);
            return;
        }

        if let Some(new_size) = size {
            if new_size != 0 {
                // Only support truncate to 0 (file clear before write)
                reply.error(ENOSYS);
                return;
            }

            // Handle truncate to 0 - clear the file content
            if let Some(slug) = self.get_slug(ino) {
                let mut cache = self.cache.write().unwrap();
                if let Some(entry) = cache.get_mut(&slug) {
                    // If old_ast is None (post was flushed) but post exists on server,
                    // preserve current AST for delta computation before clearing
                    if entry.old_ast.is_none() && entry.ast.is_some() && entry.exists_on_server {
                        debug!("Post '{}' being truncated, preserving AST for delta", slug);
                        entry.old_ast = entry.ast.clone();
                    }
                    entry.markdown = Some(String::new());
                    entry.ast = None;
                    entry.dirty = true;
                    entry.mark_modified();
                }
            }
        }

        // Return current attributes
        if ino == ROOT_INODE {
            let attr = FileAttr {
                ino: ROOT_INODE,
                size: 0,
                blocks: 0,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: Uid::current().as_raw(),
                gid: Gid::current().as_raw(),
                rdev: 0,
                blksize: 512,
                flags: 0,
            };
            reply.attr(&ATTR_TTL, &attr);
        } else if let Some(slug) = self.get_slug(ino) {
            let attr = self.get_file_attr(ino, &slug);
            reply.attr(&ATTR_TTL, &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        debug!("create(parent={}, name={:?})", parent, name);

        if parent != ROOT_INODE {
            reply.error(ENOSYS);
            return;
        }

        // Extract slug from filename (remove .md extension)
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Filter out vim temporary files
        if is_vim_temp_file(name_str) {
            debug!("Blocking creation of vim temporary file: {}", name_str);
            reply.error(libc::EACCES);
            return;
        }

        let slug = name_str.strip_suffix(".md").unwrap_or(name_str);

        // Allocate inode for new file
        let ino = self.allocate_inode(slug.to_string());

        // Create empty cache entry for the new post
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let timestamp = format!("{}", now);

        let summary = PostSummary {
            slug: slug.to_string(),
            title: None,
            created_at: timestamp.clone(),
            updated_at: timestamp,
            published: false,
        };

        let mut cache = self.cache.write().unwrap();
        let mut entry = CacheEntry::new(summary);
        entry.markdown = Some(String::new()); // Empty initial content
        entry.dirty = true; // Mark as dirty since it's new
        entry.exists_on_server = false; // New post, doesn't exist on server yet
        cache.insert(slug.to_string(), entry);
        drop(cache);

        // Return file attributes
        let attr = self.get_file_attr(ino, slug);
        reply.created(&Duration::from_secs(1), &attr, 0, 0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test CacheEntry creation and modification
    #[test]
    fn test_cache_entry_new() {
        let summary = PostSummary {
            slug: "test-post".to_string(),
            title: Some("Test Post".to_string()),
            created_at: "2026-01-19T00:00:00Z".to_string(),
            updated_at: "2026-01-19T00:00:00Z".to_string(),
            published: true,
        };

        let entry = CacheEntry::new(summary.clone());

        assert_eq!(entry.summary.slug, "test-post");
        assert_eq!(entry.summary.title, Some("Test Post".to_string()));
        assert!(entry.ast.is_none());
        assert!(entry.markdown.is_none());
        assert!(!entry.dirty);
    }

    /// Test marking cache entry as accessed
    #[test]
    fn test_cache_entry_mark_accessed() {
        let summary = PostSummary {
            slug: "test-post".to_string(),
            title: None,
            created_at: "2026-01-19T00:00:00Z".to_string(),
            updated_at: "2026-01-19T00:00:00Z".to_string(),
            published: false,
        };

        let mut entry = CacheEntry::new(summary);
        let first_access = entry.accessed_at;

        // Wait a tiny bit
        std::thread::sleep(std::time::Duration::from_millis(10));

        entry.mark_accessed();
        assert!(entry.accessed_at > first_access);
    }

    /// Test is_loaded method
    #[test]
    fn test_cache_entry_is_loaded() {
        let summary = PostSummary {
            slug: "test-post".to_string(),
            title: None,
            created_at: "2026-01-19T00:00:00Z".to_string(),
            updated_at: "2026-01-19T00:00:00Z".to_string(),
            published: false,
        };

        let mut entry = CacheEntry::new(summary);
        assert!(!entry.is_loaded());

        entry.markdown = Some("# Test".to_string());
        assert!(entry.is_loaded());
    }

    /// Test inode allocation
    #[test]
    fn test_inode_allocation() {
        let fs = BlogGenFS::new("http://localhost:3000".to_string());

        // Allocate first inode
        let inode1 = fs.allocate_inode("post-1".to_string());
        assert_eq!(inode1, FIRST_FILE_INODE);

        // Allocate second inode
        let inode2 = fs.allocate_inode("post-2".to_string());
        assert_eq!(inode2, FIRST_FILE_INODE + 1);

        // Re-allocating same slug returns same inode
        let inode1_again = fs.allocate_inode("post-1".to_string());
        assert_eq!(inode1_again, inode1);
    }

    /// Test slug retrieval by inode
    #[test]
    fn test_get_slug() {
        let fs = BlogGenFS::new("http://localhost:3000".to_string());

        // Allocate inode
        let inode = fs.allocate_inode("my-post".to_string());

        // Retrieve slug
        let slug = fs.get_slug(inode);
        assert_eq!(slug, Some("my-post".to_string()));

        // Non-existent inode
        let slug = fs.get_slug(9999);
        assert_eq!(slug, None);
    }

    /// Test markdown parsing and caching
    #[test]
    fn test_write_markdown() {
        let fs = BlogGenFS::new("http://localhost:3000".to_string());

        // Create a cache entry
        let summary = PostSummary {
            slug: "test-post".to_string(),
            title: Some("Test".to_string()),
            created_at: "2026-01-19T00:00:00Z".to_string(),
            updated_at: "2026-01-19T00:00:00Z".to_string(),
            published: false,
        };

        {
            let mut cache = fs.cache.write().unwrap();
            cache.insert("test-post".to_string(), CacheEntry::new(summary));
        }

        // Write markdown
        let markdown = "# Hello World\n\nThis is a test.".to_string();
        let result = fs.write_markdown("test-post", markdown.clone());
        assert!(result.is_ok());

        // Check cache
        let cache = fs.cache.read().unwrap();
        let entry = cache.get("test-post").unwrap();
        assert_eq!(entry.markdown, Some(markdown));
        assert!(entry.ast.is_some());
        assert!(entry.dirty);
    }

    /// Test file attributes generation
    #[test]
    fn test_get_file_attr() {
        let fs = BlogGenFS::new("http://localhost:3000".to_string());

        // Create cache entry with markdown
        let summary = PostSummary {
            slug: "test-post".to_string(),
            title: Some("Test".to_string()),
            created_at: "2026-01-19T00:00:00Z".to_string(),
            updated_at: "2026-01-19T00:00:00Z".to_string(),
            published: false,
        };

        let mut entry = CacheEntry::new(summary);
        entry.markdown = Some("# Hello\n\nWorld".to_string());

        {
            let mut cache = fs.cache.write().unwrap();
            cache.insert("test-post".to_string(), entry);
        }

        let inode = fs.allocate_inode("test-post".to_string());
        let attr = fs.get_file_attr(inode, "test-post");

        assert_eq!(attr.ino, inode);
        assert_eq!(attr.kind, FileType::RegularFile);
        assert_eq!(attr.perm, 0o644);
        assert_eq!(attr.size, 14); // Length of "# Hello\n\nWorld"
    }

    /// Test constant values
    #[test]
    fn test_constants() {
        assert_eq!(ROOT_INODE, 1);
        assert_eq!(FIRST_FILE_INODE, 2);
        assert!(ATTR_TTL.as_secs() > 0);
    }

    /// Test vim temporary file detection
    #[test]
    fn test_is_vim_temp_file() {
        // Swap files
        assert!(is_vim_temp_file(".test.md.swp"));
        assert!(is_vim_temp_file(".test.md.swo"));
        assert!(is_vim_temp_file(".test.md.swn"));

        // Undo files
        assert!(is_vim_temp_file(".test.md.un~"));

        // Backup files
        assert!(is_vim_temp_file("test.md~"));
        assert!(is_vim_temp_file("test~"));

        // Numbered files
        assert!(is_vim_temp_file("4913"));
        assert!(is_vim_temp_file("12345"));

        // Normal files (should not be detected as temp files)
        assert!(!is_vim_temp_file("test.md"));
        assert!(!is_vim_temp_file("my-post.md"));
        assert!(!is_vim_temp_file("post-123.md"));
        assert!(!is_vim_temp_file("README"));
    }

    /// Test marking cache entry as modified
    #[test]
    fn test_cache_entry_mark_modified() {
        let summary = PostSummary {
            slug: "test-post".to_string(),
            title: None,
            created_at: "2026-01-19T00:00:00Z".to_string(),
            updated_at: "2026-01-19T00:00:00Z".to_string(),
            published: false,
        };

        let mut entry = CacheEntry::new(summary);
        let first_modified = entry.modified_at;
        let first_access = entry.accessed_at;

        // Wait a tiny bit
        std::thread::sleep(std::time::Duration::from_millis(10));

        entry.mark_modified();
        assert!(entry.modified_at > first_modified);
        assert!(entry.accessed_at > first_access);
    }

    /// Test LRU eviction when loaded entries exceed limit
    #[test]
    fn test_lru_eviction() {
        let mut fs = BlogGenFS::new("http://localhost:3000".to_string());
        fs.max_loaded_entries = 3; // Small limit for testing

        // Add 4 posts with content loaded
        let mut cache = fs.cache.write().unwrap();
        for i in 0..4 {
            let summary = PostSummary {
                slug: format!("post-{}", i),
                title: Some(format!("Post {}", i)),
                created_at: "2026-01-19T00:00:00Z".to_string(),
                updated_at: "2026-01-19T00:00:00Z".to_string(),
                published: false,
            };
            let mut entry = CacheEntry::new(summary);
            entry.markdown = Some(format!("Content {}", i));
            entry.ast = Some(CasDocument {
                root_hash: crate::ast::Blake3Hash::new([0u8; 32]),
                nodes: HashMap::new(),
            });

            // Make entries have different access times
            entry.accessed_at = SystemTime::now()
                - std::time::Duration::from_secs((4 - i) as u64 * 10);

            cache.insert(format!("post-{}", i), entry);
        }
        drop(cache);

        // Trigger eviction
        fs.evict_lru_entries();

        let cache = fs.cache.read().unwrap();

        // All metadata entries should still exist
        assert_eq!(cache.len(), 4);
        assert!(cache.contains_key("post-0"));
        assert!(cache.contains_key("post-1"));
        assert!(cache.contains_key("post-2"));
        assert!(cache.contains_key("post-3"));

        // post-0 content should be evicted (oldest accessed_at)
        assert!(!cache.get("post-0").unwrap().is_loaded());
        assert!(cache.get("post-0").unwrap().markdown.is_none());

        // Others should still have content loaded
        assert!(cache.get("post-1").unwrap().is_loaded());
        assert!(cache.get("post-2").unwrap().is_loaded());
        assert!(cache.get("post-3").unwrap().is_loaded());
    }

    /// Test that dirty entries' content is not evicted
    #[test]
    fn test_lru_preserves_dirty_entries() {
        let mut fs = BlogGenFS::new("http://localhost:3000".to_string());
        fs.max_loaded_entries = 2; // Very small limit

        let mut cache = fs.cache.write().unwrap();

        // Add 3 loaded entries, make the oldest one dirty
        for i in 0..3 {
            let summary = PostSummary {
                slug: format!("post-{}", i),
                title: Some(format!("Post {}", i)),
                created_at: "2026-01-19T00:00:00Z".to_string(),
                updated_at: "2026-01-19T00:00:00Z".to_string(),
                published: false,
            };
            let mut entry = CacheEntry::new(summary);
            entry.markdown = Some(format!("Content {}", i));
            entry.ast = Some(CasDocument {
                root_hash: crate::ast::Blake3Hash::new([0u8; 32]),
                nodes: HashMap::new(),
            });
            entry.accessed_at = SystemTime::now()
                - std::time::Duration::from_secs((3 - i) as u64 * 10);

            // Make post-0 dirty
            if i == 0 {
                entry.dirty = true;
            }

            cache.insert(format!("post-{}", i), entry);
        }
        drop(cache);

        // Trigger eviction
        fs.evict_lru_entries();

        let cache = fs.cache.read().unwrap();

        // All metadata should be preserved
        assert!(cache.contains_key("post-0"));
        assert!(cache.contains_key("post-1"));
        assert!(cache.contains_key("post-2"));

        // post-0 content should NOT be evicted even though it's oldest (because it's dirty)
        assert!(cache.get("post-0").unwrap().is_loaded());
        assert!(cache.get("post-0").unwrap().markdown.is_some());

        // post-1 content should be evicted instead (next oldest non-dirty)
        assert!(!cache.get("post-1").unwrap().is_loaded());
        assert!(cache.get("post-1").unwrap().markdown.is_none());

        // post-2 should remain loaded (newest)
        assert!(cache.get("post-2").unwrap().is_loaded());
    }

    /// Test that eviction doesn't happen when loaded entries are under limit
    #[test]
    fn test_no_eviction_when_under_limit() {
        let mut fs = BlogGenFS::new("http://localhost:3000".to_string());
        fs.max_loaded_entries = 10;

        let mut cache = fs.cache.write().unwrap();
        for i in 0..5 {
            let summary = PostSummary {
                slug: format!("post-{}", i),
                title: None,
                created_at: "2026-01-19T00:00:00Z".to_string(),
                updated_at: "2026-01-19T00:00:00Z".to_string(),
                published: false,
            };
            let mut entry = CacheEntry::new(summary);
            entry.markdown = Some(format!("Content {}", i));
            cache.insert(format!("post-{}", i), entry);
        }
        drop(cache);

        // Trigger eviction (should do nothing)
        fs.evict_lru_entries();

        // All entries and their content should remain
        let cache = fs.cache.read().unwrap();
        assert_eq!(cache.len(), 5);
        for i in 0..5 {
            assert!(cache.get(&format!("post-{}", i)).unwrap().is_loaded());
        }
    }

    /// Test access time updates on cache hit
    #[test]
    fn test_access_time_updates() {
        let fs = BlogGenFS::new("http://localhost:3000".to_string());

        let summary = PostSummary {
            slug: "test-post".to_string(),
            title: None,
            created_at: "2026-01-19T00:00:00Z".to_string(),
            updated_at: "2026-01-19T00:00:00Z".to_string(),
            published: false,
        };

        let mut entry = CacheEntry::new(summary);
        entry.markdown = Some("# Test".to_string());
        let first_access = entry.accessed_at;

        {
            let mut cache = fs.cache.write().unwrap();
            cache.insert("test-post".to_string(), entry);
        }

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Access the post (cache hit)
        let _ = fs.get_markdown("test-post");

        // Access time should have been updated
        let cache = fs.cache.read().unwrap();
        let entry = cache.get("test-post").unwrap();
        assert!(entry.accessed_at > first_access);
    }

    /// Test that metadata-only entries don't count toward loaded limit
    #[test]
    fn test_metadata_only_entries_unlimited() {
        let mut fs = BlogGenFS::new("http://localhost:3000".to_string());
        fs.max_loaded_entries = 3;

        let mut cache = fs.cache.write().unwrap();

        // Add 100 metadata-only entries (no content)
        for i in 0..100 {
            let summary = PostSummary {
                slug: format!("post-{}", i),
                title: Some(format!("Post {}", i)),
                created_at: "2026-01-19T00:00:00Z".to_string(),
                updated_at: "2026-01-19T00:00:00Z".to_string(),
                published: false,
            };
            // No markdown or AST - metadata only
            cache.insert(format!("post-{}", i), CacheEntry::new(summary));
        }

        // Add 3 entries with content loaded
        for i in 100..103 {
            let summary = PostSummary {
                slug: format!("post-{}", i),
                title: Some(format!("Post {}", i)),
                created_at: "2026-01-19T00:00:00Z".to_string(),
                updated_at: "2026-01-19T00:00:00Z".to_string(),
                published: false,
            };
            let mut entry = CacheEntry::new(summary);
            entry.markdown = Some(format!("Content {}", i));
            cache.insert(format!("post-{}", i), entry);
        }
        drop(cache);

        // Trigger eviction
        fs.evict_lru_entries();

        // No eviction should occur - we have 100 metadata entries + 3 loaded
        // but only the 3 loaded count toward the limit
        let cache = fs.cache.read().unwrap();
        assert_eq!(cache.len(), 103); // All entries still present

        let loaded_count = cache.iter().filter(|(_, e)| e.is_loaded()).count();
        assert_eq!(loaded_count, 3); // Still 3 loaded entries
    }
}
