// NTS API deserialization, DiscoveryItem methods, mpv player, and sub-tab routing.

use clisten::action::Action;
use clisten::api::models::{DiscoveryItem, NtsCollectionResponse, NtsLiveResponse};
use clisten::api::nts::NtsClient;
use clisten::components::nts::{NtsSubTab, NtsTab};
use clisten::player::MpvPlayer;

// ── API deserialization ──────────────────────────────────────────────────────

#[test]
fn test_nts_live_response_deserializes() {
    let json = r#"{
        "results": [
            {
                "channel_name": "1",
                "now": {
                    "broadcast_title": "Resident Show Name",
                    "start_timestamp": "2026-02-18T14:00:00Z",
                    "end_timestamp": "2026-02-18T16:00:00Z",
                    "embeds": {
                        "details": {
                            "name": "Resident Show Name",
                            "status": "published",
                            "updated": "2026-02-18T12:00:00+00:00",
                            "description": "A show about ambient music...",
                            "description_html": "<p>A show about ambient music...</p>",
                            "external_links": [],
                            "moods": [],
                            "genres": [
                                { "id": "ambient", "value": "Ambient" },
                                { "id": "drone", "value": "Drone" }
                            ],
                            "location_short": "LDN",
                            "location_long": "London",
                            "intensity": "50",
                            "media": null,
                            "episode_alias": "resident-show-18th-february-2026",
                            "show_alias": "resident-show-name",
                            "broadcast": "2026-02-18T14:00:00Z",
                            "mixcloud": null,
                            "audio_sources": [
                                { "url": "https://soundcloud.com/ntslive/test", "source": "soundcloud" }
                            ],
                            "brand": {},
                            "embeds": {},
                            "links": []
                        }
                    },
                    "links": []
                },
                "next": {
                    "broadcast_title": "Next Show",
                    "start_timestamp": "2026-02-18T16:00:00Z",
                    "end_timestamp": "2026-02-18T18:00:00Z",
                    "embeds": null,
                    "links": []
                }
            },
            {
                "channel_name": "2",
                "now": {
                    "broadcast_title": "Channel 2 Show",
                    "start_timestamp": "2026-02-18T14:00:00Z",
                    "end_timestamp": "2026-02-18T16:00:00Z",
                    "embeds": null,
                    "links": []
                },
                "next": null
            }
        ],
        "links": [
            { "rel": "self", "href": "https://www.nts.live/api/v2/live", "type": "application/vnd.channels+json" }
        ]
    }"#;

    let resp: NtsLiveResponse =
        serde_json::from_str(json).expect("should deserialize NtsLiveResponse");
    assert_eq!(resp.results.len(), 2);
    assert_eq!(resp.results[0].channel_name, "1");
    assert_eq!(resp.results[0].now.broadcast_title, "Resident Show Name");
    assert!(resp.results[0].next.is_some());
    assert_eq!(resp.results[1].channel_name, "2");
    assert!(resp.results[1].next.is_none());

    // Check nested detail deserialization
    let detail = resp.results[0]
        .now
        .embeds
        .as_ref()
        .unwrap()
        .details
        .as_ref()
        .unwrap();
    assert_eq!(detail.name, "Resident Show Name");
    assert_eq!(detail.genres.as_ref().unwrap().len(), 2);
    assert_eq!(detail.genres.as_ref().unwrap()[0].value, "Ambient");
    assert_eq!(detail.location_long.as_deref(), Some("London"));
    let audio = detail.audio_sources.as_ref().unwrap();
    assert_eq!(audio[0].url, "https://soundcloud.com/ntslive/test");
    assert_eq!(audio[0].source, "soundcloud");
}

#[test]
fn test_nts_collection_response_deserializes() {
    let json = r#"{
        "results": [
            {
                "name": "Episode Title",
                "status": "published",
                "updated": "2026-02-17T10:00:00+00:00",
                "description": "Description text...",
                "description_html": "<p>Description text...</p>",
                "external_links": [],
                "moods": [{ "id": "chill", "value": "Chill" }],
                "genres": [{ "id": "ambient", "value": "Ambient" }],
                "location_short": "BER",
                "location_long": "Berlin",
                "intensity": null,
                "media": null,
                "episode_alias": "episode-title-17th-february-2026",
                "show_alias": "show-name",
                "broadcast": "2026-02-17T14:00:00Z",
                "mixcloud": "https://www.mixcloud.com/NTSRadio/test",
                "audio_sources": [
                    { "url": "https://soundcloud.com/ntslive/ep", "source": "soundcloud" }
                ],
                "brand": {},
                "embeds": {},
                "links": []
            }
        ],
        "links": [
            { "rel": "self", "href": "https://www.nts.live/api/v2/collections/nts-picks", "type": "application/vnd.episode-list+json" }
        ]
    }"#;

    let resp: NtsCollectionResponse =
        serde_json::from_str(json).expect("should deserialize NtsCollectionResponse");
    assert_eq!(resp.results.len(), 1);
    let ep = &resp.results[0];
    assert_eq!(ep.name, "Episode Title");
    assert_eq!(ep.show_alias.as_deref(), Some("show-name"));
    assert_eq!(
        ep.episode_alias.as_deref(),
        Some("episode-title-17th-february-2026")
    );
    assert_eq!(ep.location_long.as_deref(), Some("Berlin"));
    let genres = ep.genres.as_ref().unwrap();
    assert_eq!(genres[0].id, "ambient");
    assert_eq!(genres[0].value, "Ambient");
    let audio = ep.audio_sources.as_ref().unwrap();
    assert_eq!(audio[0].url, "https://soundcloud.com/ntslive/ep");
}

// ── DiscoveryItem ────────────────────────────────────────────────────────────

#[test]
fn test_discovery_item_title() {
    let live = DiscoveryItem::NtsLiveChannel {
        channel: 1,
        show_name: "Ambient Show".to_string(),
        genres: vec!["Ambient".to_string()],
    };
    assert_eq!(live.title(), "Ambient Show");

    let episode = DiscoveryItem::NtsEpisode {
        name: "My Episode".to_string(),
        show_alias: "my-show".to_string(),
        episode_alias: "my-episode-2026".to_string(),
        genres: vec!["Jazz".to_string()],
        location: Some("Berlin".to_string()),
        audio_url: Some("https://soundcloud.com/test".to_string()),
    };
    assert_eq!(episode.title(), "My Episode");

    let direct = DiscoveryItem::DirectUrl {
        url: "https://youtube.com/watch?v=123".to_string(),
        title: Some("My Video".to_string()),
    };
    assert_eq!(direct.title(), "My Video");

    let direct_no_title = DiscoveryItem::DirectUrl {
        url: "https://youtube.com/watch?v=456".to_string(),
        title: None,
    };
    assert_eq!(direct_no_title.title(), "https://youtube.com/watch?v=456");
}

#[test]
fn test_discovery_item_subtitle() {
    let live = DiscoveryItem::NtsLiveChannel {
        channel: 1,
        show_name: "Show".to_string(),
        genres: vec!["Ambient".to_string(), "Drone".to_string()],
    };
    assert_eq!(live.subtitle(), "Ambient, Drone");

    let episode = DiscoveryItem::NtsEpisode {
        name: "My Episode".to_string(),
        show_alias: "my-show".to_string(),
        episode_alias: "my-episode".to_string(),
        genres: vec!["Jazz".to_string()],
        location: Some("Berlin".to_string()),
        audio_url: None,
    };
    assert_eq!(episode.subtitle(), "Jazz · Berlin");

    let episode_no_loc = DiscoveryItem::NtsEpisode {
        name: "My Episode".to_string(),
        show_alias: "my-show".to_string(),
        episode_alias: "my-episode".to_string(),
        genres: vec!["Jazz".to_string()],
        location: None,
        audio_url: None,
    };
    assert_eq!(episode_no_loc.subtitle(), "Jazz");

    let direct = DiscoveryItem::DirectUrl {
        url: "https://youtube.com/watch?v=123".to_string(),
        title: None,
    };
    assert_eq!(direct.subtitle(), "Direct URL");
}

#[test]
fn test_discovery_item_playback_url() {
    let live1 = DiscoveryItem::NtsLiveChannel {
        channel: 1,
        show_name: "Show".to_string(),
        genres: vec![],
    };
    assert_eq!(
        live1.playback_url(),
        Some("https://stream-relay-geo.ntslive.net/stream".to_string())
    );

    let live2 = DiscoveryItem::NtsLiveChannel {
        channel: 2,
        show_name: "Show".to_string(),
        genres: vec![],
    };
    assert_eq!(
        live2.playback_url(),
        Some("https://stream-relay-geo.ntslive.net/stream2".to_string())
    );

    let episode_with_url = DiscoveryItem::NtsEpisode {
        name: "Episode".to_string(),
        show_alias: "show".to_string(),
        episode_alias: "ep".to_string(),
        genres: vec![],
        location: None,
        audio_url: Some("https://soundcloud.com/ntslive/ep".to_string()),
    };
    assert_eq!(
        episode_with_url.playback_url(),
        Some("https://soundcloud.com/ntslive/ep".to_string())
    );

    let episode_no_url = DiscoveryItem::NtsEpisode {
        name: "Episode".to_string(),
        show_alias: "show".to_string(),
        episode_alias: "ep".to_string(),
        genres: vec![],
        location: None,
        audio_url: None,
    };
    assert_eq!(episode_no_url.playback_url(), None);

    let direct = DiscoveryItem::DirectUrl {
        url: "https://youtube.com/watch?v=123".to_string(),
        title: None,
    };
    assert_eq!(
        direct.playback_url(),
        Some("https://youtube.com/watch?v=123".to_string())
    );
}

// ── NTS API Client (integration) ────────────────────────────────────────────

#[tokio::test]
#[ignore = "integration: requires network access"]
async fn test_nts_client_fetch_live() {
    let client = NtsClient::new();
    let items = client
        .fetch_live()
        .await
        .expect("fetch_live should succeed");
    assert_eq!(items.len(), 2, "expected 2 live channels");
    for item in &items {
        match item {
            DiscoveryItem::NtsLiveChannel {
                channel, show_name, ..
            } => {
                assert!(*channel == 1 || *channel == 2, "channel should be 1 or 2");
                assert!(!show_name.is_empty(), "show_name should not be empty");
            }
            _ => panic!("expected NtsLiveChannel variant"),
        }
    }
}

#[tokio::test]
#[ignore = "integration: requires network access"]
async fn test_nts_client_fetch_picks() {
    let client = NtsClient::new();
    let items = client
        .fetch_picks()
        .await
        .expect("fetch_picks should succeed");
    assert!(!items.is_empty(), "picks should not be empty");
    for item in &items {
        match item {
            DiscoveryItem::NtsEpisode { name, .. } => {
                assert!(!name.is_empty(), "episode name should not be empty");
            }
            _ => panic!("expected NtsEpisode variant"),
        }
    }
}

// ── MpvPlayer ────────────────────────────────────────────────────────────────

#[test]
fn test_mpv_player_new() {
    let player = MpvPlayer::new();
    let pid = std::process::id();
    let expected_socket = std::env::temp_dir().join(format!("clisten-mpv-{}.sock", pid));
    assert_eq!(player.socket_path(), expected_socket);
}

#[tokio::test]
#[ignore = "integration: requires mpv installed"]
async fn test_mpv_player_play_spawns_process() {
    let mut player = MpvPlayer::new();
    let result = player
        .play("https://stream-relay-geo.ntslive.net/stream")
        .await;
    assert!(result.is_ok(), "play() should succeed: {:?}", result);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    assert!(
        player.socket_path().exists(),
        "IPC socket should exist after play"
    );
    player.stop().await.ok();
}

// ── Sub-tab coordinator ─────────────────────────────────────────────────────

#[test]
fn test_nts_tab_initial_state() {
    let tab = NtsTab::new();
    assert_eq!(tab.active_sub(), NtsSubTab::Live);
}

#[test]
fn test_nts_tab_lazy_loading() {
    let mut tab = NtsTab::new();

    let actions = tab.switch_sub_tab(0);
    let has_load_live = actions.iter().any(|a| matches!(a, Action::LoadNtsLive));
    assert!(
        has_load_live,
        "first visit to Live should return LoadNtsLive, got: {:?}",
        actions
    );
    assert_eq!(tab.active_sub(), NtsSubTab::Live);

    let actions2 = tab.switch_sub_tab(0);
    assert!(
        actions2.is_empty(),
        "second visit should not trigger load, got: {:?}",
        actions2
    );
}

#[test]
fn test_nts_tab_switch_to_picks() {
    let mut tab = NtsTab::new();

    let actions = tab.switch_sub_tab(1);
    assert_eq!(tab.active_sub(), NtsSubTab::Picks);
    let has_load_picks = actions.iter().any(|a| matches!(a, Action::LoadNtsPicks));
    assert!(
        has_load_picks,
        "first visit to Picks should return LoadNtsPicks, got: {:?}",
        actions
    );
}

#[test]
fn test_nts_tab_switch_to_search() {
    let mut tab = NtsTab::new();

    let actions = tab.switch_sub_tab(2);
    assert_eq!(tab.active_sub(), NtsSubTab::Search);
    let has_load_genres = actions.iter().any(|a| matches!(a, Action::LoadGenres));
    assert!(
        has_load_genres,
        "first visit to Search should return LoadGenres, got: {:?}",
        actions
    );
}

#[test]
fn test_nts_tab_active_index() {
    let mut tab = NtsTab::new();
    assert_eq!(tab.active_index(), 0);
    tab.switch_sub_tab(1);
    assert_eq!(tab.active_index(), 1);
    tab.switch_sub_tab(2);
    assert_eq!(tab.active_index(), 2);
}
