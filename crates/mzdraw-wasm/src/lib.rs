//! wasm-bindgen bridge of the mzDraw editor: one editor per page, driven by pointer, wheel and key events as numbers; the page paints
//! the display list (`scene` + `strings`) and shows what `state` says. All decisions are in the crate `mzdraw`; `replay` is the same
//! function the native side runs, for the parity test. With the `mzlab` feature `state` carries the `check/1` warnings of the drawing
//! (`mzlab_feat::chem::check`, the bytes of `mzlab-cli mol check`). Errors are JSON `{"error_key", "params"}` strings.
use mzdraw::editor::{Editor, CHANGED};
use mzdraw::scene::Scene;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "mzdraw/1";

#[derive(Default)]
struct Page {
    editor: Editor,
    scene: Scene,
    /// `check/1` of the drawing as last computed, or empty.
    check: String,
}

thread_local! {
    static PAGE: RefCell<Page> = RefCell::new(Page::default());
}

fn with<T>(f: impl FnOnce(&mut Page) -> T) -> T {
    PAGE.with(|p| f(&mut p.borrow_mut()))
}

fn js_err(key: &str) -> JsValue {
    JsValue::from_str(&format!("{{\"error_key\":\"{key}\",\"params\":{{}}}}"))
}

/// Runs an editor action; after a change the warnings are recomputed.
fn act(f: impl FnOnce(&mut Editor) -> u32) -> u32 {
    with(|p| {
        let flags = f(&mut p.editor);
        if flags & CHANGED != 0 {
            recheck(p);
        }
        flags
    })
}

#[cfg(feature = "mzlab")]
fn recheck(p: &mut Page) {
    let doc = p.editor.doc();
    p.check.clear();
    let mut flagged = Vec::new();
    if !doc.atoms.is_empty() {
        let request = serde_json::json!({ "molfile": doc.to_molfile() }).to_string();
        if let Ok(j) = mzlab_feat::chem::check::check_json(&request) {
            let v: serde_json::Value = serde_json::from_str(&j).unwrap_or_default();
            for w in v["warnings"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|w| w["level"] == "warn")
            {
                for i in w["atoms"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|i| i.as_u64())
                {
                    // check/1 counts atoms from 0 in molfile order, which is the pool order
                    if let Some(a) = doc.atoms.get(i as usize) {
                        flagged.push(a.id);
                    }
                }
            }
            p.check = j;
        }
    }
    p.editor.set_warnings(flagged);
}

#[cfg(not(feature = "mzlab"))]
fn recheck(_p: &mut Page) {}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// The product name, from the one place that holds it.
#[wasm_bindgen]
pub fn name() -> String {
    mzdraw::NAME.to_string()
}

/// A pointer event: `phase` 0 down, 1 move, 2 up; `kind` 0 mouse, 1 pen, 2 finger; `mods` bit 1 Shift, bit 2 Ctrl/Cmd. Returns flags
/// (1 repaint, 2 changed, 4 tool changed, 8 handled).
#[wasm_bindgen]
pub fn pointer(phase: u32, x: f64, y: f64, kind: u32, mods: u32) -> u32 {
    act(|e| match phase {
        0 => e.pointer_down(x, y, kind, mods),
        1 => e.pointer_move(x, y, kind),
        _ => e.pointer_up(x, y),
    })
}

#[wasm_bindgen]
pub fn key(key: &str, mods: u32) -> u32 {
    act(|e| e.key(key, mods))
}

/// `what`: `undo`, `redo`, `clear`, `all` (select all), `delete` (the selection), or a tool name (`select`, `atom:N`, `bond:2`, ...).
#[wasm_bindgen]
pub fn command(what: &str) -> u32 {
    act(|e| match what {
        "undo" => e.undo(),
        "redo" => e.redo(),
        "clear" => e.clear(),
        "all" => e.select_all(),
        "delete" => e.delete_selection(),
        tool => e.set_tool(tool),
    })
}

#[wasm_bindgen]
pub fn zoom(x: f64, y: f64, factor: f64) -> u32 {
    act(|e| e.zoom(x, y, factor))
}

#[wasm_bindgen]
pub fn pan(dx: f64, dy: f64) -> u32 {
    act(|e| e.pan(dx, dy))
}

#[wasm_bindgen]
pub fn fit(w: f64, h: f64) -> u32 {
    act(|e| e.fit(w, h))
}

/// Adds a structure text (SMILES, molfile, `mzdraw/1`) to the drawing; returns `{"format", "placeholder"}`.
#[wasm_bindgen]
pub fn add(text: &str) -> Result<String, JsValue> {
    with(|p| {
        let info = p.editor.import(text).map_err(js_err)?;
        recheck(p);
        Ok(format!(
            "{{\"format\":\"{}\",\"placeholder\":{}}}",
            info.format, info.placeholder
        ))
    })
}

/// The drawing as `mzdraw` (the document), `molfile`, `svg` or `log` (the command log).
#[wasm_bindgen]
pub fn save(kind: &str) -> Result<String, JsValue> {
    with(|p| {
        let e = &p.editor;
        Ok(match kind {
            "mzdraw" => e.doc().to_json(),
            "molfile" => e.doc().to_molfile(),
            "svg" => mzdraw::svg::to_svg(e.doc()),
            "log" => e.session.log_json(),
            _ => return Err(js_err("error.draw2.request")),
        })
    })
}

/// The display list numbers (layout in `mzdraw::scene`); call `strings` after it for the texts.
#[wasm_bindgen]
pub fn scene() -> Vec<f32> {
    with(|p| {
        p.scene = p.editor.scene();
        p.scene.data.clone()
    })
}

#[wasm_bindgen]
pub fn strings() -> Vec<String> {
    with(|p| p.scene.strings.clone())
}

/// What the page shows around the canvas: `{tool, atoms, bonds, undo, redo, selected, say: {key, what}, error, check}` (`check` is
/// the `check/1` document or null).
#[wasm_bindgen]
pub fn state() -> String {
    with(|p| {
        let e = &p.editor;
        let check: serde_json::Value = serde_json::from_str(&p.check).unwrap_or_default();
        serde_json::json!({
            "tool": e.tool.name(),
            "atoms": e.doc().atoms.len(),
            "bonds": e.doc().bonds.len(),
            "undo": e.session.can_undo(),
            "redo": e.session.can_redo(),
            "selected": e.over.sel_atoms.len() + e.over.sel_bonds.len(),
            "say": { "key": e.said.0, "what": e.said.1 },
            "error": e.error,
            "check": check,
        })
        .to_string()
    })
}

/// Headless: `{"import", "log", "out"}` gives the output text, the same bytes as the native `mzdraw::replay::replay`.
#[wasm_bindgen]
pub fn replay(request: &str) -> Result<String, JsValue> {
    mzdraw::replay::replay(request).map_err(|e| {
        JsValue::from_str(&format!(
            "{{\"error_key\":\"{}\",\"params\":{{\"id\":{}}}}}",
            e.key, e.id
        ))
    })
}
