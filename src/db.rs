// SQLite persistence for queue state.
// Data lives in ~/.local/share/clisten/clisten.db.

use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::api::models::DiscoveryItem;
use crate::player::queue::QueueItem;

/// SQLite-backed store for queue persistence.
/// Data is persisted at `~/.local/share/clisten/clisten.db`.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the SQLite database.
    pub fn open() -> anyhow::Result<Self> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clisten");
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("clisten.db");
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    #[allow(dead_code)] // used by integration tests
    pub fn open_at(path: &std::path::Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> anyhow::Result<()> {
        let sql = include_str!("../migrations/001_init.sql");
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    // ── Queue persistence ──

    pub fn save_queue(&self, items: &[QueueItem], current_index: Option<usize>) -> anyhow::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM queue", [])?;
        tx.execute("DELETE FROM queue_state", [])?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO queue (position, item_json, url) VALUES (?1, ?2, ?3)",
            )?;
            for (i, qi) in items.iter().enumerate() {
                let json = serde_json::to_string(&qi.item)?;
                stmt.execute(params![i as i64, json, qi.url])?;
            }
        }

        if let Some(idx) = current_index {
            tx.execute(
                "INSERT INTO queue_state (key, value) VALUES ('current_index', ?1)",
                params![idx.to_string()],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn load_queue(&self) -> anyhow::Result<(Vec<QueueItem>, Option<usize>)> {
        let mut stmt = self.conn.prepare(
            "SELECT item_json, url FROM queue ORDER BY position ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            let url: String = row.get(1)?;
            Ok((json, url))
        })?;

        let mut items = Vec::new();
        for row in rows {
            let (json, url) = row?;
            let item: DiscoveryItem = serde_json::from_str(&json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            items.push(QueueItem {
                item,
                url,
                stream_metadata: None,
            });
        }

        let current_index: Option<usize> = self
            .conn
            .query_row(
                "SELECT value FROM queue_state WHERE key = 'current_index'",
                [],
                |row| {
                    let val: String = row.get(0)?;
                    Ok(val.parse::<usize>().ok())
                },
            )
            .unwrap_or(None);

        Ok((items, current_index))
    }
}
