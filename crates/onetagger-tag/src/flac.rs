use std::error::Error;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use metaflac::Tag;
use metaflac::block::PictureType;
use crate::Lyrics;
use crate::{Field, TagDate, CoverType, TagImpl};

// Cannot be a HashMap, because doens't implement Hash
const COVER_TYPES: [(PictureType, CoverType); 21] = [
    (PictureType::Other, CoverType::Other),
    (PictureType::Icon, CoverType::Icon),
    (PictureType::OtherIcon, CoverType::OtherIcon),
    (PictureType::CoverFront, CoverType::CoverFront),
    (PictureType::CoverBack, CoverType::CoverBack),
    (PictureType::Leaflet, CoverType::Leaflet),
    (PictureType::Media, CoverType::Media),
    (PictureType::LeadArtist, CoverType::LeadArtist),
    (PictureType::Artist, CoverType::Artist),
    (PictureType::Conductor, CoverType::Conductor),
    (PictureType::Band, CoverType::Band),
    (PictureType::Composer, CoverType::Composer),
    (PictureType::Lyricist, CoverType::Lyricist),
    (PictureType::RecordingLocation, CoverType::RecordingLocation),
    (PictureType::DuringRecording, CoverType::DuringRecording),
    (PictureType::DuringPerformance, CoverType::DuringPerformance),
    (PictureType::ScreenCapture, CoverType::ScreenCapture),
    (PictureType::BrightFish, CoverType::BrightFish),
    (PictureType::Illustration, CoverType::Illustration),
    (PictureType::BandLogo, CoverType::BandLogo),
    (PictureType::PublisherLogo, CoverType::PublisherLogo),
];

pub struct FLACTag {
    tag: Tag,
    separator: Option<String>
}

impl FLACTag {
    // Load from file
    pub fn load_file(path: impl AsRef<Path>) -> Result<FLACTag, Box<dyn Error>> {
        // Load header
        let mut file = BufReader::new(File::open(path)?);
        let mut header: [u8; 4] = [0; 4];
        file.read_exact(&mut header)?;
        // Check if not ID3
        if &header[0..3] == b"ID3" {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "FLAC ID3 not supported!").into());
        }
        // Check if FLAC
        if &header != b"fLaC" {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Not a valid FLAC!").into());
        }
        file.seek(SeekFrom::Start(0))?;

        Ok(FLACTag {
            tag: Tag::read_from(&mut file)?,
            separator: None
        }.into())
    }

    // Set date in vorbis to tag
    fn set_date_inner(&mut self, tag: &str, date: &TagDate, overwrite: bool) {
        if overwrite || self.tag.get_vorbis(tag).is_none() {
            let v = match date.has_md() {
                true => format!("{}-{:02}-{:02}", date.year, date.month.unwrap(), date.day.unwrap()),
                false => date.year.to_string()
            };
            self.tag.set_vorbis(tag, vec![v]);
        }
    }

    // Convert between different cover/picture types
    fn picture_type(&self, cover_type: &CoverType) -> PictureType {
        COVER_TYPES.iter().find(|(_, c)| c == cover_type).unwrap().0
    }
    fn cover_type(&self, picture_type: &PictureType) -> CoverType {
        COVER_TYPES.iter().find(|(p, _)| p == picture_type).unwrap().1.clone()
    }
}

impl TagImpl for FLACTag {
    // Save to path
    fn save_file(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.tag.write_to_path(path)?;
        Ok(())
    }

    // If separator is set, all values are written to single tag, separated by commas for compatibility reasons
    fn set_separator(&mut self, separator: &str) {
        if separator.is_empty() {
            self.separator = None;
        } else {
            self.separator = Some(separator.replace("\\0", "\0"))
        }
    }

    // Get all tags
    fn all_tags(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        if let Some(vorbis) = self.tag.vorbis_comments() {
            // Get value of tag with proper separators
            vorbis.comments.iter().for_each(|(k, _)| { 
                if let Some(v) = self.get_raw(k) {
                    out.insert(k.to_string(), v);
                }
            });
        }
        out
    }

    // Set date in tag
    fn set_date(&mut self, date: &TagDate, overwrite: bool) {
        self.set_date_inner("DATE", date, overwrite);
    }
    fn set_publish_date(&mut self, date: &TagDate, overwrite: bool) {
        self.set_date_inner("ORIGINALDATE", date, overwrite);
    }

    // Rating, in vorbis saved as 20,40,60,80,100
    fn get_rating(&self) -> Option<u8> {
        let rating = self.get_raw("RATING")?.first()?.parse::<i32>().ok()? / 20;
        if rating <= 5 {
            if rating == 0 {
               return Some(1)
            }
            return Some(rating as u8)
        }
        None
    }
    fn set_rating(&mut self, rating: u8, overwrite: bool) {
        let value = (rating * 20).to_string();
        if rating > 0 {
            self.set_raw("RATING", vec![value], overwrite);
        } else {
            self.tag.remove_vorbis("RATING");
        }

        // Rating WMP
        if overwrite || self.get_raw("RATING WMP").is_none() {
            self.tag.remove_vorbis("RATING WMP");
            if rating > 0 {
                let value = match rating {
                    1 => 1,
                    5 => 255,
                    i => (i - 1) * 64
                };
                self.set_raw("RATING WMP", vec![value.to_string()], overwrite);
            }
        }
    }

    // Set/Get album art
    fn set_art(&mut self, kind: CoverType, mime: &str, _description: Option<&str>, data: Vec<u8>) {
        // https://en.wikipedia.org/wiki/Vorbis_comment
        // FLAC has a smaller limit of 24-bit in a METADATA_BLOCK_VORBIS_COMMENT, 
        // because it stores thumbnails and cover art in binary big-endian METADATA_BLOCK_PICTUREs 
        // outside of the FLAC tags.
        // 
        // Cap it at 16M exactly, instead of 2^24, because the entire BLOCK is 24bits, not just the image
        if data.len() >= 16_000_000 {
            error!("Cannot add FLAC art, because of the 24bit limit");
            return;
        }

        self.tag.remove_picture_type(self.picture_type(&kind));
        self.tag.add_picture(mime, self.picture_type(&kind), data);
    }

    fn get_art(&self) -> Vec<crate::Picture> {
        self.tag.pictures().map(
            |p| crate::Picture {
                kind: self.cover_type(&p.picture_type),
                description: p.description.to_string(),
                data: p.data.clone(),
                mime: p.mime_type.to_string()
            }
        ).collect()
    }
    // Check if has album art
    fn has_art(&self) -> bool {
        self.tag.pictures().next().is_some()
    }

    fn remove_art(&mut self, kind: CoverType) { 
        self.tag.remove_picture_type(self.picture_type(&kind));
    }
    
    // Set/Get named field
    fn set_field(&mut self, field: Field, value: Vec<String>, overwrite: bool) {
        self.set_raw(field.vorbis(), value, overwrite);
    }
    fn get_field(&self, field: Field) -> Option<Vec<String>> {
        self.get_raw(field.vorbis())
    }

    // Set raw tag
    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool) {
        if overwrite || self.tag.get_vorbis(&tag).is_none() || self.tag.get_vorbis(&tag).unwrap().next().is_none() {
            // Separator override
            if let Some(separator) = &self.separator {
                self.tag.set_vorbis(tag, vec![value.join(separator)]);
                return;
            }
            
            self.tag.set_vorbis(tag, value);
        }
    }
    // Get raw tag, None even if empty array
    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        if let Some(values) = self.tag.get_vorbis(tag) {
            let mut v: Vec<&str> = values.collect();
            if v.is_empty() {
                return None;
            }

            // Separator override
            if v.len() == 1 && self.separator.is_some() {
                v = v[0].split(self.separator.as_ref().unwrap()).collect();
            }

            return Some(v.into_iter().map(|v| v.to_string()).collect());
        }
        None
    }
    
    fn remove_raw(&mut self, tag: &str) { 
        self.tag.remove_vorbis(tag);
    }

    fn get_date(&self) -> Option<TagDate> {
        let data = &self.get_raw("DATE")?[0];
        // YYYY-MM-DD
        if data.len() >= 10 {
            return Some(TagDate {
                year: data[0..4].parse().ok()?,
                month: data[5..7].parse().ok(),
                day: data[8..10].parse().ok()
            });
        }
        // YYYY
        if data.len() >= 4 {
            return Some(TagDate {
                year: data.parse().ok()?,
                month: None,
                day: None
            });
        }
        None
    }

    fn set_track_number(&mut self, track_number: &str, track_total: Option<u16>, overwrite: bool) {
        self.set_field(Field::TrackNumber, vec![track_number.to_string()], overwrite);
        if let Some(total) = track_total {
            self.set_field(Field::TrackTotal, vec![total.to_string()], overwrite);
        }
    }

    fn set_lyrics(&mut self, lyrics: &Lyrics, synced: bool, overwrite: bool) {
        if synced {
            warn!("FLAC doesn't support synchronized lyrics!");
            return;
        }
        if !overwrite || self.get_raw("LYRICS").is_some() {
            return;
        }
        self.tag.remove_vorbis("LYRICS");
        self.tag.set_vorbis("LYRICS", vec![lyrics.text()]);
    }

    fn set_explicit(&mut self, explicit: bool) {
        self.tag.remove_vorbis_pair("COMMENT", "Explicit");
        if explicit {
            let mut values = self.get_raw("COMMENT").unwrap_or(vec![]);
            values.push("Explicit".to_string());
            self.set_raw("COMMENT", values, true);
        }
    }

    fn get_separator(&self) -> Option<String> {
        self.separator.clone()
    }

    

}