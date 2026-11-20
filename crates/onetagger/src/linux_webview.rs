//! Recovery controls live in GTK so they remain usable after a WebKit process exits.
use gtk::prelude::*;
use std::cell::Cell;
use std::time::{Duration, Instant};
use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};
use wry::WebViewExtUnix;

pub fn page_visible(view: &wry::WebView) -> bool {
    view.webview().is_visible()
}

pub fn install_recovery(view: &wry::WebView, container: &gtk::Box, launch_url: &str) {
    let page = view.webview();
    let actions = gtk::Box::new(gtk::Orientation::Vertical, 12);
    actions.set_valign(gtk::Align::Center);
    actions.set_halign(gtk::Align::Center);
    let label = gtk::Label::new(Some("The OneTagger display process stopped."));
    let retry = gtk::Button::with_label("Reopen OneTagger");
    let browser = gtk::Button::with_label("Open this session in your browser");
    for widget in [&retry, &browser] { actions.pack_end(widget, false, false, 0); }
    actions.pack_start(&label, false, false, 0);
    container.pack_start(&actions, true, true, 0);
    // Tao may show the entire window later. Show this panel only on a repeated failure.
    actions.show_all();
    actions.hide();
    actions.set_no_show_all(true);

    let restore = launch_url.to_owned();
    let weak_page = page.downgrade();
    let panel = actions.downgrade();
    retry.connect_clicked(move |_| {
        if let (Some(page), Some(panel)) = (weak_page.upgrade(), panel.upgrade()) {
            panel.hide();
            page.show();
            page.load_uri(&restore);
        }
    });
    let external = launch_url.to_owned();
    browser.connect_clicked(move |_| {
        if webbrowser::open(&external).is_err() {
            label.set_text("Could not open your browser. Use the launch link printed in the terminal.");
        }
    });

    let restore = launch_url.to_owned();
    let last = Cell::new(None::<Instant>);
    page.connect_web_process_terminated(move |page, reason| {
        warn!("WebKit display process terminated: {reason:?}");
        let now = Instant::now();
        if last.replace(Some(now)).is_some_and(|previous| now.duration_since(previous) < Duration::from_secs(60)) {
            page.hide();
            actions.show();
        } else {
            if let Some(settings) = WebViewExt::settings(page) {
                settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Never);
            }
            page.load_uri(&restore);
        }
    });
}
