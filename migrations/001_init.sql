-- migrations/001_init.sql

CREATE TABLE IF NOT EXISTS queue (
    position  INTEGER PRIMARY KEY,
    item_json TEXT NOT NULL,
    url       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS queue_state (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
