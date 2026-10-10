//! The window: the web app of `mzlab/web` as it is, a Tauri webview. Files come from the system dialog (or from a drop on the window)
//! as paths and are memory-mapped by the native engine; the page asks for TIC, XIC and scans through `mzlab://` (see `protocol.rs`
//! and `shim/`). The only network code is the update of the web part (`update.rs`: the manifest and the changed files of
//! freeesta.github.io, on request of the user or once a day); no file of the user is ever sent.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::PathBuf;
use std::sync::Arc;

use mzlab_desktop::update::{self, Http, Source, UpdateError};
use mzlab_desktop::{handle, State};
use serde::Serialize;
use std::borrow::Cow;
use tauri::http::{header, Response, StatusCode};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
struct PickedFile {
    id: u32,
    name: String,
    size: u64,
}

fn register(state: &State, paths: Vec<PathBuf>) -> Vec<PickedFile> {
    paths
        .into_iter()
        .filter_map(|p| {
            let (id, f) = state.pick(p).ok()?;
            Some(PickedFile {
                id,
                name: f.name,
                size: f.size,
            })
        })
        .collect()
}

/// System file dialog (`dam`: only `.dam` files; otherwise mzML and Thermo `.raw`).
#[tauri::command]
async fn pick_files(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<State>>,
    dam: bool,
) -> Result<Vec<PickedFile>, String> {
    let dlg = app.dialog().file();
    let dlg = if dam {
        dlg.add_filter("DAM", &["dam", "DAM"])
    } else {
        dlg.add_filter("mzML / RAW", &["mzML", "mzml", "raw", "RAW"])
    };
    let picked = tauri::async_runtime::spawn_blocking(move || dlg.blocking_pick_files())
        .await
        .map_err(|e| e.to_string())?;
    let paths = picked
        .unwrap_or_default()
        .into_iter()
        .filter_map(|f| f.into_path().ok())
        .collect();
    Ok(register(&state, paths))
}

/// Files dropped on the window (the webview gives paths, not `File` objects).
#[tauri::command]
fn register_paths(state: tauri::State<'_, Arc<State>>, paths: Vec<String>) -> Vec<PickedFile> {
    register(&state, paths.into_iter().map(PathBuf::from).collect())
}

/// Version of this native binary (compared with `desktop_min` of the manifest).
const NATIVE: &str = env!("CARGO_PKG_VERSION");

fn data_dir(app: &AppHandle) -> Result<PathBuf, UpdateError> {
    let d = app.path().app_data_dir().map_err(|e| UpdateError {
        key: "update.error.io",
        detail: e.to_string(),
    })?;
    std::fs::create_dir_all(&d).map_err(|e| UpdateError {
        key: "update.error.io",
        detail: e.to_string(),
    })?;
    Ok(d)
}

/// The manifest that came with the binary.
fn embedded_manifest(app: &AppHandle) -> Vec<u8> {
    app.asset_resolver()
        .get(update::MANIFEST.to_string())
        .map(|a| a.bytes.to_vec())
        .unwrap_or_default()
}

fn json_or_error(r: Result<String, UpdateError>) -> String {
    r.unwrap_or_else(|e| e.to_json())
}

fn do_check(app: &AppHandle) -> Result<String, UpdateError> {
    let local = update::local_manifest(&data_dir(app)?, &embedded_manifest(app))?;
    let remote = update::parse_manifest(&Http::new().get(update::MANIFEST)?)?;
    serde_json::to_string(&update::check(&local, &remote, NATIVE)).map_err(|e| UpdateError {
        key: "update.error.io",
        detail: e.to_string(),
    })
}

fn do_apply(app: &AppHandle) -> Result<String, UpdateError> {
    let data = data_dir(app)?;
    let local = update::local_manifest(&data, &embedded_manifest(app))?;
    let src = Http::new();
    let remote = update::parse_manifest(&src.get(update::MANIFEST)?)?;
    let files = update::apply(&data, &local, &remote, NATIVE, &src, |p| {
        let _ = app.emit("update-progress", p);
    })?;
    Ok(serde_json::json!({ "status": "ready", "files": files }).to_string())
}

/// Compares the manifest of the site with the app's own (JSON of `update::Check` or `{status:"error", error_key}`).
#[tauri::command]
async fn update_check(app: AppHandle) -> String {
    tauri::async_runtime::spawn_blocking(move || json_or_error(do_check(&app)))
        .await
        .unwrap_or_else(|e| {
            UpdateError {
                key: "update.error.io",
                detail: e.to_string(),
            }
            .to_json()
        })
}

/// Downloads and verifies the changed files and swaps the folder; progress as `update-progress` events.
#[tauri::command]
async fn update_apply(app: AppHandle) -> String {
    tauri::async_runtime::spawn_blocking(move || json_or_error(do_apply(&app)))
        .await
        .unwrap_or_else(|e| {
            UpdateError {
                key: "update.error.io",
                detail: e.to_string(),
            }
            .to_json()
        })
}

#[tauri::command]
fn update_restart(app: AppHandle) {
    app.restart();
}

fn main() {
    let state = Arc::new(State::new());
    let for_protocol = state.clone();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            pick_files,
            register_paths,
            update_check,
            update_apply,
            update_restart
        ])
        .setup(|app| {
            // the updated web part (if any) answers before the embedded copy; a damaged folder is set aside first
            let web = data_dir(app.handle()).map(|d| {
                update::recover(&d);
                d.join("web")
            });
            let mut win = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("mzlab")
                .inner_size(1400.0, 900.0)
                .min_inner_size(900.0, 600.0);
            if let Ok(web) = web {
                win = win.on_web_resource_request(move |request, response| {
                    if let Some((body, ctype)) = update::serve_override(&web, request.uri().path())
                    {
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
        })
        .register_asynchronous_uri_scheme_protocol("mzlab", move |_ctx, request, responder| {
            let state = for_protocol.clone();
            // the engine can take long on a big file: never on the thread of the webview
            std::thread::spawn(move || {
                let uri = request.uri();
                let r = handle(&state, uri.path(), uri.query().unwrap_or(""));
                responder.respond(
                    Response::builder()
                        .status(r.status)
                        .header(header::CONTENT_TYPE, r.content_type)
                        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                        .body(r.body)
                        .unwrap_or_default(),
                );
            });
        })
        .build(tauri::generate_context!())
        .expect("mzlab: the window could not start");
    app.run(move |_handle, event| {
        if let RunEvent::Exit = event {
            state.cleanup();
        }
    });
}
