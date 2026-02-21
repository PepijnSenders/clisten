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
