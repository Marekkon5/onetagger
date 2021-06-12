extern crate rouille;
extern crate web_view;

use std::error::Error;
use std::fmt;
use std::thread;
use std::fs;
use std::io::prelude::*;
use std::fs::File;
use std::path::PathBuf;
use rouille::{router, Response};
use serde_json::Value;
use serde::{Serialize, Deserialize};
use directories::ProjectDirs;

pub mod socket;
pub mod player;
pub mod quicktag;
pub mod audiofeatures;
pub mod tageditor;

//UI
static INDEX_HTML: &'static str = include_str!("../../client/dist/dist.html");
static BG_PNG: &'static [u8] = include_bytes!("../../client/dist/bg.png");
static FAVICON_PNG: &'static [u8] = include_bytes!("../../client/dist/favicon.png");

//Onetagger settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    ui: Value,
    version: Option<i32>
}

impl Settings {
    //Create settings from UI json
    pub fn from_ui(ui: &Value) -> Settings {
        Settings {
            ui: ui.to_owned(),
            version: Some(2)
        }
    }

    //Load settings from file
    pub fn load() -> Result<Settings, Box<dyn Error>> {
        let path = Settings::get_path()?;
        let settings: Settings = serde_json::from_reader(File::open(&path)?)?;

        //v1.0 are not compatible with 1.1, create backup
        if settings.version.unwrap_or(1) == 1 {
            let new_path = format!("{}-1.0.bak", &path);
            fs::copy(&path, &new_path)?;
            info!("Backup of settings created: {}", new_path);
            fs::remove_file(&path)?;
            return Settings::load();
        }

        Ok(settings)
    }
    
    //Save settings to file
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let path = Settings::get_path()?;
        let mut file = File::create(path)?;
        file.write_all(serde_json::to_string_pretty(self)?.as_bytes())?;
        Ok(())
    }

    //Get app data folder
    pub fn get_folder() -> Result<PathBuf, Box<dyn Error>> {
        let root = ProjectDirs::from("com", "OneTagger", "OneTagger").ok_or("Error getting dir!")?;
        if !root.preference_dir().exists() {
            fs::create_dir_all(root.preference_dir())?;
        }
        Ok(root.preference_dir().to_owned())
    }

    //Get settings path
    fn get_path() -> Result<String, Box<dyn Error>> {
        let path = Settings::get_folder()?.join("settings.json");
        Ok(path.to_str().ok_or("Error converting path to string!")?.to_string())
    }
}

//Should have data from arguments and other flags (eg. port / host in future)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartContext {
    pub server_mode: bool,
    pub start_path: Option<String>
}

//Start webview window
pub fn start_webview() {
    //Normal webview
    let webview = web_view::builder()
        .invoke_handler(|_, __| Ok(()))
        .content(web_view::Content::Url("http://127.0.0.1:36913"))
        .user_data(())
        .title("One Tagger")
        .size(1280, 750)
        .min_size(1150, 550)
        .resizable(true)
        .debug(true)
        .build()
        .unwrap();
    webview.run().unwrap();
}

//Start WebSocket server
pub fn start_socket_thread(context: StartContext) {
    thread::spawn(move || {
        socket::start_socket_server(context);
    });
}

//Start webserver for hosting static index.html
pub fn start_webserver_thread() {
    thread::spawn(|| {
        rouille::start_server("127.0.0.1:36913", move |request| {
            router!(request, 
                (GET) ["/"] => {
                    Response::html(INDEX_HTML)
                },
                (GET) ["/bg.png"] => {
                    Response::from_data("image/png", BG_PNG)
                },
                (GET) ["/favicon.png"] => {
                    Response::from_data("image/png", FAVICON_PNG)
                },
                //Get thumbnail of image from tag by path
                (GET) ["/thumb"] => {
                    match request.get_param("path") {
                        Some(path) => {
                            match quicktag::QuickTagFile::get_art(&path) {
                                Ok(art) => Response::from_data("image/jpeg", art),
                                Err(e) => {
                                    warn!("Error loading album art: {} File: {}", e, path);
                                    Response::empty_404()
                                }
                            }
                        },
                        None => Response::empty_404()
                    }
                },
                _ => Response::empty_404()
            )
        });
    });
}

//Start everything
pub fn start_all(context: StartContext) {
    //Server mode
    if context.server_mode {
        info!("Starting server mode! http://localhost:36913 ws://localhost:36912");
        start_webserver_thread();
        socket::start_socket_server(context);
        return;
    }

    //Windows CEF
    #[cfg(target_os = "windows")]
    {
        use std::env;
        use std::thread::sleep;
        use std::time::Duration;
        //Check if running inside CEF
        let cef_args = vec![
            "--type=gpu-process".to_string(), 
            "--type=utility".to_string(), 
            "--type=renderer".to_string()
        ];
        if !env::args().any(|a| cef_args.contains(&a.to_lowercase())) {
            start_socket_thread(context);
            start_webserver_thread();
            start_webview_cef();
            //CEF will spawn threads, keep servers running
            loop {
                sleep(Duration::from_secs(10));
            }
        }
        start_webview_cef();
    }

    //Normal
    #[cfg(not(target_os = "windows"))]
    {
        start_socket_thread(context);
        start_webserver_thread();
        start_webview();
    }
}


//CEF Webview, intended for Windows only
#[cfg(target_os = "windows")]
pub fn start_webview_cef() {
    use winapi::um::winuser::{WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_OVERLAPPEDWINDOW, 
        WS_VISIBLE, WM_SETICON, ICON_BIG, ICON_SMALL, SendMessageA, CreateIconIndirect, 
        ICONINFO, GetDC};
    use winapi::shared::ntdef::NULL;
    use winapi::um::wingdi::{CreateBitmap, CreateCompatibleBitmap};
    use winapi::ctypes::c_void;
    use cef::{
        app::{App, AppCallbacks},
        browser::{Browser, BrowserSettings},
        browser_host::BrowserHost,
        client::{
            Client, ClientCallbacks,
            life_span_handler::{LifeSpanHandler, LifeSpanHandlerCallbacks},
        },
        settings::{Settings, LogSeverity},
        window::WindowInfo,
        Context
    };

    //Callback structs
    struct AppCallbacksImpl {}
    impl AppCallbacks for AppCallbacksImpl {}

    struct ClientCallbacksImpl {
        life_span_handler: LifeSpanHandler,
    }
    impl ClientCallbacks for ClientCallbacksImpl {
        fn get_life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(self.life_span_handler.clone())
        }
    }

    struct LifeSpanHandlerImpl {}
    impl LifeSpanHandlerCallbacks for LifeSpanHandlerImpl {
        fn on_before_close(&self, _browser: Browser) {
            cef::quit_message_loop().unwrap();
        }
    }

    //Create app
    let app = App::new(AppCallbacksImpl {});
    cef::execute_process(Some(app.clone()), None);

    //Init
    let settings = Settings::new().log_severity(LogSeverity::Info);
    let context = Context::initialize(settings, Some(app), None).unwrap();
    // let logger = Box::new(Logger::builder().level(log::LevelFilter::Info).build());
    info!("Starting CEF");

    //Create window
    let mut window_info = WindowInfo::new();
    window_info.platform_specific.style = WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_CLIPSIBLINGS | WS_VISIBLE;
    window_info.window_name = "One Tagger".into();
    window_info.width = 1280;
    window_info.height = 750;
    window_info.x = 300;
    window_info.y = 100;

    let browser_settings = BrowserSettings::new();
    let client = Client::new(ClientCallbacksImpl {
        life_span_handler: LifeSpanHandler::new(LifeSpanHandlerImpl {})
    });

    info!("Opening CEF Window");
    let browser = BrowserHost::create_browser_sync(
        &window_info,
        client,
        "http://127.0.0.1:36913/",
        &browser_settings,
        None,
        None
    );
    //Set icon
    unsafe {
        if let Some(handle) = browser.get_host().get_window_handle() {
            let raw = handle.to_cef_handle();
            //Create bitmap
            let mut icon_data = include_bytes!("../../assets/64x64.bin").to_owned();
            let bitmap = CreateBitmap(64, 64, 1, 32, icon_data.as_mut_ptr() as *mut c_void);
            let dc = GetDC(raw);
            let mask = CreateCompatibleBitmap(dc, 64, 64);
            //Create icon
            let mut icon = ICONINFO {
                fIcon: 1,
                xHotspot: NULL as u32,
                yHotspot: NULL as u32,
                hbmMask: mask,
                hbmColor: bitmap
            };
            let icon = CreateIconIndirect(&mut icon);
            //Set icon
            SendMessageA(raw, WM_SETICON, ICON_BIG as usize, icon as isize);
            SendMessageA(raw, WM_SETICON, ICON_SMALL as usize, icon as isize);
        }
    }

    context.run_message_loop();

    info!("CEF Quit");
}

//OneTagger Error, meant for UI
#[derive(Debug, Clone)]
pub struct OTError {
    message: String
}
impl OTError {
    pub fn new(msg: &str) -> OTError {
        OTError {
            message: msg.to_owned()
        }
    }
}
impl Error for OTError {}
impl fmt::Display for OTError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}