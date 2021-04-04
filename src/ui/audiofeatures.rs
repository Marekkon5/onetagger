use std::error::Error;
use std::thread;
use std::sync::mpsc::{channel, Receiver};
use serde::{Serialize, Deserialize};
use serde;

use crate::tagger::MatchingUtils;
use crate::tagger::spotify::Spotify;
use crate::tagger::{Tagger, AudioFileInfo, TaggingState};
use crate::tag::{Tag, AudioFileFormat};

// CONFIG SERIALIZATION

//Config from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFeaturesConfig {
    pub path: String,
    pub main_tag: AFTag,
    pub properties: AFProperties
}

//Audio features
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AFProperties {
    pub acousticness: AFProperty,
    pub danceability: AFProperty,
    pub energy: AFProperty,
    pub instrumentalness: AFProperty,
    pub liveness: AFProperty,
    pub speechiness: AFProperty,
    pub valence: AFProperty
}

impl AFProperties {
    //Merge properties into list with actual values
    pub fn merge_with_values(&self, features: &rspotify::model::audio::AudioFeatures, format: AudioFileFormat) -> Vec<AFPropertyMerged> {
        vec![
            AFPropertyMerged::new(features.danceability, &self.danceability, &format)
                .add_main_value("Unrhythmical", "Danceable"),
            AFPropertyMerged::new(features.acousticness, &self.acousticness, &format)
                .add_main_value("Electronic", "Acoustic"),
            AFPropertyMerged::new(features.energy, &self.energy, &format)
                .add_main_value("Passive", "Energetic"), 
            AFPropertyMerged::new(features.instrumentalness, &self.instrumentalness, &format)
                .add_main_value("Vocal", "Instrumental"),
            AFPropertyMerged::new(features.liveness, &self.liveness, &format)
                .add_main_value("Recorded", "Live"),
            AFPropertyMerged::new(features.speechiness, &self.speechiness, &format)
                .add_main_value("Speechless", "Speech"),
            AFPropertyMerged::new(features.valence, &self.valence, &format)
                .add_main_value("Negative", "Positive")
        ]
    }
}

//Property merged with value
pub struct AFPropertyMerged {
    pub tag: String,
    pub value: i8,
    range: AFRange,
    enabled: bool,
    pub main_value: String
}

impl AFPropertyMerged {
    //Create new merged property, value = rspotify value
    pub fn new(value: f32, property: &AFProperty, format: &AudioFileFormat) -> AFPropertyMerged {
        AFPropertyMerged {
            value: (value * 100.0) as i8,
            tag: property.tag.by_format(format),
            enabled: property.enabled,
            range: property.range.to_owned(),
            main_value: String::new()
        }
    }

    //Set main values by range
    pub fn add_main_value(mut self, under: &str, over: &str) -> Self {
        if self.enabled {
            self.main_value = self.range.select(self.value, under, over);
        }
        self
    }
}

//Audio Features property
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AFProperty {
    pub tag: AFTag,
    pub range: AFRange,
    pub enabled: bool
}

//Tag info from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AFTag {
    pub id3: String,
    pub flac: String
}

impl AFTag {
    //Get tag by AudioFileFormat
    pub fn by_format(&self, format: &AudioFileFormat) -> String {
        if format.to_owned() == AudioFileFormat::FLAC {
            self.flac.to_owned()
        } else {
            self.id3.to_owned()
        }
    }
}

//Threshold range in config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AFRange {
    pub min: i8,
    pub max: i8
}
impl AFRange {
    //Select value under or over range
    pub fn select(&self, v: i8, under: &str, over: &str) -> String {
        if v < self.min {
            return under.to_owned();
        }
        if v >= self.max {
            return over.to_owned();
        }
        String::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFeaturesStatus {
    pub state: TaggingState,
    pub path: String,
    pub filename: String,
}

impl AudioFeaturesStatus {
    pub fn new(path: &str, base_path: &str) -> AudioFeaturesStatus {
        //Get filename without base path
        let mut filename = path.replacen(base_path, "", 1);
        if filename.starts_with("/") || filename.starts_with("\\") {
            filename = filename[1..].to_owned();
        }

        AudioFeaturesStatus {
            state: TaggingState::Skipped,
            path: path.to_string(),
            filename 
        }
    }
}

pub struct AudioFeatures {}
impl AudioFeatures {
    pub fn start_tagging(config: AudioFeaturesConfig, spotify: Spotify) -> Receiver<AudioFeaturesStatus> {
        //Load files
        let files = Tagger::get_file_list(&config.path);
        //Start
        let (tx, rx) = channel();
        thread::spawn(move || {
            for file in files {
                //Create status
                let mut status = AudioFeaturesStatus::new(&file, &config.path);
                //Load file
                if let Ok(info) = AudioFileInfo::load_file(&file) {
                    //Match and get features
                    match AudioFeatures::find_features(&spotify, &info) {
                        Ok(features) => {
                            //Write to file
                            match AudioFeatures::write_to_path(&file, &features, &config) {
                                Ok(_) => {
                                    status.state = TaggingState::Ok;
                                },
                                Err(e) => {
                                    error!("Audio features failed writing to tag: {}", e);
                                    status.state = TaggingState::Error;
                                }
                            };
                        },
                        //Failed searching track
                        Err(e) => {
                            error!("Audio features search track by ISRC error: {}", e);
                            status.state = TaggingState::Error;
                        }
                    }
                }
                //Send status
                tx.send(status).ok();
            }
        });
        rx
    }

    //Get features from track
    fn find_features(spotify: &Spotify, track: &AudioFileInfo) -> Result<rspotify::model::audio::AudioFeatures, Box<dyn Error>> {
        let mut track_id: Option<String> = None;
        //Get by ISRC
        if let Some(isrc) = track.isrc.as_ref() {
            let results = spotify.search_tracks(&format!("isrc:{}", isrc), 1)?;
            if let Some(track) = results.first() {
                track_id = Some(track.id.as_ref().ok_or("Missing track ID")?.to_owned());
                debug!("Found track by ISRC. {:?}", track_id);
            }
        }
        //Fallback
        if track_id.is_none() {
            let q = format!("{} {}", track.artists[0], track.title);
            let results = spotify.search_tracks(&q, 20)?;
            //Match
            for t in results {
                let title_1 = MatchingUtils::clean_title_matching(&t.name);
                let title_2 = MatchingUtils::clean_title_matching(&track.title);
                let artists: Vec<String> = t.artists.iter().map(|a| a.name.to_owned()).collect();
                if title_1 == title_2 && MatchingUtils::match_artist(&artists, &track.artists, 1.0) {
                    if let Some(id) = t.id {
                        debug!("Matched by exact title. {}", id);
                        track_id = Some(id);
                        break;
                    }
                }
            }
        }

        //Get features
        let features = spotify.audio_features(&track_id.ok_or("Invalid track")?)?;
        Ok(features)
    }

    //Write to path
    fn write_to_path(path: &str, features: &rspotify::model::audio::AudioFeatures, config: &AudioFeaturesConfig) -> Result<(), Box<dyn Error>> {
        //Load tag
        let mut tag_wrap = Tag::load_file(path)?;
        let format = tag_wrap.format.clone();
        let tag = tag_wrap.tag_mut().ok_or("No tag!")?;

        //Get properties
        let mut main_tag = vec![];
        for property in config.properties.merge_with_values(features, format.clone()) {
            if !property.tag.is_empty() {
                tag.set_raw(&property.tag, vec![property.value.to_string()], true);
            }
            if !property.main_value.is_empty() {
                main_tag.push(property.main_value);
            }
        }
        //Set main tag
        if !main_tag.is_empty() {
            tag.set_raw(&config.main_tag.by_format(&format), main_tag, true);
        }

        //Save
        tag.save_file(path)?;
        Ok(())
    }
}
