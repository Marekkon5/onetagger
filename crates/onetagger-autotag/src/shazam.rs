use std::error::Error;
use std::thread::Builder;
use rodio::source::UniformSourceIterator;
use serde::{Serialize, Deserialize};
use songrec::SignatureGenerator;
use onetagger_player::AudioSources;

pub struct Shazam;

impl Shazam {
    /// Recognize song on Shazam from path, returns Track, Duration
    pub fn recognize_from_file(path: &str) -> Result<(ShazamTrack, u128), Box<dyn Error>> {
        // Load file
        let source = AudioSources::from_path(path)?;
        let duration = source.duration();
        let conv = UniformSourceIterator::new(source.get_source()?, 1, 16000);
        // Get 12s part from middle
        let buffer = if duration >= 12000 {
            // ((duration / 1000) * 16KHz) / 2 (half duration) - (6 * 16KHz) seconds.
            conv.skip((duration * 8 - 96000) as usize).take(16000 * 12).collect::<Vec<i16>>()
        } else {
            conv.collect::<Vec<i16>>()
        };
        // Calculating singnature requires 6MB stack, because it allocates >2MB of buffers for some reasno
        let signature = Builder::new()
            .stack_size(1024 * 1024 * 6)
            .spawn(move || {
                SignatureGenerator::make_signature_from_buffer(&buffer)
            })
            .unwrap()
            .join()
            .unwrap();
        let response = songrec::recognize_song_from_signature(&signature)?;
        let response: ShazamResponse = serde_json::from_value(response)?;
        let track = response.track.ok_or("Shazam returned no matches!")?;
        Ok((track, duration))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamResponse {
    pub timestamp: u64,
    pub tagid: String,
    pub track: Option<ShazamTrack>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamTrack {
    pub albumadamid: Option<String>,
    pub artists: Option<Vec<ShazamSmall>>,
    pub genres: Option<ShazamGenres>,
    pub images: Option<ShazamImages>,
    pub isrc: Option<String>,
    pub key: String,
    pub sections: Vec<ShazamSection>,
    /// Song title
    pub title: String,
    /// Artist
    pub subtitle: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamSmall {
    pub adamid: String,
    pub id: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamGenres {
    pub primary: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamImages {
    pub background: String,
    pub coverart: String,
    pub coverarthq: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ShazamSection {
    MetaSection {
        metadata: Vec<ShazamMetadataSection>
    },
    ArtistSection {
        id: String,
        name: String,
        tabname: String,
        // Has to == "ARTIST"
        #[serde(rename = "type")]
        _type: String
    },
    Other {}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamMetadataSection {
    pub text: String,
    pub title: String
}