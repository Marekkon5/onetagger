use std::error::Error;
use std::collections::HashMap;
use std::convert::TryInto;
use mp4ameta::{Tag, Data};
use mp4ameta::ident::DataIdent;
use chrono::NaiveDate;

use crate::tag::{TagImpl, TagDate, CoverType, Picture, Field};

const MAGIC: u8 = 0xa9;

pub struct MP4Tag {
    tag: Tag,
    date_year_only: bool
}

impl MP4Tag {
    pub fn load_file(path: &str) -> Result<MP4Tag, Box<dyn Error>> {
        let tag = Tag::read_from_path(&path)?;
        Ok(MP4Tag {
            tag,
            date_year_only: false
        })
    }

    //Convert DataIdent to string value
    pub fn ident_to_string(ident: &DataIdent) -> String {
        match ident {
            DataIdent::Fourcc(d) => format!("{}", d),
            DataIdent::Freeform { mean, name } => format!("{}:{}", mean, name)
        }
    }

    pub fn string_to_ident(ident: &str) -> DataIdent {
        let mut bytes = ident.as_bytes().to_owned();
        //Replace UTF-8 © with the proper character
        if bytes.len() == 5 && bytes[0..2] == [194, 169] {
            bytes = vec![MAGIC, bytes[2], bytes[3], bytes[4]];
        }
        //Fourcc
        if bytes.len() == 4 {
            return DataIdent::fourcc(bytes.try_into().unwrap());
        }
        //Convert string freeform
        let ident = ident.replacen("----:", "", 1);
        let mut mean = "com.apple.iTunes";
        let mut name = ident.to_string();
        let split: Vec<&str> = ident.split(":").collect();
        if split.len() > 1 {
            mean = split[0];
            name = split[1].to_owned();
        }
        DataIdent::freeform(mean, name)
    }

    //Convert field to MP4 tag
    fn field_to_ident(field: Field) -> DataIdent {
        match field {
            Field::Title => DataIdent::fourcc(*b"\xa9nam"),
            Field::Artist => DataIdent::fourcc(*b"\xa9ART"),
            Field::Album => DataIdent::fourcc(*b"\xa9alb"),
            Field::BPM => DataIdent::fourcc(*b"tmpo"),
            Field::Genre => DataIdent::fourcc(*b"\xa9gen"),
            Field::Label => DataIdent::freeform("com.apple.iTunes", "LABEL"),
            Field::ISRC => DataIdent::freeform("com.apple.iTunes", "ISRC"),
            //Custom/Unofficial
            Field::Key => DataIdent::freeform("com.apple.iTunes", "KEY"),
            Field::Style => DataIdent::freeform("com.apple.iTunes", "STYLE"),
        }
    }

    //Get raw tag value by identifier
    fn raw_by_ident(&self, ident: &DataIdent) -> Option<Vec<String>> {
        let data: Vec<String> = self.tag.data(ident).filter_map(|data| {
            //Save only text values
            match data {
                Data::Utf8(d) => Some(d.to_owned()),
                Data::Utf16(d) => Some(d.to_owned()),
                _ => None
            }
        }).collect();
        if data.is_empty() {
            return None;
        }
        Some(data)
    }
}

impl TagImpl for MP4Tag {
    fn save_file(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        self.tag.write_to_path(path)?;
        Ok(())
    }

    fn all_tags(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        for atom in self.tag.atoms.clone() {
            let mut values = vec![];
            for data in atom.data {
                //Save only text values
                match data {
                    Data::Utf8(d) => values.push(d),
                    Data::Utf16(d) => values.push(d),
                    _ => {}
                }
            }
            out.insert(MP4Tag::ident_to_string(&atom.ident), values);
        }
        out
    }

    fn set_date(&mut self, date: &TagDate, overwrite: bool) {
        let ident = DataIdent::fourcc(*b"\xa9day");
        if self.raw_by_ident(&ident).is_none() || overwrite {
            //Write year or ISO timestamp
            let value = if self.date_year_only || !date.has_md() {
                date.year.to_string()
            } else {
                let date = NaiveDate::from_ymd(date.year, date.month.unwrap() as u32, date.day.unwrap() as u32);
                format!("{}", date.format("%+"))
            };

            self.tag.set_data(ident, Data::Utf8(value));
        }
    }

    fn set_publish_date(&mut self, _date: &TagDate, _overwrite: bool) {
        //Unsupported (mp4 barely even supports dates)
    }

    //RATING NOT FINAL, used same as ID3
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
        self.set_raw("rate", vec![val.to_string()], overwrite);
    }

    fn set_art(&mut self, _kind: CoverType, mime: &str, _description: Option<&str>, data: Vec<u8>) {
        if mime == "image/jpeg" || mime == "image/jpg" {
            self.tag.add_artwork(Data::Jpeg(data));
        } else if mime == "image/png" {
            self.tag.add_artwork(Data::Png(data));
        } else if mime == "image/bmp" {
            self.tag.add_artwork(Data::Bmp(data));
        }
    }

    fn has_art(&self) -> bool {
        self.tag.artwork().is_some()
    }

    fn get_art(&self) -> Vec<Picture> {
        let types = CoverType::types();
        let mut type_i = 0;
        self.tag.artworks().filter_map(|data| {
            //Use all cover types in order to not break removing covers
            let kind = types[type_i].clone();
            type_i += 1;
            let description = String::new();
            //Convert to mime from type
            match data {
                Data::Jpeg(d) => Some(Picture {
                    data: d.to_owned(),
                    mime: "image/jpeg".to_string(),
                    description, kind
                }),
                Data::Png(d) => Some(Picture {
                    data: d.to_owned(),
                    mime: "image/png".to_string(),
                    description, kind
                }),
                Data::Bmp(d) => Some(Picture {
                    data: d.to_owned(),
                    mime: "image/bmp".to_string(),
                    description, kind
                }),
                _ => None
            }
        }).collect()
    }

    fn remove_art(&mut self, kind: CoverType) {
        //Because M4A doesn't really have cover types, 1t assigns them in sequence
        //to make removing possible, so it acts as index
        let arts = self.get_art();
        let artworks: Vec<&Picture> = arts.iter().filter(|p| p.kind != kind).collect();
        self.tag.remove_artwork();
        for art in artworks {
            self.set_art(art.kind.clone(), &art.mime, None, art.data.clone());
        }
    }

    fn set_field(&mut self, field: Field, value: Vec<String>, overwrite: bool) {
        let ident = MP4Tag::field_to_ident(field);
        if self.tag.data(&ident).next().is_none() || overwrite {
            self.tag.remove_data(&ident);
            //Add each data separately
            for v in value {
                self.tag.add_data(ident.clone(), Data::Utf8(v));
            }
        }
    }

    fn get_field(&self, field: Field) -> Option<Vec<String>> {
        self.raw_by_ident(&MP4Tag::field_to_ident(field))
    }

    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool) {
        if self.get_raw(tag).is_none() || overwrite {
            let ident = MP4Tag::string_to_ident(tag);
            self.tag.remove_data(&ident);
            //Add each data separately
            for v in value {
                self.tag.add_data(ident.clone(), Data::Utf8(v));
            }
        }
    }

    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        let ident = MP4Tag::string_to_ident(tag);
        self.raw_by_ident(&ident)
    }

    fn remove_raw(&mut self, tag: &str) {
        self.tag.remove_data(&MP4Tag::string_to_ident(tag));
    }
}