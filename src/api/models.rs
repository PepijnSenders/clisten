// src/api/models.rs

use serde::{Deserialize, Serialize};

// ── Shared types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiLink {
    pub rel: String,
    pub href: String,
    #[serde(rename = "type")]
    pub link_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genre {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsMedia {
    pub background_large: Option<String>,
    pub background_medium_large: Option<String>,
    pub background_medium: Option<String>,
    pub background_small: Option<String>,
    pub background_thumb: Option<String>,
    pub picture_large: Option<String>,
    pub picture_medium_large: Option<String>,
    pub picture_medium: Option<String>,
    pub picture_small: Option<String>,
    pub picture_thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSource {
    pub url: String,
    pub source: String,
}

// ── Pagination ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSet {
    pub count: u64,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMetadata {
    pub resultset: ResultSet,
}

// ── Live endpoint ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsLiveResponse {
    pub results: Vec<NtsChannel>,
    pub links: Vec<ApiLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsChannel {
    pub channel_name: String,
    pub now: NtsBroadcast,
    pub next: Option<NtsBroadcast>,
    // next2 through next17 — flatten into a method or use serde_json::Value
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl NtsChannel {
    /// Extract all "nextN" broadcasts from the extra fields.
    #[allow(dead_code)]
    pub fn upcoming(&self) -> Vec<NtsBroadcast> {
        let mut broadcasts = Vec::new();
        if let Some(next) = &self.next {
            broadcasts.push(next.clone());
        }
        for i in 2..=17 {
            let key = format!("next{}", i);
            if let Some(val) = self.extra.get(&key) {
                if let Ok(b) = serde_json::from_value::<NtsBroadcast>(val.clone()) {
                    broadcasts.push(b);
                }
            }
        }
        broadcasts
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsBroadcast {
    pub broadcast_title: String,
    pub start_timestamp: String,
    pub end_timestamp: String,
    pub embeds: Option<BroadcastEmbeds>,
    pub links: Vec<ApiLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastEmbeds {
    pub details: Option<NtsEpisodeDetail>,
}

// ── Episode (used by collections, shows, live embeds) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsEpisodeDetail {
    pub status: Option<String>,
    pub updated: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub description_html: Option<String>,
    pub external_links: Option<Vec<String>>,
    pub moods: Option<Vec<Genre>>,
    pub genres: Option<Vec<Genre>>,
    pub location_short: Option<String>,
    pub location_long: Option<String>,
    pub intensity: Option<String>,
    pub media: Option<NtsMedia>,
    pub episode_alias: Option<String>,
    pub show_alias: Option<String>,
    pub broadcast: Option<String>,
    pub mixcloud: Option<String>,
    pub audio_sources: Option<Vec<AudioSource>>,
    pub brand: Option<serde_json::Value>,
    pub embeds: Option<serde_json::Value>,
    pub links: Option<Vec<ApiLink>>,
}

// ── Genres ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsGenresResponse {
    pub results: Vec<NtsGenreCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsGenreCategory {
    pub id: String,
    pub name: String,
    pub subgenres: Vec<NtsSubgenre>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsSubgenre {
    pub id: String,
    pub name: String,
}

// ── Search episodes endpoint ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsSearchResponse {
    pub metadata: Option<PaginationMetadata>,
    pub results: Vec<NtsSearchEpisode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsSearchGenre {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsSearchArticle {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsSearchEpisode {
    pub title: String,
    pub article: Option<NtsSearchArticle>,
    pub audio_sources: Option<Vec<AudioSource>>,
    pub genres: Option<Vec<NtsSearchGenre>>,
    pub location: Option<String>,
    pub local_date: Option<String>,
}

// ── Collection (nts-picks, recently-added) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsCollectionResponse {
    pub metadata: Option<PaginationMetadata>,
    pub results: Vec<NtsEpisodeDetail>,
    pub links: Vec<ApiLink>,
}

// ── Shows ──

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsShowsResponse {
    pub metadata: Option<PaginationMetadata>,
    pub results: Vec<NtsShow>,
    pub links: Vec<ApiLink>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsShow {
    pub status: Option<String>,
    pub updated: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub description_html: Option<String>,
    pub external_links: Option<Vec<String>>,
    pub moods: Option<Vec<Genre>>,
    pub genres: Option<Vec<Genre>>,
    pub location_short: Option<String>,
    pub location_long: Option<String>,
    pub intensity: Option<String>,
    pub media: Option<NtsMedia>,
    pub show_alias: String,
    pub timeslot: Option<String>,
    pub frequency: Option<String>,
    pub brand: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub show_type: Option<String>,
    pub embeds: Option<serde_json::Value>,
    pub links: Option<Vec<ApiLink>>,
}

// ── Mixtapes ──

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsMixtapesResponse {
    pub metadata: Option<serde_json::Value>,
    pub results: Vec<NtsMixtape>,
    pub links: Vec<ApiLink>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixtapeCredit {
    pub name: String,
    pub path: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixtapeMedia {
    pub animation_large_landscape: Option<String>,
    pub animation_large_portrait: Option<String>,
    pub animation_thumb: Option<String>,
    pub icon_black: Option<String>,
    pub icon_white: Option<String>,
    pub picture_large: Option<String>,
    pub picture_medium: Option<String>,
    pub picture_medium_large: Option<String>,
    pub picture_small: Option<String>,
    pub picture_thumb: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsMixtape {
    pub mixtape_alias: String,
    pub title: String,
    pub subtitle: String,
    pub description: Option<String>,
    pub description_html: Option<String>,
    pub audio_stream_endpoint: String,
    pub credits: Vec<MixtapeCredit>,
    pub media: Option<MixtapeMedia>,
    pub now_playing_topic: Option<String>,
    pub links: Option<Vec<ApiLink>>,
}

// ── DiscoveryItem — Universal UI Type ──

/// Universal type bridging API models to the UI.
/// The discovery list renders Vec<DiscoveryItem> regardless of source.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DiscoveryItem {
    NtsLiveChannel {
        channel: u8, // 1 or 2
        show_name: String,
        broadcast_title: String,
        genres: Vec<String>,
        start: String,
        end: String,
    },
    NtsEpisode {
        name: String,
        show_alias: String,
        episode_alias: String,
        genres: Vec<String>,
        location: Option<String>,
        audio_url: Option<String>, // from audio_sources[0].url
        description: Option<String>,
    },
    NtsMixtape {
        title: String,
        subtitle: String,
        stream_url: String,
        mixtape_alias: String,
    },
    NtsShow {
        name: String,
        show_alias: String,
        genres: Vec<String>,
        location: Option<String>,
        description: Option<String>,
    },
    DirectUrl {
        url: String,
        title: Option<String>,
    },
    NtsGenre {
        name: String,
        genre_id: String,
    },
}

impl DiscoveryItem {
    /// Display title for the discovery list.
    pub fn title(&self) -> &str {
        match self {
            Self::NtsLiveChannel { show_name, .. } => show_name,
            Self::NtsEpisode { name, .. } => name,
            Self::NtsMixtape { title, .. } => title,
            Self::NtsShow { name, .. } => name,
            Self::DirectUrl { title: Some(t), .. } => t,
            Self::DirectUrl { url, .. } => url,
            Self::NtsGenre { name, .. } => name,
        }
    }

    /// Display title with source prefix for NTS content.
    pub fn display_title(&self) -> String {
        match self {
            Self::NtsLiveChannel { show_name, channel, .. } => {
                format!("NTS {} - {}", channel, show_name)
            }
            Self::NtsEpisode { name, .. } => format!("NTS Radio: {}", name),
            Self::NtsMixtape { title, .. } => format!("NTS Radio: {}", title),
            Self::NtsShow { name, .. } => format!("NTS Radio: {}", name),
            Self::DirectUrl { title: Some(t), .. } => t.clone(),
            Self::DirectUrl { url, .. } => url.clone(),
            Self::NtsGenre { name, .. } => name.clone(),
        }
    }

    /// Secondary info line (artist, subtitle, genres).
    pub fn subtitle(&self) -> String {
        match self {
            Self::NtsLiveChannel { genres, .. } => genres.join(", "),
            Self::NtsEpisode { genres, location, .. } => {
                let mut parts = vec![genres.join(", ")];
                if let Some(loc) = location {
                    parts.push(loc.clone());
                }
                parts.join(" · ")
            }
            Self::NtsMixtape { subtitle, .. } => subtitle.clone(),
            Self::NtsShow { genres, location, .. } => {
                let mut parts = vec![genres.join(", ")];
                if let Some(loc) = location {
                    parts.push(loc.clone());
                }
                parts.join(" · ")
            }
            Self::DirectUrl { .. } => "Direct URL".to_string(),
            Self::NtsGenre { .. } => "Genre".to_string(),
        }
    }

    /// The URL to pass to mpv for playback.
    pub fn playback_url(&self) -> Option<String> {
        match self {
            Self::NtsLiveChannel { channel, .. } => Some(match channel {
                1 => "https://stream-relay-geo.ntslive.net/stream".to_string(),
                2 => "https://stream-relay-geo.ntslive.net/stream2".to_string(),
                _ => return None,
            }),
            Self::NtsEpisode { audio_url, .. } => audio_url.clone(),
            Self::NtsMixtape { stream_url, .. } => Some(stream_url.clone()),
            Self::NtsShow { .. } => None, // drill-down, not playable
            Self::DirectUrl { url, .. } => Some(url.clone()),
            Self::NtsGenre { .. } => None, // not playable, triggers search
        }
    }

    /// Unique key for favorites lookup.
    pub fn favorite_key(&self) -> String {
        match self {
            Self::NtsLiveChannel { channel, .. } => format!("nts:live:{}", channel),
            Self::NtsEpisode { show_alias, episode_alias, .. } => {
                format!("nts:episode:{}:{}", show_alias, episode_alias)
            }
            Self::NtsMixtape { mixtape_alias, .. } => format!("nts:mixtape:{}", mixtape_alias),
            Self::NtsShow { show_alias, .. } => format!("nts:show:{}", show_alias),
            Self::DirectUrl { url, .. } => format!("direct:{}", url),
            Self::NtsGenre { genre_id, .. } => format!("genre:{}", genre_id),
        }
    }
}
