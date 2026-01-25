use crate::error::Result;
use async_trait::async_trait;

/// Trait for garbage collection operations
///
/// Garbage collection identifies and removes orphaned AST nodes (those with ref_count = 0)
/// that are no longer referenced by any posts. A grace period is enforced to prevent
/// accidental deletion of nodes that might be part of an in-progress transaction.
#[async_trait]
pub trait GarbageCollector: Send + Sync {
    /// Run garbage collection with the specified grace period
    ///
    /// Deletes nodes that have:
    /// - ref_count = 0 (no longer referenced)
    /// - created_at older than grace_period_days ago
    ///
    /// Returns the number of nodes deleted.
    ///
    /// # Safety
    /// The grace period prevents deletion of nodes that might be part of an
    /// in-progress write operation. For example, if a client is uploading nodes
    /// for a new post but hasn't yet created the post record, those nodes would
    /// temporarily have ref_count = 0. The grace period ensures we don't delete
    /// them before the transaction completes.
    ///
    /// Recommended grace period: 30 days (very conservative)
    async fn garbage_collect(&self, grace_period_days: i32) -> Result<u64>;

    /// Count orphaned nodes that would be deleted by garbage collection
    ///
    /// Useful for monitoring and metrics. Returns the count without actually
    /// deleting anything.
    async fn count_orphaned(&self, grace_period_days: i32) -> Result<u64>;
}

/// PostgreSQL implementation of GarbageCollector
pub struct PostgresGarbageCollector {
    pool: sqlx::PgPool,
}

impl PostgresGarbageCollector {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GarbageCollector for PostgresGarbageCollector {
    async fn garbage_collect(&self, grace_period_days: i32) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM ast_nodes
            WHERE ref_count = 0
              AND created_at < NOW() - INTERVAL '1 day' * $1
            "#,
        )
        .bind(grace_period_days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn count_orphaned(&self, grace_period_days: i32) -> Result<u64> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) as count
            FROM ast_nodes
            WHERE ref_count = 0
              AND created_at < NOW() - INTERVAL '1 day' * $1
            "#,
        )
        .bind(grace_period_days)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0 as u64)
    }
}

/// Background task that runs garbage collection on a schedule
///
/// This spawns a tokio task that runs GC periodically. In a production system,
/// this would typically run once per day during off-peak hours.
///
/// # Example
/// ```no_run
/// # use server::storage::gc::{start_gc_background_task, PostgresGarbageCollector};
/// # use sqlx::PgPool;
/// # async fn example(pool: PgPool) {
/// let gc = PostgresGarbageCollector::new(pool);
/// let task_handle = start_gc_background_task(
///     gc,
///     30,                                    // grace period: 30 days
///     std::time::Duration::from_secs(86400), // run every 24 hours
/// );
/// # }
/// ```
pub fn start_gc_background_task(
    gc: impl GarbageCollector + 'static,
    grace_period_days: i32,
    interval: std::time::Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval_timer = tokio::time::interval(interval);

        loop {
            interval_timer.tick().await;

            match gc.garbage_collect(grace_period_days).await {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!(
                            "Garbage collection completed: {} orphaned nodes deleted",
                            count
                        );
                    } else {
                        tracing::debug!("Garbage collection completed: no orphaned nodes found");
                    }
                }
                Err(e) => {
                    tracing::error!("Garbage collection failed: {}", e);
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gc_background_task_starts() {
        // Mock GC that does nothing
        struct MockGC;

        #[async_trait]
        impl GarbageCollector for MockGC {
            async fn garbage_collect(&self, _grace_period_days: i32) -> Result<u64> {
                Ok(0)
            }

            async fn count_orphaned(&self, _grace_period_days: i32) -> Result<u64> {
                Ok(0)
            }
        }

        let handle = start_gc_background_task(MockGC, 30, std::time::Duration::from_millis(100));

        // Let it run for a bit
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Abort the task
        handle.abort();

        // Verify it was running
        assert!(handle.await.unwrap_err().is_cancelled());
    }
}
