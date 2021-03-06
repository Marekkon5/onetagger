use std::error::Error;

use crate::Track;

/// Version of supported custom platform
pub const CUSTOM_PLATFORM_COMPATIBILITY: i32 = 10;

/// Macro for creating custom platform plugins
#[macro_export]
macro_rules! create_plugin {
    ($builder_type:ty, $plugin_type:ty) => {
        #[no_mangle]
        pub static _1T_PLATFORM_COMPATIBILITY: i32 = onetagger_tagger::custom::CUSTOM_PLATFORM_COMPATIBILITY;

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