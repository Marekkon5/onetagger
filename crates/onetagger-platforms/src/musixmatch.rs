use std::collections::HashMap;
use std::error::Error;
use std::sync::{Mutex, Arc};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use serde_json::Value;
use onetagger_tagger::{AutotaggerSource, AudioFileInfo, TaggerConfig, Track, Lyrics, LyricsLine, LyricsLinePart, AutotaggerSourceBuilder, PlatformInfo, PlatformCustomOptions, supported_tags};

#[derive(Clone)]
pub struct Musixmatch {
    client: Client,
    token: Arc<Mutex<Option<String>>>
}

impl Musixmatch {
    /// Create new instance
    pub fn new() -> Musixmatch {
        let mut header_map = HeaderMap::new();
        header_map.insert("authority", HeaderValue::from_static("apic-desktop.musixmatch.com"));
        header_map.insert("cookie", HeaderValue::from_static("AWSELBCORS=0; AWSELB=0"));
        Musixmatch { 
            client: ClientBuilder::new()
                .default_headers(header_map)
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/86.0.4240.183 Safari/537.36")
                .build()
                .unwrap(),
            token: Arc::new(Mutex::new(None)),
        }
    }

    /// Get Musixmatch token
    fn fetch_token(&self) -> Result<(), Box<dyn Error>> {
        debug!("Fetching Musixmatch token");
        let t: Value = self.get("token.get", &[("user_language", "en")])?;
        // Capcha retry
        if t["message"]["header"]["status_code"].as_i64() == Some(401) {
            warn!("Musixmatch capcha when getting token! Waiting for 10s...");
            std::thread::sleep(Duration::from_secs(10));
            return self.fetch_token();
        }
        let token = t["message"]["body"]["user_token"].as_str().ok_or("Couldn't fetch the token")?.to_string();
        *self.token.lock().unwrap() = Some(token);
        Ok(())
    }

    /// Make a GET request to musixmatch
    fn get<O: DeserializeOwned>(&self, action: &str, query: &[(&str, &str)]) -> Result<O, Box<dyn Error>> {
        // Get token
        if action != "token.get" && self.token.lock().unwrap().is_none() {
            self.fetch_token()?;
        }
        // Modify qp
        debug!("Musixmatch GET: {action}");
        let mut query = query.to_vec();
        query.push(("app_id", "web-desktop-app-v1.0"));
        // Add token
        let token = self.token.lock().unwrap().to_owned();
        if let Some(token) = token.as_ref() {
            query.push(("usertoken", token));
        }
        // Timestamp
        let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string();
        query.push(("t", t.as_str()));
        // Make request
        let o = self.client.get(&format!("https://apic-desktop.musixmatch.com/ws/1.1/{action}"))
            .query(&query)
            .send()?
            .json()?;
        Ok(o)
    }

    /// Fetch the lyrics
    pub fn fetch_lyrics(&self, title: &str, artist: &str) -> Result<MusixmatchMacroCallsBody<MusixmatchBody>, Box<dyn Error>> {
        let r: MusixmatchResponse<MusixmatchMacroCallsBody<MusixmatchBody>> = 
            self.get("macro.subtitles.get", &[
                ("format", "json"),
                ("namespace", "lyrics_richsynced"),
                ("optional_calls", "track.richsync"),
                ("subtitle_format", "lrc"),
                ("q_artist", artist),
                ("q_track", title),
            ])?;
        Ok(r.message.body.ok_or("Missing response body")?)
    }
}

impl AutotaggerSource for Musixmatch {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        // Fetch
        if !config.synced_lyrics && !config.unsynced_lyrics {
            return Ok(None);
        }
        let _ = info.artist()?;
        let lyrics = self.fetch_lyrics(info.title()?, &info.artists.join(","))?;
        // Output
        let mut track = Track {
            platform: "musixmatch".to_string(),
            ..Default::default()
        };

        // Get richsync if available
        if let Some(rich) = lyrics.macro_calls.get("track.richsync.get") {
            if let Some(MusixmatchBody::Richsync { richsync }) = &rich.message.body {
                let rs = richsync.parse_richsync()?;
                track.lyrics = Some(Lyrics {
                    paragraphs: vec![rs.into_iter().map(|l| l.into()).collect()],
                    language: richsync.richssync_language.to_owned()
                });
                return Ok(Some((1.0, track)));
            }
        }

        // Get subtitle if available
        if let Some(subtitle) = lyrics.macro_calls.get("track.subtitles.get") {
            if let Some(MusixmatchBody::SubtitleList { subtitle_list }) = &subtitle.message.body {
                if let Some(subtitle) = subtitle_list.first() {
                    let lines = subtitle.subtitle.parse_subtitle()?;
                    track.lyrics = Some(Lyrics {
                        paragraphs: vec![lines.into_iter().map(|l| l.into()).collect()],
                        language: subtitle.subtitle.subtitle_language.to_owned()
                    });
                    return Ok(Some((1.0, track)));
                }
            }
        }

        // Normal lyrics
        if let Some(lyrics) = lyrics.macro_calls.get("track.lyrics.get") {
            if let Some(MusixmatchBody::Lyrics { lyrics }) = &lyrics.message.body {
                // Instrumental
                if lyrics.lyrics_body.trim().is_empty() || lyrics.lyrics_body.trim().to_lowercase() == "instrumental" {
                    return Ok(None);
                }
                
                track.lyrics = Some(Lyrics { 
                    paragraphs: lyrics.lyrics_body.split("\n\n").map(|l| l.split("\n").map(|l| LyricsLine {
                        text: l.to_string(), start: None, end: None, parts: vec![]
                    }).collect::<Vec<_>>()).collect::<Vec<_>>(),
                    language: lyrics.lyrics_language.to_owned()
                });
                return Ok(Some((1.0, track)));
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusixmatchResponse<B> {
    pub message: MusixmatchHeader<B>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusixmatchHeader<B> {
    pub header: Value,
    pub body: Option<B>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusixmatchMacroCallsBody<B> {
    pub macro_calls: HashMap<String, MusixmatchResponse<B>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum MusixmatchBody {
    Lyrics { lyrics: MusixmatchLyrics },
    SubtitleList { subtitle_list: Vec<MusixmatchSubtitleWrap> },
    Richsync { richsync: MusixmatchRichsync },
    Other(Value)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusixmatchLyrics {
    pub lyrics_id: i64,
    pub lyrics_body: String,
    pub lyrics_language: String,
    pub lyrics_language_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusixmatchSubtitleWrap {
    pub subtitle: MusixmatchSubtitle
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusixmatchSubtitle {
    pub subtitle_id: i64,
    pub subtitle_body: String,
    pub subtitle_length: u32,
    pub subtitle_language: String,
    pub subtitle_language_description: String
}

impl MusixmatchSubtitle {
    /// Parse the LRC inside subtitle_body
    pub fn parse_subtitle(&self) -> Result<Vec<SubtitleLine>, Box<dyn Error>> {
        let mut output = vec![];
        for line in self.subtitle_body.lines() {
            if line.len() < 11 {
                continue;
            }
            let ts = Lyrics::parse_lrc_timestamp(&line[1..9])?;
            output.push(SubtitleLine {
                timestamp: ts,
                line: line[11..].to_string()
            });
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusixmatchRichsync {
    pub richsync_id: i64,
    pub richsync_body: String,
    pub richsync_length: u32,
    pub richssync_language: String,
    pub richsync_language_description: String
}

impl MusixmatchRichsync {
    /// Parse richsync_body
    pub fn parse_richsync(&self) -> Result<Vec<RichsyncLine>, Box<dyn Error>> {
        Ok(serde_json::from_str(&self.richsync_body)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichsyncLine {
    /// Time start seconds
    pub ts: f32,
    /// Time end seconds
    pub te: f32,
    /// Full line text
    pub x: String,
    /// Per word sync data
    pub l: Vec<RichsyncWord>
}

impl Into<LyricsLine> for RichsyncLine {
    fn into(self) -> LyricsLine {
        LyricsLine {
            start: Some(Duration::from_secs_f32(self.ts)),
            end: Some(Duration::from_secs_f32(self.ts)),
            text: self.x,
            parts: self.l.into_iter().map(|p| LyricsLinePart {
                text: p.c,
                start: Some(Duration::from_secs_f32(self.ts + p.o)),
                end: None,
            }).collect()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichsyncWord {
    /// Word text
    pub c: String,
    /// Offset seconds
    pub o: f32
}

#[derive(Debug, Clone)]
pub struct SubtitleLine {
    pub timestamp: Duration,
    pub line: String
}

impl Into<LyricsLine> for SubtitleLine {
    fn into(self) -> LyricsLine {
        LyricsLine { text: self.line, start: Some(self.timestamp), end: None, parts: vec![] }
    }
}

/// 1T source builder
pub struct MusixmatchBuilder {
    mxm: Musixmatch
}

impl AutotaggerSourceBuilder for MusixmatchBuilder {
    fn new() -> Self {
        MusixmatchBuilder {
            mxm: Musixmatch::new()
        }
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        Ok(Box::new(self.mxm.clone()))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "musixmatch".to_string(),
            name: "Musixmatch".to_string(),
            description: "Fetch lyrics from the largest lyrics platform in the world".to_string(),
            version: "1.0.0".to_string(),
            icon: include_bytes!("../assets/musixmatch.png"),
            max_threads: 1,
            custom_options: PlatformCustomOptions::new(),
            supported_tags: supported_tags!(SyncedLyrics, UnsyncedLyrics),
            requires_auth: false
        }
    }
}

