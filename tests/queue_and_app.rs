// Queue operations, action dispatch, error/help overlays, and keybinding integration.

use clisten::action::Action;
use clisten::api::models::DiscoveryItem;
use clisten::db::Database;
use clisten::player::queue::{Queue, QueueItem};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Create an App backed by a temporary database so tests never pollute the
/// production database.
fn test_app() -> clisten::app::App {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(&dir.path().join("test.db")).unwrap();
    // Leak the tempdir so it lives until the process exits (tests are short-lived).
    std::mem::forget(dir);
    clisten::app::App::with_db(clisten::config::Config::default(), db).unwrap()
}

fn make_queue_item(title: &str, url: &str) -> QueueItem {
    QueueItem {
        item: DiscoveryItem::NtsEpisode {
            name: title.to_string(),
            show_alias: title.to_string(),
            episode_alias: title.to_string(),
            genres: vec![],
            location: None,
            audio_url: Some(url.to_string()),
        },
        url: url.to_string(),
        stream_metadata: None,
    }
}

fn make_item(title: &str) -> DiscoveryItem {
    DiscoveryItem::NtsEpisode {
        name: title.to_string(),
        show_alias: title.to_string(),
        episode_alias: title.to_string(),
        genres: vec![],
        location: None,
        audio_url: Some(format!("http://{}", title)),
    }
}

// ── Queue ────────────────────────────────────────────────────────────────────

#[test]
fn test_queue_new_empty() {
    let q = Queue::new();
    assert!(q.is_empty());
    assert_eq!(q.len(), 0);
    assert!(q.current().is_none());
    assert!(q.current_index().is_none());
}

#[test]
fn test_queue_add() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    assert_eq!(q.len(), 1);
    assert_eq!(q.current_index(), Some(0));
    assert_eq!(q.current().unwrap().url, "http://a");
}

#[test]
fn test_queue_add_multiple_sets_current_to_first() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    q.add(make_queue_item("Track 2", "http://b"));
    // current stays at 0 after additional adds
    assert_eq!(q.current_index(), Some(0));
    assert_eq!(q.len(), 2);
}

#[test]
fn test_queue_add_next() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    q.add(make_queue_item("Track 3", "http://c"));
    // insert after current (index 0)
    q.add_next(make_queue_item("Track 2", "http://b"));
    assert_eq!(q.len(), 3);
    assert_eq!(q.items()[1].url, "http://b");
}

#[test]
fn test_queue_add_next_empty_queue() {
    let mut q = Queue::new();
    q.add_next(make_queue_item("Track 1", "http://a"));
    assert_eq!(q.len(), 1);
    assert_eq!(q.current_index(), Some(0));
}

#[test]
fn test_queue_remove() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    q.add(make_queue_item("Track 2", "http://b"));
    q.add(make_queue_item("Track 3", "http://c"));
    q.remove(1);
    assert_eq!(q.len(), 2);
    assert_eq!(q.items()[0].url, "http://a");
    assert_eq!(q.items()[1].url, "http://c");
}

#[test]
fn test_queue_remove_current_adjusts_index() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    q.add(make_queue_item("Track 2", "http://b"));
    // advance to track 2
    q.advance();
    assert_eq!(q.current_index(), Some(1));
    // remove item before current
    q.remove(0);
    // current_index should adjust down
    assert_eq!(q.current_index(), Some(0));
}

#[test]
fn test_queue_remove_all_clears_index() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    q.remove(0);
    assert!(q.is_empty());
    assert!(q.current_index().is_none());
}

#[test]
fn test_queue_clear() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    q.add(make_queue_item("Track 2", "http://b"));
    q.clear();
    assert!(q.is_empty());
    assert!(q.current_index().is_none());
    assert!(q.current().is_none());
}

#[test]
fn test_queue_next() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    q.add(make_queue_item("Track 2", "http://b"));
    let item = q.advance();
    assert!(item.is_some());
    assert_eq!(item.unwrap().url, "http://b");
    assert_eq!(q.current_index(), Some(1));
}

#[test]
fn test_queue_next_at_end() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    let item = q.advance();
    assert!(item.is_none());
    // current_index stays at 0
    assert_eq!(q.current_index(), Some(0));
}

#[test]
fn test_queue_prev() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    q.add(make_queue_item("Track 2", "http://b"));
    q.advance(); // move to index 1
    let item = q.prev();
    assert!(item.is_some());
    assert_eq!(item.unwrap().url, "http://a");
    assert_eq!(q.current_index(), Some(0));
}

#[test]
fn test_queue_prev_at_start() {
    let mut q = Queue::new();
    q.add(make_queue_item("Track 1", "http://a"));
    let item = q.prev();
    assert!(item.is_none());
    assert_eq!(q.current_index(), Some(0));
}

fn make_live_queue_item(channel: u8, show_name: &str, genres: Vec<&str>) -> QueueItem {
    let item = DiscoveryItem::NtsLiveChannel {
        channel,
        show_name: show_name.to_string(),
        genres: genres.into_iter().map(String::from).collect(),
    };
    let url = item.playback_url().unwrap_or_default();
    QueueItem {
        item,
        url,
        stream_metadata: None,
    }
}

fn make_live_item(channel: u8, show_name: &str, genres: Vec<&str>) -> DiscoveryItem {
    DiscoveryItem::NtsLiveChannel {
        channel,
        show_name: show_name.to_string(),
        genres: genres.into_iter().map(String::from).collect(),
    }
}

// ── find_live_channel / deduplication ─────────────────────────────────────────

#[test]
fn test_find_live_channel() {
    let mut q = Queue::new();
    q.add(make_queue_item("Episode", "http://ep"));
    q.add(make_live_queue_item(1, "Show A", vec!["Jazz"]));
    q.add(make_live_queue_item(2, "Show B", vec!["Ambient"]));

    assert_eq!(q.find_live_channel(1), Some(1));
    assert_eq!(q.find_live_channel(2), Some(2));
    assert_eq!(q.find_live_channel(3), None);
}

#[test]
fn test_play_live_channel_twice_no_duplicate() {
    let mut q = Queue::new();
    q.add(make_live_queue_item(1, "Show A", vec!["Jazz"]));

    // Same channel, different show name — should find existing entry
    assert!(q.find_live_channel(1).is_some());
    assert_eq!(q.len(), 1);

    // Update metadata and jump instead of adding
    let fresh = DiscoveryItem::NtsLiveChannel {
        channel: 1,
        show_name: "Show B".to_string(),
        genres: vec!["Techno".to_string()],
    };
    let idx = q.find_live_channel(1).unwrap();
    q.update_live_channel_at(idx, &fresh);
    q.play_at(idx);

    assert_eq!(q.len(), 1); // Still only one entry
    assert_eq!(q.current_index(), Some(0));
    if let DiscoveryItem::NtsLiveChannel { show_name, .. } = &q.items()[0].item {
        assert_eq!(show_name, "Show B");
    } else {
        panic!("expected NtsLiveChannel");
    }
}

#[test]
fn test_different_live_channels_remain_separate() {
    let mut q = Queue::new();
    q.add(make_live_queue_item(1, "Show A", vec!["Jazz"]));
    q.add(make_live_queue_item(2, "Show B", vec!["Ambient"]));

    assert_eq!(q.len(), 2);
    assert_eq!(q.find_live_channel(1), Some(0));
    assert_eq!(q.find_live_channel(2), Some(1));
}

// ── update_live_channels ─────────────────────────────────────────────────────

#[test]
fn test_update_live_channels_updates_stale_metadata() {
    let mut q = Queue::new();
    q.add(make_live_queue_item(1, "Old Show", vec!["Jazz"]));
    q.add(make_live_queue_item(2, "Old Show 2", vec!["Ambient"]));

    let fresh = vec![
        make_live_item(1, "New Show", vec!["Techno", "House"]),
        make_live_item(2, "New Show 2", vec!["Drum & Bass"]),
    ];
    let changed = q.update_live_channels(&fresh);

    assert!(changed);
    if let DiscoveryItem::NtsLiveChannel {
        show_name, genres, ..
    } = &q.items()[0].item
    {
        assert_eq!(show_name, "New Show");
        assert_eq!(genres, &["Techno", "House"]);
    } else {
        panic!("expected NtsLiveChannel");
    }
    if let DiscoveryItem::NtsLiveChannel {
        show_name, genres, ..
    } = &q.items()[1].item
    {
        assert_eq!(show_name, "New Show 2");
        assert_eq!(genres, &["Drum & Bass"]);
    } else {
        panic!("expected NtsLiveChannel");
    }
}

#[test]
fn test_update_live_channels_returns_false_when_unchanged() {
    let mut q = Queue::new();
    q.add(make_live_queue_item(1, "Same Show", vec!["Jazz"]));

    let fresh = vec![make_live_item(1, "Same Show", vec!["Jazz"])];
    let changed = q.update_live_channels(&fresh);
    assert!(!changed);
}

#[test]
fn test_update_live_channels_ignores_non_live_items() {
    let mut q = Queue::new();
    q.add(make_queue_item("Episode", "http://ep"));

    let fresh = vec![make_live_item(1, "New Show", vec!["Techno"])];
    let changed = q.update_live_channels(&fresh);
    assert!(!changed);
    // Episode item should be untouched
    assert_eq!(q.items()[0].item.title(), "Episode");
}

// ── Queue action variants ────────────────────────────────────────────────────

#[test]
fn test_queue_action_variants_exist() {
    let item = make_item("track1");
    // These must compile — confirms Action variants exist
    let _add = Action::AddToQueue(item.clone());
    let _add_next = Action::AddToQueueNext(item.clone());
    let _clear = Action::ClearQueue;
    let _next = Action::NextTrack;
    let _prev = Action::PrevTrack;
}

// ── Error handling ───────────────────────────────────────────────────────────

#[test]
fn test_show_error_action_exists() {
    let _a = Action::ShowError("test error".to_string());
    let _b = Action::ClearError;
}

// ── Help overlay ─────────────────────────────────────────────────────────────

#[test]
fn test_help_action_variants_exist() {
    let _a = Action::ShowHelp;
    let _b = Action::HideHelp;
}

// ── App integration tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_show_error_sets_message() {
    let mut app = test_app();
    app.handle_action(Action::ShowError("test error".to_string()))
        .await
        .unwrap();
    assert_eq!(app.error_message.as_deref(), Some("test error"));
}

#[tokio::test]
async fn test_clear_error_clears_message() {
    let mut app = test_app();
    app.handle_action(Action::ShowError("err".to_string()))
        .await
        .unwrap();
    app.handle_action(Action::ClearError).await.unwrap();
    assert!(app.error_message.is_none());
}

#[tokio::test]
async fn test_help_toggle_on() {
    let mut app = test_app();
    app.handle_action(Action::ShowHelp).await.unwrap();
    assert!(app.show_help);
}

#[tokio::test]
async fn test_help_toggle_off() {
    let mut app = test_app();
    app.handle_action(Action::ShowHelp).await.unwrap();
    app.handle_action(Action::HideHelp).await.unwrap();
    assert!(!app.show_help);
}

#[tokio::test]
async fn test_add_to_queue() {
    let mut app = test_app();
    app.queue.clear();
    app.handle_action(Action::AddToQueue(make_item("track1")))
        .await
        .unwrap();
    assert_eq!(app.queue.len(), 1);
    assert_eq!(app.queue.current_index(), Some(0));
}

#[tokio::test]
async fn test_add_to_queue_next() {
    let mut app = test_app();
    app.queue.clear();
    app.handle_action(Action::AddToQueue(make_item("track1")))
        .await
        .unwrap();
    app.handle_action(Action::AddToQueue(make_item("track3")))
        .await
        .unwrap();
    app.handle_action(Action::AddToQueueNext(make_item("track2")))
        .await
        .unwrap();
    // track2 should be inserted after current (index 0), so at index 1
    assert_eq!(app.queue.items()[1].url, "http://track2");
}

#[tokio::test]
async fn test_key_a_adds_to_queue() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = test_app();
    app.queue.clear();
    // Set a selected item in discovery_list
    app.discovery_list.set_items(vec![make_item("track1")]);

    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    // Process the queued action
    app.flush_actions().await;
    assert_eq!(app.queue.len(), 1);
}

#[tokio::test]
async fn test_key_c_clears_queue() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = test_app();
    app.handle_action(Action::AddToQueue(make_item("track1")))
        .await
        .unwrap();

    let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert!(app.queue.is_empty());
}

#[tokio::test]
async fn test_playback_finished_advances_queue() {
    let mut app = test_app();
    app.queue.clear();
    app.handle_action(Action::AddToQueue(make_item("track1")))
        .await
        .unwrap();
    app.handle_action(Action::AddToQueue(make_item("track2")))
        .await
        .unwrap();
    // current is at 0 (track1); PlaybackFinished should auto-advance to track2
    app.handle_action(Action::PlaybackFinished).await.unwrap();
    // queue advances — current index should be 1
    assert_eq!(app.queue.current_index(), Some(1));
}

#[tokio::test]
async fn test_playback_finished_empty_queue() {
    let mut app = test_app();
    app.queue.clear();
    // No queue — PlaybackFinished should not panic
    app.handle_action(Action::PlaybackFinished).await.unwrap();
    assert!(app.queue.is_empty());
}

#[tokio::test]
async fn test_question_mark_toggles_help() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = test_app();
    assert!(!app.show_help);

    let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert!(app.show_help);

    // Press again — should hide
    let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert!(!app.show_help);
}

#[tokio::test]
async fn test_retry_key_resends_load() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = test_app();
    app.error_message = Some("some error".to_string());

    let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    // error_message should be cleared after 'r' press
    assert!(app.error_message.is_none());
}

#[tokio::test]
async fn test_retry_key_ignored_without_error() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = test_app();
    assert!(app.error_message.is_none());

    // 'r' without error — should not panic or crash
    let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    // still no error
    assert!(app.error_message.is_none());
}

// ── NtsLiveLoaded refreshes queue ─────────────────────────────────────────────

#[tokio::test]
async fn test_nts_live_loaded_updates_queue_items() {
    let mut app = test_app();
    app.queue.clear();

    // Add a live channel to the queue with stale metadata
    app.queue
        .add(make_live_queue_item(1, "Old Show", vec!["Jazz"]));

    // Simulate NtsLiveLoaded with fresh data
    let fresh = vec![
        make_live_item(1, "Fresh Show", vec!["Techno", "House"]),
        make_live_item(2, "Channel 2 Show", vec!["Ambient"]),
    ];
    app.handle_action(Action::NtsLiveLoaded(fresh))
        .await
        .unwrap();

    // Queue should be updated with fresh metadata
    if let DiscoveryItem::NtsLiveChannel {
        show_name, genres, ..
    } = &app.queue.items()[0].item
    {
        assert_eq!(show_name, "Fresh Show");
        assert_eq!(genres, &["Techno", "House"]);
    } else {
        panic!("expected NtsLiveChannel");
    }
}

// ── Live channel queue deduplication ─────────────────────────────────────────

#[tokio::test]
async fn test_enqueue_live_channel_skips_duplicate() {
    let mut app = test_app();
    app.queue.clear();

    let ch1_a = make_live_item(1, "Show A", vec!["Jazz"]);
    let ch1_b = make_live_item(1, "Show B", vec!["Techno"]);

    app.handle_action(Action::AddToQueue(ch1_a)).await.unwrap();
    assert_eq!(app.queue.len(), 1);

    // Adding the same channel again should be skipped
    app.handle_action(Action::AddToQueue(ch1_b)).await.unwrap();
    assert_eq!(app.queue.len(), 1);
}

#[tokio::test]
async fn test_enqueue_different_live_channels_allowed() {
    let mut app = test_app();
    app.queue.clear();

    let ch1 = make_live_item(1, "Show A", vec!["Jazz"]);
    let ch2 = make_live_item(2, "Show B", vec!["Ambient"]);

    app.handle_action(Action::AddToQueue(ch1)).await.unwrap();
    app.handle_action(Action::AddToQueue(ch2)).await.unwrap();
    assert_eq!(app.queue.len(), 2);
}

// ── Error display ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_error_displayed_in_status() {
    // When error_message is Some, the app should hold it for rendering
    let mut app = test_app();
    assert!(app.error_message.is_none());
    app.handle_action(Action::ShowError("network timeout".to_string()))
        .await
        .unwrap();
    assert_eq!(app.error_message.as_deref(), Some("network timeout"));
}

// ── Any key dismisses help overlay ───────────────────────────────────────────

#[tokio::test]
async fn test_any_key_dismisses_help() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = test_app();
    app.show_help = true;

    // Press 'j' — should dismiss help, not scroll
    let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert!(!app.show_help);
}

#[tokio::test]
async fn test_help_overlay_dismisses_on_escape() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = test_app();
    app.show_help = true;

    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    app.handle_key(key).unwrap();
    app.flush_actions().await;
    assert!(!app.show_help);
}

// ── Dependency check ─────────────────────────────────────────────────────────

#[test]
#[ignore = "requires mpv to be installed"]
fn test_check_mpv_present() {
    // Integration test — mpv must be installed in the environment
    let result = which::which("mpv");
    assert!(result.is_ok(), "mpv must be installed for clisten to work");
}
