use std::sync::{Arc, Mutex, OnceLock};
use std::collections::{HashMap, HashSet};
use anyhow::{anyhow, Error};
use std::time::Duration;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use chrono::{NaiveDate, Datelike};
use serde::{Serialize, Deserialize};
use onetagger_tag::FrameName;
use onetagger_tagger::{
    supported_tags, Album, AudioFileInfo, AutotaggerSource, AutotaggerSourceBuilder,
    MatchingUtils, PlatformCustomOptionValue, PlatformCustomOptions, PlatformInfo,
    SupportedTag, TaggerConfig, Track, TrackMatch, TrackNumber
};
use serde_json::json;

const INVALID_ART: &'static str = "ab2d1d04-233d-4b08-8234-9782b34dcab8";

static MIX_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static FEATURE_REGEX: OnceLock<regex::Regex> = OnceLock::new();

#[derive(PartialEq, Eq, Debug)]
enum MixType {
    Original,
    Extended,
    Club,
    Radio,
    Edit,
    Remix,
    Dub,
    Unknown,
}

impl MixType {
    fn from_str(s: &str) -> Self {
        let m = s.to_lowercase();
        
        if m.contains("remix") || m.contains("rmx") || m.contains("rework") {
            MixType::Remix
        } else if m.contains("dub") {
            MixType::Dub
        } else if m.contains("extended") {
            MixType::Extended
        } else if m.contains("club") {
            MixType::Club 
        } else if m.contains("radio") {
            MixType::Radio
        } else if m.contains("edit") || m.contains("short") {
            MixType::Edit
        } else if m.is_empty() || m == "original mix" || m == "original" || m == "orig. mix" {
            MixType::Original
        } else {
            MixType::Unknown
        }
    }
}

pub struct Beatport {
    client: Client,
    access_token: Arc<Mutex<Option<BeatportOAuth>>>
}

impl Beatport {
    pub fn new(access_token: Arc<Mutex<Option<BeatportOAuth>>>) -> Beatport {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        Beatport {
            client,
            access_token
        }
    }

    pub fn search(&self, query: &str, page: i32, results_per_page: usize) -> Result<BeatportTrackResults, Error> {
        let query = Self::clear_search_query(query);
        let token = self.update_token()?;

        let response: BeatportTrackResults = self.client
            .get("https://api.beatport.com/v4/catalog/search/")
            .bearer_auth(token)
            .query(&[
                ("q", query.as_str()),
                ("type", "tracks"),
                ("page", &page.to_string()),
                ("per_page", &results_per_page.to_string())
            ])
            .send()?
            .json()?;

        Ok(response)
    }

    pub fn update_token(&self) -> Result<String, Error> {
        // BOUNDED RETRY LOOP: Prevents infinite recursion if the Beatport API 
        // continually serves expired tokens (e.g., due to severe clock skew).
        for attempt in 1..=2 {
            let mut token = self.access_token.lock().unwrap();

            if (*token).is_none() {
                let mut response: BeatportOAuth = self.client.post("https://account.beatport.com/o/token/")
                    .form(&json!({
                        "client_id": "2tiTbKxmQFwnbFjMONU4k7njMRZmV3ZMwRBndiZs",
                        "client_secret": "RDUJyAk4zFEGtQ8rsTmylDSfxmALRNBn3D1BsRr7MKi3oa1TL9Mq9QxqUPK7loiumXolEWbJcWa4IGAhtwnTz1cSXClGJ1tkkNCNWwRwjxIKTZJKOJxbwaNt0Rm3WG0v",
                        "grant_type": "client_credentials"
                    }))
                    .send()?
                    .json()?;

                response.expires_in = response.expires_in * 1000 + timestamp!() - 10_000;
                *token = Some(response);
                debug!("OAuth: {:?}", token);
            }

            let t = token.clone().unwrap();
            
            // If the token is valid, return it immediately.
            if t.expires_in > timestamp!() {
                return Ok(t.access_token);
            }

            // If we are here, the token registered as instantly expired.
            if attempt == 2 {
                return Err(anyhow!("Beatport API continuously provided an expired token. Please check your system clock sync."));
            }

            // Clear the token to force a fresh fetch on the next iteration.
            *token = None;
            // The Mutex guard (`token`) is automatically and safely dropped here at the end of the loop scope.
        }

        Err(anyhow!("Failed to acquire a valid Beatport token."))
    }

    
    pub fn track(&self, id: i64) -> Result<Option<BeatportTrack>, Error> {
        let token = self.update_token()?;

        let response = self.client
            .get(&format!("https://api.beatport.com/v4/catalog/tracks/{}", id))
            .bearer_auth(token)
            .send()?;

        if response.status() == StatusCode::FORBIDDEN {
            return Ok(None);
        }

        Ok(response.json()?)
    }

    pub fn release(&self, id: i64) -> Result<BeatportRelease, Error> {
        let token = self.update_token()?;

        let response = self.client
            .get(&format!("https://api.beatport.com/v4/catalog/releases/{}", id))
            .bearer_auth(token)
            .send()?
            .json()?;

        Ok(response)
    }

    pub fn release_tracks(&self, id: i64) -> Result<Vec<BeatportTrack>, Error> {
        let token = self.update_token()?;

        let response: BeatportPagination<BeatportTrack> = self.client
            .get(&format!("https://api.beatport.com/v4/catalog/releases/{}/tracks?per_page=200", id))
            .bearer_auth(token)
            .send()?
            .json()?;

        Ok(response.results)
    }

    pub fn clear_search_query(query: &str) -> String {
        let re = FEATURE_REGEX.get_or_init(|| regex::Regex::new(r"(?i)\s+(?:ft|feat|featuring)\.?\s+[^()]+").unwrap());
        let stripped_query = re.replace_all(query, "").to_string();

        stripped_query
            .replace("(", " ")
            .replace(")", " ")
            .replace("[", " ")
            .replace("]", " ")
            .replace(",", " ")
            .replace("Ft.", "")
            .replace("ft.", "")
            .replace(" Ft ", " ")
            .replace(" ft ", " ")
            .replace(" feat. ", " ")
            .replace(" feat ", " ")
            .replace("  ", " ")
            .trim()
            .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportOAuth {
    pub access_token: String,
    pub expires_in: u128
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportTrackResults {
    #[serde(rename = "tracks")]
    pub data: Vec<BeatportTrack>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportTrack {
    pub artists: Vec<BeatportGeneric>,
    pub bpm: Option<i64>,
    pub catalog_number: Option<String>,
    pub exclusive: bool,
    pub genre: BeatportGeneric,
    pub id: i64,
    pub image: Option<BeatportImage>,
    pub isrc: Option<String>,
    pub key: Option<BeatportGeneric>,
    pub length_ms: Option<u64>,
    pub mix_name: String,
    pub name: String,
    pub number: Option<i64>,
    pub publish_date: Option<String>,
    pub release: BeatportRelease,
    pub remixers: Vec<BeatportGeneric>,
    pub slug: String,
    pub sub_genre: Option<BeatportGeneric>,
    pub new_release_date: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportGeneric {
    pub id: i64,
    pub name: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportImage {
    pub id: Option<i64>,
    pub dynamic_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportRelease {
    pub id: i64,
    pub name: String,
    pub label: BeatportGeneric,
    pub image: BeatportImage,
    pub upc: Option<String>,
    pub track_count: Option<u16>,
    pub artists: Option<Vec<BeatportGeneric>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeatportPagination<T> {
    pub results: Vec<T>
}

impl BeatportTrack {
    pub fn to_track(self, art_resolution: u32) -> Track {
        let art = self.get_art(art_resolution);
        let thumbnail = self.get_art(150);

        let mut track = Track {
            platform: "beatport".to_string(),
            title: self.name,
            version: Some(self.mix_name),
            artists: self.artists.into_iter().map(|a| a.name).collect(),
            album: Some(self.release.name),
            key: self.key.map(|k| k.name.replace(" Major", "").replace(" Minor", "m")),
            bpm: self.bpm,
            genres: vec![self.genre.name],
            styles: match self.sub_genre {
                Some(s) => vec![s.name],
                None => vec![],
            },
            art,
            url: format!("https://www.beatport.com/track/{}/{}", self.slug, self.id),
            label: Some(self.release.label.name),
            catalog_number: self.catalog_number,
            other: vec![
                (FrameName::same("UNIQUEFILEID"), vec![format!("https://beatport.com|{}", &self.id)])
            ],
            track_id: Some(self.id.to_string()),
            release_id: Some(self.release.id.to_string()),
            duration: Duration::from_millis(self.length_ms.unwrap_or(0)).into(),
            remixers: self.remixers.into_iter().map(|r| r.name).collect(),
            track_number: self.number.map(|n| TrackNumber::Number(n as i32)),
            isrc: self.isrc,
            release_year: self.new_release_date.as_ref().and_then(|d| d.chars().take(4).collect::<String>().parse().ok()),
            publish_year: self.publish_date.as_ref().and_then(|d| d.chars().take(4).collect::<String>().parse().ok()),
            release_date: self.new_release_date.as_ref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            publish_date: self.publish_date.as_ref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            thumbnail,
            ..Default::default()
        };

        if self.exclusive {
            track.other.push((FrameName::same("BEATPORT_EXCLUSIVE"), vec!["1".to_string()]));
        }

        track
    }

    pub fn get_art(&self, art_resolution: u32) -> Option<String> {
        if self.release.image.dynamic_uri.contains(&INVALID_ART) {
            return None;
        }

        let r = art_resolution.to_string();

        Some(
            self.release.image.dynamic_uri
                .replace("{w}", &r)
                .replace("{h}", &r)
                .replace("{x}", &r)
                .replace("{y}", &r)
        )
    }
}

impl AutotaggerSource for Beatport {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        let custom_config: BeatportConfig = config.get_custom("beatport")?;
        let mut output = vec![];

        if let Some(id) = info.tags.get("BEATPORT_TRACK_ID").map(|t| t.first().and_then(|id| id.trim().replace("\0", "").parse().ok())).flatten() {
            info!("Fetching by ID: {}", id);

            match self.track(id) {
                Ok(Some(api_track)) => {
                    let track = TrackMatch::new_id(api_track.to_track(custom_config.art_resolution));
                    if !config.fetch_all_results {
                        return Ok(vec![track]);
                    }
                    output.push(track);
                },
                Ok(None) => warn!("Matching by ID failed, track restricted, matching normally"),
                Err(e) => warn!("Matching by ID failed, matching normally: {e}")
            }
        }

        if let Some(isrc) = info.isrc.as_ref() {
            match self.search(isrc, 1, 25) {
                Ok(results) => {
                    if !results.data.is_empty() {
                        // FIX: Graceful failover if track detail fetch fails, rather than hard crashing `?`
                        match self.track(results.data[0].id) {
                            Ok(Some(track)) => {
                                let track = TrackMatch::new_isrc(track.to_track(custom_config.art_resolution));
                                if !config.fetch_all_results {
                                    return Ok(vec![track]);
                                }
                                output.push(track);
                            },
                            Ok(None) => warn!("Matching by ISRC failed, track restricted, trying normal."),
                            Err(e) => warn!("Failed fetching track details by ISRC: {e}, falling back to normal match."),
                        }
                    }
                },
                Err(e) => warn!("Failed fetching track by ISRC: {e}"),
            }
        }

        let raw_title = info.title().unwrap_or("");
        let re_feat = FEATURE_REGEX.get_or_init(|| regex::Regex::new(r"(?i)\s+(?:ft|feat|featuring)\.?\s+[^()]+").unwrap());
        let virtual_title = re_feat.replace_all(raw_title, "").to_string();

        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(&virtual_title));
        debug!("BP Query: {}", query);

        for page in 1..custom_config.max_pages + 1 {
            match self.search(&query, page, 50) {
                Ok(res) => {
                    let api_tracks = res.data
                        .into_iter()
                        .map(|t| t.to_track(custom_config.art_resolution))
                        .collect::<Vec<_>>();

                    let mut matched_tracks = if custom_config.ignore_version {
                        let t = api_tracks.clone().into_iter().map(|mut t| {
                            t.version = None;
                            t
                        }).collect();

                        MatchingUtils::match_track(info, &t, config, true)
                            .into_iter()
                            .map(|mut t| {
                                if let Some(ot) = api_tracks.iter().find(|ot| ot.url == t.track.url) {
                                    t.track.version = ot.version.to_owned();
                                }
                                t
                            })
                            .collect()
                    } else {
                        MatchingUtils::match_track(info, &api_tracks, config, true)
                    };

                    // --- NEW FALLBACK LOGIC START ---
                    let local_title_full = virtual_title.clone();
                    
                    let re_mix = MIX_REGEX.get_or_init(|| regex::Regex::new(r"^(.*?)\s*(?:\(|\[)([^()\[\]]+)(?:\)|\])$").unwrap());
                    
                    let (local_title, local_mix) = if let Some(caps) = re_mix.captures(&local_title_full) {
                        (caps.get(1).map_or("", |m| m.as_str()).trim(), caps.get(2).map_or("", |m| m.as_str()).trim())
                    } else {
                        (local_title_full.trim(), "")
                    };

                    let normalize_punctuation = |s: &str| -> String {
                        s.to_lowercase().replace(['(', ')', '[', ']', '-', ',', '.', '!'], " ")
                    };

                    let normalize_artists = |artists: &Vec<String>| -> HashSet<String> {
                        artists.iter()
                            .flat_map(|a| a.to_lowercase().replace("&", "and").split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>())
                            .filter(|s| !s.is_empty())
                            .collect()
                    };

                    let local_mix_clean = normalize_punctuation(local_mix);
                    let local_cat = MixType::from_str(&local_mix_clean);

                    for track in &api_tracks {
                        let beatport_title_original = &track.title;
                        let beatport_mix_clean = normalize_punctuation(track.version.as_deref().unwrap_or(""));
                        let api_cat = MixType::from_str(&beatport_mix_clean);

                        let mut is_compatible = false;
                        let mut mix_acc = 1.0;

                        if local_cat == api_cat 
                            || (local_cat == MixType::Club && api_cat == MixType::Extended)
                            || (local_cat == MixType::Extended && api_cat == MixType::Club) {
                            
                            if local_cat == MixType::Remix || local_cat == MixType::Dub {
                                let stopwords = ["remix", "rmx", "mix", "edit", "version", "vip", "rework", "dub"];
                                let words_local: HashSet<&str> = local_mix_clean.split_whitespace().filter(|w| !stopwords.contains(w)).collect();
                                let words_api: HashSet<&str> = beatport_mix_clean.split_whitespace().filter(|w| !stopwords.contains(w)).collect();
                                let intersection = words_local.intersection(&words_api).count() as f64;
                                let union = words_local.union(&words_api).count() as f64;
                                let jaccard = if union == 0.0 { 1.0 } else { intersection / union };
                                
                                if jaccard >= 0.50 {
                                    is_compatible = true;
                                    mix_acc = jaccard;
                                }
                            } else {
                                is_compatible = true;
                            }
                        } else if (local_cat == MixType::Original && api_cat == MixType::Unknown) || 
                                  (local_cat == MixType::Unknown && api_cat == MixType::Original) ||
                                  (local_cat == MixType::Unknown && api_cat == MixType::Unknown) {
                            is_compatible = true;
                            mix_acc = 0.9; 
                        }

                        if !is_compatible {
                            continue; 
                        }

                        let beatport_title_clean = re_feat.replace_all(beatport_title_original, "").to_string();

                        let title_acc = strsim::normalized_levenshtein(
                            &MatchingUtils::clean_title_matching(&local_title),
                            &MatchingUtils::clean_title_matching(&beatport_title_clean)
                        );

                        let artist_acc = if MatchingUtils::match_artist(&info.artists, &track.artists, config.strictness) {
                            1.0 
                        } else {
                            let api_artist_words = normalize_artists(&track.artists);
                            let mut local_artists_rescued = normalize_artists(&info.artists);
                            let beatport_title_original_lower = beatport_title_original.to_lowercase();
                                
                            for api_artist in &api_artist_words {
                                if beatport_title_original_lower.contains(api_artist) {
                                    local_artists_rescued.insert(api_artist.clone());
                                }
                            }

                            let intersection = local_artists_rescued.intersection(&api_artist_words).count() as f64;
                            let union = local_artists_rescued.union(&api_artist_words).count() as f64;
                            
                            if union == 0.0 { 0.0 } else { intersection / union }
                        };

                        mix_acc = mix_acc.clamp(0.0, 1.0); 
                        let final_acc = (title_acc * 0.5) + (artist_acc * 0.3) + (mix_acc * 0.2);

                        if final_acc >= config.strictness {
                            if let Some(existing) = matched_tracks.iter_mut().find(|m| m.track.url == track.url) {
                                if final_acc > existing.accuracy {
                                    existing.accuracy = final_acc;
                                }
                            } else {
                                matched_tracks.push(TrackMatch::new(final_acc, track.clone()));
                            }
                        }
                    }
                    
                    matched_tracks.sort_by(|a, b| b.accuracy.partial_cmp(&a.accuracy).unwrap_or(std::cmp::Ordering::Equal));
                    // --- NEW FALLBACK LOGIC END ---

                    output.extend(matched_tracks);

                    if config.fetch_all_results {
                        continue;
                    }
                    
                    if output.iter().any(|m| m.accuracy >= config.strictness) {
                        return Ok(output);
                    }
                },
                Err(e) => {
                    warn!("Beatport search failed, query: {}. {}", query, e);
                    return Ok(output);
                }
            }
        }

        Ok(output)
    }

    fn extend_track(&mut self, track: &mut Track, config: &TaggerConfig) -> Result<(), Error> {
        let custom_config: BeatportConfig = config.get_custom("beatport")?;

        if track.other.is_empty() {
            if let Some(track_id_str) = &track.track_id {
                if let Ok(id) = track_id_str.parse() {
                    if let Ok(Some(api_track)) = self.track(id) {
                        *track = api_track.to_track(custom_config.art_resolution);
                    }
                }
            }
        }

        if !config.tag_enabled(SupportedTag::AlbumArtist) && !config.tag_enabled(SupportedTag::TrackTotal) {
            return Ok(());
        }

        if let Some(release_id_str) = &track.release_id {
            if let Ok(id) = release_id_str.parse() {
                if let Ok(release) = self.release(id) {
                    track.track_total = release.track_count;
                    track.album_artists = match release.artists {
                        Some(a) => a.into_iter().map(|a| a.name).collect(),
                        None => vec![],
                    };
                }
            }
        }

        Ok(())
    }

    fn get_album(&mut self, id: &str, config: &TaggerConfig) -> Result<Option<Album>, Error> {
        let custom_config: BeatportConfig = config.get_custom("beatport")?;
        let id: i64 = id.trim().parse()?;
        let release = self.release(id)?;
        let tracks = self.release_tracks(id)?;

        let album = Album {
            id: id.to_string(),
            name: release.name,
            tracks: tracks.into_iter().map(|t| t.to_track(custom_config.art_resolution)).collect()
        };

        Ok(Some(album))
    }
}

#[derive(Debug, Clone)]
pub struct BeatportBuilder {
    access_token: Arc<Mutex<Option<BeatportOAuth>>>
}

impl AutotaggerSourceBuilder for BeatportBuilder {
    fn new() -> BeatportBuilder {
        BeatportBuilder {
            access_token: Arc::new(Mutex::new(None))
        }
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        Ok(Box::new(Beatport::new(self.access_token.clone())))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "beatport".to_string(),
            name: "Beatport".to_string(),
            description: "Overall more specialized in Techno, can match using ISRC".to_string(),
            icon: include_bytes!("../assets/beatport.png"),
            max_threads: 0,
            version: "1.0.1".to_string(),
            requires_auth: false,
            supported_tags: supported_tags!(
                Title, Version, Artist, AlbumArtist, Album, BPM, Genre, Style,
                Label, URL, ReleaseDate, PublishDate, Key, AlbumArt, OtherTags,
                TrackId, ReleaseId, Duration, Remixer, CatalogNumber, TrackTotal,
                ISRC, TrackNumber
            ),
            custom_options: PlatformCustomOptions::new()
                .add("art_resolution", "Album art resolution", PlatformCustomOptionValue::Number {
                    min: 200,
                    max: 1600,
                    step: 100,
                    value: 500
                })
                .add_tooltip("max_pages", "Max pages", "How many pages of search results to scan for tracks", PlatformCustomOptionValue::Number {
                    min: 1,
                    max: 10,
                    step: 1,
                    value: 1
                })
                .add_tooltip("ignore_version", "Ignore version when matching", "Ignores (Extended Mix), (Original Mix) and such", PlatformCustomOptionValue::Boolean {
                    value: false
                })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeatportConfig {
    pub art_resolution: u32,
    pub max_pages: i32,
    pub ignore_version: bool
}

#[test]
fn test_album() {
    let mut builder = BeatportBuilder::new();
    let mut config = TaggerConfig::default();
    let custom_config = builder.info().custom_options.get_defaults();
    config.custom.0.insert("beatport".to_string(), custom_config);

    let mut bp = builder.get_source(&config).unwrap();
    bp.get_album("2174307", &config).unwrap();
}
