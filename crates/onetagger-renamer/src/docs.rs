use serde::Serialize;

use crate::ac::{SymbolDoc, DocParameter};

lazy_static! {
    pub static ref VARIABLES: [SymbolDoc; 30] = [
        SymbolDoc::var("title", "Get the Title frame from tag.\n\n  Used tags:<br> **MP3**: `TIT2`<br> **FLAC**: `TITLE`<br> **MP4**: `©nam`"),
        SymbolDoc::var("artist", "Get the first Artist from tag.\n\n  Used tags:<br> **MP3**: `TPE1`<br> **FLAC**: `ARTIST`<br> **MP4**: `©ART`"),
        SymbolDoc::var("artists", "Get the Artists frame from tag.\n\n  Used tags:<br> **MP3**: `TPE1`<br> **FLAC**: `ARTIST`<br> **MP4**: `©ART`"),
        SymbolDoc::var("album", "Get the Album frame from tag.\n\n  Used tags:<br> **MP3**: `TALB`<br> **FLAC**: `ALBUM`<br> **MP4**: `©alb`"),
        SymbolDoc::var("albumartist", "Get the first Album Artist from tag.\n\n  Used tags:<br> **MP3**: `TPE2`<br> **FLAC**: `ALBUMARTIST`<br> **MP4**: `aART`"),
        SymbolDoc::var("albumartists", "Get the Album Artists frame from tag.\n\n  Used tags:<br> **MP3**: `TPE2`<br> **FLAC**: `ALBUMARTIST`<br> **MP4**: `aART`"),
        SymbolDoc::var("key", "Get the Key frame from tag.\n\n  Used tags:<br> **MP3**: `TKEY`<br> **FLAC**: `INITIALKEY`<br> **MP4**: `com.apple.iTunes:initialkey`"),
        SymbolDoc::var("bpm", "Get the BPM frame from tag.\n\n  Used tags:<br> **MP3**: `TBPM`<br> **FLAC**: `BPM`<br> **MP4**: `tmpo`"),
        SymbolDoc::var("genre", "Get the Genre frame from tag.\n\n  Used tags:<br> **MP3**: `TCON`<br> **FLAC**: `GENRE`<br> **MP4**: `©gen`"),
        SymbolDoc::var("label", "Get the Label frame from tag.\n\n  Used tags:<br> **MP3**: `TPUB`<br> **FLAC**: `LABEL`<br> **MP4**: `com.apple.iTunes:LABEL`"),
        SymbolDoc::var("style", "Get the Style frame from tag.\n\n  Used tags:<br> **MP3**: `STYLE`<br> **FLAC**: `STYLE`<br> **MP4**: `com.apple.iTunes:STYLE`"),
        SymbolDoc::var("isrc", "Get the ISRC frame from tag.\n\n  Used tags:<br> **MP3**: `TSRC`<br> **FLAC**: `ISRC`<br> **MP4**: `com.apple.iTunes:ISRC`"),
        SymbolDoc::var("catalognumber", "Get the Catalog Number frame from tag.\n\n  Used tags:<br> **MP3**: `CATALOGNUMBER`<br> **FLAC**: `CATALOGNUMBER`<br> **MP4**: `com.apple.iTunes:CATALOGNUMBER`"),
        SymbolDoc::var("version", "Get the Version frame from tag.\n\n  Used tags:<br> **MP3**: `TIT3`<br> **FLAC**: `SUBTITLE`<br> **MP4**: `com.apple.iTunes:SUBTITLE`"),
        SymbolDoc::var("tracknumber", "Get the Track Number frame from tag.\n\n  Used tags:<br> **MP3**: `TRCK`<br> **FLAC**: `TRACKNUMBER`<br> **MP4**: `trkn`"),
        SymbolDoc::var("track", "Get the Track Number frame from tag.\n\n  Used tags:<br> **MP3**: `TRCK`<br> **FLAC**: `TRACKNUMBER`<br> **MP4**: `trkn`"),
        SymbolDoc::var("duration", "Get the Duration frame from tag.\n\n  Used tags:<br> **MP3**: `TLEN`<br> **FLAC**: `LENGTH`<br> **MP4**: `com.apple.iTunes:LENGTH`"),
        SymbolDoc::var("remixer", "Get the Remixer frame from tag.\n\n  Used tags:<br> **MP3**: `TPE4`<br> **FLAC**: `REMIXER`<br> **MP4**: `com.apple.iTunes:REMIXER`"),
        SymbolDoc::var("year", "Get the release year from tag.\n\n  Used tags:<br> **MP3**: `TYER` or `TDRC`<br> **FLAC**: `DATE`<br> **MP4**: `©day`"),
        SymbolDoc::var("month", "Get the release month from tag.\n\n  Used tags:<br> **MP3**: `TYER` or `TDRC`<br> **FLAC**: `DATE`<br> **MP4**: `©day`"),
        SymbolDoc::var("day", "Get the release day from tag.\n\n  Used tags:<br> **MP3**: `TYER` or `TDRC`<br> **FLAC**: `DATE`<br> **MP4**: `©day`"),
        SymbolDoc::var("filename", "Original name of the file.\n\n **NOTE:** Without extension, extension is always automatically added at end."),
        SymbolDoc::var("tracktotal", "Get the Track Total frame from tag.\n\n  Used tags:<br> **MP3**: `TRCK`<br> **FLAC**: `TRACKNUMBER`<br> **MP4**: `trkn`"),
        SymbolDoc::var("total", "Get the Track Total frame from tag.\n\n  Used tags:<br> **MP3**: `TRCK`<br> **FLAC**: `TRACKNUMBER`<br> **MP4**: `trkn`"),
        SymbolDoc::var("disk", "Get the Disc Number frame from tag.\n\n  Used tags:<br> **MP3**: `TPOS`<br> **FLAC**: `DISCNUMBER`<br> **MP4**: `disk`"),
        SymbolDoc::var("disknumber", "Get the Disc Number frame from tag.\n\n  Used tags:<br> **MP3**: `TPOS`<br> **FLAC**: `DISCNUMBER`<br> **MP4**: `disk`"),
        SymbolDoc::var("disc", "Get the Disc Number frame from tag.\n\n  Used tags:<br> **MP3**: `TPOS`<br> **FLAC**: `DISCNUMBER`<br> **MP4**: `disk`"),
        SymbolDoc::var("discnumber", "Get the Disc Number frame from tag.\n\n  Used tags:<br> **MP3**: `TPOS`<br> **FLAC**: `DISCNUMBER`<br> **MP4**: `disk`"),
        SymbolDoc::var("path", "Path to the file"),
        SymbolDoc::var("abspath", "Absolute path to the file"),

    ];

    pub static ref PROPERTIES: [SymbolDoc; 2] = [
        SymbolDoc::prop("first", "Get the first item in an array"),
        SymbolDoc::prop("last", "Get the last item in an array"),
    ];

    pub static ref FUNCTIONS: [SymbolDoc; 14] = [
        SymbolDoc::f("lower", "Convert all to lowercase", vec![]),
        SymbolDoc::f("lowercase", "Convert all to lowercase", vec![]),
        SymbolDoc::f("upper", "Convert all to uppercase", vec![]),
        SymbolDoc::f("uppercase", "Convert all to uppercase", vec![]),
        SymbolDoc::f("slice", "Take a range out of array or substring", vec![DocParameter::n("start", true), DocParameter::n("end", false)]),
        SymbolDoc::f("range", "Take a range out of array or substring", vec![DocParameter::n("start", true), DocParameter::n("end", false)]),
        SymbolDoc::f("capitalize", "Convert first letter to uppercase", vec![]),
        SymbolDoc::f("replace", "Replace text. `from` parameter accepts regex expressions.", vec![DocParameter::s("from", true), DocParameter::s("to", true)]),
        SymbolDoc::f("pad", "Pad on the left side with given character to reach given length", vec![DocParameter::s("char", true), DocParameter::n("length", true)]),
        SymbolDoc::f("sort", "Sort the array alphabetically", vec![]),
        SymbolDoc::f("reverse", "Reverse the array", vec![]),
        SymbolDoc::f("join", "Join array into string with custom separator", vec![DocParameter::s("separator", true)]),
        SymbolDoc::f("parent", "Get parent folder of path", vec![]),
        SymbolDoc::f("filename", "Get file/folder name of path", vec![]),

    ];
}

/// Holds all of the docs for sending into UI
#[derive(Debug, Serialize)]
pub struct FullDocs {
    pub variables: Vec<SymbolDoc>,
    pub properties: Vec<SymbolDoc>,
    pub functions: Vec<SymbolDoc>
}

impl FullDocs {
    /// Get all of the docs
    pub fn get() -> FullDocs {
        FullDocs { variables: VARIABLES.to_vec(), properties: PROPERTIES.to_vec(), functions: FUNCTIONS.to_vec() }
    }

    /// Convert to html
    pub fn html(mut self) -> Self {
        self.variables.iter_mut().for_each(|i| i.html());
        self.properties.iter_mut().for_each(|i| i.html());
        self.functions.iter_mut().for_each(|i| i.html());
        self
    }
}