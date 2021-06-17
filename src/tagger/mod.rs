use std::error::Error;
use std::thread;
use std::path::Path;
use std::fs::File;
use std::default::Default;
use std::io::prelude::*;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver};
use regex::Regex;
use reqwest::StatusCode;
use walkdir::WalkDir;
use threadpool::ThreadPool;
use strsim::normalized_levenshtein;
use chrono::{NaiveDate, Datelike};
use serde::{Serialize, Deserialize};
use crate::tag::{AudioFileFormat, Tag, Field, TagDate, CoverType, TagImpl, UITag, EXTENSIONS};

pub mod beatport;
pub mod traxsource;
pub mod discogs;
pub mod junodownload;
pub mod spotify;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MusicPlatform {
    Beatport,
    Traxsource,
    Discogs,
    JunoDownload,

    //Currently only used in Audio Features
    Spotify
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaggerConfig {
    //Global
    pub platforms: Vec<MusicPlatform>,
    pub path: Option<String>,

    //Tags
    pub title: bool,
    pub artist: bool,
    pub album: bool,
    pub key: bool,
    pub bpm: bool,
    pub genre: bool,
    pub style: bool,
    pub label: bool,
    pub release_date: bool,
    pub publish_date: bool,
    pub album_art: bool,
    pub other_tags: bool,
    pub catalog_number: bool,

    //Advanced
    pub id3_separator: String,
    pub flac_separator: Option<String>,
    pub id3v24: bool,
    pub overwrite: bool,
    pub threads: i16,
    //From 0 to 1
    pub strictness: f64,
    pub merge_genres: bool,
    pub album_art_file: bool,
    pub camelot: bool,
    pub parse_filename: bool,
    pub filename_template: Option<String>,

    //Platform specific
    pub beatport: BeatportConfig,
    pub discogs: DiscogsConfig
}

//Beatport specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BeatportConfig {
    pub art_resolution: i64,
    pub max_pages: i64
}

//Discogs specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscogsConfig {
    pub token: Option<String>,
    pub max_results: i16,
    pub styles: DiscogsStyles,
    //Option to prevent update errors
    pub styles_custom_tag: Option<UITag>
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiscogsStyles {
    Default,
    OnlyGenres,
    OnlyStyles,
    MergeToGenres,
    MergeToStyles,
    StylesToGenre,
    GenresToStyle,
    CustomTag
}

impl Default for DiscogsStyles {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub platform: MusicPlatform,
    pub title: String,
    pub version: Option<String>,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub key: Option<String>,
    pub bpm: Option<i64>,
    pub genres: Vec<String>,
    pub styles: Vec<String>,
    pub art: Option<String>,
    pub url: Option<String>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
    
    //Only year OR date should be available
    pub release_year: Option<i64>,
    pub release_date: Option<NaiveDate>,
    pub publish_year: Option<i64>,
    pub publish_date: Option<NaiveDate>
}

const CAMELOT_NOTES: [(&str, &str); 35] = [
    ("Abm", "1A"),
    ("G#m", "1A"),
    ("B",   "1B"),
    ("D#m", "2A"),
    ("Ebm", "2A"),
    ("Gb",  "2B"),
    ("F#",  "2B"),
    ("A#m", "3A"),
    ("Bbm", "3A"),
    ("C#",  "3B"),
    ("Db",  "3B"),
    ("Dd",  "3B"),
    ("Fm",  "4A"),
    ("G#",  "4B"),
    ("Ab",  "4B"),
    ("Cm",  "5A"),
    ("D#",  "5B"),
    ("Eb",  "5B"),
    ("Gm",  "6A"),
    ("A#",  "6B"),
    ("Bb",  "6B"),
    ("Dm",  "7A"),
    ("F",   "7B"),
    ("Am",  "8A"),
    ("C",   "8B"),
    ("Em",  "9A"),
    ("G",   "9B"),
    ("Bm",  "10A"),
    ("D",   "10B"),
    ("Gbm", "11A"),
    ("F#m", "11A"),
    ("A",   "11B"),
    ("C#m", "12A"),
    ("Dbm", "12A"),
    ("E",   "12B"),
];

impl Track {
    //Write tags to file
    pub fn write_to_file(&self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<(), Box<dyn Error>> {        
        //Get tag
        let mut tag_wrap = Tag::load_file(&info.path, true)?;
        let format = tag_wrap.format.to_owned();
        //Configure ID3 and FLAC
        if let Some(t) = tag_wrap.id3.as_mut() {
            t.set_id3_separator(&config.id3_separator);
            t.set_id3v24(config.id3v24);
        }
        if let Some(flac) = tag_wrap.flac.as_mut() {
            if let Some(s) = config.flac_separator.as_ref() {
                flac.set_separator(Some(s));
            }
        }
        //MP4 Album art override
        if let Some(mp4) = tag_wrap.mp4.as_mut() {
            //Has art
            if (config.overwrite || mp4.get_art().is_empty()) && self.art.is_some() && config.album_art {
                mp4.remove_all_artworks();
            }
        }

        let tag = tag_wrap.tag_mut().unwrap();

        //Set tags
        if config.title {
            tag.set_field(Field::Title, vec![self.title.to_string()], config.overwrite);
        }
        if config.artist {
            tag.set_field(Field::Artist, self.artists.clone(), config.overwrite);
        }
        if self.album.is_some() && config.album  {
            tag.set_field(Field::Album, vec![self.album.as_ref().unwrap().to_string()], config.overwrite);
        }
        if config.key && self.key.is_some() {
            let mut value = self.key.as_ref().unwrap().to_string();
            //Convert to camelot
            if config.camelot {
                if let Some((_, c)) = CAMELOT_NOTES.iter().find(|(o, _)| o == &value) {
                    value = c.to_string();
                }
            }
            tag.set_field(Field::Key, vec![value], config.overwrite);
        }
        if config.bpm && self.bpm.is_some() {
            tag.set_field(Field::BPM, vec![self.bpm.unwrap().to_string()], config.overwrite);
        }
        if config.label && self.label.is_some() {
            tag.set_field(Field::Label, vec![self.label.as_ref().unwrap().to_string()], config.overwrite);
        }
        if config.genre && !self.genres.is_empty() {
            if config.merge_genres {
                //Merge with existing ones
                let mut current: Vec<String> = tag.get_field(Field::Genre).unwrap_or(vec![]).iter().map(|g| g.to_lowercase()).collect();
                let mut genres = self.genres.clone().into_iter().filter(|g| !current.iter().any(|i| i == &g.to_lowercase())).collect();
                current.append(&mut genres);
                tag.set_field(Field::Genre, current, config.overwrite); 
            } else {
                tag.set_field(Field::Genre, self.genres.clone(), config.overwrite);
            }
        }
        if config.style && !self.styles.is_empty() {
            if config.discogs.styles == DiscogsStyles::CustomTag && config.discogs.styles_custom_tag.is_some() {
                //Custom style tag
                let ui_tag = config.discogs.styles_custom_tag.as_ref().unwrap();
                tag.set_raw(&ui_tag.by_format(&format), self.styles.clone(), config.overwrite);

            } else if config.merge_genres {
                //Merge with existing ones
                let mut current: Vec<String> = tag.get_field(Field::Style).unwrap_or(vec![]).iter().map(|s| s.to_lowercase()).collect();
                let mut styles = self.styles.clone().into_iter().filter(|s| !current.iter().any(|i| i == &s.to_lowercase())).collect();
                current.append(&mut styles);
                tag.set_field(Field::Style, current, config.overwrite); 

            } else {
                //Default write to style
                tag.set_field(Field::Style, self.styles.clone(), config.overwrite);
            }
        }
        //Release dates
        if config.release_date {
            if let Some(date) = self.release_date {
                tag.set_date(&TagDate {
                    year: date.year() as i32,
                    month: Some(date.month() as u8),
                    day: Some(date.day() as u8)
                }, config.overwrite);
            } else if let Some(year) = self.release_year {
                tag.set_date(&TagDate {
                    year: year as i32,
                    month: None,
                    day: None
                }, config.overwrite);
            }
        }
        //Publish date
        if config.publish_date {
            if let Some(date) = self.publish_date {
                tag.set_publish_date(&TagDate {
                    year: date.year() as i32,
                    month: Some(date.month() as u8),
                    day: Some(date.day() as u8)
                }, config.overwrite);
            } else if let Some(year) = self.publish_year {
                tag.set_publish_date(&TagDate {
                    year: year as i32,
                    month: None,
                    day: None
                }, config.overwrite);
            }
        }
        //Other tags
        if config.other_tags {
            if self.url.is_some() {
                tag.set_raw("WWWAUDIOFILE", vec![self.url.as_ref().unwrap().to_string()], config.overwrite);
            }
        }
        //Catalog number
        if config.catalog_number && self.catalog_number.is_some() {
            tag.set_field(Field::CatalogNumber, vec![self.catalog_number.as_ref().unwrap().to_string()], config.overwrite);
        }
        //Album art
        if (config.overwrite || tag.get_art().is_empty()) && self.art.is_some() && config.album_art {
            match self.download_art(self.art.as_ref().unwrap()) {
                Ok(data) => {
                    match data {
                        Some(data) => {
                            tag.set_art(CoverType::CoverFront, "image/jpeg", Some("Cover"), data.clone());
                            //Save to file
                            if config.album_art_file {
                                let path = Path::new(&info.path).parent().unwrap().join("cover.jpg");
                                if !path.exists() {
                                    if let Ok(mut file) = File::create(path) {
                                        file.write_all(&data).ok();
                                    }
                                }
                            }
                        },
                        None => warn!("Invalid album art!")
                    } 
                },
                Err(e) => warn!("Error downloading album art! {}", e)
            }
        }

        //Save
        tag.save_file(&info.path)?;
        Ok(())
    }

    //Download album art, None if invalid album art
    fn download_art(&self, url: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let response = reqwest::blocking::get(url)?;
        if response.status() != StatusCode::OK {
            return Ok(None);
        }
        //Too small, most likely a text response
        if let Some(cl) = response.content_length() {
            if cl < 2048 {
                return Ok(None);
            }
        }
        //Content-type needs image
        let headers = response.headers();
        if let Some(ct) = headers.get("content-type") {
            if !ct.to_str()?.contains("image") {
                return Ok(None);
            }
        }
        
        Ok(Some(response.bytes()?.to_vec()))
    }
}

#[derive(Debug, Clone)]
pub struct AudioFileInfo {
    pub title: String,
    pub artists: Vec<String>,
    pub format: AudioFileFormat,
    pub path: String,
    pub isrc: Option<String>
}

impl AudioFileInfo {
    //Load audio file info from path
    pub fn load_file(path: &str, filename_template: Option<Regex>) -> Result<AudioFileInfo, Box<dyn Error>> {
        let tag_wrap = Tag::load_file(&path, true)?;
        let tag = tag_wrap.tag().unwrap();
        //Get title artist from tag
        let mut title = tag.get_field(Field::Title).map(|t| t.first().map(|t| t.to_owned())).flatten();
        let mut artists = tag.get_field(Field::Artist)
            .map(|a| AudioFileInfo::parse_artist_tag(a.iter().map(|a| a.as_str()).collect()));

        //Parse filename
        if (title.is_none() || artists.is_none()) && filename_template.is_some() {
            let p = Path::new(path);
            let filename = p.file_name().ok_or("Missing filename!")?.to_str().ok_or("Missing filename")?;

            if let Some(captures) = filename_template.unwrap().captures(filename) {
                //Title
                if title.is_none() {
                    if let Some(m) = captures.name("title") {
                        title = Some(m.as_str().trim().to_string());
                    }
                }
                //Artists
                if artists.is_none() {
                    if let Some(m) = captures.name("artists") {
                        artists = Some(AudioFileInfo::parse_artist_tag(vec![m.as_str().trim()]));
                    }
                }
            }
        }

        Ok(AudioFileInfo {
            format: tag_wrap.format.to_owned(),
            title: title.ok_or("Missing title!")?,
            artists: artists.ok_or("Missing artists!")?,
            path: path.to_owned(),
            isrc: tag.get_field(Field::ISRC).unwrap_or(vec![]).first().map(String::from)
        })
    }

    //Convert template into a regex
    pub fn parse_template(template: &str) -> Option<Regex> {
        //Regex reserved
        let reserved = ".?+*$^()[]/|";
        let mut template = template.to_string();
        for c in reserved.chars() {
            template = template.replace(c, &format!("\\{}", c));
        };
        //Replace variables
        template = template
            .replace("%title%", "(?P<title>.+)")
            .replace("%artists%", "(?P<artists>.+)");
        //Remove all remaining variables
        let re = Regex::new("%[a-zA-Z0-9 ]+%").unwrap();
        template = re.replace(&template, "(.+)").to_string();
        //Extension
        template = format!("{}\\.[a-zA-Z0-9]{{2,4}}$", template);
        //Final regex
        Regex::new(&template).ok()
    }

    //Try to split artist string with common separators
    fn parse_artist_tag(input: Vec<&str>) -> Vec<String> {
        //Already an array
        if input.len() > 1 {
            return input.into_iter().map(|v| v.to_owned()).collect();
        }
        let src = input.first().unwrap();

        if src.contains(';') {
            return src.split(';').collect::<Vec<&str>>().into_iter().map(|v| v.to_owned()).collect();
        }
        if src.contains(',') {
            return src.split(',').collect::<Vec<&str>>().into_iter().map(|v| v.to_owned()).collect();
        }
        if src.contains('/') {
            return src.split('/').collect::<Vec<&str>>().into_iter().map(|v| v.to_owned()).collect();
        }
        vec![src.to_owned().to_owned()]
    }
}

//For all the platforms
pub trait TrackMatcher {
    //Returns (accuracy, track)
    fn match_track(&self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>>;
}

//Single threaded, mutable
pub trait TrackMatcherST {
    //Returns (accuracy, track)
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>>;
}

pub struct MatchingUtils {}
impl MatchingUtils {
    //Clean title for searching
    pub fn clean_title(input: &str) -> String {
        let step1 = input.to_lowercase()
            //Remove - because search engines
            .replace("-", " ")
            .replace("  ", " ");
        let step2 = step1.trim();
        //Remove original mix
        let mut re = Regex::new(r"((\(|\[)*)original( (mix|version|edit))*((\)|\])*)$").unwrap();
        let step3 = re.replace(&step2, "");
        //Remove initial a/an/the
        re = Regex::new(r"^((a|an|the) )").unwrap();
        let step4 = re.replace(&step3, "");
        //Remove attributes
        let step5 = step4
            .replace("(intro)", "")
            .replace("(clean)", "");
        //Remove - and trim
        let step6 = step5.replace("-", "").replace("  ", " ");
        //Remove feat.
        re = Regex::new(r"(\(|\[)?(feat|ft)\.?.+?(\)|\]|\(|$)").unwrap();
        let out = re.replace(&step6, "");
        out.trim().to_string()
    }

    //Remove spacial characters
    pub fn remove_special(input: &str) -> String {
        let special = ".,()[]&_\"'-";
        let mut out = input.to_string();
        for c in special.chars() {
            out = out.replace(c, "");
        }
        out = out.replace("  ", " ");
        out.trim().to_string()
    }

    //Clean list of artists
    pub fn clean_artists(input: &Vec<String>) -> Vec<String> {
        let mut clean: Vec<String> = input.into_iter().map(
            |a| MatchingUtils::remove_special(&a.to_lowercase()).trim().to_string()
        ).collect();
        clean.sort();
        clean
    }

    //Clean title for matching, removes special characters etc
    pub fn clean_title_matching(input: &str) -> String {
        let title = MatchingUtils::clean_title(input);
        //Remove edit, specials
        let step1 = title.replace("edit", "");
        let step2 = MatchingUtils::remove_special(&step1);
        step2.to_string()
    }

    //Match atleast 1 artist
    pub fn match_artist(a: &Vec<String>, b: &Vec<String>, strictness: f64) -> bool {
        //Exact match atleast 1 artist
        let clean_a = MatchingUtils::clean_artists(a);
        let clean_b = MatchingUtils::clean_artists(b);
        for artist in &clean_a {
            if clean_b.contains(&artist) {
                return true;
            }
        }

        //String exact match (for separator problems)
        let clean_a_joined = clean_a.join(" ");
        for artist in &clean_b {
            if clean_a_joined.contains(artist) {
                return true;
            }
        }
        let clean_b_joined = clean_b.join(" ");
        for artist in &clean_a {
            if clean_b_joined.contains(artist) {
                return true;
            }
        }

        //Fuzzy
        let acc = normalized_levenshtein(&clean_a.join(" "), &clean_b.join(", "));
        if acc >= strictness {
            return true;
        }

        false
    }

    //Default track matching
    pub fn match_track(info: &AudioFileInfo, tracks: &Vec<Track>, config: &TaggerConfig) -> Option<(f64, Track)> {
        let clean_title = MatchingUtils::clean_title_matching(&info.title);
        //Exact match
        for track in tracks {
            if clean_title == MatchingUtils::clean_title_matching(&track.title) {
                if MatchingUtils::match_artist(&info.artists, &track.artists, config.strictness) {
                    return Some((1.0, track.clone()));
                }
            }
        }
        //Fuzzy match - value, track
        let mut fuzz: Vec<(f64, &Track)> = vec![];
        for track in tracks {
            //Artist
            if !MatchingUtils::match_artist(&info.artists, &track.artists, config.strictness) {
                continue;
            }
            //Match title
            let clean = MatchingUtils::clean_title_matching(&track.title);
            let l = normalized_levenshtein(&clean, &clean_title);
            if l >= config.strictness {
                fuzz.push((l, track));
            }
        }
        //Empty array
        if fuzz.is_empty() {
            return None;
        }
        //Sort
        fuzz.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        Some((fuzz[0].0, fuzz[0].1.to_owned()))
    }

    //Match track, but ignore artist
    pub fn match_track_no_artist(info: &AudioFileInfo, tracks: &Vec<Track>, config: &TaggerConfig) -> Option<(f64, Track)> {
        let clean_title = MatchingUtils::clean_title_matching(&info.title);
        //Exact match
        for track in tracks {
            if clean_title == MatchingUtils::clean_title_matching(&track.title) {
                return Some((1.0, track.clone()));
            }
        }
        //Fuzzy match - value, track
        let mut fuzz: Vec<(f64, &Track)> = vec![];
        for track in tracks {
            //Match title
            let clean = MatchingUtils::clean_title_matching(&track.title);
            let l = normalized_levenshtein(&clean, &clean_title);
            if l >= config.strictness {
                fuzz.push((l, track));
            }
        }
        //Empty array
        if fuzz.is_empty() {
            return None;
        }
        //Sort
        fuzz.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        Some((fuzz[0].0, fuzz[0].1.to_owned()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TaggingState {
    Ok, Error, Skipped
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingStatus {
    pub status: TaggingState,
    pub path: String,
    pub message: Option<String>,
    pub accuracy: Option<f64>,
}

//Wrap for sending into UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingStatusWrap {
    pub status: TaggingStatus,
    pub platform: MusicPlatform,
    pub progress: f64,
}
impl TaggingStatusWrap {
    //pi = platform index, pl = platforms length, p = processed, total = total tracks in this platform
    pub fn wrap(platform: MusicPlatform, status: &TaggingStatus, pi: usize, pl: usize, p: i64, total: usize) -> TaggingStatusWrap {
        TaggingStatusWrap {
            platform,
            status: status.to_owned(),
            progress: (pi as f64 / pl as f64) + ((p as f64 / total as f64) / pl as f64)
        }
    }
}

pub struct Tagger {}
impl Tagger {

    //Returtns progress receiver, and file count
    pub fn tag_files(cfg: &TaggerConfig, mut files: Vec<String>) -> Receiver<TaggingStatusWrap> {
        let total_files = files.len();
        info!("Starting tagger with: {} files!", total_files);

        //Create thread
        let (tx, rx) = channel();
        let config = cfg.clone();
        thread::spawn(move || {
            //Tag
            for (platform_index, platform) in config.platforms.iter().enumerate() {
                //For progress
                let mut processed = 0;
                let total = files.len();
                //No more files
                if files.is_empty() {
                    info!("All tagged succesfully!");
                    break;
                }
                match platform {
                    //Discogs
                    MusicPlatform::Discogs => {
                        //Auth discogs
                        let mut discogs = discogs::Discogs::new();
                        if config.discogs.token.as_ref().is_none() {
                            error!("Missing Discogs token! Skipping Discogs...");
                            continue;
                        }
                        discogs.set_auth_token(config.discogs.token.as_ref().unwrap());
                        if !discogs.validate_token() {
                            error!("Invalid Discogs token! Skipping Discogs...");
                            continue;
                        }
                        //Tag
                        let rx = Tagger::tag_dir_single_thread(&files, discogs, &config);
                        info!("Starting Discogs");
                        for status in rx {
                            info!("[{:?}] State: {:?}, Accuracy: {:?}, Path: {}", MusicPlatform::Discogs, status.status, status.accuracy, status.path);
                            processed += 1;
                            //Send to UI
                            tx.send(TaggingStatusWrap::wrap(MusicPlatform::Discogs, &status, 
                                platform_index, config.platforms.len(), processed, total
                            )).ok();
                            //Fallback
                            if status.status == TaggingState::Ok {
                                files.remove(files.iter().position(|f| f == &status.path).unwrap());
                            }
                        }
                    },
                    platform => {
                        //No config platforms
                        let tagger: Box<dyn TrackMatcher + Send + Sync + 'static> = match platform {
                            MusicPlatform::Beatport => Box::new(beatport::Beatport::new()),
                            MusicPlatform::Traxsource => Box::new(traxsource::Traxsource::new()),
                            MusicPlatform::JunoDownload => Box::new(junodownload::JunoDownload::new()),
                            _ => unreachable!()
                        };
                        info!("Starting {:?}", platform);
                        
                        let rx = if platform == &MusicPlatform::JunoDownload {
                            //JunoDownload cap max threads due to rate limiting
                            let mut config = config.clone();
                            if config.threads > 4 {
                                config.threads = 4;
                            }
                            Tagger::tag_dir_multi_thread(&files, tagger, &config)
                        } else {
                            Tagger::tag_dir_multi_thread(&files, tagger, &config)
                        };
                         
                        //Get statuses
                        for status in rx {
                            info!("[{:?}] State: {:?}, Accuracy: {:?}, Path: {}", platform, status.status, status.accuracy, status.path);
                            processed += 1;
                            //Send to UI
                            tx.send(TaggingStatusWrap::wrap(platform.to_owned(), &status, 
                                platform_index, (&config.platforms).len(), processed, total
                            )).ok();
                            //Fallback
                            if status.status == TaggingState::Ok {
                                files.remove(files.iter().position(|f| f == &status.path).unwrap());
                            }
                        }

                    }
                }
            }
        });
        
        rx
    }

    //Tag single track
    pub fn tag_track(path: &str, tagger_mt: Option<&dyn TrackMatcher>, tagger_st: Option<&mut dyn TrackMatcherST>, config: &TaggerConfig) -> TaggingStatus {
        //Output
        let mut out = TaggingStatus {
            status: TaggingState::Error,
            path: path.to_owned(),
            accuracy: None,
            message: None
        };

        //Filename template
        let mut template = None;
        if config.parse_filename {
            if let Some(t) = &config.filename_template {
                template = AudioFileInfo::parse_template(&t);
            }
        }

        match AudioFileInfo::load_file(path, template) {
            Ok(info) => {
                //Match track
                let result = if let Some(tagger) = tagger_mt {
                    tagger.match_track(&info, &config)
                } else if let Some(tagger) = tagger_st {
                    tagger.match_track(&info, &config)
                } else {
                    out.message = Some("No tagger!".to_owned());
                    return out;
                };
                match result {
                    Ok(o) => {
                        match o {
                            Some((acc, track)) => {
                                //Save to file
                                match track.write_to_file(&info, &config) {
                                    Ok(_) => {
                                        out.accuracy = Some(acc);
                                        out.status = TaggingState::Ok;
                                    },
                                    Err(e) => out.message = Some(format!("Failed writing tags to file: {}", e))
                                }
                            },
                            None => out.message = Some("No match!".to_owned())
                        }
                    },
                    //Failed matching track
                    Err(e) => out.message = Some(format!("Error marching track: {}", e))
                }
            },
            //Failed loading file
            Err(e) => {
                out.status = TaggingState::Skipped;
                warn!("Error loading file: {}", e);
                out.message = Some(format!("Error loading file: {}", e));
            }
        }
        out
    }

    //Get list of all files in with supported extensions
    pub fn get_file_list(path: &str) -> Vec<String> {
        let files: Vec<String> = WalkDir::new(path).into_iter().filter(
            |e| e.is_ok() && 
            EXTENSIONS.iter().any(|&i| e.as_ref().unwrap().path().to_str().unwrap().to_lowercase().ends_with(i))
        ).map(|e| e.unwrap().path().to_str().unwrap().to_owned()).collect();
        files
    }

    //Tag all files with threads specified in config
    pub fn tag_dir_multi_thread(files: &Vec<String>, tagger: Box<(dyn TrackMatcher + Send + Sync + 'static)>, config: &TaggerConfig) -> Receiver<TaggingStatus> {
        info!("Starting tagging: {} files, {} threads!", files.len(), config.threads);
        //Create threadpool
        let pool = ThreadPool::new(config.threads as usize);
        let (tx, rx) = channel();
        let tagger_arc = Arc::new(tagger);
        for file in files {
            let tx = tx.clone();
            let config = config.clone();
            let t = tagger_arc.clone();
            let f = file.to_owned();
            pool.execute(move || {
                let res: TaggingStatus = Tagger::tag_track(&f, Some(&**t), None, &config);
                tx.send(res).ok();
            });
        }
        rx
    }

    //Tag all files with single thread
    pub fn tag_dir_single_thread(files: &Vec<String>, mut tagger: (impl TrackMatcherST + Send + 'static), config: &TaggerConfig) -> Receiver<TaggingStatus> {
        info!("Starting single threaded tagging of {} files!", files.len());
        //Spawn thread
        let (tx, rx) = channel();
        let c = config.clone();
        let f = files.clone();
        thread::spawn(move || {
            for file in f {
                tx.send(Tagger::tag_track(&file, None, Some(&mut tagger), &c)).ok();
            }
        });
        rx
    }
}