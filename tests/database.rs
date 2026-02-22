// SQLite database: queue persistence tests.

use clisten::api::models::DiscoveryItem;
use clisten::db::Database;
use clisten::player::queue::QueueItem;

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

// ── Queue persistence ────────────────────────────────────────────────────────

#[test]
fn test_save_and_load_queue() {
    let (db, _dir) = open_temp_db();
    let items = vec![
        QueueItem {
            item: make_episode("Episode 1", "ep-1"),
            url: "https://example.com/1".to_string(),
            stream_metadata: None,
        },
        QueueItem {
            item: make_episode("Episode 2", "ep-2"),
            url: "https://example.com/2".to_string(),
            stream_metadata: None,
        },
    ];

    db.save_queue(&items, Some(1)).expect("save_queue");

    let (loaded, current_index) = db.load_queue().expect("load_queue");
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].item.title(), "Episode 1");
    assert_eq!(loaded[0].url, "https://example.com/1");
    assert_eq!(loaded[1].item.title(), "Episode 2");
    assert_eq!(loaded[1].url, "https://example.com/2");
    assert_eq!(current_index, Some(1));
}

#[test]
fn test_save_empty_queue() {
    let (db, _dir) = open_temp_db();

    // Save non-empty first
    let items = vec![QueueItem {
        item: make_episode("Episode 1", "ep-1"),
        url: "https://example.com/1".to_string(),
        stream_metadata: None,
    }];
    db.save_queue(&items, Some(0)).expect("save_queue");

    // Now save empty
    db.save_queue(&[], None).expect("save empty queue");

    let (loaded, current_index) = db.load_queue().expect("load_queue");
    assert_eq!(loaded.len(), 0);
    assert_eq!(current_index, None);
}

#[test]
fn test_load_queue_empty_db() {
    let (db, _dir) = open_temp_db();

    let (loaded, current_index) = db.load_queue().expect("load_queue");
    assert_eq!(loaded.len(), 0);
    assert_eq!(current_index, None);
}

#[test]
fn test_save_queue_overwrites_previous() {
    let (db, _dir) = open_temp_db();

    let items1 = vec![
        QueueItem {
            item: make_episode("Episode A", "ep-a"),
            url: "https://example.com/a".to_string(),
            stream_metadata: None,
        },
        QueueItem {
            item: make_episode("Episode B", "ep-b"),
            url: "https://example.com/b".to_string(),
            stream_metadata: None,
        },
    ];
    db.save_queue(&items1, Some(0)).expect("save_queue 1");

    let items2 = vec![QueueItem {
        item: make_episode("Episode C", "ep-c"),
        url: "https://example.com/c".to_string(),
        stream_metadata: None,
    }];
    db.save_queue(&items2, Some(0)).expect("save_queue 2");

    let (loaded, current_index) = db.load_queue().expect("load_queue");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].item.title(), "Episode C");
    assert_eq!(current_index, Some(0));
}

#[test]
fn test_save_queue_with_direct_url() {
    let (db, _dir) = open_temp_db();
    let items = vec![QueueItem {
        item: DiscoveryItem::DirectUrl {
            url: "https://youtube.com/watch?v=123".to_string(),
            title: Some("My Video".to_string()),
        },
        url: "https://youtube.com/watch?v=123".to_string(),
        stream_metadata: None,
    }];

    db.save_queue(&items, Some(0)).expect("save_queue");

    let (loaded, _) = db.load_queue().expect("load_queue");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].item.title(), "My Video");
    assert!(matches!(loaded[0].item, DiscoveryItem::DirectUrl { .. }));
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
    assert_eq!(app.nts_tab.active_sub(), NtsSubTab::Live);

    // Tab → Picks
    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub(), NtsSubTab::Picks);

    // Tab → Search
    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub(), NtsSubTab::Search);

    // Tab → wraps to Live
    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub(), NtsSubTab::Live);
}

#[tokio::test]
async fn test_backtab_cycles_sub_tabs_reverse() {
    use clisten::components::nts::NtsSubTab;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = clisten::app::App::new(clisten::config::Config::default()).unwrap();
    assert_eq!(app.nts_tab.active_sub(), NtsSubTab::Live);

    // BackTab → wraps to Search
    let key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub(), NtsSubTab::Search);

    // BackTab → Picks
    let key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert_eq!(app.nts_tab.active_sub(), NtsSubTab::Picks);
}
