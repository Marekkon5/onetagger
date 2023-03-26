use std::{error::Error, ffi::c_void};
use std::path::PathBuf;
use std::io::Cursor;
use base64::Engine;
use libloading::{Library, Symbol};
use onetagger_platforms::{beatport, junodownload, spotify, traxsource, discogs, itunes, musicbrainz, beatsource, bpmsupreme, bandcamp, deezer};
use image::{io::Reader as ImageReader, ImageOutputFormat};
use onetagger_shared::Settings;
use onetagger_tagger::custom::MatchTrackResult;
use onetagger_tagger::{AutotaggerSourceBuilder, PlatformInfo, AutotaggerSource, TaggerConfig, AudioFileInfo, Track, SupportedTag};

lazy_static::lazy_static! {
    /// Globally loaded all platforms
    pub static ref AUTOTAGGER_PLATFORMS: AutotaggerPlatforms = AutotaggerPlatforms::all();
}


/// For passing platform list into UI
#[derive(Serialize, Deserialize)]
pub struct AutotaggerPlatforms(pub Vec<AutotaggerPlatform>);

impl AutotaggerPlatforms {
    /// Get all the available platforms
    pub fn all() -> AutotaggerPlatforms {
        let mut output = vec![];

        // Built-ins
        AutotaggerPlatforms::add_builtin::<beatport::BeatportBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<traxsource::TraxsourceBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<beatsource::BeatsourceBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<junodownload::JunoDownloadBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<spotify::SpotifyBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<musicbrainz::MusicBrainzBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<discogs::DiscogsBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<itunes::ITunesBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<bpmsupreme::BPMSupremeBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<bandcamp::BandcampBuilder>(&mut output);
        AutotaggerPlatforms::add_builtin::<deezer::DeezerBuilder>(&mut output);

        // Custom
        let mut platforms = AutotaggerPlatforms(output);
        match platforms.load_custom() {
            Ok(_) => {},
            Err(e) => warn!("Failed loading custom platforms: {e}")
        };
        platforms
    }

    /// Get the source
    pub fn get_builder(&self, id: &str) -> Option<Box<dyn AutotaggerSourceBuilder>> {
        let platform = self.0.iter().find(|p| p.id == id)?;
        if platform.built_in {
            let platform: Box<dyn AutotaggerSourceBuilder> = match platform.id.as_ref() {
                "beatport" => Box::new(beatport::BeatportBuilder::new()),
                "discogs" => Box::new(discogs::DiscogsBuilder::new()),
                "beatsource" => Box::new(beatsource::BeatsourceBuilder::new()),
                "itunes" => Box::new(itunes::ITunesBuilder::new()),
                "junodownload" => Box::new(junodownload::JunoDownloadBuilder::new()),
                "musicbrainz" => Box::new(musicbrainz::MusicBrainzBuilder::new()),
                "spotify" => Box::new(spotify::SpotifyBuilder::new()),
                "traxsource" => Box::new(traxsource::TraxsourceBuilder::new()),
                "bpmsupreme" => Box::new(bpmsupreme::BPMSupremeBuilder::new()),
                "bandcamp" => Box::new(bandcamp::BandcampBuilder::new()),
                "deezer" => Box::new(deezer::DeezerBuilder::new()),
                _ => unreachable!()
            };
            Some(platform)
        } else {
            // Custom platform
            Some(Box::new(platform.custom.as_ref().unwrap().get_builder().unwrap()))
        }
    }

    /// Add a builtin platform to output list
    fn add_builtin<P: AutotaggerSourceBuilder>(output: &mut Vec<AutotaggerPlatform>) {
        let info = P::new().info();
        output.push(AutotaggerPlatform {
            id: info.id.clone(),
            built_in: true,
            icon: match Self::reencode_image(info.icon) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Failed generating icon for platform id: {}. {e}", info.id);
                    String::new()
                }
            },
            supported_tags: info.supported_tags.clone(),
            requires_auth: info.requires_auth,
            platform: info,
            custom: None
        })
    }

    /// Prepare image for the UI
    fn reencode_image(data: &'static [u8]) -> Result<String, Box<dyn Error>> {
        let img = ImageReader::new(Cursor::new(data)).with_guessed_format()?.decode()?;
        let mut buf = vec![];
        img.write_to(&mut Cursor::new(&mut buf), ImageOutputFormat::Png)?;
        Ok(format!("data:image/png;charset=utf-8;base64,{}", base64::engine::general_purpose::STANDARD.encode(buf)))
    }

    /// Load custom platforms
    fn load_custom(&mut self) -> Result<(), Box<dyn Error>> {
        // Path
        let folder = Settings::get_folder()?.join("platforms");
        if !folder.exists() {
            std::fs::create_dir(folder)?;
            return Ok(())
        }
        for entry in std::fs::read_dir(folder)? {
            let entry = entry?;
            match CustomPlatform::open_platform(&entry.path()) {
                Ok(p) => {
                    info!("Loaded custom platform: {}@{}", p.id, p.platform.version);
                    self.0.push(p);
                },
                Err(e) => {
                    error!("Failed loading custom platform from {:?}: {e}", entry.path());
                    continue;
                }
            }
        }

        Ok(())
    }
}

/// For passing platform list into UI
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutotaggerPlatform {
    pub id: String,
    pub built_in: bool,
    pub platform: PlatformInfo,
    /// Encoded for UI
    pub icon: String,
    pub requires_auth: bool,
    pub supported_tags: Vec<SupportedTag>,

    /// For custom platforms
    #[serde(skip)]
    custom: Option<CustomPlatform>
} 



/// Wrapper for loaded custom platform
pub struct CustomPlatform {
    library: Library,
    path: PathBuf,
    info: PlatformInfo,
    builder: PtrWrap
}

impl CustomPlatform {
    /// Load library
    pub fn open(path: &PathBuf) -> Result<CustomPlatform, Box<dyn Error>> {
        let p = unsafe {
            let lib = Library::new(&path)?;
            // Check version compatibility
            let version: Symbol<*const i32> = lib.get(b"_1T_PLATFORM_COMPATIBILITY")?;
            if **version != onetagger_tagger::custom::CUSTOM_PLATFORM_COMPATIBILITY {
                warn!("Plugin is incompatible! Plugin version: {}, Supported version: {}", **version, onetagger_tagger::custom::CUSTOM_PLATFORM_COMPATIBILITY);
                return Err("Plugin is incompatible!".into());
            }
            // Setup logging
            let logging_cb_fn: Symbol<unsafe extern fn(extern fn (*mut onetagger_tagger::custom::FFIRecord))> = lib.get(b"_1t_register_logger")?;
            logging_cb_fn(onetagger_tagger::custom::write_log);
            // Get builder
            let builder_fn: Symbol<unsafe extern fn() -> *mut c_void> = lib.get(b"_1t_create_builder")?;
            let builder = builder_fn();
            // Get info
            let info_fn: Symbol<unsafe extern fn(*mut std::ffi::c_void) -> *mut PlatformInfo> = lib.get(b"_1t_builder_info")?; 
            //TODO: unsure if doesn't leak memory or cause segfaults later on
            let info = (*Box::from_raw(info_fn(builder))).clone();
            CustomPlatform { 
                library: lib,
                path: path.clone(),
                info,
                builder: PtrWrap(builder)
            }
        };
        Ok(p)
    }

    /// Open and convert into Autotagger platfrom
    pub fn open_platform(path: &PathBuf) -> Result<AutotaggerPlatform, Box<dyn Error>> {
        let filename = path.file_name().ok_or("Invalid filename")?.to_str().ok_or("Invalid filename")?.to_string();
        let lib = Self::open(path)?;
        Ok(AutotaggerPlatform {
            id: filename,
            built_in: false,
            platform: lib.info.clone(),
            icon: match AutotaggerPlatforms::reencode_image(lib.info.icon) {
                Ok(i) => i,
                Err(e) => {
                    warn!("Failed loading custom platform icon: {e}");
                    String::new()
                },
            },
            supported_tags: lib.info.supported_tags.clone(),
            requires_auth: lib.info.requires_auth,
            custom: Some(lib),
        })
    }

    //TODO: BETTER WAY THAN JUST REOPENING THE DLL
    pub fn get_builder(&self) -> Result<CustomPlatform, Box<dyn Error>> {
        Ok(Self::open(&self.path)?)
    }

    /// Get source
    fn get_source_raw(&self, config: &TaggerConfig) -> Result<CustomPlatformSource, Box<dyn Error>> {
        let ptr = unsafe {
            let get_source_fn: Symbol<unsafe extern fn(*mut c_void, &TaggerConfig) -> *mut c_void> = self.library.get(b"_1t_builder_get_source")?;
            let source_ptr = get_source_fn(self.builder.0, config);
            if source_ptr.is_null() {
                return Err("Failed creating custom platform source!".into());
            }
            source_ptr
        };
        // Lifetime fix
        let match_fn = unsafe {
            let f: Symbol<unsafe extern fn(*mut c_void, &AudioFileInfo, &TaggerConfig) -> *mut MatchTrackResult> = 
                self.library.get(b"_1t_match_track")?;
            std::mem::transmute(f)
        };
        Ok(CustomPlatformSource {
            ptr: PtrWrap(ptr),
            match_fn
        })
    }
}

impl Drop for CustomPlatform {
    fn drop(&mut self) {
        // Free the builder in the plugin itself
        unsafe {
            let drop_fn: Symbol<unsafe extern fn(*mut std::ffi::c_void)> = self.library.get(b"_1t_free_builder").unwrap();
            drop_fn(self.builder.0);
        }
    }
}

impl AutotaggerSourceBuilder for CustomPlatform {
    fn new() -> Self where Self: Sized {
        panic!("Not used / use CustomPlatform::load()");
    }

    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        Ok(Box::new(self.get_source_raw(config)?))
    }

    fn info(&self) -> PlatformInfo {
        self.info.clone()
    }
}

struct CustomPlatformSource {
    ptr: PtrWrap,
    match_fn: Symbol<'static, unsafe extern fn(*mut c_void, &AudioFileInfo, &TaggerConfig) -> *mut MatchTrackResult>
}

impl AutotaggerSource for CustomPlatformSource {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        unsafe {
            let f = &self.match_fn;
            let r = Box::from_raw(f(self.ptr.0, info, config));
            match *r {
                MatchTrackResult::Ok(acc, track) => Ok(Some((acc, track))),
                MatchTrackResult::NoMatch => Ok(None),
                MatchTrackResult::Err(e) => Err(e.into()),
            }
        }
    }
}


/// Pointer wrapper
struct PtrWrap(pub *mut c_void);
unsafe impl Send for PtrWrap {}
unsafe impl Sync for PtrWrap {}
