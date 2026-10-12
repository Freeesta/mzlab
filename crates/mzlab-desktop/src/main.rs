//! The window: the web app of `mzlab/web` as it is, a Tauri webview. Files come from the system dialog (or from a drop on the window)
//! as paths and are memory-mapped by the native engine; the page asks for TIC, XIC and scans through `mzlab://` (see `protocol.rs`
//! and `shim/`). The only network code is the update of the web part (`update.rs`: the manifest and the changed files of
//! freeesta.github.io, on request of the user or once a day); no file of the user is ever sent.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use mzlab_desktop::librerie::{self, Config};
use mzlab_desktop::scarica::{self, Policy};
use mzlab_desktop::update::{self, Http, Source, UpdateError};
use mzlab_desktop::{handle_body, State};
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

/// «Scarica mzML»: system save dialog for a converted mzML (by its number); None if the user cancels, otherwise the chosen path.
#[tauri::command]
async fn save_mzml(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<State>>,
    id: u32,
    name: String,
) -> Result<Option<String>, String> {
    let dlg = app
        .dialog()
        .file()
        .add_filter("mzML", &["mzML"])
        .set_file_name(name);
    let picked = tauri::async_runtime::spawn_blocking(move || dlg.blocking_save_file())
        .await
        .map_err(|e| e.to_string())?;
    let Some(path) = picked.and_then(|f| f.into_path().ok()) else {
        return Ok(None);
    };
    state.save_copy(id, &path).map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().into_owned()))
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

/// The folder of the libraries and the switch of the external downloads, as the page shows them in the window «Librerie» and in the settings.
#[derive(Serialize)]
struct LibConfig {
    dir: String,
    default: bool,
    external: bool,
}

fn lib_config(app: &AppHandle, state: &State) -> LibConfig {
    let c = data_dir(app)
        .map(|d| librerie::read_config(&d))
        .unwrap_or_default();
    LibConfig {
        dir: state.libs.root().to_string_lossy().into_owned(),
        default: c.cartella.is_none(),
        external: c.esterne,
    }
}

fn save_config(app: &AppHandle, f: impl FnOnce(&mut Config)) -> Result<(), String> {
    let d = data_dir(app).map_err(|e| e.detail)?;
    let mut c = librerie::read_config(&d);
    f(&mut c);
    librerie::write_config(&d, &c).map_err(|e| e.key.to_string())
}

#[tauri::command]
fn lib_get_config(app: AppHandle, state: tauri::State<'_, Arc<State>>) -> LibConfig {
    lib_config(&app, &state)
}

/// System dialog to choose the folder of the libraries; the path is kept in `config.json`. None if the user cancels.
#[tauri::command]
async fn lib_choose_dir(
    app: AppHandle,
    state: tauri::State<'_, Arc<State>>,
) -> Result<Option<String>, String> {
    let dlg = app.dialog().file();
    let picked = tauri::async_runtime::spawn_blocking(move || dlg.blocking_pick_folder())
        .await
        .map_err(|e| e.to_string())?;
    let Some(path) = picked.and_then(|f| f.into_path().ok()) else {
        return Ok(None);
    };
    let text = path.to_string_lossy().into_owned();
    save_config(&app, |c| c.cartella = Some(text.clone()))?;
    state.libs.set_root(path);
    Ok(Some(text))
}

/// The switch «Scarica librerie pubbliche da fonti esterne» (off until the user turns it on).
#[tauri::command]
fn lib_set_external(app: AppHandle, on: bool) -> Result<bool, String> {
    save_config(&app, |c| c.esterne = on)?;
    Ok(on)
}

/// Set by `lib_cancel`, read between the pieces of a download.
struct Cancel(Arc<AtomicBool>);

fn err_json(key: &str, detail: &str) -> String {
    serde_json::json!({"status": "error", "error_key": key, "detail": detail}).to_string()
}

/// The list of the public packages of the site (a network call, only when the user asks for it).
#[tauri::command]
async fn lib_catalog() -> String {
    tauri::async_runtime::spawn_blocking(|| match scarica::fetch_catalog_raw(&Http::new()) {
        Ok((_, raw)) => serde_json::json!({"status": "ok", "catalog": raw}).to_string(),
        Err(e) => e.to_json(),
    })
    .await
    .unwrap_or_else(|e| err_json("lib.dl.err.io", &e.to_string()))
}

fn do_download(app: &AppHandle, state: &State, cancel: &AtomicBool, ids: Vec<String>) -> String {
    cancel.store(false, Ordering::Relaxed);
    let src = Http::new();
    let cat = match scarica::fetch_catalog(&src) {
        Ok(c) => c,
        Err(e) => return e.to_json(),
    };
    let (mut done, mut failed) = (Vec::new(), Vec::new());
    for id in ids {
        let Some(pkg) = cat.librerie.iter().find(|p| p.id == id) else {
            failed.push(
                serde_json::json!({"id": id, "error_key": "lib.dl.err.catalog", "detail": id}),
            );
            continue;
        };
        match scarica::download_package(&state.libs, &src, pkg, cancel, &mut |p| {
            let _ = app.emit("lib-progress", p);
        }) {
            Ok(()) => done.push(id),
            Err(e) => {
                let stop = e.key == "lib.dl.err.cancelled";
                failed.push(serde_json::json!({"id": id, "error_key": e.key, "detail": e.detail}));
                if stop {
                    break;
                }
            }
        }
    }
    serde_json::json!({"status": "ok", "done": done, "failed": failed}).to_string()
}

/// Downloads and verifies the packages the user ticked; progress as `lib-progress` events.
#[tauri::command]
async fn lib_download(
    app: AppHandle,
    state: tauri::State<'_, Arc<State>>,
    cancel: tauri::State<'_, Cancel>,
    ids: Vec<String>,
) -> Result<String, String> {
    let (st, flag) = (state.inner().clone(), cancel.0.clone());
    tauri::async_runtime::spawn_blocking(move || do_download(&app, &st, &flag, ids))
        .await
        .map_err(|e| e.to_string())
}

/// An external public source (GNPS, MassBank): ONLY with the switch «Scarica librerie pubbliche da fonti esterne» on, only from the listed hosts.
#[tauri::command]
async fn lib_download_external(
    app: AppHandle,
    state: tauri::State<'_, Arc<State>>,
    cancel: tauri::State<'_, Cancel>,
    id: String,
) -> Result<String, String> {
    let (st, flag) = (state.inner().clone(), cancel.0.clone());
    tauri::async_runtime::spawn_blocking(move || {
        if !data_dir(&app)
            .map(|d| librerie::read_config(&d).esterne)
            .unwrap_or(false)
        {
            return err_json("lib.dl.err.externalOff", "");
        }
        flag.store(false, Ordering::Relaxed);
        let cat = match scarica::fetch_catalog(&Http::new()) {
            Ok(c) => c,
            Err(e) => return e.to_json(),
        };
        let Some(ext) = cat.esterne.iter().find(|e| e.id == id) else {
            return err_json("lib.dl.err.catalog", &id);
        };
        match scarica::download_external(&st.libs, ext, &Policy::default(), &flag, &mut |p| {
            let _ = app.emit("lib-progress", p);
        }) {
            Ok(sha) => serde_json::json!({"status": "ok", "sha256": sha}).to_string(),
            Err(e) => e.to_json(),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn lib_cancel(cancel: tauri::State<'_, Cancel>) {
    cancel.0.store(true, Ordering::Relaxed);
}

fn main() {
    let state = Arc::new(State::new());
    let for_protocol = state.clone();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state.clone())
        .manage(Cancel(Arc::new(AtomicBool::new(false))))
        .invoke_handler(tauri::generate_handler![
            pick_files,
            register_paths,
            save_mzml,
            update_check,
            update_apply,
            update_restart,
            lib_get_config,
            lib_choose_dir,
            lib_set_external,
            lib_catalog,
            lib_download,
            lib_download_external,
            lib_cancel
        ])
        .setup(|app| {
            // the indexes (.mzidx) built natively live in the cache folder of the app, where the system may clear them (they are rebuilt)
            if let Ok(cache) = app.path().app_cache_dir() {
                let _ = app
                    .state::<Arc<State>>()
                    .set_index_folder(cache.join("mzidx"));
            }
            // the list of the files, the notebook and the files the app makes itself: the data folder of the app
            if let Ok(d) = data_dir(app.handle()) {
                app.state::<Arc<State>>().set_data_folder(d.join("sessione"));
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
                let r = handle_body(
                    &state,
                    uri.path(),
                    uri.query().unwrap_or(""),
                    request.body(),
                );
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
