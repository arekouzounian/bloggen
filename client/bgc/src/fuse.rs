//! FUSE filesystem implementation for BlogGen
//!
//! This module provides a FUSE filesystem that allows interacting with BlogGen posts
//! as if they were regular markdown files in a directory. Posts are fetched from the
//! server on read, cached locally, and pushed back using delta updates on write.

use anyhow::{Context, Result};
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyWrite, Request,
};
use libc::{ENOENT, ENOSYS};
use log::{debug, error, info};
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

/// Time-to-live for cached attributes (in seconds)
const ATTR_TTL: Duration = Duration::from_secs(60);

/// Cache entry for a post
#[derive(Debug, Clone)]
pub(crate) struct CacheEntry {
    /// Post metadata from server
    summary: PostSummary,
    /// Cached AST (if fetched)
    ast: Option<CasDocument>,
    /// Cached markdown rendering
    markdown: Option<String>,
    /// Whether the post has been modified locally
    dirty: bool,
    /// Last access time
    accessed_at: SystemTime,
}

impl CacheEntry {
    fn new(summary: PostSummary) -> Self {
        Self {
            summary,
            ast: None,
            markdown: None,
            dirty: false,
            accessed_at: SystemTime::now(),
        }
    }

    fn mark_accessed(&mut self) {
        self.accessed_at = SystemTime::now();
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
    /// Cache of posts (slug -> cache entry)
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Inode mapping (inode -> slug)
    inodes: Arc<RwLock<HashMap<u64, String>>>,
    /// Reverse inode mapping (slug -> inode)
    slugs: Arc<RwLock<HashMap<String, u64>>>,
    /// Next available inode number
    next_inode: Arc<RwLock<u64>>,
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
        }
    }

    /// Helper to run async code from sync context
    /// Uses a separate thread to avoid "runtime within runtime" issues
    fn run_async<F, T>(&self, future: F) -> Result<T>
    where
        F: std::future::Future<Output = anyhow::Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        let handle = self.runtime_handle.clone();
        let result = std::thread::spawn(move || {
            handle.block_on(future)
        })
        .join()
        .map_err(|e| anyhow::anyhow!("Thread panicked: {:?}", e))??;
        Ok(result)
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

        fuser::mount2(self, mount_point, &options)
            .context("Failed to mount FUSE filesystem")?;

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

    /// Refresh the post list from the server
    fn refresh_posts(&self) -> Result<()> {
        debug!("Refreshing post list from server");

        let client = self.client.clone();
        let posts = self
            .run_async(async move { client.list_posts().await })
            .context("Failed to list posts from server")?;

        let mut cache = self.cache.write().unwrap();

        // Update cache with new posts
        for summary in posts.posts {
            let slug = summary.slug.clone();

            // If entry exists and is dirty, don't overwrite
            if let Some(entry) = cache.get(&slug) {
                if entry.dirty {
                    debug!("Skipping update for dirty post: {}", slug);
                    continue;
                }
            }

            // Allocate inode for this post
            self.allocate_inode(slug.clone());

            // Update or insert cache entry
            let summary_clone = summary.clone();
            cache.entry(slug).or_insert_with(|| CacheEntry::new(summary.clone())).summary = summary_clone;
        }

        info!("Refreshed {} posts from server", cache.len());
        Ok(())
    }

    /// Fetch a post from the server and cache it
    fn fetch_post(&self, slug: &str) -> Result<String> {
        debug!("Fetching post from server: {}", slug);

        // Download AST from server
        let client = self.client.clone();
        let slug_owned = slug.to_string();
        let response = self
            .run_async(async move { client.download_post(&slug_owned).await })
            .context("Failed to download post from server")?;

        // Convert to NodeStore
        let cas_doc = CasDocument {
            root_hash: response.root_hash,
            nodes: response.nodes,
        };

        let store = cas_doc.to_store();

        // Render to markdown
        let root_node = store.get(&response.root_hash)
            .context("Root node not found in store")?;
        let mut renderer = MarkdownRenderer::new(&store);
        let markdown = renderer.render(root_node)
            .context("Failed to render AST to markdown")?;

        // Update cache
        let mut cache = self.cache.write().unwrap();
        if let Some(entry) = cache.get_mut(slug) {
            entry.ast = Some(cas_doc);
            entry.markdown = Some(markdown.clone());
            entry.mark_accessed();
        }

        Ok(markdown)
    }

    /// Get or fetch the markdown for a post
    fn get_markdown(&self, slug: &str) -> Result<String> {
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(entry) = cache.get(slug) {
                if let Some(markdown) = &entry.markdown {
                    debug!("Cache hit for post: {}", slug);
                    return Ok(markdown.clone());
                }
            }
        }

        // Cache miss - fetch from server
        debug!("Cache miss for post: {}", slug);
        self.fetch_post(slug)
    }

    /// Write markdown for a post
    fn write_markdown(&self, slug: &str, markdown: String) -> Result<()> {
        debug!("Writing markdown for post: {}", slug);

        // Parse the new markdown
        let mut new_store = NodeStore::new();
        let new_root = parse_markdown(&markdown, &mut new_store)
            .map_err(|e| anyhow::anyhow!("Failed to parse markdown: {}", e))?;
        let new_doc = CasDocument::new(&new_store, new_root)
            .context("Failed to create CasDocument")?;

        // Update cache with new content
        let mut cache = self.cache.write().unwrap();
        if let Some(entry) = cache.get_mut(slug) {
            entry.markdown = Some(markdown);
            entry.ast = Some(new_doc);
            entry.dirty = true;
            entry.mark_accessed();
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

        let cache = self.cache.read().unwrap();
        let entry = cache.get(slug).context(format!("Post '{}' not found in cache during flush", slug))?;

        if !entry.dirty {
            debug!("Post '{}' is not dirty, skipping flush", slug);
            return Ok(());
        }

        debug!("Post '{}' is dirty, uploading to server...", slug);

        let new_doc = match entry.ast.as_ref() {
            Some(doc) => doc,
            None => {
                // No AST yet - file was created but not written to
                // This is normal for files created with touch or cat redirection
                debug!("Post '{}' has no AST yet, skipping flush", slug);
                return Ok(());
            }
        };

        // Check if this is a new post or an update
        // For now, we'll assume it's an update and use delta
        // TODO: Handle new post creation properly

        // If we have the old AST, compute delta
        // For simplicity, let's just upload the full AST for now
        // TODO: Implement proper delta update

        let client = self.client.clone();
        let slug_owned = slug.to_string();
        let title = entry.summary.title.clone();
        let root_hash = new_doc.root_hash;
        let nodes = new_doc.nodes.clone();

        let response = self.run_async(async move {
            client
                .upload_post(&slug_owned, title.as_deref(), root_hash, nodes)
                .await
        })?;

        info!("Post uploaded successfully: {}", response.slug);

        Ok(())
    }

    /// Get file attributes for a post
    fn get_file_attr(&self, inode: u64, slug: &str) -> FileAttr {
        let cache = self.cache.read().unwrap();
        let size = if let Some(entry) = cache.get(slug) {
            if let Some(markdown) = &entry.markdown {
                markdown.len() as u64
            } else {
                // Estimate size if not loaded
                4096
            }
        } else {
            4096
        };

        FileAttr {
            ino: inode,
            size,
            blocks: (size + 511) / 512,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            blksize: 512,
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
                uid: unsafe { libc::getuid() },
                gid: unsafe { libc::getgid() },
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
                .unwrap_or_else(String::new)
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

    fn flush(&mut self, _req: &Request, ino: u64, _fh: u64, _lock_owner: u64, reply: fuser::ReplyEmpty) {
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
                // Clear dirty flag
                let mut cache = self.cache.write().unwrap();
                if let Some(entry) = cache.get_mut(&slug) {
                    entry.dirty = false;
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
                    entry.markdown = Some(String::new());
                    entry.ast = None;
                    entry.dirty = true;
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
                uid: 1000,
                gid: 1000,
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

        let slug = if name_str.ends_with(".md") {
            &name_str[..name_str.len() - 3]
        } else {
            name_str
        };

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
        cache.insert(
            slug.to_string(),
            CacheEntry {
                summary,
                ast: None,
                markdown: Some(String::new()), // Empty initial content
                dirty: true, // Mark as dirty since it's new
                accessed_at: SystemTime::now(),
            },
        );
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
}
