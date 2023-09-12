use anyhow::Error;
use serde_json::Value;
use std::ffi::c_void;
use std::path::PathBuf;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use base64::Engine;
use libloading::{Library, Symbol};
use onetagger_platforms::{beatport, junodownload, spotify, traxsource, discogs, itunes, musicbrainz, beatsource, bpmsupreme, bandcamp, deezer, musixmatch};
use image::io::Reader as ImageReader;
use image::ImageOutputFormat;
use onetagger_shared::Settings;
use onetagger_tagger::custom::MatchTrackResult;
use onetagger_tagger::{AutotaggerSourceBuilder, PlatformInfo, AutotaggerSource, TaggerConfig, AudioFileInfo, Track, SupportedTag, TrackMatch, ConfigCallbackResponse};

lazy_static::lazy_static! {
    /// Globally loaded all platforms
    pub static ref AUTOTAGGER_PLATFORMS: Arc<Mutex<AutotaggerPlatforms>> = Arc::new(Mutex::new(AutotaggerPlatforms::all()));
}


/// For passing platform list into UI
pub struct AutotaggerPlatforms {
    pub platforms: Vec<AutotaggerPlatform>,
}

impl AutotaggerPlatforms {
    /// Get all the available platforms
    fn all() -> AutotaggerPlatforms {
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
        AutotaggerPlatforms::add_builtin::<musixmatch::MusixmatchBuilder>(&mut output);

        // Custom
        let mut platforms = AutotaggerPlatforms { platforms: output };
        match platforms.load_custom() {
            Ok(_) => {},
            Err(e) => warn!("Failed loading custom platforms: {e}")
        };
        match platforms.load_python() {
            Ok(_) => {},
            Err(e) => warn!("Failed loading Python platforms: {e}"),
        }

        platforms
    }

    /// Get the source
    pub fn get_builder(&mut self, id: &str) -> Option<&mut Box<dyn AutotaggerSourceBuilder + Send + Sync>> {
        let platform = self.platforms.iter_mut()
            .find(|p| p.info.platform.id == id)?;
        Some(&mut platform.platform)
    }

    /// Add a builtin platform to output list
    fn add_builtin<P: AutotaggerSourceBuilder>(output: &mut Vec<AutotaggerPlatform>) {
        let info = P::new().info();
        output.push(AutotaggerPlatform {
            info: AutotaggerPlatformInfo {
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
            },
            platform: Box::new(P::new())
        })
    }

    /// Prepare image for the UI
    fn reencode_image(data: &[u8]) -> Result<String, Error> {
        let img = ImageReader::new(Cursor::new(data)).with_guessed_format()?.decode()?;
        let mut buf = vec![];
        img.write_to(&mut Cursor::new(&mut buf), ImageOutputFormat::Png)?;
        Ok(format!("data:image/png;charset=utf-8;base64,{}", base64::engine::general_purpose::STANDARD.encode(buf)))
    }

    /// Get custom platforms dir
    fn platforms_dir() -> Result<PathBuf, Error> {
        let folder = Settings::get_folder()?.join("platforms");
        if !folder.exists() {
            std::fs::create_dir(&folder)?;
        }
        Ok(folder)
    }

    /// Load custom platforms
    fn load_custom(&mut self) -> Result<(), Error> {
        let folder = Self::platforms_dir()?;
        for entry in std::fs::read_dir(folder)? {
            let entry = entry?;
            if entry.path().is_dir() {
                continue;
            }
            match CustomPlatform::open_platform(&entry.path()) {
                Ok(p) => {
                    info!("Loaded custom platform: {}@{}", p.info.platform.id, p.info.platform.version);
                    self.platforms.push(p);
                },
                Err(e) => {
                    error!("Failed loading custom platform from {:?}: {e}", entry.path());
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Load python custom platforms
    fn load_python(&mut self) -> Result<(), Error> {
        let folder = Self::platforms_dir()?;
        onetagger_python::setup()?;
        
        for entry in std::fs::read_dir(folder)?.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()) {
            let platform = onetagger_python::load_python_platform(entry.path())?;
            
            // Load icon
            let icon = match Self::reencode_image(&std::fs::read(&entry.path().join("icon.png")).unwrap_or(vec![])) {
                Ok(icon) => icon,
                Err(e) => {
                    warn!("Failed generating icon for platform: {:?} {e}", entry.path());
                    String::new()
                },
            };

            // Generate info
            let info = AutotaggerPlatformInfo {
                built_in: false,
                platform: platform.info.info.to_owned(),
                icon,
                requires_auth: platform.info.info.requires_auth,
                supported_tags: platform.info.info.supported_tags.clone(),
            };

            self.platforms.push(AutotaggerPlatform { info, platform: Box::new(platform) });
        }
        Ok(())
    }
}

/// Autotagger Platform
pub struct AutotaggerPlatform {
    pub info: AutotaggerPlatformInfo,
    pub platform: Box<dyn AutotaggerSourceBuilder + Send + Sync>
}

/// For passing platform list into UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutotaggerPlatformInfo {
    pub built_in: bool,
    pub platform: PlatformInfo,
    /// Encoded for UI
    pub icon: String,
    pub requires_auth: bool,
    pub supported_tags: Vec<SupportedTag>,
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
    pub fn open(path: &PathBuf) -> Result<CustomPlatform, Error> {
        let p = unsafe {
            let lib = Library::new(&path)?;
            // Check version compatibility
            let version: Symbol<*const i32> = lib.get(b"_1T_PLATFORM_COMPATIBILITY")?;
            if **version != onetagger_tagger::custom::CUSTOM_PLATFORM_COMPATIBILITY {
                warn!("Plugin is incompatible! Plugin version: {}, Supported version: {}", **version, onetagger_tagger::custom::CUSTOM_PLATFORM_COMPATIBILITY);
                return Err(anyhow!("Plugin is incompatible!"));
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
    pub fn open_platform(path: &PathBuf) -> Result<AutotaggerPlatform, Error> {
        let lib = Self::open(path)?;
        Ok(AutotaggerPlatform {
            info: AutotaggerPlatformInfo {
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
            },
            platform: Box::new(lib)
        })
    }

    //TODO: BETTER WAY THAN JUST REOPENING THE DLL
    pub fn get_builder(&self) -> Result<CustomPlatform, Error> {
        Ok(Self::open(&self.path)?)
    }

    /// Get source
    fn get_source_raw(&self, config: &TaggerConfig) -> Result<CustomPlatformSource, Error> {
        let ptr = unsafe {
            let get_source_fn: Symbol<unsafe extern fn(*mut c_void, &TaggerConfig) -> *mut c_void> = self.library.get(b"_1t_builder_get_source")?;
            let source_ptr = get_source_fn(self.builder.0, config);
            if source_ptr.is_null() {
                return Err(anyhow!("Failed creating custom platform source!"));
            }
            source_ptr
        };
        // Lifetime fix
        let match_fn = unsafe {
            let f: Symbol<unsafe extern fn(*mut c_void, &AudioFileInfo, &TaggerConfig) -> *mut MatchTrackResult> = 
                self.library.get(b"_1t_match_track")?;
            std::mem::transmute(f)
        };
        let extend_fn = unsafe {
            let f: Symbol<unsafe extern fn(*mut c_void, &mut Track, &TaggerConfig) -> *mut MatchTrackResult> = 
                self.library.get(b"_1t_extend_track")?;
            std::mem::transmute(f)
        };
        Ok(CustomPlatformSource {
            ptr: PtrWrap(ptr),
            extend_fn,
            match_fn
        })
    }

    /// Get config callback
    fn config_callback_raw(&self, name: &str, config: Value) -> Result<ConfigCallbackResponse, Error> {
        let config = Box::new(config);
        let output = unsafe {
            let f: Symbol<unsafe extern fn(*mut c_void, &str, *mut Value) -> *mut ConfigCallbackResponse> =
                self.library.get(b"_1t_builder_config_callback")?;
            let response = Box::from_raw(f(self.builder.0, name, Box::into_raw(config)));
            let output = (*response).clone();
            drop(response);
            output
        };
        Ok(output)
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

    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        Ok(Box::new(self.get_source_raw(config)?))
    }

    fn info(&self) -> PlatformInfo {
        self.info.clone()
    }

    fn config_callback(&mut self, name: &str, config: Value) -> ConfigCallbackResponse {
        match self.config_callback_raw(name, config) {
            Ok(r) => r,
            Err(e) => ConfigCallbackResponse::Error { error: e.to_string() },
        }
    }
    
}

struct CustomPlatformSource {
    ptr: PtrWrap,
    match_fn: Symbol<'static, unsafe extern fn(*mut c_void, &AudioFileInfo, &TaggerConfig) -> *mut MatchTrackResult>,
    extend_fn: Symbol<'static, unsafe extern fn(*mut c_void, &mut Track, &TaggerConfig) -> *mut Option<String>>,
}

impl AutotaggerSource for CustomPlatformSource {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        unsafe {
            let f = &self.match_fn;
            let r = Box::from_raw(f(self.ptr.0, info, config));
            match *r {
                MatchTrackResult::Ok(r) => Ok(r),
                MatchTrackResult::Err(e) => Err(anyhow!("{e}")),
            }
        }
    }

    fn extend_track(&mut self, track: &mut Track, config: &TaggerConfig) -> Result<(), Error> {
        unsafe {
            let f = &self.extend_fn;
            let r = Box::from_raw(f(self.ptr.0, track, config));
            match *r {
                Some(e) => Err(anyhow!("{e}")),
                None => Ok(()),
            }
        }
    }

    
}


/// Pointer wrapper
struct PtrWrap(pub *mut c_void);
unsafe impl Send for PtrWrap {}
unsafe impl Sync for PtrWrap {}
