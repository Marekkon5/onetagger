extern crate rouille;
extern crate web_view;

use std::error::Error;
use std::fmt;
use std::thread;
use std::io::prelude::*;
use std::fs::File;
use std::path::PathBuf;
use rouille::router;
use serde_json::Value;
use serde::{Serialize, Deserialize};
use app_dirs::{app_root, AppInfo, AppDataType};

pub mod socket;
pub mod player;
pub mod quicktag;
pub mod audiofeatures;

//UI
static INDEX_HTML: &'static str = include_str!("../../client/dist/dist.html");
//For directories
static APP_INFO: AppInfo = AppInfo {name: "OneTagger", author: "OneTagger"};

//Onetagger settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    ui: Value
}

impl Settings {
    //Create settings from UI json
    pub fn from_ui(ui: &Value) -> Settings {
        Settings {
            ui: ui.to_owned()
        }
    }

    //Load settings from file
    pub fn load() -> Result<Settings, Box<dyn Error>> {
        let path = Settings::get_path()?;
        let settings: Settings = serde_json::from_reader(File::open(path)?)?;
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
        let root = app_root(AppDataType::UserConfig, &APP_INFO)?;
        Ok(root)
    }

    //Get settings path
    fn get_path() -> Result<String, Box<dyn Error>> {
        let path = Settings::get_folder()?.join("settings.json");
        Ok(path.to_str().ok_or("Error converting path to string!")?.to_string())
    }
}

//Start webview window
pub fn start_webview() {
    //Normal webview
    let webview = web_view::builder()
        .invoke_handler(|_, __| Ok(()))
        .content(web_view::Content::Url("http://127.0.0.1:36913"))
        .user_data(())
        .title("OneTagger")
        .size(1280, 720)
        .resizable(true)
        .debug(true)
        .build()
        .unwrap();
    webview.run().unwrap();
}

//Start WebSocket server
pub fn start_socket_thread() {
    thread::spawn(|| {
        socket::start_socket_server();
    });
}

//Start webserver for hosting static index.html
pub fn start_webserver_thread() {
    thread::spawn(|| {
        rouille::start_server("127.0.0.1:36913", move |request| {
            router!(request, 
                (GET) (/) => {
                    rouille::Response::html(INDEX_HTML)
                },
                _ => rouille::Response::empty_404()
            )
        });
    });
}

//Start everything
pub fn start_all() {
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
            start_socket_thread();
            start_webserver_thread();
            start_webview_cef();
            //CEF will spawn threads, keep servers running
            loop {
                sleep(Duration::from_secs(10));
            }
        }
        start_webview_cef();
        return;
    }

    //Normal
    start_socket_thread();
    start_webserver_thread();
    start_webview();
}


//CEF Webview, intended for Windows only
#[cfg(target_os = "windows")]
pub fn start_webview_cef() {
    use winapi::um::winuser::{WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_OVERLAPPEDWINDOW, WS_VISIBLE};
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
    window_info.height = 720;
    window_info.x = 300;
    window_info.y = 100;

    let browser_settings = BrowserSettings::new();
    let client = Client::new(ClientCallbacksImpl {
        life_span_handler: LifeSpanHandler::new(LifeSpanHandlerImpl {})
    });

    info!("Opening CEF Window");
    let _browser = BrowserHost::create_browser_sync(
        &window_info,
        client,
        "http://127.0.0.1:36913/",
        &browser_settings,
        None,
        None
    );
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