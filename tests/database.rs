// SQLite database: favorites, history, record-to-DiscoveryItem conversion.

use clisten::api::models::DiscoveryItem;
use clisten::db::Database;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_episode(name: &str, alias: &str) -> DiscoveryItem {
    DiscoveryItem::NtsEpisode {
        name: name.to_string(),
        show_alias: "test-show".to_string(),
        episode_alias: alias.to_string(),
        genres: vec!["Ambient".to_string()],
        location: Some("London".to_string()),
        audio_url: Some(format!("https://soundcloud.com/ntslive/{}", alias)),
    }
}

fn open_temp_db() -> (Database, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = Database::open_at(&dir.path().join("test.db")).expect("open db");
    (db, dir) // caller keeps _dir alive so the directory isn't deleted mid-test
}

// ── SQLite operations ────────────────────────────────────────────────────────

#[test]
fn test_database_open_creates_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("clisten_test.db");
    assert!(!path.exists());
    let _db = Database::open_at(&path).expect("open db");
    assert!(path.exists());
}

#[test]
fn test_database_add_favorite() {
    let (db, _dir) = open_temp_db();
    let item = make_episode("Test Show", "test-show-ep1");
    db.add_favorite(&item).expect("add_favorite");
    assert!(db
        .is_favorite("nts:episode:test-show:test-show-ep1")
        .expect("is_favorite"));
}

#[test]
fn test_database_remove_favorite() {
    let (db, _dir) = open_temp_db();
    let item = make_episode("Test Show", "test-show-ep1");
    db.add_favorite(&item).expect("add_favorite");
    assert!(db
        .is_favorite("nts:episode:test-show:test-show-ep1")
        .expect("is_favorite"));
    db.remove_favorite("nts:episode:test-show:test-show-ep1")
        .expect("remove_favorite");
    assert!(!db
        .is_favorite("nts:episode:test-show:test-show-ep1")
        .expect("is_favorite"));
}

#[test]
fn test_database_add_duplicate_favorite() {
    let (db, _dir) = open_temp_db();
    let item = make_episode("Test Show", "test-show-ep1");
    // First insert
    db.add_favorite(&item).expect("first add");
    // Second insert should be silently ignored (INSERT OR IGNORE)
    db.add_favorite(&item).expect("duplicate add");
    let favorites = db.list_favorites(None, 100, 0).expect("list");
    assert_eq!(favorites.len(), 1, "duplicate should be ignored");
}

#[test]
fn test_database_list_favorites() {
    let (db, _dir) = open_temp_db();
    let ep1 = make_episode("Episode One", "ep-one");
    let ep2 = make_episode("Episode Two", "ep-two");
    db.add_favorite(&ep1).expect("add ep1");
    db.add_favorite(&ep2).expect("add ep2");
    let favorites = db.list_favorites(None, 100, 0).expect("list");
    assert_eq!(favorites.len(), 2);
    // Both titles present (order by created_at DESC; within same second rowid DESC)
    let titles: Vec<&str> = favorites.iter().map(|f| f.title.as_str()).collect();
    assert!(titles.contains(&"Episode One"));
    assert!(titles.contains(&"Episode Two"));
}

#[test]
fn test_database_list_favorites_by_source() {
    let (db, _dir) = open_temp_db();
    let nts_item = make_episode("NTS Episode", "nts-ep");
    let direct_item = DiscoveryItem::DirectUrl {
        url: "https://youtube.com/watch?v=123".to_string(),
        title: Some("My Video".to_string()),
    };
    db.add_favorite(&nts_item).expect("add nts");
    db.add_favorite(&direct_item).expect("add direct");

    let nts_only = db.list_favorites(Some("nts"), 100, 0).expect("list nts");
    assert_eq!(nts_only.len(), 1);
    assert_eq!(nts_only[0].source, "nts");

    let direct_only = db
        .list_favorites(Some("direct"), 100, 0)
        .expect("list direct");
    assert_eq!(direct_only.len(), 1);
    assert_eq!(direct_only[0].source, "direct");

    let all = db.list_favorites(None, 100, 0).expect("list all");
    assert_eq!(all.len(), 2);
}

#[test]
fn test_database_add_to_history() {
    let (db, _dir) = open_temp_db();
    let item = make_episode("History Show", "history-ep1");
    db.add_to_history(&item).expect("add_to_history");
    let history = db.list_history(100, 0).expect("list_history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].title, "History Show");
}

#[test]
fn test_database_list_history() {
    let (db, _dir) = open_temp_db();
    let ep1 = make_episode("First Episode", "ep-first");
    let ep2 = make_episode("Second Episode", "ep-second");
    db.add_to_history(&ep1).expect("add ep1");
    db.add_to_history(&ep2).expect("add ep2");
    let history = db.list_history(100, 0).expect("list_history");
    assert_eq!(history.len(), 2);
    // Both titles present (order by played_at DESC)
    let titles: Vec<&str> = history.iter().map(|h| h.title.as_str()).collect();
    assert!(titles.contains(&"First Episode"));
    assert!(titles.contains(&"Second Episode"));
}

#[test]
fn test_database_clear_history() {
    let (db, _dir) = open_temp_db();
    db.add_to_history(&make_episode("Show A", "ep-a"))
        .expect("add");
    db.add_to_history(&make_episode("Show B", "ep-b"))
        .expect("add");
    let before = db.list_history(100, 0).expect("list");
    assert_eq!(before.len(), 2);
    db.clear_history().expect("clear");
    let after = db.list_history(100, 0).expect("list");
    assert_eq!(after.len(), 0);
}

#[test]
fn test_database_history_allows_duplicates() {
    let (db, _dir) = open_temp_db();
    let item = make_episode("Repeated Show", "repeated-ep");
    db.add_to_history(&item).expect("first play");
    db.add_to_history(&item).expect("second play");
    let history = db.list_history(100, 0).expect("list");
    // History allows duplicates (unlike favorites which uses INSERT OR IGNORE)
    assert_eq!(history.len(), 2);
}

// ── Favorites integration ────────────────────────────────────────────────────

#[test]
fn test_toggle_favorite_adds() {
    let (db, _dir) = open_temp_db();
    let item = make_episode("My Favorite Show", "fav-ep1");
    let key = item.favorite_key();

    // Not favorited initially
    assert!(!db.is_favorite(&key).expect("is_favorite"));

    // Add it
    db.add_favorite(&item).expect("add_favorite");

    // Now it's favorited
    assert!(db.is_favorite(&key).expect("is_favorite"));

    // Also in list_favorites
    let favorites = db.list_favorites(None, 100, 0).expect("list_favorites");
    assert_eq!(favorites.len(), 1);
    assert_eq!(favorites[0].title, "My Favorite Show");
    assert_eq!(favorites[0].source, "nts");
}

#[test]
fn test_toggle_favorite_removes() {
    let (db, _dir) = open_temp_db();
    let item = make_episode("Removable Show", "remove-ep1");
    let key = item.favorite_key();

    // Add then remove
    db.add_favorite(&item).expect("add_favorite");
    assert!(db.is_favorite(&key).expect("is_favorite"));

    db.remove_favorite(&key).expect("remove_favorite");
    assert!(!db.is_favorite(&key).expect("is_favorite"));

    // List should be empty
    let favorites = db.list_favorites(None, 100, 0).expect("list_favorites");
    assert_eq!(favorites.len(), 0);
}

#[test]
fn test_favorite_record_to_discovery_item() {
    let (db, _dir) = open_temp_db();
    let item = make_episode("Discovery Item Show", "discovery-ep1");
    db.add_favorite(&item).expect("add_favorite");

    let records = db.list_favorites(None, 100, 0).expect("list_favorites");
    assert_eq!(records.len(), 1);

    // Convert back to DiscoveryItem
    let discovery = records[0].to_discovery_item();
    assert_eq!(discovery.title(), "Discovery Item Show");
    assert!(matches!(
        discovery,
        clisten::api::models::DiscoveryItem::NtsEpisode { .. }
    ));
}

#[test]
fn test_history_record_to_discovery_item() {
    let (db, _dir) = open_temp_db();
    let item = make_episode("History Item Show", "history-ep1");
    db.add_to_history(&item).expect("add_to_history");

    let records = db.list_history(100, 0).expect("list_history");
    assert_eq!(records.len(), 1);

    // Convert back to DiscoveryItem
    let discovery = records[0].to_discovery_item();
    assert_eq!(discovery.title(), "History Item Show");
    assert!(matches!(
        discovery,
        clisten::api::models::DiscoveryItem::NtsEpisode { .. }
    ));
}

// ── Number keys for sub-tabs ─────────────────────────────────────────────────

#[test]
fn test_number_keys_send_switch_sub_tab() {
    // When not in search mode, keys 1-3 send SwitchSubTab(0-2).
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    for (digit, expected_idx) in [('1', 0), ('2', 1), ('3', 2)] {
        let key = KeyEvent {
            code: KeyCode::Char(digit),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        let idx = match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => c.to_digit(10).unwrap_or(0) as usize - 1,
            _ => panic!("expected digit"),
        };
        assert_eq!(
            idx, expected_idx,
            "key '{}' should map to sub-tab {}",
            digit, expected_idx
        );
    }
}

#[test]
fn test_number_keys_ignored_in_search() {
    // When search is focused, digit keys type into search input rather than switching tabs.
    use clisten::action::Action;
    use clisten::components::search_bar::SearchBar;
    use clisten::components::Component;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use tokio::sync::mpsc;

    let (tx, _rx) = mpsc::unbounded_channel::<Action>();
    let mut bar = SearchBar::new();
    bar.register_action_handler(tx);
    bar.update(&Action::FocusSearch).unwrap();

    // Type digits into the search bar
    for c in ['1', '2', '3'] {
        let key = KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        let consumed = bar.handle_key_event(key).unwrap();
        assert!(consumed, "digit '{}' in search mode should be consumed", c);
    }
    assert_eq!(
        bar.input(),
        "123",
        "digits should be typed into search input"
    );
}

// ── NtsGenre DiscoveryItem tests ──────────────────────────────────────────────

#[test]
fn test_nts_genre_discovery_item() {
    let genre = DiscoveryItem::NtsGenre {
        name: "Ambient".to_string(),
        genre_id: "ambient".to_string(),
    };
    assert_eq!(genre.title(), "Ambient");
    assert_eq!(genre.subtitle(), "Genre");
    assert_eq!(genre.playback_url(), None);
    assert_eq!(genre.favorite_key(), "genre:ambient");
}

#[test]
fn test_nts_genre_special_ids() {
    let favorites = DiscoveryItem::NtsGenre {
        name: "My Favorites (5)".to_string(),
        genre_id: "_favorites".to_string(),
    };
    assert_eq!(favorites.title(), "My Favorites (5)");
    assert_eq!(favorites.favorite_key(), "genre:_favorites");

    let history = DiscoveryItem::NtsGenre {
        name: "History (10)".to_string(),
        genre_id: "_history".to_string(),
    };
    assert_eq!(history.title(), "History (10)");
}

// ── append_items tests ────────────────────────────────────────────────────────

#[test]
fn test_discovery_list_append_items() {
    use clisten::components::discovery_list::DiscoveryList;

    let mut list = DiscoveryList::new();
    list.set_items(vec![make_episode("Episode 1", "ep-1")]);
    assert_eq!(list.total_item_count(), 1);

    list.append_items(vec![
        make_episode("Episode 2", "ep-2"),
        make_episode("Episode 3", "ep-3"),
    ]);
    assert_eq!(list.total_item_count(), 3);
    assert_eq!(list.visible_items().len(), 3);
}

#[test]
fn test_discovery_list_append_items_with_filter() {
    use clisten::components::discovery_list::DiscoveryList;

    let mut list = DiscoveryList::new();
    list.set_items(vec![
        make_episode("Jazz Night", "jazz-1"),
        make_episode("Rock Show", "rock-1"),
    ]);
    list.set_filter(Some("jazz".to_string()));
    assert_eq!(list.visible_items().len(), 1);

    // Append more items — filter should be re-applied
    list.append_items(vec![make_episode("Jazz Morning", "jazz-2")]);
    assert_eq!(list.total_item_count(), 3);
    assert_eq!(list.visible_items().len(), 2); // both jazz items visible
}

// ── Tab/BackTab cycling tests ─────────────────────────────────────────────────

#[tokio::test]
async fn test_tab_cycles_sub_tabs() {
    use clisten::components::nts::NtsSubTab;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = clisten::app::App::new(clisten::config::Config::default()).unwrap();
    assert_eq!(app.nts_tab.active_sub, NtsSubTab::Live);

    // Tab → Picks
    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub, NtsSubTab::Picks);

    // Tab → Search
    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub, NtsSubTab::Search);

    // Tab → wraps to Live
    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub, NtsSubTab::Live);
}

#[tokio::test]
async fn test_backtab_cycles_sub_tabs_reverse() {
    use clisten::components::nts::NtsSubTab;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = clisten::app::App::new(clisten::config::Config::default()).unwrap();
    assert_eq!(app.nts_tab.active_sub, NtsSubTab::Live);

    // BackTab → wraps to Search
    let key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub, NtsSubTab::Search);

    // BackTab → Picks
    let key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub, NtsSubTab::Picks);
}
