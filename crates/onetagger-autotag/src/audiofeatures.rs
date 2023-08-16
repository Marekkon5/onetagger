use std::error::Error;
use std::path::{PathBuf, Path};
use std::sync::atomic::Ordering;
use std::thread;
use chrono::Local;
use crossbeam_channel::{unbounded, Receiver};
use onetagger_platforms::spotify::rspotify::model::{TrackId, Id};
use serde::{Serialize, Deserialize};
use onetagger_tagger::{AudioFileInfo, MatchingUtils};
use onetagger_platforms::spotify::{Spotify, rspotify};
use onetagger_platforms::spotify::rspotify::model::track::FullTrack;
use onetagger_tag::{Tag, AudioFileFormat, FrameName, TagSeparators};

use crate::{TaggingState, TaggingStatus, TaggingStatusWrap, AudioFileInfoImpl, STOP_TAGGING};


// Config from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFeaturesConfig {
    pub path: Option<PathBuf>,
    pub main_tag: FrameName,
    pub separators: TagSeparators,
    pub properties: AFProperties,
    pub meta_tag: bool,
    pub skip_tagged: bool,
    pub include_subfolders: bool,
}

impl Default for AudioFeaturesConfig {
    fn default() -> Self {
        Self { 
            path: None,
            main_tag: FrameName::same("AUDIO_FEATURES"),
            separators: Default::default(),
            properties: Default::default(),
            meta_tag: true,
            skip_tagged: false,
            include_subfolders: true,
        }
    }
}

// Audio features
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AFProperties {
    pub acousticness: AFProperty,
    pub danceability: AFProperty,
    pub energy: AFProperty,
    pub instrumentalness: AFProperty,
    pub liveness: AFProperty,
    pub speechiness: AFProperty,
    pub valence: AFProperty,
    pub popularity: AFProperty
}

impl Default for AFProperties {
    fn default() -> Self {
        // Suffering, but copy pasted from AudioFeatures.vue
        Self {
            acousticness: AFProperty { enabled: true, range: AFRange {min: 0, max: 90}, 
                tag: FrameName {id3: "1T_ACOUSTICNESS".to_string(), vorbis: "1T_ACOUSTICNESS".to_string(), mp4: "1T_ACOUSTICNESS".to_string()}},
            danceability: AFProperty { enabled: true, range: AFRange {min: 20, max: 80}, 
                tag: FrameName {id3: "1T_DANCEABILITY".to_string(), vorbis: "1T_DANCEABILITY".to_string(), mp4: "1T_DANCEABILITY".to_string()}},
            energy: AFProperty { enabled: true, range: AFRange {min: 20, max: 90}, 
                tag: FrameName {id3: "1T_ENERGY".to_string(), vorbis: "1T_ENERGY".to_string(), mp4: "1T_ENERGY".to_string()}},
            instrumentalness: AFProperty { enabled: true, range: AFRange {min: 50, max: 90}, 
                tag: FrameName {id3: "1T_INSTRUMENTALNESS".to_string(), vorbis: "1T_INSTRUMENTALNESS".to_string(), mp4: "1T_INSTRUMENTALNESS".to_string()}},
            liveness: AFProperty { enabled: true, range: AFRange {min: 0, max: 80}, 
                tag: FrameName {id3: "1T_LIVENESS".to_string(), vorbis: "1T_LIVENESS".to_string(), mp4: "1T_LIVENESS".to_string()}},
            speechiness: AFProperty { enabled: true, range: AFRange {min: 0, max: 70}, 
                tag: FrameName {id3: "1T_SPEECHINESS".to_string(), vorbis: "1T_SPEECHINESS".to_string(), mp4: "1T_SPEECHINESS".to_string()}},
            valence: AFProperty { enabled: true, range: AFRange {min: 15, max: 85}, 
                tag: FrameName {id3: "1T_VALENCE".to_string(), vorbis: "1T_VALENCE".to_string(), mp4: "1T_VALENCE".to_string()}},
            popularity: AFProperty { enabled: true, range: AFRange {min: 0, max: 80}, 
                tag: FrameName {id3: "1T_POPULARITY".to_string(), vorbis: "1T_POPULARITY".to_string(), mp4: "1T_POPULARITY".to_string()}}
        }
    }
}

// Audio Features property
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AFProperty {
    pub tag: FrameName,
    pub range: AFRange,
    pub enabled: bool
}


impl AFProperties {
    // Merge properties into list with actual values
    pub fn merge_with_values(&self, features: &rspotify::model::audio::AudioFeatures, track: &FullTrack, format: AudioFileFormat) -> Vec<AFPropertyMerged> {
        vec![
            AFPropertyMerged::new(features.danceability, &self.danceability, &format)
                .add_main_value("#dance-low", "#dance-med", "#dance-high"),
            AFPropertyMerged::new(features.acousticness, &self.acousticness, &format)
                .add_main_value("#electronic", "", "#acoustic"),
            AFPropertyMerged::new(features.energy, &self.energy, &format)
                .add_main_value("#energy-low", "#energy-med", "#energy-high"), 
            AFPropertyMerged::new(features.instrumentalness, &self.instrumentalness, &format)
                .add_main_value("#vocal-high", "#vocal-med", "#vocal-low"),
            AFPropertyMerged::new(features.liveness, &self.liveness, &format)
                .add_main_value("#recording", "", "#live"),
            AFPropertyMerged::new(features.speechiness, &self.speechiness, &format)
                .add_main_value("#music", "", "#speech"),
            AFPropertyMerged::new(features.valence, &self.valence, &format)
                .add_main_value("#negative", "#balanced", "#positive"),
            AFPropertyMerged::new(track.popularity as f32 / 100.0, &self.popularity, &format)
                .add_main_value("#unpopular", "", "#popular")
        ]
    }
}

// Property merged with value
pub struct AFPropertyMerged {
    pub tag: String,
    pub value: i8,
    range: AFRange,
    enabled: bool,
    pub main_value: String
}

impl AFPropertyMerged {
    // Create new merged property, value = rspotify value
    pub fn new(value: f32, property: &AFProperty, format: &AudioFileFormat) -> AFPropertyMerged {
        AFPropertyMerged {
            value: (value * 100.0) as i8,
            tag: property.tag.by_format(format),
            enabled: property.enabled,
            range: property.range.to_owned(),
            main_value: String::new()
        }
    }

    // Set main values by range
    pub fn add_main_value(mut self, under: &str, middle: &str, over: &str) -> Self {
        if self.enabled {
            self.main_value = self.range.select(self.value, under, middle, over);
        }
        self
    }
}

// Threshold range in config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AFRange {
    pub min: i8,
    pub max: i8
}
impl AFRange {
    // Select value under or over range
    pub fn select(&self, v: i8, under: &str, middle: &str, over: &str) -> String {
        if v < self.min {
            return under.to_owned();
        }
        if v >= self.max {
            return over.to_owned();
        }
        middle.to_owned()
    }
}

pub struct AudioFeatures {}
impl AudioFeatures {
    // Returtns progress receiver, and file count
    pub fn start_tagging(config: AudioFeaturesConfig, spotify: Spotify, files: Vec<PathBuf>) -> Receiver<TaggingStatusWrap> {
        STOP_TAGGING.store(false, Ordering::SeqCst);
        let file_count = files.len();
        // Start
        let (tx, rx) = unbounded();
        thread::spawn(move || {
            for (i, file) in files.iter().enumerate() {
                // Stop tagging midway
                if STOP_TAGGING.load(Ordering::SeqCst) {
                    break;
                }

                // Create status
                let mut status = TaggingStatus {
                    status: TaggingState::Error,
                    path: file.to_owned(),
                    message: None, accuracy: None, used_shazam: false
                };
                // Load file
                if let Ok(info) = AudioFileInfo::load_file(&file, None, None) {
                    if config.skip_tagged && info.tagged.af() {
                        // Skip tagged
                        status.status = TaggingState::Skipped;
                        status.message = Some("Already tagged!".to_string());
                    } else {
                        // Match and get features
                        match AudioFeatures::find_features(&spotify, &info) {
                            Ok((features, full_track)) => {
                                // Write to file
                                match AudioFeatures::write_to_path(&file, &features, &full_track, &config) {
                                    Ok(_) => {
                                        status.status = TaggingState::Ok;
                                    },
                                    Err(e) => {
                                        error!("Audio features failed writing to tag: {}", e);
                                        status.message = Some(format!("Audio features failed writing to tag: {}", e));
                                        status.status = TaggingState::Error;
                                    }
                                };
                            },
                            // Failed searching track
                            Err(e) => {
                                error!("Audio features search track by ISRC error: {}", e);
                                status.message = Some(format!("Audio features search track by ISRC error: {}", e));
                                status.status = TaggingState::Error;
                            }
                        }
                    }
                }
                // Send status
                tx.send(TaggingStatusWrap::wrap(
                    "spotify", 
                    &status, 
                    0,
                    1,
                    i as i64, 
                    file_count
                )).ok();
            }
        });
        rx
    }

    // Get features from track
    fn find_features(spotify: &Spotify, track: &AudioFileInfo) -> Result<(rspotify::model::audio::AudioFeatures, FullTrack), Box<dyn Error>> {
        let (mut track_id, mut full_track): (Option<String>, Option<FullTrack>) = (None, None);
        // Get by ISRC
        if let Some(isrc) = track.isrc.as_ref() {
            let results = spotify.search_tracks(&format!("isrc:{}", isrc), 1)?;
            if let Some(t) = results.first() {
                track_id = Some(t.id.as_ref().ok_or("Missing track ID")?.id().to_owned());
                full_track = Some(t.clone());
                info!("[AF] Found track by ISRC. {:?}", track_id);
            }
        }
        // Fallback
        if track_id.is_none() {
            let q = format!("{} {}", track.artists.first().ok_or("Track is missing artist")?.to_lowercase(), MatchingUtils::clean_title(track.title()?));
            let results = spotify.search_tracks(&q, 20)?;
            // Match
            for t in results {
                let title_1 = MatchingUtils::clean_title_matching(&t.name);
                let title_2 = MatchingUtils::clean_title_matching(track.title()?);
                let artists: Vec<String> = t.artists.iter().map(|a| a.name.to_owned()).collect();
                if title_1 == title_2 && MatchingUtils::match_artist(&artists, &track.artists, 1.0) {
                    if let Some(id) = &t.id {
                        info!("[AF] Matched by exact title. {id}");
                        track_id = Some(id.id().to_owned());
                        full_track = Some(t.clone());
                        break;
                    }
                }
            }
        }

        // Get features
        let features = spotify.audio_features(&TrackId::from_id(&track_id.ok_or("Invalid track / no match")?)?)?;
        Ok((features, full_track.unwrap()))
    }

    // Write to path
    fn write_to_path(path: impl AsRef<Path>, features: &rspotify::model::audio::AudioFeatures, full_track: &FullTrack, config: &AudioFeaturesConfig) -> Result<(), Box<dyn Error>> {
        // Load tag
        let mut tag_wrap = Tag::load_file(&path, false)?;
        tag_wrap.set_separators(&config.separators);

        let format = tag_wrap.format();
        let tag = tag_wrap.tag_mut();

        // Get properties
        let mut main_tag = vec![];
        for property in config.properties.merge_with_values(features, full_track, format.clone()) {
            if !property.tag.is_empty() {
                tag.set_raw(&property.tag, vec![property.value.to_string()], true);
            }
            if !property.main_value.is_empty() {
                main_tag.push(property.main_value);
            }
        }
        // Set main tag
        if !main_tag.is_empty() {
            tag.set_raw(&config.main_tag.by_format(&format), main_tag, true);
        }

        // Meta tag
        if config.meta_tag {
            let time = Local::now();
            tag.set_raw("1T_TAGGEDDATE", vec![format!("{}_AF", time.format("%Y-%m-%d %H:%M:%S"))], true);
        }

        // Save
        tag.save_file(path.as_ref())?;
        Ok(())
    }
}