## 6. API Reference

### 6.1 NTS Radio API

**Base URL**: `https://www.nts.live`
**Authentication**: None required — all endpoints are public.

#### GET /api/v2/live

Returns current and upcoming broadcasts for both channels.

**Response:**
```json
{
  "results": [
    {
      "channel_name": "1",
      "now": {
        "broadcast_title": "Resident Show Name",
        "start_timestamp": "2026-02-18T14:00:00Z",
        "end_timestamp": "2026-02-18T16:00:00Z",
        "embeds": {
          "details": {
            "status": "published",
            "updated": "2026-02-18T12:00:00+00:00",
            "name": "Resident Show Name",
            "description": "A show about ambient music...",
            "description_html": "<p>A show about ambient music...</p>",
            "external_links": ["https://instagram.com/artist"],
            "moods": [],
            "genres": [
              { "id": "ambient", "value": "Ambient" },
              { "id": "drone", "value": "Drone" }
            ],
            "location_short": "LDN",
            "location_long": "London",
            "intensity": "50",
            "media": {
              "background_large": "https://media.ntslive.co.uk/resize/1600x1600/...",
              "background_medium_large": "https://media.ntslive.co.uk/resize/800x800/...",
              "background_medium": "https://media.ntslive.co.uk/resize/400x400/...",
              "background_small": "https://media.ntslive.co.uk/resize/200x200/...",
              "background_thumb": "https://media.ntslive.co.uk/resize/100x100/...",
              "picture_large": "https://media.ntslive.co.uk/resize/1600x1600/...",
              "picture_medium_large": "https://media.ntslive.co.uk/resize/800x800/...",
              "picture_medium": "https://media.ntslive.co.uk/resize/400x400/...",
              "picture_small": "https://media.ntslive.co.uk/resize/200x200/...",
              "picture_thumb": "https://media.ntslive.co.uk/resize/100x100/..."
            },
            "episode_alias": "resident-show-18th-february-2026",
            "show_alias": "resident-show-name",
            "broadcast": "2026-02-18T14:00:00Z",
            "mixcloud": "https://www.mixcloud.com/NTSRadio/...",
            "audio_sources": [
              { "url": "https://soundcloud.com/ntslive/...", "source": "soundcloud" }
            ],
            "brand": {},
            "embeds": {},
            "links": [
              {
                "rel": "self",
                "href": "https://www.nts.live/api/v2/shows/resident-show-name/episodes/...",
                "type": "application/vnd.episode+json"
              }
            ]
          }
        },
        "links": [
          {
            "href": "https://www.nts.live/api/v2/shows/resident-show-name/episodes/...",
            "rel": "related",
            "type": "application/vnd.episode+json"
          }
        ]
      },
      "next": { "broadcast_title": "...", "start_timestamp": "...", "end_timestamp": "...", "embeds": { "details": { "..." : "..." } }, "links": [] },
      "next2": { "..." : "..." }
    },
    {
      "channel_name": "2",
      "now": { "..." : "..." },
      "next": { "..." : "..." }
    }
  ],
  "links": [
    { "rel": "self", "href": "https://www.nts.live/api/v2/live", "type": "application/vnd.channels+json" }
  ]
}
```

**Notes:**
- Two results: channel `"1"` and channel `"2"`
- Each channel has `now` + `next` through `next17` (18 upcoming slots)
- `embeds.details` on `now` and `next` contains full episode info; later slots may be abbreviated

#### Live Stream URLs

| Channel | URL |
|---|---|
| Channel 1 | `https://stream-relay-geo.ntslive.net/stream` |
| Channel 2 | `https://stream-relay-geo.ntslive.net/stream2` |

#### GET /api/v2/collections/nts-picks

Returns ~15 curated episodes (not paginated).

**Response:**
```json
{
  "results": [
    {
      "status": "published",
      "updated": "2026-02-17T10:00:00+00:00",
      "name": "Episode Title",
      "description": "Description text...",
      "description_html": "<p>Description text...</p>",
      "external_links": [],
      "moods": [{ "id": "chill", "value": "Chill" }],
      "genres": [{ "id": "ambient", "value": "Ambient" }],
      "location_short": "BER",
      "location_long": "Berlin",
      "intensity": null,
      "media": { "background_large": "...", "picture_large": "..." },
      "episode_alias": "episode-title-17th-february-2026",
      "show_alias": "show-name",
      "broadcast": "2026-02-17T14:00:00Z",
      "mixcloud": "https://www.mixcloud.com/NTSRadio/...",
      "audio_sources": [
        { "url": "https://soundcloud.com/ntslive/...", "source": "soundcloud" }
      ],
      "brand": {},
      "embeds": {},
      "links": [
        {
          "rel": "self",
          "href": "https://www.nts.live/api/v2/shows/show-name/episodes/...",
          "type": "application/vnd.episode+json"
        }
      ]
    }
  ],
  "links": [
    { "rel": "self", "href": "https://www.nts.live/api/v2/collections/nts-picks", "type": "application/vnd.episode-list+json" }
  ]
}
```

#### GET /api/v2/collections/recently-added?offset={n}&limit={n}

Same episode structure as nts-picks. Paginated (84,000+ episodes).

**Response wrapper:**
```json
{
  "metadata": { "resultset": { "count": 84000, "offset": 0, "limit": 12 } },
  "results": [ "/* array of Episode objects */" ],
  "links": []
}
```

#### GET /api/v2/shows?offset={n}&limit={n}

Returns paginated list of shows (1,678 total).

**Show object:**
```json
{
  "status": "published",
  "updated": "2026-02-10T09:00:00+00:00",
  "name": "Show Name",
  "description": "A weekly exploration of...",
  "description_html": "<p>A weekly exploration of...</p>",
  "external_links": ["https://instagram.com/showname"],
  "moods": [{ "id": "deep", "value": "Deep" }],
  "genres": [{ "id": "house", "value": "House" }, { "id": "techno", "value": "Techno" }],
  "location_short": "NYC",
  "location_long": "New York",
  "intensity": "50",
  "media": { "background_large": "...", "picture_large": "..." },
  "show_alias": "show-name",
  "timeslot": "MONTHLY",
  "frequency": "MONTHLY",
  "brand": {},
  "type": "show",
  "embeds": {},
  "links": [
    { "rel": "self", "href": "https://www.nts.live/api/v2/shows/show-name", "type": "application/vnd.show+json" },
    { "rel": "episodes", "href": "https://www.nts.live/api/v2/shows/show-name/episodes", "type": "application/vnd.episode-list+json" }
  ]
}
```

#### GET /api/v2/shows/{show_alias}/episodes?offset={n}&limit={n}

Returns episodes for a specific show. Same episode object structure as collections.

#### GET /api/v2/mixtapes

Returns all 18 Infinite Mixtapes (not paginated).

**Mixtape object:**
```json
{
  "mixtape_alias": "poolside",
  "title": "Poolside",
  "subtitle": "Sun-pointed selections",
  "description": "Upbeat and warm sounds...",
  "description_html": "<p>Upbeat and warm sounds...</p>",
  "audio_stream_endpoint": "https://stream-mixtape-geo.ntslive.net/mixtape4",
  "credits": [{ "name": "Show Name", "path": "/shows/show-name" }],
  "media": {
    "animation_large_landscape": "...", "animation_large_portrait": "...",
    "animation_thumb": "...", "icon_black": "...", "icon_white": "...",
    "picture_large": "...", "picture_medium": "...", "picture_medium_large": "...",
    "picture_small": "...", "picture_thumb": "..."
  },
  "now_playing_topic": "",
  "links": [
    { "rel": "self", "href": "https://www.nts.live/api/v2/mixtapes/poolside", "type": "application/vnd.mixtape+json" }
  ]
}
```

**All 18 Mixtapes:** Poolside, Slow Focus, Low Key, Memory Lane, 4 To The Floor, Island Time, The Tube, Sheet Music, Feelings, Expansions, Rap House, Labyrinth, Sweat, Otaku, The Pit, Field Recordings (+2 more).

#### NTS Endpoint Summary

| Endpoint | Returns | Paginated |
|---|---|---|
| `GET /api/v2/live` | Channels 1 & 2, now + next 17 broadcasts | No |
| `GET /api/v2/collections/nts-picks` | ~15 curated episodes | No |
| `GET /api/v2/collections/recently-added?offset=&limit=` | Episodes | Yes (84k+) |
| `GET /api/v2/shows?offset=&limit=` | Shows with genres/moods | Yes (1678) |
| `GET /api/v2/shows/{alias}/episodes?offset=&limit=` | Episodes for a show | Yes |
| `GET /api/v2/mixtapes` | 18 mixtapes with `audio_stream_endpoint` | No |

#### Stream URL Resolution

| Content Type | How to Play |
|---|---|
| Live stream | Direct URL: `https://stream-relay-geo.ntslive.net/stream` or `/stream2` |
| Mixtape | Direct URL from `audio_stream_endpoint` field |
| Episode | Pass `audio_sources[0].url` to mpv (yt-dlp resolves SoundCloud/Mixcloud) |


## Appendix: NTS API Client

```rust
// src/api/nts.rs

use crate::api::models::*;

const NTS_BASE: &str = "https://www.nts.live";

#[derive(Clone)]
pub struct NtsClient {
    http: reqwest::Client,
}

impl NtsClient {
    pub fn new() -> Self {
        Self { http: reqwest::Client::new() }
    }

    pub async fn fetch_live(&self) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsLiveResponse = self.http
            .get(format!("{}/api/v2/live", NTS_BASE))
            .send()
            .await?
            .json()
            .await?;

        let mut items = Vec::new();
        for channel in resp.results {
            let ch_num: u8 = channel.channel_name.parse().unwrap_or(1);
            let broadcast = &channel.now;
            let detail = broadcast.embeds.as_ref()
                .and_then(|e| e.details.as_ref());

            items.push(DiscoveryItem::NtsLiveChannel {
                channel: ch_num,
                show_name: detail.map_or_else(
                    || broadcast.broadcast_title.clone(),
                    |d| d.name.clone(),
                ),
                broadcast_title: broadcast.broadcast_title.clone(),
                genres: detail
                    .and_then(|d| d.genres.as_ref())
                    .map_or_else(Vec::new, |g| g.iter().map(|g| g.value.clone()).collect()),
                start: broadcast.start_timestamp.clone(),
                end: broadcast.end_timestamp.clone(),
            });
        }
        Ok(items)
    }

    pub async fn fetch_picks(&self) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsCollectionResponse = self.http
            .get(format!("{}/api/v2/collections/nts-picks", NTS_BASE))
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.results.into_iter().map(episode_to_discovery).collect())
    }

    pub async fn fetch_recent(&self, offset: u64, limit: u64) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsCollectionResponse = self.http
            .get(format!("{}/api/v2/collections/recently-added", NTS_BASE))
            .query(&[("offset", offset.to_string()), ("limit", limit.to_string())])
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.results.into_iter().map(episode_to_discovery).collect())
    }

    pub async fn fetch_shows(&self, offset: u64, limit: u64) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsShowsResponse = self.http
            .get(format!("{}/api/v2/shows", NTS_BASE))
            .query(&[("offset", offset.to_string()), ("limit", limit.to_string())])
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.results.into_iter().map(|show| {
            DiscoveryItem::NtsShow {
                name: show.name.clone(),
                show_alias: show.show_alias.clone(),
                genres: show.genres.as_ref()
                    .map_or_else(Vec::new, |g| g.iter().map(|g| g.value.clone()).collect()),
                location: show.location_long.clone(),
                description: show.description.clone(),
            }
        }).collect())
    }

    pub async fn fetch_show_episodes(
        &self,
        show_alias: &str,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsCollectionResponse = self.http
            .get(format!("{}/api/v2/shows/{}/episodes", NTS_BASE, show_alias))
            .query(&[("offset", offset.to_string()), ("limit", limit.to_string())])
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.results.into_iter().map(episode_to_discovery).collect())
    }

    pub async fn fetch_mixtapes(&self) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsMixtapesResponse = self.http
            .get(format!("{}/api/v2/mixtapes", NTS_BASE))
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.results.into_iter().map(|m| {
            DiscoveryItem::NtsMixtape {
                title: m.title,
                subtitle: m.subtitle,
                stream_url: m.audio_stream_endpoint,
                mixtape_alias: m.mixtape_alias,
            }
        }).collect())
    }
}

fn episode_to_discovery(ep: NtsEpisodeDetail) -> DiscoveryItem {
    DiscoveryItem::NtsEpisode {
        name: ep.name.clone(),
        show_alias: ep.show_alias.clone().unwrap_or_default(),
        episode_alias: ep.episode_alias.clone().unwrap_or_default(),
        genres: ep.genres.as_ref()
            .map_or_else(Vec::new, |g| g.iter().map(|g| g.value.clone()).collect()),
        location: ep.location_long.clone(),
        audio_url: ep.audio_sources.as_ref()
            .and_then(|sources| sources.first())
            .map(|s| s.url.clone()),
        description: ep.description.clone(),
    }
}
```
