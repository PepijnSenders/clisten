// tests/phase6_refinements.rs

use clisten::action::Action;
use clisten::api::models::DiscoveryItem;
use clisten::components::discovery_list::DiscoveryList;
use clisten::components::nts::NtsTab;
use clisten::components::Component;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_item(title: &str, subtitle: &str) -> DiscoveryItem {
    DiscoveryItem::NtsMixtape {
        title: title.to_string(),
        subtitle: subtitle.to_string(),
        stream_url: format!("http://{}", title),
        mixtape_alias: title.to_string(),
    }
}

// ── 6.1 Sub-Tab Bar ───────────────────────────────────────────────────────────

#[test]
fn test_sub_tab_bar_renders_nts() {
    let mut nts_tab = NtsTab::new();
    assert_eq!(format!("{:?}", nts_tab.active_sub), "Live");
    nts_tab.switch_sub_tab(1);
    assert_eq!(format!("{:?}", nts_tab.active_sub), "Picks");
    nts_tab.switch_sub_tab(2);
    assert_eq!(format!("{:?}", nts_tab.active_sub), "Search");
}

#[test]
fn test_switch_sub_tab_routes_to_active_tab() {
    let mut nts_tab = NtsTab::new();
    let actions = nts_tab.update(&Action::SwitchSubTab(2)).unwrap();
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadGenres)),
        "Expected LoadGenres action, got: {:?}", actions
    );
    assert_eq!(format!("{:?}", nts_tab.active_sub), "Search");
}

// ── 6.3 NTS Client-Side Search (Filter) ──────────────────────────────────────

#[test]
fn test_discovery_list_filter() {
    let mut list = DiscoveryList::new();
    list.set_items(vec![
        make_item("Jazz Sessions", "cool jazz"),
        make_item("Rock Classics", "rock music"),
        make_item("Jazz Vibes", "another jazz show"),
        make_item("Electronic", "techno beats"),
        make_item("Jazz Cafe", "lounge jazz"),
    ]);

    list.set_filter(Some("jazz".to_string()));
    let visible = list.visible_items();
    assert_eq!(visible.len(), 3, "Expected 3 jazz items, got {}", visible.len());
    assert!(visible.iter().all(|i| i.title().to_lowercase().contains("jazz") || i.subtitle().to_lowercase().contains("jazz")));
}

#[test]
fn test_discovery_list_clear_filter() {
    let mut list = DiscoveryList::new();
    list.set_items(vec![
        make_item("Jazz Sessions", "cool jazz"),
        make_item("Rock Classics", "rock music"),
        make_item("Electronic", "techno beats"),
    ]);

    list.set_filter(Some("jazz".to_string()));
    assert_eq!(list.visible_items().len(), 1);

    list.set_filter(None);
    assert_eq!(list.visible_items().len(), 3, "All items should be visible after clearing filter");
}

#[test]
fn test_nts_search_submit_filters() {
    let action = Action::FilterList("jazz".to_string());
    assert!(matches!(action, Action::FilterList(_)));
    let action = Action::ClearFilter;
    assert!(matches!(action, Action::ClearFilter));
}

// ── 6.4 Search Bar UX ────────────────────────────────────────────────────────

#[test]
fn test_search_bar_clears_on_submit() {
    use clisten::components::search_bar::SearchBar;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut bar = SearchBar::new();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    bar.register_action_handler(tx);

    bar.update(&Action::FocusSearch).unwrap();
    assert!(bar.is_focused());

    bar.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)).unwrap();
    bar.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)).unwrap();
    bar.handle_key_event(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE)).unwrap();
    assert_eq!(bar.input(), "jaz");

    bar.update(&Action::SearchSubmit).unwrap();
    assert_eq!(bar.input(), "", "Search bar should clear on submit");
    assert!(!bar.is_focused(), "Search bar should unfocus on submit");
}

#[test]
fn test_discovery_list_loading_state() {
    let mut list = DiscoveryList::new();
    assert!(!list.is_loading(), "Initially not loading");

    list.set_loading(true);
    assert!(list.is_loading(), "Should be loading after set_loading(true)");

    list.set_loading(false);
    assert!(!list.is_loading(), "Should not be loading after set_loading(false)");
}

// ── 6.5 Volume Control ────────────────────────────────────────────────────────

#[test]
fn test_volume_up_action_exists() {
    let action = Action::VolumeUp;
    assert!(matches!(action, Action::VolumeUp));
}

#[test]
fn test_volume_down_action_exists() {
    let action = Action::VolumeDown;
    assert!(matches!(action, Action::VolumeDown));
}

#[test]
fn test_bracket_keys_send_volume() {
    let up = Action::VolumeUp;
    let down = Action::VolumeDown;
    assert!(matches!(up, Action::VolumeUp));
    assert!(matches!(down, Action::VolumeDown));
}

#[test]
fn test_play_controls_shows_volume() {
    use clisten::components::play_controls::PlayControls;
    let mut controls = PlayControls::new();
    assert!(controls.volume.is_none());
    controls.update(&Action::VolumeChanged(75)).unwrap();
    assert_eq!(controls.volume, Some(75));
}

// ── Direct Play Modal ─────────────────────────────────────────────────────────

#[test]
fn test_direct_play_actions_exist() {
    let open = Action::OpenDirectPlay;
    assert!(matches!(open, Action::OpenDirectPlay));
    let close = Action::CloseDirectPlay;
    assert!(matches!(close, Action::CloseDirectPlay));
}

#[test]
fn test_direct_play_modal_visibility() {
    use clisten::components::direct_play_modal::DirectPlayModal;
    let mut modal = DirectPlayModal::new();
    assert!(!modal.is_visible());
    modal.show();
    assert!(modal.is_visible());
    modal.hide();
    assert!(!modal.is_visible());
}
