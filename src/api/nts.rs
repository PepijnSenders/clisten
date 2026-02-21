// HTTP client for the NTS Radio public API (live streams, picks, genre search).
// Also contains the static TOP_GENRES list (genres with 500+ episodes).

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
                genres: detail
                    .and_then(|d| d.genres.as_ref())
                    .map_or_else(Vec::new, |g| g.iter().map(|g| g.value.clone()).collect()),
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

    pub async fn search_episodes(&self, genre_id: &str, offset: u64, limit: u64) -> anyhow::Result<Vec<DiscoveryItem>> {
        let resp: NtsSearchResponse = self.http
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

        Ok(resp.results.into_iter().map(search_episode_to_discovery).collect())
    }
}

pub fn episode_to_discovery(ep: NtsEpisodeDetail) -> DiscoveryItem {
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
    }
}

fn search_episode_to_discovery(ep: NtsSearchEpisode) -> DiscoveryItem {
    let (show_alias, episode_alias) = ep.article.as_ref()
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
        genres: ep.genres.as_ref()
            .map_or_else(Vec::new, |g| g.iter().map(|g| g.name.clone()).collect()),
        location: ep.location,
        audio_url: ep.audio_sources.as_ref()
            .and_then(|sources| sources.first())
            .map(|s| s.url.clone()),
    }
}

/// Genres with 500+ episodes on NTS, sorted by episode count.
pub const TOP_GENRES: &[(&str, &str)] = &[
    ("housetechno", "House / Techno"),
    ("hiphoprandb", "Hip-Hop / R'n'B"),
    ("postpunkwave", "Post Punk / New Wave"),
    ("soulrhythm", "Soul / Rhythm & Blues"),
    ("ambientnewage", "Ambient / New Age"),
    ("rock", "Rock"),
    ("housetechno-house", "House"),
    ("electronicadowntempo", "Electronica / Downtempo"),
    ("ambientnewage-ambient", "Ambient"),
    ("other", "Other"),
    ("avantgarde", "Avant Garde"),
    ("jazz", "Jazz"),
    ("housetechno-techno", "Techno"),
    ("newclub", "New Club"),
    ("hiphoprandb-hiphop", "Hip Hop"),
    ("altrockpunk", "Alternative Rock / Punk"),
    ("soulrhythm-soul", "Soul"),
    ("discoboogie", "Disco / Boogie"),
    ("electronicadowntempo-electronica", "Electronica"),
    ("avantgarde-experimental", "Experimental"),
    ("caribbean", "Caribbean"),
    ("rock-folk", "Folk"),
    ("ukdance", "UK Dance / Grime"),
    ("newclub-club", "Club"),
    ("hiphoprandb-rnb", "RNB"),
    ("rock-psychedelic-rock", "Psychedelic Rock"),
    ("postpunkwave-synthpop", "Synth Pop"),
    ("postpunkwave-postpunk", "Post Punk"),
    ("discoboogie-classicdisco", "Classic Disco"),
    ("postpunkwave-newwave", "New Wave"),
    ("classicalopera", "Classical / Opera"),
    ("soulrhythm-funk", "Funk"),
    ("caribbean-dub", "Dub"),
    ("altrockpunk-indie", "Indie Rock"),
    ("postpunkwave-minimalsynth", "Minimal Synth"),
    ("hiphoprandb-trap", "Trap"),
    ("housetechno-electro", "Electro"),
    ("discoboogie-boogie", "Boogie"),
    ("africanmiddleeast", "African / Middle Eastern"),
    ("postpunkwave-industrial", "Industrial"),
    ("housetechno-house-deephouse", "Deep House"),
    ("newclub-bass", "Bass"),
    ("electronicadowntempo-beats", "Beats"),
    ("jazz-souljazz", "Soul Jazz"),
    ("housetechno-techno-leftfieldtechno", "Leftfield Techno"),
    ("jazz-straightjazz", "Straight Jazz"),
    ("other-leftfield-pop", "Leftfield Pop"),
    ("avantgarde-drone", "Drone"),
    ("caribbean-reggae", "Reggae"),
    ("jazz-jazzfusion", "Jazz Fusion"),
    ("latinbrazil", "Latin / Brazilian"),
    ("caribbean-dancehall", "Dancehall"),
    ("housetechno-house-leftfieldhouse", "Leftfield House"),
    ("discoboogie-leftfielddisco", "Leftfield Disco"),
    ("other-pop", "Pop"),
    ("ukdance-grime", "Grime"),
    ("jazz-contemporaryjazz", "Contemporary Jazz"),
    ("altrockpunk-dreampop", "Dream Pop"),
    ("avantgarde-noise", "Noise"),
    ("asia", "Asia"),
    ("ambientnewage-newage", "New Age"),
    ("altrockpunk-punk", "Punk"),
    ("jazz-freejazz", "Free Jazz"),
    ("other-talk", "Talk"),
    ("housetechno-techno-ambienttechno", "Ambient Techno"),
    ("other-soundtrack", "Soundtrack"),
    ("metal", "Metal"),
    ("classicalopera-modern-classical", "Modern Classical"),
    ("housetechno-breaks", "Breaks"),
    ("avantgarde-musiqueconcrete", "Musique Concrete"),
    ("jazz-spiritualjazz", "Spiritual Jazz"),
    ("altrockpunk-artrock", "Art Rock"),
    ("newclub-footwork", "Footwork"),
    ("housetechno-acid", "Acid"),
    ("classicalopera-minimalism", "Minimalism"),
    ("ukdance-jungle", "Jungle"),
    ("classicalopera-classical", "Classical"),
    ("rock-classicrock", "Classic Rock"),
    ("ukdance-ukgarage", "Garage"),
    ("ambientnewage-kosmiche", "Kosmische"),
    ("housetechno-brokenbeat", "Broken Beat"),
    ("altrockpunk-noiserock", "Noise Rock"),
    ("africanmiddleeast-afrobeat", "Afrobeat"),
    ("rock-psychadelicfolk", "Psychedelic Folk"),
    ("ukdance-drumandbass", "Drum & Bass"),
    ("rock-soft-rock", "Soft Rock"),
    ("postpunkwave-ebm", "EBM"),
    ("discoboogie-italo", "Italo"),
    ("newclub-afrobeats", "Afrobeats"),
    ("other-spoken-word", "Spoken Word"),
    ("altrockpunk-garagerock", "Garage Rock"),
    ("hiphoprandb-uk-drill", "Drill"),
    ("hiphoprandb-rap", "Rap"),
    ("altrockpunk-shoegaze", "Shoegaze"),
    ("latinbrazil-latinjazz", "Latin Jazz"),
    ("other-live-performance", "Live Performance"),
    ("housetechno-minimal", "Minimal"),
    ("soulrhythm-slowjams", "Slow Jams"),
    ("hiphoprandb-classichiphop", "Classic Hip Hop"),
    ("housetechno-house-balearichouse", "Balearic House"),
    ("housetechno-trance", "Trance"),
    ("metal-heavymetal", "Heavy Metal"),
    ("newclub-reggaeton", "Reggaeton"),
    ("rock-progrock", "Prog Rock"),
    ("electronicadowntempo-glitch", "Glitch"),
    ("avantgarde-darkambient", "Dark Ambient"),
    ("ambientnewage-forthworld", "Fourth World"),
    ("rock-country", "Country"),
    ("hiphoprandb-experimentalhiphop", "Experimental Hip Hop"),
    ("soulrhythm-rhythmandblues", "Rhythm & Blues"),
    ("housetechno-techno-dubtechno", "Dub Techno"),
    ("electronicadowntempo-trip-hop", "Trip Hop"),
    ("altrockpunk-postrock", "Post Rock"),
    ("other-interview", "Interview"),
    ("rock-krautrock", "Krautrock"),
    ("jazz-modal", "Modal"),
    ("jazz-ambientjazz", "Ambient Jazz"),
    ("housetechno-house-detroithouse", "Detroit House"),
    ("asia-jpop", "J-Pop"),
    ("jazz-jazzrock", "Jazz Rock"),
    ("hiphoprandb-ganstarap", "Gangsta Rap"),
];
