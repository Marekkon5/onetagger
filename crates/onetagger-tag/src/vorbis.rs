use std::collections::HashMap;
use std::error::Error;
use lofty::{TagType, TaggedFileExt, AudioFile, PictureType, MimeType};
use lofty::ogg::{VorbisComments, OggPictureStorage};

use crate::{Lyrics, Picture};
use crate::{Field, TagDate, CoverType, TagImpl};

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

pub struct VorbisTag {
    tag: VorbisComments,
    separator: Option<String>
}

impl VorbisTag {
    /// Load from path
    pub fn load_file(path: &str) -> Result<VorbisTag, Box<dyn Error>> {
        let file = lofty::read_from_path(path)?;
        let tag = file.tag(TagType::VorbisComments).ok_or("Missing vorbis tag")?;
        let vorbis: VorbisComments = tag.to_owned().into();
        Ok(VorbisTag {
            tag: vorbis,
            separator: None
        })
    }

    /// Set date in vorbis to tag
    fn set_date_inner(&mut self, tag: &str, date: &TagDate, overwrite: bool) {
        if overwrite || self.get_raw(tag).is_none() {
            let v = match date.has_md() {
                true => format!("{}-{:02}-{:02}", date.year, date.month.unwrap(), date.day.unwrap()),
                false => date.year.to_string()
            };
            self.tag.insert(tag.to_string(), v);
        }
    }

    /// Convert CoverType -> PictureType
    fn picture_type(&self, cover_type: &CoverType) -> PictureType {
        COVER_TYPES.iter().find(|(_, c)| c == cover_type).unwrap().0
    }
    /// Convert PictureType -> CoverType
    fn cover_type(&self, picture_type: &PictureType) -> CoverType {
        COVER_TYPES.iter().find(|(p, _)| p == picture_type).unwrap().1.clone()
    }
}


impl TagImpl for VorbisTag {
    fn save_file(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut file = lofty::read_from_path(path)?;
        file.remove(TagType::VorbisComments);
        file.insert_tag(self.tag.clone().into());
        file.save_to_path(path)?;
        Ok(())
    }

    fn set_separator(&mut self, separator: &str) {
        if separator.is_empty() {
            self.separator = None;
        } else {
            self.separator = Some(separator.replace("\\0", "\0"))
        }
    }

    fn all_tags(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        for (key, _) in self.tag.items() {
            if out.contains_key(key) {
                continue;
            }
            if let Some(values) = self.get_raw(key) {
                out.insert(key.to_string(), values);
            }
        }
        out
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

    fn set_date(&mut self, date: &TagDate, overwrite: bool) {
        self.set_date_inner("DATE", date, overwrite);
    }

    fn set_publish_date(&mut self, date: &TagDate, overwrite: bool) {
        self.set_date_inner("ORIGINALDATE", date, overwrite);
    }

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
            self.remove_raw("RATING");
        }

        // Rating WMP
        if overwrite || self.get_raw("RATING WMP").is_none() {
            self.remove_raw("RATING WMP");
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

    fn set_art(&mut self, kind: CoverType, mime: &str, description: Option<&str>, data: Vec<u8>) {
        self.tag.remove_picture_type(self.picture_type(&kind));
        match self.tag.insert_picture(
            lofty::Picture::new_unchecked(
                self.picture_type(&kind),
                MimeType::from_str(&mime.trim().to_lowercase()),
                description.map(String::from),
                data
            ),
            None
        ) {
            Ok(_) => {},
            Err(e) => error!("Error adding picture to Vorbis tag: {e}"),
        }
    }

    fn has_art(&self) -> bool {
        self.tag.pictures().is_empty()
    }

    fn get_art(&self) -> Vec<Picture> {
        self.tag.pictures().iter().map(
            |(p, _)| crate::Picture {
                kind: self.cover_type(&p.pic_type()),
                description: p.description().map(String::from).unwrap_or(String::new()),
                data: p.data().to_vec(),
                mime: p.mime_type().to_string()
            }
        ).collect()
    }

    fn remove_art(&mut self, kind: CoverType) {
        self.tag.remove_picture_type(self.picture_type(&kind));
    }

    fn set_field(&mut self, field: Field, value: Vec<String>, overwrite: bool) {
        self.set_raw(field.vorbis(), value, overwrite);
    }

    fn get_field(&self, field: Field) -> Option<Vec<String>> {
        self.get_raw(field.vorbis())
    }

    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool) {
        if overwrite || self.tag.get(&tag).is_none() || self.tag.get_all(tag).next().is_none() {
            self.tag.remove(tag).for_each(|_| {});
            
            // Separator override
            if let Some(separator) = &self.separator {
                self.tag.push(tag.to_string(), value.join(separator));
                return;
            }
            
            for value in value {
                self.tag.push(tag.to_string(), value);
            }
        }
    }

    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        let mut values = self.tag.get_all(tag).collect::<Vec<_>>();
        if values.is_empty() {
            return None;
        }

        // Separator override
        if values.len() == 1 && self.separator.is_some() {
            values = values[0].split(self.separator.as_ref().unwrap()).collect();
        }

        Some(values.into_iter().map(|v| v.to_string()).collect())
    }

    fn remove_raw(&mut self, tag: &str) {
        self.tag.remove(tag).for_each(|i| drop(i));
    }

    fn set_lyrics(&mut self, lyrics: &Lyrics, synced: bool, overwrite: bool) {
        if synced {
            warn!("Vorbis doesn't support synchronized lyrics!");
            return;
        }
        if !overwrite || self.get_raw("LYRICS").is_some() {
            return;
        }
        self.tag.insert("LYRICS".to_string(), lyrics.text());
    }

    fn set_track_number(&mut self, track_number: &str, track_total: Option<u16>, overwrite: bool) {
        self.set_field(Field::TrackNumber, vec![track_number.to_string()], overwrite);
        if let Some(total) = track_total {
            self.set_field(Field::TrackTotal, vec![total.to_string()], overwrite);
        }
    }

    fn set_explicit(&mut self, explicit: bool) {
        let mut comments: Vec<_> = self.get_raw("COMMENT").unwrap_or(vec![]).into_iter().filter(|i| i != "Explicit").collect();
        if explicit {
            comments.push("Explicit".to_string());
        }
        self.set_raw("COMMENT", comments, true);
    }

    fn get_separator(&self) -> Option<String> {
        self.separator.clone()
    }

    
}