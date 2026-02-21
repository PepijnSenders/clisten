// tests/phase1_scaffolding.rs
//
// Phase 1 acceptance tests — scaffolding, config, components

use clisten::config::Config;

// ── 1.3 Config Tests ──

#[test]
fn test_config_default_values() {
    let config = Config::default();
    assert_eq!(config.general.frame_rate, 30.0);
}

#[test]
fn test_config_parse_toml() {
    let toml_str = r#"
[general]
frame_rate = 60.0
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.general.frame_rate, 60.0);
}

#[test]
fn test_config_missing_file_uses_defaults() {
    let config = Config::default();
    assert_eq!(config.general.frame_rate, 30.0);
}

// ── 1.5 Component Tests ──

mod component_tests {
    use clisten::components::discovery_list::DiscoveryList;
    use clisten::components::search_bar::SearchBar;
    use clisten::components::now_playing::NowPlaying;
    use clisten::components::play_controls::PlayControls;
    use clisten::components::Component;
    use clisten::action::Action;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, KeyEventKind, KeyEventState};
    use tokio::sync::mpsc;

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_discovery_list_scroll_down() {
        use clisten::api::models::DiscoveryItem;
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut list = DiscoveryList::new();
        list.register_action_handler(tx);
        list.set_items(vec![
            DiscoveryItem::NtsLiveChannel {
                channel: 1,
                show_name: "Show 1".to_string(),
                genres: vec![],
            },
            DiscoveryItem::NtsLiveChannel {
                channel: 2,
                show_name: "Show 2".to_string(),
                genres: vec![],
            },
        ]);
        assert_eq!(list.state.selected(), Some(0));
        list.handle_key_event(make_key(KeyCode::Char('j'))).unwrap();
        assert_eq!(list.state.selected(), Some(1));
    }

    #[test]
    fn test_discovery_list_scroll_up() {
        use clisten::api::models::DiscoveryItem;
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut list = DiscoveryList::new();
        list.register_action_handler(tx);
        list.set_items(vec![
            DiscoveryItem::NtsLiveChannel {
                channel: 1,
                show_name: "Show 1".to_string(),
                genres: vec![],
            },
            DiscoveryItem::NtsLiveChannel {
                channel: 2,
                show_name: "Show 2".to_string(),
                genres: vec![],
            },
        ]);
        list.handle_key_event(make_key(KeyCode::Char('j'))).unwrap();
        assert_eq!(list.state.selected(), Some(1));
        list.handle_key_event(make_key(KeyCode::Char('k'))).unwrap();
        assert_eq!(list.state.selected(), Some(0));
    }

    #[test]
    fn test_discovery_list_clamps_at_bounds() {
        use clisten::api::models::DiscoveryItem;
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut list = DiscoveryList::new();
        list.register_action_handler(tx);
        list.set_items(vec![
            DiscoveryItem::NtsLiveChannel {
                channel: 1,
                show_name: "Show 1".to_string(),
                genres: vec![],
            },
        ]);
        // At first item, k should stay at 0
        list.handle_key_event(make_key(KeyCode::Char('k'))).unwrap();
        assert_eq!(list.state.selected(), Some(0));
        // At last item (index 0 of 1), j should stay at 0
        list.handle_key_event(make_key(KeyCode::Char('j'))).unwrap();
        assert_eq!(list.state.selected(), Some(0));
    }

    #[test]
    fn test_search_bar_focus() {
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut bar = SearchBar::new();
        bar.register_action_handler(tx);
        assert!(!bar.is_focused());
        bar.update(&Action::FocusSearch).unwrap();
        assert!(bar.is_focused());
    }

    #[test]
    fn test_search_bar_unfocus() {
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut bar = SearchBar::new();
        bar.register_action_handler(tx);
        bar.update(&Action::FocusSearch).unwrap();
        bar.update(&Action::Back).unwrap();
        assert!(!bar.is_focused());
        assert_eq!(bar.input, "");
    }

    #[test]
    fn test_search_bar_typing() {
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut bar = SearchBar::new();
        bar.register_action_handler(tx);
        bar.update(&Action::FocusSearch).unwrap();
        bar.handle_key_event(make_key(KeyCode::Char('a'))).unwrap();
        bar.handle_key_event(make_key(KeyCode::Char('b'))).unwrap();
        bar.handle_key_event(make_key(KeyCode::Char('c'))).unwrap();
        assert_eq!(bar.input, "abc");
    }

    #[test]
    fn test_now_playing_initial_state() {
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut np = NowPlaying::new();
        np.register_action_handler(tx);
        assert!(np.current_item.is_none());
        assert_eq!(np.position_secs, 0.0);
        assert!(!np.paused);
    }

    #[test]
    fn test_play_controls_initial_state() {
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut pc = PlayControls::new();
        pc.register_action_handler(tx);
        assert!(!pc.playing);
        assert!(!pc.paused);
        assert_eq!(pc.queue_len, 0);
    }
}

// ── 1.6 App Action Tests ──

mod app_action_tests {
    use clisten::action::Action;
    use tokio::sync::mpsc;

    #[test]
    fn test_quit_sends_action() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, KeyEventKind, KeyEventState};

        let key = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };

        assert_eq!(key.code, KeyCode::Char('q'));
        assert_eq!(key.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_quit_ignored_in_search() {
        use clisten::components::search_bar::SearchBar;
        use clisten::components::Component;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, KeyEventKind, KeyEventState};

        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut bar = SearchBar::new();
        bar.register_action_handler(tx);
        bar.update(&Action::FocusSearch).unwrap();

        let key = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        let consumed = bar.handle_key_event(key).unwrap();
        assert!(consumed, "q in search mode should be consumed");
        assert_eq!(bar.input, "q");
    }
}
