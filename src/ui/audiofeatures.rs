use std::error::Error;
use std::thread;
use std::sync::mpsc::{channel, Receiver};
use serde::{Serialize, Deserialize};
use serde;

use crate::tagger::spotify::Spotify;
use crate::tagger::{Tagger, AudioFileInfo, TaggingState};
use crate::tag::Tag;

//Config from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFeaturesConfig {
    pub path: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFeaturesStatus {
    state: TaggingState
}

impl AudioFeaturesStatus {
    //For easier creation
    pub fn new(state: TaggingState) -> AudioFeaturesStatus {
        AudioFeaturesStatus {
            state
        }
    }
}

pub struct AudioFeatures {}
impl AudioFeatures {
    pub fn start_tagging(config: &AudioFeaturesConfig, spotify: Spotify) -> Receiver<AudioFeaturesStatus> {
        //Load files
        let files = Tagger::get_file_list(&config.path);
        //Start
        let (tx, rx) = channel();
        thread::spawn(move || {
            for file in files {
                //Load file
                if let Ok(info) = AudioFileInfo::load_file(&file) {
                    if let Some(isrc) = info.isrc {
                        //Get by ISRC
                        match AudioFeatures::features_by_isrc(&spotify, &isrc) {
                            Ok(features) => {
                                //Write to file
                                match AudioFeatures::write_to_path(&file, &features) {
                                    Ok(_) => {
                                        tx.send(AudioFeaturesStatus::new(TaggingState::Ok)).ok();
                                    },
                                    Err(e) => {
                                        error!("Audio features failed writing to tag: {}", e);
                                        tx.send(AudioFeaturesStatus::new(TaggingState::Error)).ok();
                                    }
                                };
                            },
                            //Failed searching track
                            Err(e) => {
                                error!("Audio features search track by ISRC error: {}", e);
                                tx.send(AudioFeaturesStatus::new(TaggingState::Error)).ok();
                            }
                        }

                        continue;
                    }
                }
                //Missing ISRC or invalid track
                tx.send(AudioFeaturesStatus::new(TaggingState::Skipped)).ok();
            }
        });
        rx
    }

    //Get spotify track features from isrc
    fn features_by_isrc(spotify: &Spotify, isrc: &str) -> Result<rspotify::model::audio::AudioFeatures, Box<dyn Error>> {
        //Search
        let results = spotify.search_tracks(&format!("isrc:{}", isrc), 1)?;
        let track = results.first().ok_or("No results")?;
        //Get features
        let features = spotify.spotify.audio_features(track.id.as_ref().ok_or("Invalid track")?)?;
        Ok(features)
    }

    //Write to path
    fn write_to_path(path: &str, features: &rspotify::model::audio::AudioFeatures) -> Result<(), Box<dyn Error>> {
        //Load tag
        let mut tag_wrap = Tag::load_file(path)?;
        let tag = tag_wrap.tag_mut().ok_or("No tag!")?;

        //Write features
        tag.set_raw("1T_ACOUSTICNESS", vec![((features.acousticness * 100.0) as u8).to_string()], true);
        tag.set_raw("1T_DANCEABILITY", vec![((features.danceability * 100.0) as u8).to_string()], true);
        tag.set_raw("1T_ENERGY", vec![((features.energy * 100.0) as u8).to_string()], true);
        tag.set_raw("1T_INSTRUMENTALNESS", vec![((features.instrumentalness * 100.0) as u8).to_string()], true);
        tag.set_raw("1T_LIVENESS", vec![((features.liveness * 100.0) as u8).to_string()], true);
        tag.set_raw("1T_SPEECHINESS", vec![((features.speechiness * 100.0) as u8).to_string()], true);
        tag.set_raw("1T_VALENCE", vec![((features.valence * 100.0) as u8).to_string()], true);

        //Save
        tag.save_file(path)?;
        Ok(())
    }
}
