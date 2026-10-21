//! Files opened from outside the window and the notice «make mzLab the default app?»: the glue between Tauri and `apertura.rs` (single instance,
//! the command line, `RunEvent::Opened` on macOS) and the commands the page calls (`predefinito.js`). The rules are in `apertura.rs`.

use super::{data_dir, register, save_config, PickedFile};
use mzlab_desktop::apertura::{self, Pending, Risposta};
use mzlab_desktop::librerie;
use mzlab_desktop::State;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// The files that arrived from outside (double click, «Open with», a second launch) and whether the notice «make mzLab the default app?» is due.
#[derive(Serialize)]
pub struct Opened {
    pub files: Vec<PickedFile>,
    pub ask: bool,
}

/// The page empties the queue when it is ready (`desktop-shim.js`): the files get their numbers like those of the system dialog.
#[tauri::command]
pub fn take_opened(app: AppHandle, state: tauri::State<'_, Arc<State>>, pending: tauri::State<'_, Pending>) -> Opened {
    let files = register(&state, pending.take());
    let ask = !files.is_empty()
        && data_dir(&app).is_ok_and(|d| {
            let mut c = librerie::read_config(&d);
            let ask = apertura::registra_apertura(&mut c);
            let _ = librerie::write_config(&d, &c);
            ask
        });
    Opened { files, ask }
}

/// Queues files from outside and tells the page (which may not be loaded yet: then it takes them at start); the window comes to the front.
fn hand_over(app: &AppHandle, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    app.state::<Pending>().push(paths);
    let _ = app.emit("mzlab-open", ());
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// The notice and the setting: `{answer, count}` as kept in `config.json` (the answer is "si", "mai" or null).
#[tauri::command]
pub fn assoc_get(app: AppHandle) -> serde_json::Value {
    let c = data_dir(&app).map(|d| librerie::read_config(&d)).unwrap_or_default();
    serde_json::json!({ "answer": c.risposta, "count": c.aperture })
}

/// The answer to the notice: "si", "mai" (both end it) or "no" (not now: the counter goes on).
#[tauri::command]
pub fn assoc_answer(app: AppHandle, answer: String) -> Result<(), String> {
    let r = match answer.as_str() {
        "si" => Some(Risposta::Si),
        "mai" => Some(Risposta::Mai),
        "no" => None,
        _ => return Err("bad answer".into()),
    };
    save_config(&app, |c| apertura::rispondi(c, r))
}

/// The setting «ask again»: forgets the answer, so the next file opened from outside asks.
#[tauri::command]
pub fn assoc_reset(app: AppHandle) -> Result<(), String> {
    save_config(&app, apertura::rifai)
}

/// Registered first: a second launch (double click on a file with the app open) gives its arguments to this window and exits.
pub fn single_instance() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_single_instance::init(|app, args, cwd| {
        let cwd = PathBuf::from(cwd);
        hand_over(app, apertura::paths_from_args(args.get(1..).unwrap_or_default(), Some(&cwd)));
    })
}

/// A file given on the command line (Windows, Linux: double click with the app closed) waits for the page.
pub fn queue_command_line(app: &AppHandle) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    app.state::<Pending>().push(apertura::paths_from_args(&args, None));
}

/// macOS gives the files of a double click (or «Open with») as events, also with the app already open.
#[allow(unused_variables)]
pub fn run_event(app: &AppHandle, event: &tauri::RunEvent) {
    #[cfg(target_os = "macos")]
    if let tauri::RunEvent::Opened { urls } = event {
        hand_over(app, urls.iter().filter_map(|u| u.to_file_path().ok()).filter(|p| apertura::is_openable(p)).collect());
    }
}
