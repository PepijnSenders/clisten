// SQLite persistence for favorites and listening history.
// Data lives in ~/.local/share/clisten/clisten.db.

use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::api::models::DiscoveryItem;

#[derive(Debug, Clone)]
#[allow(dead_code)] // fields read in integration tests
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

#[derive(Debug, Clone)]
#[allow(dead_code)] // fields read in integration tests
pub struct HistoryRecord {
    pub id: i64,
    pub key: String,
    pub source: String,
    pub title: String,
    pub url: Option<String>,
    pub played_at: String,
    pub duration_secs: Option<u64>,
}

fn map_favorite_row(row: &rusqlite::Row) -> rusqlite::Result<FavoriteRecord> {
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
}

/// SQLite-backed store for favorites and listening history.
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

    // ── Favorites ──

    pub fn add_favorite(&self, item: &DiscoveryItem) -> anyhow::Result<()> {
        let key = item.favorite_key();
        let (source, item_type) = match item {
            DiscoveryItem::NtsLiveChannel { .. } => ("nts", "live"),
            DiscoveryItem::NtsEpisode { .. } => ("nts", "episode"),
            DiscoveryItem::DirectUrl { .. } => ("direct", "url"),
            DiscoveryItem::NtsGenre { .. } => return Ok(()), // genres aren't favoritable
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
        self.conn
            .execute("DELETE FROM favorites WHERE key = ?1", params![key])?;
        Ok(())
    }

    #[allow(dead_code)] // used by integration tests
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
        match source {
            Some(s) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, key, source, item_type, title, url, metadata_json, created_at
                     FROM favorites WHERE source = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
                )?;
                let rows = stmt.query_map(params![s, limit, offset], map_favorite_row)?;
                rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, key, source, item_type, title, url, metadata_json, created_at
                     FROM favorites ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
                )?;
                let rows = stmt.query_map(params![limit, offset], map_favorite_row)?;
                rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
            }
        }
    }

    // ── History ──

    pub fn add_to_history(&self, item: &DiscoveryItem) -> anyhow::Result<()> {
        let key = item.favorite_key();
        let source = match item {
            DiscoveryItem::NtsLiveChannel { .. } => "nts",
            DiscoveryItem::NtsEpisode { .. } => "nts",
            DiscoveryItem::DirectUrl { .. } => "direct",
            DiscoveryItem::NtsGenre { .. } => return Ok(()), // genres aren't historied
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
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    #[allow(dead_code)] // used by integration tests
    pub fn clear_history(&self) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM history", [])?;
        Ok(())
    }
}

/// Reconstruct a DiscoveryItem from stored record fields.
fn record_to_discovery_item(
    source: &str,
    item_type: &str,
    key: &str,
    title: &str,
    url: &Option<String>,
) -> DiscoveryItem {
    match (source, item_type) {
        ("nts", "live") => DiscoveryItem::NtsLiveChannel {
            channel: 1,
            show_name: title.to_string(),
            genres: vec![],
        },
        ("direct", _) => DiscoveryItem::DirectUrl {
            url: url.clone().unwrap_or_default(),
            title: Some(title.to_string()),
        },
        _ => DiscoveryItem::NtsEpisode {
            name: title.to_string(),
            show_alias: String::new(),
            episode_alias: key.to_string(),
            genres: vec![],
            location: None,
            audio_url: url.clone(),
        },
    }
}

impl FavoriteRecord {
    pub fn to_discovery_item(&self) -> DiscoveryItem {
        record_to_discovery_item(
            &self.source,
            &self.item_type,
            &self.key,
            &self.title,
            &self.url,
        )
    }
}

impl HistoryRecord {
    pub fn to_discovery_item(&self) -> DiscoveryItem {
        // History records don't store item_type — infer from source.
        let item_type = match self.source.as_str() {
            "direct" => "url",
            _ => "episode",
        };
        record_to_discovery_item(&self.source, item_type, &self.key, &self.title, &self.url)
    }
}
