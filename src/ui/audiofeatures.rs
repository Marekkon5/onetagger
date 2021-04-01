use std::error::Error;
use std::thread;
use std::sync::mpsc::{channel, Receiver};
use serde::{Serialize, Deserialize};
use serde;

use crate::tagger::MatchingUtils;
use crate::tagger::spotify::Spotify;
use crate::tagger::{Tagger, AudioFileInfo, TaggingState};
use crate::tag::Tag;

//Config from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFeaturesConfig {
    pub path: String,
    pub save_raw: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFeaturesStatus {
    pub state: TaggingState,
    pub path: String,
    pub filename: String,
}

impl AudioFeaturesStatus {
    pub fn new(path: &str, base_path: &str) -> AudioFeaturesStatus {
        AudioFeaturesStatus {
            state: TaggingState::Skipped,
            path: path.to_string(),
            filename: path.replacen(base_path, "", 1)
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

    // //Get spotify track features from isrc
    // fn features_by_isrc(spotify: &Spotify, isrc: &str) -> Result<rspotify::model::audio::AudioFeatures, Box<dyn Error>> {
    //     //Search
    //     let results = spotify.search_tracks(&format!("isrc:{}", isrc), 1)?;
    //     let track = results.first().ok_or("No results")?;
    //     //Get features
    //     let features = spotify.audio_features(track.id.as_ref().ok_or("Invalid track")?)?;
    //     Ok(features)
    // }

    //Write to path
    fn write_to_path(path: &str, features: &rspotify::model::audio::AudioFeatures, config: &AudioFeaturesConfig) -> Result<(), Box<dyn Error>> {
        //Load tag
        let mut tag_wrap = Tag::load_file(path)?;
        let tag = tag_wrap.tag_mut().ok_or("No tag!")?;

        //Write features
        if config.save_raw {
            tag.set_raw("1T_ACOUSTICNESS", vec![((features.acousticness * 100.0) as u8).to_string()], true);
            tag.set_raw("1T_DANCEABILITY", vec![((features.danceability * 100.0) as u8).to_string()], true);
            tag.set_raw("1T_ENERGY", vec![((features.energy * 100.0) as u8).to_string()], true);
            tag.set_raw("1T_INSTRUMENTALNESS", vec![((features.instrumentalness * 100.0) as u8).to_string()], true);
            tag.set_raw("1T_LIVENESS", vec![((features.liveness * 100.0) as u8).to_string()], true);
            tag.set_raw("1T_SPEECHINESS", vec![((features.speechiness * 100.0) as u8).to_string()], true);
            tag.set_raw("1T_VALENCE", vec![((features.valence * 100.0) as u8).to_string()], true);    
        }

        //Save
        tag.save_file(path)?;
        Ok(())
    }
}
