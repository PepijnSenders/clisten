// API response types for NTS endpoints plus DiscoveryItem, the unified type
// that the UI renders. All JSON deserialization happens here.
//
// Response structs mirror the NTS API JSON schema. Many fields exist for
// serde compatibility and are read in tests but not in production code.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct Genre {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudioSource {
    pub url: String,
    pub source: String,
}

// ── Live endpoint ──

#[derive(Debug, Clone, Deserialize)]
pub struct NtsLiveResponse {
    pub results: Vec<NtsChannel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NtsChannel {
    pub channel_name: String,
    pub now: NtsBroadcast,
    pub next: Option<NtsBroadcast>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NtsBroadcast {
    pub broadcast_title: String,
    pub start_timestamp: String,
    pub end_timestamp: String,
    pub embeds: Option<BroadcastEmbeds>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BroadcastEmbeds {
    pub details: Option<NtsEpisodeDetail>,
}

// ── Episode (used by collections and live embeds) ──

#[derive(Debug, Clone, Deserialize)]
pub struct NtsEpisodeDetail {
    pub name: String,
    pub genres: Option<Vec<Genre>>,
    pub location_long: Option<String>,
    pub episode_alias: Option<String>,
    pub show_alias: Option<String>,
    pub audio_sources: Option<Vec<AudioSource>>,
}

// ── Search episodes endpoint ──

#[derive(Debug, Clone, Deserialize)]
pub struct NtsSearchResponse {
    pub results: Vec<NtsSearchEpisode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NtsSearchGenre {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NtsSearchArticle {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NtsSearchEpisode {
    pub title: String,
    pub article: Option<NtsSearchArticle>,
    pub audio_sources: Option<Vec<AudioSource>>,
    pub genres: Option<Vec<NtsSearchGenre>>,
    pub location: Option<String>,
    pub local_date: Option<String>,
}

// ── Collection (nts-picks) ──

#[derive(Debug, Clone, Deserialize)]
pub struct NtsCollectionResponse {
    pub results: Vec<NtsEpisodeDetail>,
}

// ── DiscoveryItem — the unified type rendered in the discovery list ──

const NTS_STREAM_1: &str = "https://stream-relay-geo.ntslive.net/stream";
const NTS_STREAM_2: &str = "https://stream-relay-geo.ntslive.net/stream2";

/// Unified type for everything that can appear in the discovery list.
/// Covers live NTS channels, archived episodes, direct URLs, and genre entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            Self::DirectUrl { title: Some(t), .. } => t,
            Self::DirectUrl { url, .. } => url,
            Self::NtsGenre { name, .. } => name,
        }
    }

    pub fn display_title(&self) -> String {
        match self {
            Self::NtsLiveChannel {
                show_name, channel, ..
            } => {
                format!("NTS {} - {}", channel, show_name)
            }
            Self::NtsEpisode { name, .. } => format!("NTS Radio: {}", name),
            Self::DirectUrl { title: Some(t), .. } => t.clone(),
            Self::DirectUrl { url, .. } => url.clone(),
            Self::NtsGenre { name, .. } => name.clone(),
        }
    }

    pub fn subtitle(&self) -> String {
        match self {
            Self::NtsLiveChannel { genres, .. } => genres.join(", "),
            Self::NtsEpisode {
                genres, location, ..
            } => match location {
                Some(loc) => format!("{} · {}", genres.join(", "), loc),
                None => genres.join(", "),
            },
            Self::DirectUrl { .. } => "Direct URL".to_string(),
            Self::NtsGenre { .. } => "Genre".to_string(),
        }
    }

    pub fn playback_url(&self) -> Option<String> {
        match self {
            Self::NtsLiveChannel { channel: 1, .. } => Some(NTS_STREAM_1.to_string()),
            Self::NtsLiveChannel { channel: 2, .. } => Some(NTS_STREAM_2.to_string()),
            Self::NtsLiveChannel { .. } => None,
            Self::NtsEpisode { audio_url, .. } => audio_url.clone(),
            Self::DirectUrl { url, .. } => Some(url.clone()),
            Self::NtsGenre { .. } => None,
        }
    }

    /// Resolve display title and subtitle, incorporating stream metadata when
    /// available (for DirectUrl items that receive ICY/ID3 tags at runtime).
    ///
    /// `meta_station`, `meta_title`, and `meta_subtitle` are pre-resolved from
    /// StreamMetadata by the caller, keeping this type independent of the player module.
    pub fn display_pair(
        &self,
        meta_station: Option<&str>,
        meta_title: Option<&str>,
        meta_subtitle: Option<&str>,
    ) -> (String, String) {
        if !matches!(self, DiscoveryItem::DirectUrl { .. })
            || (meta_station.is_none() && meta_title.is_none())
        {
            return (self.display_title(), self.subtitle());
        }
        let title = meta_station
            .or(meta_title)
            .map(String::from)
            .unwrap_or_else(|| self.display_title());
        let subtitle = if meta_station.is_some() {
            meta_title
                .map(String::from)
                .unwrap_or_else(|| "Direct URL".to_string())
        } else {
            meta_subtitle
                .map(String::from)
                .unwrap_or_else(|| "Direct URL".to_string())
        };
        (title, subtitle)
    }

}
