//! Native file dialogs of the desktop app (only with the `app` feature): they open on the Desktop folder, then the OS remembers the last folder used.

use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

/// A file dialog starting on the Desktop folder (the plain dialog when the OS gives none).
pub fn at_desktop(app: &AppHandle) -> FileDialogBuilder<Wry> {
    let dlg = app.dialog().file();
    match app.path().desktop_dir() {
        Ok(dir) => dlg.set_directory(dir),
        Err(_) => dlg,
    }
}
