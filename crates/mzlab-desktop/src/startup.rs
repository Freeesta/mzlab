//! Start of the app (`setup` of the builder in `main.rs`): the folders of the engine, the files given on the command line, and the window, which
//! answers from the updated web part (if any) before the embedded copy.

use super::{apri, data_dir};
use mzlab_desktop::librerie;
use mzlab_desktop::update;
use mzlab_desktop::State;
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::http::{header, StatusCode};
use tauri::{App, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    apri::queue_command_line(app.handle());
    // the indexes (.mzidx) built natively live in the cache folder of the app, where the system may clear them (they are rebuilt)
    if let Ok(cache) = app.path().app_cache_dir() {
        let _ = app
            .state::<Arc<State>>()
            .set_index_folder(cache.join("mzidx"));
        app.state::<Arc<State>>()
            .set_convert_folder(cache.join("mzml"));
    }
    // the list of the files, the notebook and the files the app makes itself: the data folder of the app
    if let Ok(d) = data_dir(app.handle()) {
        app.state::<Arc<State>>()
            .set_data_folder(d.join("sessione"));
    }
    if let Some(dir) = data_dir(app.handle())
        .ok()
        .and_then(|d| librerie::read_config(&d).cartella)
    {
        app.state::<Arc<State>>().libs.set_root(PathBuf::from(dir));
    }
    // the updated web part (if any) answers before the embedded copy; a damaged folder is set aside first
    let web = data_dir(app.handle()).map(|d| {
        update::recover(&d);
        d.join("web")
    });
    let mut win = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("mzlab")
        .inner_size(1400.0, 900.0)
        .min_inner_size(900.0, 600.0);
    // macOS: the toolbar of the page sits on the title bar line, next to the traffic lights (desktop-titlebar.css); elsewhere the native bar stays
    #[cfg(target_os = "macos")]
    {
        win = win
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true);
    }
    if let Ok(web) = web {
        win = win.on_web_resource_request(move |request, response| {
            if let Some((body, ctype)) = update::serve_override(&web, request.uri().path()) {
                *response.status_mut() = StatusCode::OK;
                response.headers_mut().remove(header::CONTENT_LENGTH);
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    header::HeaderValue::from_static(ctype),
                );
                *response.body_mut() = Cow::Owned(body);
            }
        });
    }
    win.build()?;
    Ok(())
}
