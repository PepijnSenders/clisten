// HTTP client for the NTS Radio public API (live streams, picks, genre search).

use crate::api::models::*;

const NTS_BASE: &str = "https://www.nts.live";

#[derive(Clone, Default)]
pub struct NtsClient {
    http: reqwest::Client,
}

impl NtsClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn fetch_live(&self) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsLiveResponse = self
            .http
            .get(format!("{}/api/v2/live", NTS_BASE))
            .send()
            .await?
            .json()
            .await?;

        let mut items = Vec::new();
        for channel in resp.results {
            let ch_num: u8 = channel.channel_name.parse().unwrap_or(1);
            let broadcast = &channel.now;
            let detail = broadcast.embeds.as_ref().and_then(|e| e.details.as_ref());

            items.push(DiscoveryItem::NtsLiveChannel {
                channel: ch_num,
                show_name: detail
                    .map_or_else(|| broadcast.broadcast_title.clone(), |d| d.name.clone()),
                genres: detail
                    .and_then(|d| d.genres.as_ref())
                    .map_or_else(Vec::new, |g| g.iter().map(|g| g.value.clone()).collect()),
            });
        }
        Ok(items)
    }

    pub async fn fetch_picks(&self) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsCollectionResponse = self
            .http
            .get(format!("{}/api/v2/collections/nts-picks", NTS_BASE))
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.results.into_iter().map(episode_to_discovery).collect())
    }

    pub async fn search_episodes(
        &self,
        genre_id: &str,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsSearchResponse = self
            .http
            .get(format!("{}/api/v2/search/episodes", NTS_BASE))
            .query(&[
                ("offset", offset.to_string()),
                ("limit", limit.to_string()),
                ("genres[]", genre_id.to_string()),
            ])
            .send()
            .await?
            .json()
            .await?;

        Ok(resp
            .results
            .into_iter()
            .map(search_episode_to_discovery)
            .collect())
    }
}

fn episode_to_discovery(ep: NtsEpisodeDetail) -> DiscoveryItem {
    DiscoveryItem::NtsEpisode {
        name: ep.name.clone(),
        show_alias: ep.show_alias.clone().unwrap_or_default(),
        episode_alias: ep.episode_alias.clone().unwrap_or_default(),
        genres: ep
            .genres
            .as_ref()
            .map_or_else(Vec::new, |g| g.iter().map(|g| g.value.clone()).collect()),
        location: ep.location_long.clone(),
        audio_url: ep
            .audio_sources
            .as_ref()
            .and_then(|sources| sources.first())
            .map(|s| s.url.clone()),
    }
}

fn search_episode_to_discovery(ep: NtsSearchEpisode) -> DiscoveryItem {
    let (show_alias, episode_alias) = ep
        .article
        .as_ref()
        .map(|a| {
            let parts: Vec<&str> = a.path.trim_start_matches('/').split('/').collect();
            match parts.as_slice() {
                ["shows", show, "episodes", episode] => (show.to_string(), episode.to_string()),
                _ => (String::new(), String::new()),
            }
        })
        .unwrap_or_default();

    DiscoveryItem::NtsEpisode {
        name: ep.title,
        show_alias,
        episode_alias,
        genres: ep
            .genres
            .as_ref()
            .map_or_else(Vec::new, |g| g.iter().map(|g| g.name.clone()).collect()),
        location: ep.location,
        audio_url: ep
            .audio_sources
            .as_ref()
            .and_then(|sources| sources.first())
            .map(|s| s.url.clone()),
    }
}
