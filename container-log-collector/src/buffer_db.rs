use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct BufferStats {
    pub pending_count: i64,
    pub processing_count: i64,
    pub failed_count: i64,
    pub oldest_pending_timestamp: Option<i64>,
    pub total_retries: i64,
}

pub struct BufferDb {
    conn: Arc<Mutex<Connection>>,
}

impl BufferDb {
    pub async fn new(db_path: &str) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = Path::new(db_path).parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create database directory")?;
        }

        let conn = Connection::open(db_path)
            .context("Failed to open SQLite database")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        // Initialize database schema
        db.init_schema().await?;
        
        log::info!("SQLite buffer database initialized at: {}", db_path);
        Ok(db)
    }

    async fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS log_buffer (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                raw_syslog TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                retry_count INTEGER DEFAULT 0,
                status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'failed')),
                last_attempt INTEGER
            )
            "#,
            [],
        )?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_status_created ON log_buffer(status, created_at)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_retry_lookup ON log_buffer(status, retry_count, last_attempt)",
            [],
        )?;

        log::debug!("Database schema initialized successfully");
        Ok(())
    }

    pub async fn store_log(&self, raw_syslog: &str) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;

        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO log_buffer (raw_syslog, created_at) VALUES (?1, ?2)",
            params![raw_syslog, timestamp],
        )?;

        log::trace!("Stored log entry in buffer database");
        Ok(())
    }

    pub async fn get_pending_logs(&self, limit: i32) -> Result<Vec<(i64, String)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, raw_syslog FROM log_buffer WHERE status = 'pending' ORDER BY created_at LIMIT ?1"
        )?;

        let logs = stmt.query_map(params![limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, rusqlite::Error>>()?;

        log::debug!("Retrieved {} pending logs from buffer", logs.len());
        Ok(logs)
    }

    pub async fn mark_processing(&self, id: i64) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;

        let conn = self.conn.lock().await;
        let updated = conn.execute(
            "UPDATE log_buffer SET status = 'processing', last_attempt = ?1 WHERE id = ?2",
            params![timestamp, id],
        )?;

        if updated == 0 {
            anyhow::bail!("No log entry found with id {}", id);
        }

        log::trace!("Marked log {} as processing", id);
        Ok(())
    }

    pub async fn mark_sent(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().await;
        let deleted = conn.execute(
            "DELETE FROM log_buffer WHERE id = ?1",
            params![id],
        )?;

        if deleted == 0 {
            log::warn!("Attempted to delete non-existent log entry with id {}", id);
        } else {
            log::trace!("Successfully sent and removed log {}", id);
        }

        Ok(())
    }

    pub async fn mark_failed(&self, id: i64) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;

        let conn = self.conn.lock().await;
        let updated = conn.execute(
            "UPDATE log_buffer SET status = 'failed', retry_count = retry_count + 1, last_attempt = ?1 WHERE id = ?2",
            params![timestamp, id],
        )?;

        if updated == 0 {
            log::warn!("Attempted to mark non-existent log entry {} as failed", id);
        } else {
            log::debug!("Marked log {} as failed", id);
        }

        Ok(())
    }

    pub async fn get_retry_candidates(&self, max_retries: i32, retry_delay_secs: i64) -> Result<Vec<(i64, String)>> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;
        
        let retry_after = current_time - retry_delay_secs;

        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, raw_syslog FROM log_buffer 
            WHERE status = 'failed' 
            AND retry_count < ?1 
            AND (last_attempt IS NULL OR last_attempt < ?2)
            ORDER BY created_at
            LIMIT 50
            "#
        )?;

        let logs = stmt.query_map(params![max_retries, retry_after], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, rusqlite::Error>>()?;

        if !logs.is_empty() {
            log::debug!("Found {} logs eligible for retry", logs.len());
        }

        Ok(logs)
    }

    pub async fn get_stats(&self) -> Result<BufferStats> {
        let conn = self.conn.lock().await;

        let pending_count = conn.query_row(
            "SELECT COUNT(*) FROM log_buffer WHERE status = 'pending'",
            [],
            |row| row.get::<_, i64>(0),
        )?;

        let processing_count = conn.query_row(
            "SELECT COUNT(*) FROM log_buffer WHERE status = 'processing'",
            [],
            |row| row.get::<_, i64>(0),
        )?;

        let failed_count = conn.query_row(
            "SELECT COUNT(*) FROM log_buffer WHERE status = 'failed'",
            [],
            |row| row.get::<_, i64>(0),
        )?;

        let oldest_pending_timestamp = conn.query_row(
            "SELECT MIN(created_at) FROM log_buffer WHERE status = 'pending'",
            [],
            |row| row.get::<_, Option<i64>>(0),
        ).optional()?.flatten();

        let total_retries = conn.query_row(
            "SELECT SUM(retry_count) FROM log_buffer",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )?.unwrap_or(0);

        Ok(BufferStats {
            pending_count,
            processing_count,
            failed_count,
            oldest_pending_timestamp,
            total_retries,
        })
    }

    pub async fn cleanup_old_failed(&self, max_age_hours: i64) -> Result<i64> {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64 - (max_age_hours * 3600);

        let conn = self.conn.lock().await;
        let deleted = conn.execute(
            "DELETE FROM log_buffer WHERE status = 'failed' AND created_at < ?1",
            params![cutoff_time],
        )?;

        if deleted > 0 {
            log::info!("Cleaned up {} old failed log entries", deleted);
        }

        Ok(deleted as i64)
    }

    pub async fn reset_processing_to_pending(&self) -> Result<i64> {
        // Reset any logs stuck in 'processing' state back to 'pending'
        // This handles cases where the service was restarted mid-processing
        let conn = self.conn.lock().await;
        let updated = conn.execute(
            "UPDATE log_buffer SET status = 'pending' WHERE status = 'processing'",
            [],
        )?;

        if updated > 0 {
            log::info!("Reset {} processing logs back to pending state", updated);
        }

        Ok(updated as i64)
    }
}