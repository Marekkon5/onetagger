use std::error::Error;
use std::collections::HashMap;
use std::convert::TryInto;
use mp4ameta::{Tag, Data, Img, ImgFmt, AdvisoryRating};
use mp4ameta::ident::DataIdent;
use chrono::{DateTime, NaiveDate, Utc, Datelike, NaiveTime};

use crate::{TagImpl, TagDate, CoverType, Picture, Field, Lyrics};

const MAGIC: u8 = 0xa9;

pub struct MP4Tag {
    tag: Tag,
    date_year_only: bool,
    separator: String
}

impl MP4Tag {
    pub fn load_file(path: &str) -> Result<MP4Tag, Box<dyn Error>> {
        let tag = Tag::read_from_path(&path)?;
        Ok(MP4Tag {
            tag,
            date_year_only: false,
            separator: ", ".to_string()
        })
    }

    // Convert DataIdent to string value
    pub fn ident_to_string(ident: &DataIdent) -> String {
        match ident {
            DataIdent::Fourcc(d) => format!("{}", d),
            DataIdent::Freeform { mean, name } => format!("{}:{}", mean, name)
        }
    }

    pub fn string_to_ident(ident: &str) -> DataIdent {
        let mut bytes = ident.as_bytes().to_owned();
        // Replace UTF-8 © with the proper character
        if bytes.len() == 5 && bytes[0..2] == [194, 169] {
            bytes = vec![MAGIC, bytes[2], bytes[3], bytes[4]];
        }
        // Fourcc
        if bytes.len() == 4 {
            return DataIdent::fourcc(bytes.try_into().unwrap());
        }
        // Convert string freeform
        let mut ident = ident.replacen("----:", "", 1);
        // iTunes:VALUE abstraction
        if ident.starts_with("iTunes:") {
            ident = format!("com.apple.{}", ident);
        }
        let mut mean = "com.apple.iTunes";
        let mut name = ident.to_string();
        let split: Vec<&str> = ident.split(":").collect();
        if split.len() > 1 {
            mean = split[0];
            name = split[1].to_owned();
        }
        DataIdent::freeform(mean, name)
    }

    // Convert field to MP4 tag
    fn field_to_ident(field: Field) -> DataIdent {
        match field {
            Field::Title => DataIdent::fourcc(*b"\xa9nam"),
            Field::Artist => DataIdent::fourcc(*b"\xa9ART"),
            Field::AlbumArtist => DataIdent::fourcc(*b"aART"),
            Field::Album => DataIdent::fourcc(*b"\xa9alb"),
            Field::BPM => DataIdent::fourcc(*b"tmpo"),
            Field::Genre => DataIdent::fourcc(*b"\xa9gen"),
            Field::Label => DataIdent::freeform("com.apple.iTunes", "LABEL"),
            Field::ISRC => DataIdent::freeform("com.apple.iTunes", "ISRC"),
            Field::CatalogNumber => DataIdent::freeform("com.apple.iTunes", "CATALOGNUMBER"),
            Field::Version => DataIdent::fourcc(*b"desc"),
            Field::TrackNumber => DataIdent::fourcc(*b"trkn"),
            Field::Remixer => DataIdent::freeform("com.apple.iTunes", "REMIXER"),
            // Custom/Unofficial
            Field::Key => DataIdent::freeform("com.apple.iTunes", "initialkey"),
            Field::Style => DataIdent::freeform("com.apple.iTunes", "STYLE"),
            Field::Duration => DataIdent::freeform("com.apple.iTunes", "LENGTH"),
            Field::Mood => DataIdent::freeform("com.apple.iTunes", "MOOD"),
            Field::TrackTotal => DataIdent::fourcc(*b"trkn"),
            Field::DiscNumber => DataIdent::fourcc(*b"disk"),
        }
    }

    // Get raw tag value by identifier
    fn raw_by_ident(&self, ident: &DataIdent) -> Option<Vec<String>> {
        let data: Vec<String> = self.tag.data_of(ident).filter_map(|data| {
            // Save only text values
            match data {
                Data::Utf8(d) => Some(d.to_owned()),
                Data::Utf16(d) => Some(d.to_owned()),
                _ => None
            }
        }).collect();
        if data.is_empty() {
            return None;
        }
        // Convert multi tag to single with separator
        Some(data.join(&self.separator).split(&self.separator).map(String::from).collect())
    }

    // Raw version of set_art
    fn add_art(&mut self, mime: &str, data: Vec<u8>) {
        if mime == "image/jpeg" || mime == "image/jpg" {
            self.tag.add_artwork(Img::jpeg(data));
        } else if mime == "image/png" {
            self.tag.add_artwork(Img::png(data));
        } else if mime == "image/bmp" {
            self.tag.add_artwork(Img::bmp(data));
        }
    }

    pub fn remove_all_artworks(&mut self) {
        self.tag.set_artworks(vec![]);
    }
}

impl TagImpl for MP4Tag {
    fn save_file(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        self.tag.write_to_path(path)?;
        Ok(())
    }

    fn set_separator(&mut self, separator: &str) {
        self.separator = separator.replace("\\0", "\0");
    }

    fn all_tags(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();

        for (ident, data) in self.tag.data() {
            let mut values = vec![];
            // Save only text values
            match data {
                Data::Utf8(d) => values = d.split(&self.separator).map(String::from).collect(),
                Data::Utf16(d) => values = d.split(&self.separator).map(String::from).collect(),
                Data::BeSigned(i) => {
                    let mut i = i.to_owned();
                    while i.len() < 8 {
                        i.insert(0, 0);
                    }
                    values = vec![i64::from_be_bytes(i[..8].try_into().unwrap()).to_string()]
                }
                _ => {}
            }
            if !values.is_empty() {
                out.insert(MP4Tag::ident_to_string(ident), values);
            }
        }
        
        out
    }

    fn set_date(&mut self, date: &TagDate, overwrite: bool) {
        let ident = DataIdent::fourcc(*b"\xa9day");
        if self.raw_by_ident(&ident).is_none() || overwrite {
            // Write year or ISO timestamp
            let value = if self.date_year_only || !date.has_md() {
                date.year.to_string()
            } else {
                let naive_date =NaiveDate::from_ymd_opt(date.year, date.month.unwrap() as u32, date.day.unwrap() as u32).unwrap()
                    .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                let date: DateTime<Utc> = DateTime::from_utc(naive_date, Utc);
                format!("{}", date.format("%+"))
            };

            self.tag.set_data(ident, Data::Utf8(value));
        }
    }

    fn set_publish_date(&mut self, _date: &TagDate, _overwrite: bool) {
        // Unsupported (mp4 barely even supports dates)
        warn!("M4A Publish date isn't supported, skipping!");
    }

    // RATING NOT FINAL, used same as KID3
    fn get_rating(&self) -> Option<u8> {
        let val = self.get_raw("rate")?.first()?.parse::<u8>().ok()?;
        let rating = val / 20;
        if rating == 0 {
            Some(1)
        } else {
            Some(rating)
        }
    }

    fn set_rating(&mut self, rating: u8, overwrite: bool) {
        let val = rating * 20;
        if rating > 0 {
            self.set_raw("rate", vec![val.to_string()], overwrite);
        } else {
            self.remove_raw("rate");
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

    fn set_art(&mut self, _kind: CoverType, mime: &str, _description: Option<&str>, data: Vec<u8>) {
        self.add_art(mime, data);
    }

    fn has_art(&self) -> bool {
        self.tag.artwork().is_some()
    }

    fn get_art(&self) -> Vec<Picture> {
        let types = CoverType::types();
        let mut type_i = 0;
        self.tag.artworks().filter_map(|img| {
            // Use all cover types in order to not break removing covers
            let kind = types[type_i].clone();
            type_i += 1;
            let description = String::new();
            // Convert to mime from type
            let mime = match img.fmt {
                ImgFmt::Bmp => "image/bmp",
                ImgFmt::Jpeg => "image/jpeg",
                ImgFmt::Png => "image/png"
            }.to_string();
            let data = img.data.to_vec();
            Some(Picture {data, mime, description, kind})
        }).collect()
    }

    fn remove_art(&mut self, kind: CoverType) {
        // Because M4A doesn't really have cover types, 1t assigns them in sequence
        // to make removing possible, so it acts as index
        let arts = self.get_art();
        let artworks: Vec<&Picture> = arts.iter().filter(|p| p.kind != kind).collect();
        self.tag.remove_artworks();
        for art in artworks {
            self.add_art(&art.mime, art.data.clone());
        }
    }

    fn set_field(&mut self, field: Field, value: Vec<String>, overwrite: bool) {
        let ident = MP4Tag::field_to_ident(field.clone());
        if self.tag.data_of(&ident).next().is_none() || overwrite {
            self.tag.remove_data_of(&ident);

            // Overrides
            if field == Field::BPM {
                if let Some(bpm) = value.first().map(|v| v.parse().ok()).flatten() {
                    self.tag.set_bpm(bpm);
                }
                return;
            }

            // Add each data separately
            if self.separator.is_empty() {
                for v in value {
                    self.tag.add_data(ident.clone(), Data::Utf8(v));
                }
            } else {
                self.tag.add_data(ident.clone(), Data::Utf8(value.join(&self.separator)));
            }
        }
    }

    fn get_field(&self, field: Field) -> Option<Vec<String>> {
        // Overrides
        if field == Field::TrackNumber {
            return self.tag.track_number().map(|v| vec![v.to_string()]);
        }
        if field == Field::TrackTotal {
            return self.tag.total_tracks().map(|v| vec![v.to_string()]);
        }
        if field == Field::DiscNumber {
            return self.tag.disc_number().map(|v| vec![v.to_string()]);
        }
        if field == Field::BPM {
            return self.tag.bpm().map(|b| vec![b.to_string()]);
        }

        self.raw_by_ident(&MP4Tag::field_to_ident(field))
    }

    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool) {
        if self.get_raw(tag).is_none() || overwrite {
            let ident = MP4Tag::string_to_ident(tag);
            self.tag.remove_data_of(&ident);
            self.tag.add_data(ident.clone(), Data::Utf8(value.join(&self.separator)));
        }
    }

    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        let ident = MP4Tag::string_to_ident(tag);
        self.raw_by_ident(&ident)
    }

    fn remove_raw(&mut self, tag: &str) {
        self.tag.remove_data_of(&MP4Tag::string_to_ident(tag));
    }

    fn get_date(&self) -> Option<TagDate> {
        let ident = DataIdent::fourcc(*b"\xa9day");
        let data = &self.raw_by_ident(&ident)?[0];
        // Only year
        if let Ok(year) = data.parse() {
            return Some(TagDate {
                year: year,
                month: None,
                day: None
            });
        }
        // Parse ISO timestamp 
        let date = DateTime::parse_from_str(&data, "%+").ok()?;
        Some(TagDate {
            year: date.year(),
            month: Some(date.month() as u8),
            day: Some(date.day() as u8)
        })
    }

    fn set_track_number(&mut self, track_number: &str, track_total: Option<u16>, overwrite: bool) {
        let track_number = match track_number.parse() {
            Ok(tn) => tn,
            Err(e) => {
                error!("Failed parsing track number, it won't be written: {e}");
                return;
            }
        };

        if overwrite || self.tag.track().0.is_none() {
            if let Some(total) = track_total {
                self.tag.set_track(track_number, total);
            } else {
                self.tag.set_track_number(track_number);
            }
        }
    }

    fn set_lyrics(&mut self, lyrics: &Lyrics, synced: bool, overwrite: bool) {
        if synced {
            warn!("MP4 doesn't support synchronized lyrics!");
            return;
        }
        if !overwrite || self.tag.lyrics().is_some() {
            return;
        }
        self.tag.remove_lyrics();
        self.tag.set_lyrics(lyrics.text());
    }

    fn set_explicit(&mut self, explicit: bool) {
        self.tag.set_advisory_rating(match explicit {
            true => AdvisoryRating::Explicit,
            false => AdvisoryRating::Clean,
        });
    }

    fn get_separator(&self) -> Option<String> {
        Some(self.separator.clone())
    }
    
}