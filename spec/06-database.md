## 10. Database Schema

### 10.1 SQL Migration

```sql
-- migrations/001_init.sql

CREATE TABLE IF NOT EXISTS favorites (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    key         TEXT NOT NULL UNIQUE,     -- DiscoveryItem::favorite_key()
    source      TEXT NOT NULL,            -- "nts" or "soundcloud"
    item_type   TEXT NOT NULL,            -- "live", "episode", "mixtape", "show", "track"
    title       TEXT NOT NULL,
    url         TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS history (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    key           TEXT NOT NULL,           -- DiscoveryItem::favorite_key()
    source        TEXT NOT NULL,
    title         TEXT NOT NULL,
    url           TEXT,
    played_at     TEXT NOT NULL DEFAULT (datetime('now')),
    duration_secs INTEGER
);

CREATE INDEX IF NOT EXISTS idx_favorites_key ON favorites(key);
CREATE INDEX IF NOT EXISTS idx_favorites_source ON favorites(source);
CREATE INDEX IF NOT EXISTS idx_history_played_at ON history(played_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_key ON history(key);
```

### 10.2 Database Wrapper

```rust
// src/db.rs

use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::api::models::DiscoveryItem;

pub struct FavoriteRecord {
    pub id: i64,
    pub key: String,
    pub source: String,
    pub item_type: String,
    pub title: String,
    pub url: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
}

pub struct HistoryRecord {
    pub id: i64,
    pub key: String,
    pub source: String,
    pub title: String,
    pub url: Option<String>,
    pub played_at: String,
    pub duration_secs: Option<u64>,
}

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

    fn run_migrations(&self) -> anyhow::Result<()> {
        let sql = include_str!("../migrations/001_init.sql");
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    // ── Favorites ──

    pub fn add_favorite(&self, item: &DiscoveryItem) -> anyhow::Result<()> {
        let key = item.favorite_key();
        let (source, item_type) = match item {
            DiscoveryItem::NtsLiveChannel { .. } => ("nts", "live"),
            DiscoveryItem::NtsEpisode { .. } => ("nts", "episode"),
            DiscoveryItem::NtsMixtape { .. } => ("nts", "mixtape"),
            DiscoveryItem::NtsShow { .. } => ("nts", "show"),
            DiscoveryItem::SoundCloudTrack { .. } => ("soundcloud", "track"),
        };
        let title = item.title().to_string();
        let url = item.playback_url();
        let metadata = serde_json::to_string(&item.subtitle())?;

        self.conn.execute(
            "INSERT OR IGNORE INTO favorites (key, source, item_type, title, url, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![key, source, item_type, title, url, metadata],
        )?;
        Ok(())
    }

    pub fn remove_favorite(&self, key: &str) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM favorites WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn is_favorite(&self, key: &str) -> anyhow::Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM favorites WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn list_favorites(
        &self,
        source: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> anyhow::Result<Vec<FavoriteRecord>> {
        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match source {
            Some(s) => (
                "SELECT id, key, source, item_type, title, url, metadata_json, created_at
                 FROM favorites WHERE source = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
                    .to_string(),
                vec![Box::new(s.to_string()), Box::new(limit), Box::new(offset)],
            ),
            None => (
                "SELECT id, key, source, item_type, title, url, metadata_json, created_at
                 FROM favorites ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
                    .to_string(),
                vec![Box::new(limit), Box::new(offset)],
            ),
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(FavoriteRecord {
                id: row.get(0)?,
                key: row.get(1)?,
                source: row.get(2)?,
                item_type: row.get(3)?,
                title: row.get(4)?,
                url: row.get(5)?,
                metadata_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;

        let mut results = vec![];
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    // ── History ──

    pub fn add_to_history(&self, item: &DiscoveryItem) -> anyhow::Result<()> {
        let key = item.favorite_key();
        let (source, _) = match item {
            DiscoveryItem::NtsLiveChannel { .. } => ("nts", "live"),
            DiscoveryItem::NtsEpisode { .. } => ("nts", "episode"),
            DiscoveryItem::NtsMixtape { .. } => ("nts", "mixtape"),
            DiscoveryItem::NtsShow { .. } => ("nts", "show"),
            DiscoveryItem::SoundCloudTrack { .. } => ("soundcloud", "track"),
        };
        let title = item.title().to_string();
        let url = item.playback_url();

        self.conn.execute(
            "INSERT INTO history (key, source, title, url) VALUES (?1, ?2, ?3, ?4)",
            params![key, source, title, url],
        )?;
        Ok(())
    }

    pub fn list_history(&self, limit: u32, offset: u32) -> anyhow::Result<Vec<HistoryRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, key, source, title, url, played_at, duration_secs
             FROM history ORDER BY played_at DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit, offset], |row| {
            Ok(HistoryRecord {
                id: row.get(0)?,
                key: row.get(1)?,
                source: row.get(2)?,
                title: row.get(3)?,
                url: row.get(4)?,
                played_at: row.get(5)?,
                duration_secs: row.get(6)?,
            })
        })?;

        let mut results = vec![];
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn clear_history(&self) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM history", [])?;
        Ok(())
    }
}
```

---

