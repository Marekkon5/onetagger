use std::error::Error;
use log::{Record, Level, RecordBuilder};

use crate::Track;

/// Version of supported custom platform
pub const CUSTOM_PLATFORM_COMPATIBILITY: i32 = 34;

/// Logging from plugins
#[no_mangle]
pub extern "C" fn write_log(record: *mut FFIRecord) {
    unsafe {
        let mut builder = RecordBuilder::new();
        let target = format!("onetagger_custom_platform::{}", (*record).target);
        log::logger().log(&builder
            .args(format_args!("{}", (*record).body))
            .level((*record).level)
            .target(target.as_str())
            .file((*record).file.as_ref().map(|x| &**x))
            .line((*record).line)
            .module_path((*record).module_path.as_ref().map(|x| &**x))
            .build()
        );
    }
}

/// Macro for creating custom platform plugins
#[macro_export]
macro_rules! create_plugin {
    ($builder_type:ty, $plugin_type:ty) => {
        /// For checking if plugin in compatible
        #[no_mangle]
        pub static _1T_PLATFORM_COMPATIBILITY: i32 = onetagger_tagger::custom::CUSTOM_PLATFORM_COMPATIBILITY;

        /// 1T logging
        struct _OnetaggerCustomPlatformLogger;

        static mut _1T_LOG_FN: Option<extern fn(*mut onetagger_tagger::custom::FFIRecord)> = None;
        static _1T_LOGGER: _OnetaggerCustomPlatformLogger = _OnetaggerCustomPlatformLogger;

        impl log::Log for _OnetaggerCustomPlatformLogger {
            /// Filtered in 1T
            fn enabled(&self, _: &log::Metadata) -> bool {
                true
            }

            /// Send log to 1T
            fn log(&self, record: &log::Record) {
                unsafe {
                    if let Some(f) = _1T_LOG_FN {
                        let mut record: onetagger_tagger::custom::FFIRecord = record.into();
                        let mut b = Box::new(record);
                        (f)(&mut *b)
                    }
                }
            }

            fn flush(&self) {}
        }

        /// Pass the logging callback to the plugin
        #[no_mangle]
        pub extern "C" fn _1t_register_logger(f: extern fn (*mut onetagger_tagger::custom::FFIRecord)) {
            unsafe { _1T_LOG_FN = Some(f) }
            log::set_logger(&_1T_LOGGER).ok();
            log::set_max_level(log::LevelFilter::Trace);
        }

        /// Create new instance of the AutotaggerSourceBuilder and convert it into a raw pointer
        #[no_mangle]
        pub extern "C" fn _1t_create_builder() -> *mut std::ffi::c_void {
            let boxed = Box::new(<$builder_type>::new());
            Box::into_raw(boxed) as *mut std::ffi::c_void
        }

        /// Consume the builder pointer and free it
        #[no_mangle]
        pub extern "C" fn _1t_free_builder(ptr: *mut std::ffi::c_void) {
            let builder: Box<$builder_type> = unsafe { Box::from_raw(ptr as *mut $builder_type) };
            drop(builder)
        }

        /// Call .info() on the builder
        #[no_mangle]
        pub extern "C" fn _1t_builder_info(ptr: *mut std::ffi::c_void) -> *mut onetagger_tagger::PlatformInfo {
            let builder: Box<$builder_type> = unsafe { Box::from_raw(ptr as *mut $builder_type) };
            let info = Box::new(builder.info());
            std::mem::forget(builder);
            Box::into_raw(info)
        }

        /// Call .get_source() on the builder, returns result the Source object and String error
        #[no_mangle]
        pub extern "C" fn _1t_builder_get_source(
            ptr: *mut std::ffi::c_void, 
            config: &onetagger_tagger::TaggerConfig
        ) -> *mut std::ffi::c_void {
            let mut builder: Box<$builder_type> = unsafe { Box::from_raw(ptr as *mut $builder_type) };
            let source = builder.get_source(config);
            if source.is_err() {
                std::mem::forget(builder);
                return std::ptr::null_mut();
            }
            std::mem::forget(builder);
            Box::into_raw(source.unwrap()) as *mut std::ffi::c_void
        }

        /// Call .match_track on source
        #[no_mangle]
        pub extern "C" fn _1t_match_track(
            ptr: *mut std::ffi::c_void,
            info: &onetagger_tagger::AudioFileInfo,
            config: &onetagger_tagger::TaggerConfig
        ) -> *mut onetagger_tagger::custom::MatchTrackResult {
            let mut source: Box<dyn onetagger_tagger::AutotaggerSource> = unsafe { 
                Box::from_raw(ptr as *mut $plugin_type) 
            };
            let r = onetagger_tagger::custom::MatchTrackResult::from_result(source.match_track(info, config));
            std::mem::forget(source);
            Box::into_raw(Box::new(r))
        }
    }
}

/// Custom result for returning from FFI
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum MatchTrackResult {
    Ok(f64, Track),
    NoMatch,
    Err(String)
}

impl MatchTrackResult {
    /// Convert match_track Result into MatchTrackResult
    pub fn from_result(r: Result<Option<(f64, Track)>, Box<dyn Error>>) -> MatchTrackResult {
        match r {
            Ok(Some((acc, track))) => MatchTrackResult::Ok(acc, track),
            Ok(None) => MatchTrackResult::NoMatch,
            Err(e) => MatchTrackResult::Err(e.to_string())
        }
    }
}

/// FFI logging
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FFIRecord {
    body: String,
    level: Level,
    target: String,
    file: Option<String>,
    line: Option<u32>,
    module_path: Option<String>,
}

impl<'a> From<&Record<'a>> for FFIRecord {
    fn from(value: &Record<'a>) -> Self {
        FFIRecord {
            body: value.args().to_string(),
            level: value.level(),
            target: value.target().to_string(),
            file: value.file().map(String::from),
            line: value.line(),
            module_path: value.module_path().map(String::from),
        }
    }
}
