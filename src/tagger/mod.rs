use std::error::Error;
use std::thread;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver};
use regex::Regex;
use walkdir::WalkDir;
use threadpool::ThreadPool;
use strsim::normalized_levenshtein;
use chrono::{NaiveDate, Datelike};
use serde::{Serialize, Deserialize};
use crate::tag::{AudioFileFormat, Tag, Field, TagDate, CoverType};

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
    JunoDownload
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggerConfig {
    //Global
    pub platforms: Vec<MusicPlatform>,
    pub path: String,

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

    //Platform specific
    pub beatport: BeatportConfig,
    pub discogs: DiscogsConfig
}

//Beatport specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatportConfig {
    pub art_resolution: i64,
    pub max_pages: i64
}

//Discogs specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscogsConfig {
    pub token: Option<String>,
    pub max_results: i16,
    pub styles: DiscogsStyles
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscogsStyles {
    Default,
    OnlyGenres,
    OnlyStyles,
    MergeToGenres,
    MergeToStyles,
    StylesToGenre,
    GenresToStyle
}


#[derive(Debug, Clone)]
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
    
    //Only year OR date should be available
    pub release_year: Option<i64>,
    pub release_date: Option<NaiveDate>,
    pub publish_year: Option<i64>,
    pub publish_date: Option<NaiveDate>
}

impl Track {
    //Write tags to file
    pub fn write_to_file(&self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<(), Box<dyn Error>> {
        //Get tag
        let mut tag_wrap = Tag::load_file(&info.path)?;
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
            tag.set_field(Field::Key, vec![self.key.as_ref().unwrap().to_string()], config.overwrite);
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
            if config.merge_genres {
                //Merge with existing ones
                let mut current: Vec<String> = tag.get_field(Field::Style).unwrap_or(vec![]).iter().map(|s| s.to_lowercase()).collect();
                let mut styles = self.styles.clone().into_iter().filter(|s| !current.iter().any(|i| i == &s.to_lowercase())).collect();
                current.append(&mut styles);
                tag.set_field(Field::Style, current, config.overwrite); 
            } else {
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
        //Album art
        if (config.overwrite || tag.get_art().is_empty()) && self.art.is_some() && config.album_art {
            match self.download_art(self.art.as_ref().unwrap()) {
                Ok(data) => {
                    tag.set_art(CoverType::Front, "image/jpeg", Some("Cover"), data.clone());
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
                Err(e) => warn!("Error downloading album art! {}", e)
            }
        }

        //Save
        tag.save_file(&info.path)?;
        Ok(())
    }

    //Download album art
    fn download_art(&self, url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(reqwest::blocking::get(url)?.bytes()?.to_vec())
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
    pub fn load_file(path: &str) -> Result<AudioFileInfo, Box<dyn Error>> {
        let tag_wrap = Tag::load_file(&path)?;
        let tag = tag_wrap.tag().unwrap();

        Ok(AudioFileInfo {
            format: tag_wrap.format.to_owned(),
            title: tag.get_field(Field::Title).ok_or("Missing title!")?.first().unwrap().to_owned().to_owned(),
            artists: AudioFileInfo::parse_artist_tag(tag.get_field(Field::Artist).ok_or("Missing artists!")?
                .iter().map(|a| a.as_ref()).collect()),
            path: path.to_owned(),
            isrc: tag.get_field(Field::ISRC).unwrap_or(vec![]).first().map(String::from)
        })
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
        let step1 = input.to_lowercase();
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
        let special = ".,()[]&_\"'";
        let mut out = String::new();
        for c in special.chars() {
            out = input.replace(c, "");
        }
        out.trim().to_string()
    }

    //Clean list of artists
    pub fn clean_artists(input: &Vec<String>) -> Vec<String> {
        let mut clean: Vec<String> = input.into_iter().map(
            |a| MatchingUtils::remove_special(&a.to_lowercase())
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
        let clean_a = MatchingUtils::clean_artists(a);
        let clean_b = MatchingUtils::clean_artists(b);
        for artist in &clean_a {
            if clean_b.contains(&artist) {
                return true;
            }
        }
        //Fuzzy
        let acc = normalized_levenshtein(&clean_a.join(", "), &clean_b.join(", "));
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
    pub progress: f64
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

    //Tag directory
    pub fn tag_dir(cfg: &TaggerConfig) -> Receiver<TaggingStatusWrap> {
        //Load files
        let mut files = Tagger::get_file_list(&cfg.path);
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
                        let rx = Tagger::tag_dir_multi_thread(&files, tagger, &config);
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

        match AudioFileInfo::load_file(path) {
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
                out.message = Some(format!("Error loading file: {}", e));
            }
        }
        out
    }

    //Get list of all files in with supported extensions
    pub fn get_file_list(path: &str) -> Vec<String> {
        let supported_extensions = vec![".mp3", ".flac", ".aif", ".aiff"];
        let files: Vec<String> = WalkDir::new(path).into_iter().filter(
            |e| e.is_ok() && 
            supported_extensions.iter().any(|&i| e.as_ref().unwrap().path().to_str().unwrap().to_lowercase().ends_with(i))
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

