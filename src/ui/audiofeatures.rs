use std::error::Error;
use std::thread;
use std::sync::mpsc::{channel, Receiver};
use rspotify::model::track::FullTrack;
use serde::{Serialize, Deserialize};
use serde;

use crate::tagger::MatchingUtils;
use crate::tagger::spotify::Spotify;
use crate::tagger::{AudioFileInfo, TaggingState, TaggingStatus, TaggingStatusWrap, MusicPlatform};
use crate::tag::{Tag, AudioFileFormat, UITag, TagSeparators};

//  CONFIG SERIALIZATION

// Config from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFeaturesConfig {
    pub path: Option<String>,
    pub main_tag: UITag,
    pub separators: TagSeparators,
    pub properties: AFProperties
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

impl AFProperties {
    // Merge properties into list with actual values
    pub fn merge_with_values(&self, features: &rspotify::model::audio::AudioFeatures, track: &FullTrack, format: AudioFileFormat) -> Vec<AFPropertyMerged> {
        vec![
            AFPropertyMerged::new(features.danceability, &self.danceability, &format)
                .add_main_value("#dynamics-high", "#dynamics-med", "#dynamics-low"),
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
                .add_main_value("#negative", "#neutral", "#positive"),
            AFPropertyMerged::new(track.popularity as f32 / 100.0, &self.popularity, &format)
                .add_main_value("", "", "#popular")
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

// Audio Features property
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AFProperty {
    pub tag: UITag,
    pub range: AFRange,
    pub enabled: bool
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
    pub fn start_tagging(config: AudioFeaturesConfig, spotify: Spotify, files: Vec<String>) -> Receiver<TaggingStatusWrap> {
        let file_count = files.len();
        // Start
        let (tx, rx) = channel();
        thread::spawn(move || {
            for (i, file) in files.iter().enumerate() {
                // Create status
                let mut status = TaggingStatus {
                    status: TaggingState::Error,
                    path: file.to_owned(),
                    message: None, accuracy: None
                };
                // Load file
                if let Ok(info) = AudioFileInfo::load_file(&file, None) {
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
                                    status.status = TaggingState::Error;
                                }
                            };
                        },
                        // Failed searching track
                        Err(e) => {
                            error!("Audio features search track by ISRC error: {}", e);
                            status.status = TaggingState::Error;
                        }
                    }
                }
                // Send status
                tx.send(TaggingStatusWrap::wrap(
                    MusicPlatform::Spotify, 
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
                track_id = Some(t.id.as_ref().ok_or("Missing track ID")?.to_owned());
                full_track = Some(t.clone());
                info!("[AF] Found track by ISRC. {:?}", track_id);
            }
        }
        // Fallback
        if track_id.is_none() {
            let q = format!("{} {}", track.artists[0].to_lowercase(), MatchingUtils::clean_title(track.title()?));
            let results = spotify.search_tracks(&q, 20)?;
            // Match
            for t in results {
                let title_1 = MatchingUtils::clean_title_matching(&t.name);
                let title_2 = MatchingUtils::clean_title_matching(track.title()?);
                let artists: Vec<String> = t.artists.iter().map(|a| a.name.to_owned()).collect();
                if title_1 == title_2 && MatchingUtils::match_artist(&artists, &track.artists, 1.0) {
                    if let Some(id) = &t.id {
                        info!("[AF] Matched by exact title. {}", id);
                        track_id = Some(id.to_string());
                        full_track = Some(t.clone());
                        break;
                    }
                }
            }
        }

        // Get features
        let features = spotify.audio_features(&track_id.ok_or("Invalid track / no match")?)?;
        Ok((features, full_track.unwrap()))
    }

    // Write to path
    fn write_to_path(path: &str, features: &rspotify::model::audio::AudioFeatures, full_track: &FullTrack, config: &AudioFeaturesConfig) -> Result<(), Box<dyn Error>> {
        // Load tag
        let mut tag_wrap = Tag::load_file(path, false)?;
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

        // Save
        tag.save_file(path)?;
        Ok(())
    }
}
