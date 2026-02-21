### 6.2 SoundCloud API

**Base URL**: `https://api-v2.soundcloud.com`
**Authentication**: All requests require `client_id` query parameter. User-specific endpoints also require `Authorization: OAuth {token}` header.

#### GET /search/tracks?q={query}&limit={n}&offset={n}&client_id={id}

```json
{
  "collection": [
    {
      "id": 123456789,
      "urn": "soundcloud:tracks:123456789",
      "title": "Track Title",
      "user": { "id": 987654, "username": "Artist Name", "permalink": "artist-name" },
      "duration": 240000,
      "genre": "Electronic",
      "tag_list": "ambient chill",
      "description": "Track description text",
      "artwork_url": "https://i1.sndcdn.com/.../artworks-...-large.jpg",
      "permalink_url": "https://soundcloud.com/artist/track-name",
      "stream_url": "https://api-v2.soundcloud.com/tracks/123456789/streams",
      "playback_count": 50000,
      "likes_count": 1200
    }
  ],
  "total_results": 12345,
  "next_href": "https://api-v2.soundcloud.com/search/tracks?q=...&offset=50&limit=50&client_id=..."
}
```

#### GET /resolve?url={encoded_url}&client_id={id}

Resolves a SoundCloud permalink to a full track object. Returns the same track structure as search.

#### GET /me/likes/tracks?limit={n}&offset={n}&client_id={id}

Requires `Authorization: OAuth {token}` header. Returns same `collection` structure.

#### GET /tracks/{urn}/streams?client_id={id}

```json
{
  "hls_aac_160_url": "https://playback.media-streaming.soundcloud.cloud/.../playlist.m3u8",
  "hls_aac_96_url": "https://playback.media-streaming.soundcloud.cloud/.../playlist.m3u8",
  "http_mp3_128_url": "https://...",
  "hls_mp3_128_url": "https://...",
  "hls_opus_64_url": "https://..."
}
```

**Note:** Stream URLs are single-use (403 on second access). MP3/Opus deprecated — use `hls_aac_160_url`.

#### SoundCloud Playback Strategy

**Primary approach** (simple, reliable): Pass `permalink_url` to mpv. mpv uses yt-dlp internally to resolve the stream. **Alternative** (more control): Resolve via `/tracks/{urn}/streams`, pass HLS URL to mpv directly.

**Note:** The `/resolve` and `/tracks/{urn}/streams` endpoints are documented above for reference but intentionally not implemented in the SoundCloudClient. The primary playback approach (mpv + yt-dlp) makes them unnecessary. The `SoundCloudStreamResponse` serde type is provided for future use if direct stream resolution is desired.

---


## 13. SoundCloud client_id Extraction

The `client_id` is required for all SoundCloud API calls but is not publicly documented. It must be extracted from SoundCloud's JavaScript bundles and refreshed periodically.

### Algorithm

1. Fetch `https://soundcloud.com` (the homepage HTML)
2. Parse the HTML for `<script crossorigin src="https://a-v2.sndcdn.com/assets/0-abc123.js">` tags
3. Iterate through the script URLs **in reverse order** (the client_id is typically in one of the later bundles)
4. Fetch each JS bundle
5. Search the JS source for the pattern `client_id:"[alphanumeric]"` using regex
6. Extract and return the matched client_id

### Code

```rust
// src/api/soundcloud.rs

use regex::Regex;
use std::time::Instant;

pub struct SoundCloudClient {
    http: reqwest::Client,
    client_id: Option<String>,
    client_id_fetched_at: Option<Instant>,
    oauth_token: Option<String>,
}

const CLIENT_ID_TTL_SECS: u64 = 4 * 3600; // 4 hours

impl SoundCloudClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            client_id: None,
            client_id_fetched_at: None,
            oauth_token: None,
        }
    }

    /// Ensure client_id is valid. Fetch if missing or expired.
    pub async fn ensure_client_id(&mut self) -> anyhow::Result<&str> {
        let expired = self.client_id_fetched_at
            .map_or(true, |t| t.elapsed().as_secs() > CLIENT_ID_TTL_SECS);

        if self.client_id.is_none() || expired {
            let id = self.fetch_client_id().await?;
            self.client_id = Some(id);
            self.client_id_fetched_at = Some(Instant::now());
        }

        Ok(self.client_id.as_ref().unwrap())
    }

    /// Extract client_id from SoundCloud's JS bundles.
    async fn fetch_client_id(&self) -> anyhow::Result<String> {
        // Step 1: Fetch SoundCloud homepage
        let html = self.http
            .get("https://soundcloud.com")
            .send()
            .await?
            .text()
            .await?;

        // Step 2: Find script URLs
        let script_re = Regex::new(
            r#"<script[^>]+src="(https://a-v2\.sndcdn\.com/assets/[^"]+\.js)"#
        )?;

        let script_urls: Vec<String> = script_re
            .captures_iter(&html)
            .map(|cap| cap[1].to_string())
            .collect();

        if script_urls.is_empty() {
            anyhow::bail!("No SoundCloud JS bundles found");
        }

        // Step 3: Search bundles in reverse (client_id is usually in later bundles)
        let client_id_re = Regex::new(r#"client_id:"([a-zA-Z0-9]{32})""#)?;

        for url in script_urls.iter().rev() {
            let js = match self.http.get(url).send().await {
                Ok(resp) => match resp.text().await {
                    Ok(text) => text,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            if let Some(captures) = client_id_re.captures(&js) {
                return Ok(captures[1].to_string());
            }
        }

        anyhow::bail!("Could not extract SoundCloud client_id from any JS bundle")
    }

    pub fn set_oauth_token(&mut self, token: String) {
        self.oauth_token = Some(token);
    }

    /// Search tracks.
    pub async fn search_tracks(
        &mut self,
        query: &str,
        limit: u32,
        offset: u32,
    ) -> anyhow::Result<Vec<crate::api::models::SoundCloudTrack>> {
        let client_id = self.ensure_client_id().await?.to_string();
        let resp: crate::api::models::SoundCloudSearchResponse = self.http
            .get("https://api-v2.soundcloud.com/search/tracks")
            .query(&[
                ("q", query),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
                ("client_id", &client_id),
            ])
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.collection)
    }

    /// Get user's liked tracks (requires OAuth token).
    pub async fn get_likes(
        &mut self,
        limit: u32,
        offset: u32,
    ) -> anyhow::Result<Vec<crate::api::models::SoundCloudTrack>> {
        let client_id = self.ensure_client_id().await?.to_string();
        let token = self.oauth_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No OAuth token set. Run: clisten auth soundcloud"))?;

        let resp: crate::api::models::SoundCloudSearchResponse = self.http
            .get("https://api-v2.soundcloud.com/me/likes/tracks")
            .query(&[
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
                ("client_id", &client_id),
            ])
            .header("Authorization", format!("OAuth {}", token))
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.collection)
    }
}

impl Clone for SoundCloudClient {
    fn clone(&self) -> Self {
        Self {
            http: self.http.clone(),
            client_id: self.client_id.clone(),
            client_id_fetched_at: self.client_id_fetched_at,
            oauth_token: self.oauth_token.clone(),
        }
    }
}
```

### Refresh Strategy

- Cache the `client_id` with a timestamp
- TTL: 4 hours
- On 401/403 from SoundCloud API: force-refresh client_id and retry once
- Lazy initialization: don't fetch at app startup — fetch on first SoundCloud tab activation

---

## 14. SoundCloud Auth Flow

SoundCloud's official OAuth is deprecated (no new API app registrations). The practical approach:

### Step-by-Step

1. User runs `clisten auth soundcloud`
2. App opens `https://soundcloud.com` in default browser
3. App prints instructions to terminal
4. User logs into SoundCloud in browser
5. User opens DevTools → Network tab
6. User finds any request to `api-v2.soundcloud.com`
7. User copies the `Authorization` header value
8. User pastes it into the terminal
9. App strips the `OAuth ` prefix, saves to `~/.config/clisten/config.toml`

### Code

```rust
// In src/main.rs, as part of the CLI subcommand handler

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clisten", about = "NTS Radio & SoundCloud TUI player")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with a service
    Auth {
        /// Service to authenticate with (soundcloud)
        provider: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Auth { provider }) => {
            match provider.as_str() {
                "soundcloud" => {
                    println!("Opening SoundCloud in your browser...");
                    println!();
                    println!("Instructions:");
                    println!("  1. Log in to SoundCloud");
                    println!("  2. Open DevTools (F12) → Network tab");
                    println!("  3. Look for any request to api-v2.soundcloud.com");
                    println!("  4. Copy the 'Authorization' header value");
                    println!("  5. Paste it below and press Enter:");
                    println!();
                    open::that("https://soundcloud.com")?;
                    let mut token = String::new();
                    std::io::stdin().read_line(&mut token)?;
                    let token = token.trim().trim_start_matches("OAuth ").to_string();
                    if token.is_empty() {
                        eprintln!("No token provided.");
                        std::process::exit(1);
                    }
                    crate::config::save_soundcloud_token(&token)?;
                    println!("Token saved to {:?}", crate::config::Config::config_path());
                    println!("Note: SoundCloud tokens expire in ~6 hours.");
                }
                other => {
                    eprintln!("Unknown provider: {}. Supported: soundcloud", other);
                    std::process::exit(1);
                }
            }
        }
        None => {
            // Check dependencies, then run the TUI
            check_dependencies()?;

            let config = crate::config::Config::load()?;
            crate::logging::init(&config)?;

            let mut app = crate::app::App::new(config)?;
            app.run().await?;
        }
    }

    Ok(())
}

fn check_dependencies() -> anyhow::Result<()> {
    if which::which("mpv").is_err() {
        eprintln!("Error: mpv is required but not found in PATH.");
        eprintln!("Install it with: brew install mpv");
        std::process::exit(1);
    }
    if which::which("yt-dlp").is_err() {
        eprintln!("Warning: yt-dlp not found. SoundCloud/Mixcloud playback will not work.");
        eprintln!("Install it with: brew install yt-dlp");
    }
    Ok(())
}
```

### Token Characteristics

- Format: opaque string (transitioning to JWT)
- TTL: ~6 hours
- HTTP header: `Authorization: OAuth {token}`
- Stored in `~/.config/clisten/config.toml` under `[soundcloud]`

---
