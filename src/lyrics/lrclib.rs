use crate::lyrics::model::SyncedLyrics;
use crate::lyrics::parser::LyricsParser;
use crate::lyrics::provider::{LyricsError, LyricsProvider};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, info};

#[derive(Debug, Deserialize)]
struct LrclibResponse {
    #[serde(rename = "trackName")]
    track_name: Option<String>,
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(default)]
    instrumental: bool,
}

pub struct LrclibProvider {
    client: Client,
}

impl LrclibProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(6))
            .user_agent("Verci-Linux/0.1.0 (https://github.com/verci-linux)")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    pub fn sanitize_title(title: &str) -> String {
        let mut clean = title.to_string();
        // If pipe separated (common on YouTube, e.g. "Song | Artist | Movie"), take the first segment
        if let Some((first, _)) = clean.split_once('|') {
            clean = first.trim().to_string();
        }
        let patterns = [
            " (from \"",
            " (from ",
            " [from \"",
            " [from ",
            " (feat.",
            " [feat.",
            " (feat ",
            " [feat ",
            " (ft.",
            " [ft.",
            " (ft ",
            " [ft ",
            " feat.",
            " ft.",
            " (with ",
            " [with ",
            " (prod.",
            " [prod.",
            " (prod by",
            " [prod by",
            " (official video)",
            " [official video]",
            " (official music video)",
            " [official music video]",
            " (official audio)",
            " [official audio]",
            " (audio)",
            " [audio]",
            " (lyric video)",
            " [lyric video]",
            " (lyrics)",
            " [lyrics]",
            " (visualizer)",
            " [visualizer]",
            " (cover)",
            " [cover]",
            " (full song)",
            " [full song]",
            " (full video)",
            " [full video]",
            " | official video",
            " | lyrical video",
            " | coke studio",
            " - remastered",
            " [remastered]",
            " (remastered)",
            " (original soundtrack)",
            " (original motion picture soundtrack)",
            " (soundtrack version)",
            " (slowed + reverb)",
            " (slowed and reverb)",
            " (sped up)",
            " (speed up)",
            " (4k)",
            " [4k]",
            " (hd)",
            " [hd]",
            " - lyrical",
            " (lyrical)",
        ];
        for p in patterns {
            if let Some(pos) = clean.to_lowercase().find(p) {
                clean.truncate(pos);
            }
        }
        clean
            .trim()
            .trim_matches(|c| c == '"' || c == '\'' || c == '-' || c == ':')
            .trim()
            .to_string()
    }

    pub fn primary_artist(artist: &str) -> String {
        let mut a = artist.to_string();
        if let Some((first, _)) = a.split_once(',') {
            a = first.trim().to_string();
        }
        if let Some((first, _)) = a.split_once('&') {
            a = first.trim().to_string();
        }
        for sep in [" feat.", " ft.", " vs.", " x "] {
            if let Some(pos) = a.to_lowercase().find(sep) {
                a.truncate(pos);
            }
        }
        a.trim()
            .trim_matches(|c| c == '"' || c == '\'' || c == '-')
            .trim()
            .to_string()
    }

    async fn search_query(&self, query: &str) -> Result<Vec<LrclibResponse>, reqwest::Error> {
        let resp = self
            .client
            .get("https://lrclib.net/api/search")
            .query(&[("q", query)])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        resp.json::<Vec<LrclibResponse>>().await
    }

    fn pick_best_match(
        &self,
        results: &[LrclibResponse],
        clean_title: &str,
    ) -> Option<SyncedLyrics> {
        if results.is_empty() {
            return None;
        }

        let clean_lower = clean_title.to_lowercase();
        let best_match = results
            .iter()
            .find(|r| {
                r.synced_lyrics.is_some()
                    && r.track_name
                        .as_deref()
                        .map(|tn| {
                            let tn_l = tn.to_lowercase();
                            tn_l == clean_lower
                                || tn_l.contains(&clean_lower)
                                || clean_lower.contains(&tn_l)
                        })
                        .unwrap_or(false)
            })
            .or_else(|| {
                results.iter().find(|r| {
                    r.track_name
                        .as_deref()
                        .map(|tn| {
                            let tn_l = tn.to_lowercase();
                            tn_l == clean_lower
                                || tn_l.contains(&clean_lower)
                                || clean_lower.contains(&tn_l)
                        })
                        .unwrap_or(false)
                })
            })
            .or_else(|| results.iter().find(|r| r.synced_lyrics.is_some()))
            .or_else(|| results.iter().find(|r| r.plain_lyrics.is_some()));

        if let Some(match_item) = best_match {
            info!(
                "Found LRCLIB match: '{}' by '{}' (has_synced: {})",
                match_item.track_name.as_deref().unwrap_or(""),
                match_item.artist_name.as_deref().unwrap_or(""),
                match_item.synced_lyrics.is_some()
            );

            if match_item.instrumental {
                return Some(LyricsParser::parse_plain_text("[Instrumental]"));
            }
            if let Some(ref synced) = match_item.synced_lyrics {
                return Some(LyricsParser::parse_lrc(synced));
            }
            if let Some(ref plain) = match_item.plain_lyrics {
                return Some(LyricsParser::parse_plain_text(plain));
            }
        }

        None
    }
    async fn query_exact(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<SyncedLyrics, LyricsError> {
        let mut req = self
            .client
            .get("https://lrclib.net/api/get")
            .query(&[("artist_name", artist), ("track_name", title)]);

        if let Some(alb) = album {
            if !alb.is_empty() {
                req = req.query(&[("album_name", alb)]);
            }
        }

        let resp = req
            .send()
            .await
            .map_err(|e| LyricsError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(LyricsError::NotFound(artist.to_string(), title.to_string()));
        }

        let data = resp
            .json::<LrclibResponse>()
            .await
            .map_err(|e| LyricsError::Network(e.to_string()))?;

        if data.instrumental {
            return Ok(LyricsParser::parse_plain_text("[Instrumental]"));
        }
        if let Some(synced) = data.synced_lyrics {
            return Ok(LyricsParser::parse_lrc(&synced));
        }
        if let Some(plain) = data.plain_lyrics {
            return Ok(LyricsParser::parse_plain_text(&plain));
        }

        Err(LyricsError::NotFound(artist.to_string(), title.to_string()))
    }
}

impl Default for LrclibProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LyricsProvider for LrclibProvider {
    fn name(&self) -> &str {
        "lrclib"
    }

    async fn get_lyrics(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<SyncedLyrics, LyricsError> {
        let clean_title = Self::sanitize_title(title);
        let primary_art = Self::primary_artist(artist);
        info!(
            "Fetching lyrics from LRCLIB for '{}' - '{}' (sanitized: '{}' by '{}')",
            artist, title, clean_title, primary_art
        );

        // Step 1: Exact match with primary_artist
        if let Ok(lyrics) = self.query_exact(&primary_art, &clean_title, album).await {
            return Ok(lyrics);
        }

        // Step 2: Exact match with full artist string (if different)
        if primary_art != artist {
            if let Ok(lyrics) = self.query_exact(artist, &clean_title, album).await {
                return Ok(lyrics);
            }
        }

        // Step 3: Split title if "Artist - Title" pattern
        if let Some((part_a, part_b)) = clean_title.split_once(" - ") {
            let part_a = part_a.trim();
            let part_b = part_b.trim();
            if !part_a.is_empty() && !part_b.is_empty() {
                debug!("Trying split query: '{}' - '{}'", part_a, part_b);
                if let Ok(results) = self.search_query(&format!("{} {}", part_a, part_b)).await {
                    if let Some(matched) = self.pick_best_match(&results, part_b) {
                        return Ok(matched);
                    }
                }
            }
        }

        // Step 4: Search query with primary_artist + clean_title
        let q1 = format!("{} {}", primary_art, clean_title);
        if let Ok(results) = self.search_query(&q1).await {
            if let Some(matched) = self.pick_best_match(&results, &clean_title) {
                return Ok(matched);
            }
        }

        // Step 5: Search with clean_title only (handles movie soundtracks & featured artists)
        if let Ok(results) = self.search_query(&clean_title).await {
            if let Some(matched) = self.pick_best_match(&results, &clean_title) {
                return Ok(matched);
            }
        }

        // Step 6: Search with full artist + clean_title
        if primary_art != artist {
            let q2 = format!("{} {}", artist, clean_title);
            if let Ok(results) = self.search_query(&q2).await {
                if let Some(matched) = self.pick_best_match(&results, &clean_title) {
                    return Ok(matched);
                }
            }
        }

        Err(LyricsError::NotFound(artist.to_string(), title.to_string()))
    }
}
