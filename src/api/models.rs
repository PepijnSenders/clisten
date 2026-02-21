// API response types for NTS endpoints plus DiscoveryItem, the unified type
// that the UI renders. All JSON deserialization happens here.

use serde::{Deserialize, Serialize};

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
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
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

// ── Episode (used by collections and live embeds) ──

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

// ── Collection (nts-picks) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtsCollectionResponse {
    pub metadata: Option<PaginationMetadata>,
    pub results: Vec<NtsEpisodeDetail>,
    pub links: Vec<ApiLink>,
}

// ── DiscoveryItem — the unified type rendered in the discovery list ──

#[derive(Debug, Clone)]
pub enum DiscoveryItem {
    NtsLiveChannel {
        channel: u8,
        show_name: String,
        genres: Vec<String>,
    },
    NtsEpisode {
        name: String,
        show_alias: String,
        episode_alias: String,
        genres: Vec<String>,
        location: Option<String>,
        audio_url: Option<String>,
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

    pub fn subtitle(&self) -> String {
        match self {
            Self::NtsLiveChannel { genres, .. } => genres.join(", "),
            Self::NtsEpisode { genres, location, .. } | Self::NtsShow { genres, location, .. } => {
                let mut parts = vec![genres.join(", ")];
                if let Some(loc) = location {
                    parts.push(loc.clone());
                }
                parts.join(" · ")
            }
            Self::NtsMixtape { subtitle, .. } => subtitle.clone(),
            Self::DirectUrl { .. } => "Direct URL".to_string(),
            Self::NtsGenre { .. } => "Genre".to_string(),
        }
    }

    pub fn playback_url(&self) -> Option<String> {
        match self {
            Self::NtsLiveChannel { channel, .. } => Some(match channel {
                1 => "https://stream-relay-geo.ntslive.net/stream".to_string(),
                2 => "https://stream-relay-geo.ntslive.net/stream2".to_string(),
                _ => return None,
            }),
            Self::NtsEpisode { audio_url, .. } => audio_url.clone(),
            Self::NtsMixtape { stream_url, .. } => Some(stream_url.clone()),
            Self::NtsShow { .. } | Self::NtsGenre { .. } => None,
            Self::DirectUrl { url, .. } => Some(url.clone()),
        }
    }

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
