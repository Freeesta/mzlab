//! wasm-bindgen bridge of the table of levels of Attributio (`mzlab_feat::attributio::table`). The page gives the files of a campaign folder as
//! `{"root": name, "files": [{"path", "size"}]}` and `read(k, offset, length) -> Uint8Array` for the file `k` of that list (a `File` slice); only the files that
//! `campaign.toml` names are read. The table is written to `write(offset, bytes)` in order; the answer is a small JSON. Errors are JSON `{"error_key", "params"}`.
#![forbid(unsafe_code)]
use js_sys::{Function, Uint8Array};
use mzlab_core::error::Error;
use mzlab_core::json::{self, Out};
use mzlab_feat::attributio::io::{Entry, Io};
use mzlab_feat::attributio::table::{self, Run};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "mzfeat-attributio/1";

#[wasm_bindgen]
pub fn version() -> String {
    format!("{BRIDGE} feat={}", mzlab_feat::FEAT_VERSION)
}

/// Version of the crate `mzlab-feat` inside this `.wasm` (`x.y.z`): the worker asks it when it loads the module.
#[wasm_bindgen]
pub fn crate_version() -> String {
    mzlab_feat::CRATE_VERSION.to_string()
}

/// The files of the folder: path → (index in the list of the page, size).
pub struct Files {
    root: String,
    by_path: BTreeMap<String, (usize, u64)>,
}

impl Files {
    pub fn from_json(names_json: &str) -> Result<Files, Error> {
        let bad = || Error::new("error.badRequest");
        let v = json::parse(names_json.as_bytes()).map_err(|_| bad())?;
        let mut by_path = BTreeMap::new();
        for (k, f) in v.get("files").ok_or_else(bad)?.arr().iter().enumerate() {
            let path = f.get("path").and_then(|p| p.str()).ok_or_else(bad)?;
            let size = f.get("size").and_then(|s| s.f64()).ok_or_else(bad)?;
            by_path.insert(path.to_string(), (k, size as u64));
        }
        Ok(Files {
            root: v
                .get("root")
                .and_then(|r| r.str())
                .unwrap_or("")
                .to_string(),
            by_path,
        })
    }

    fn list(&self, dir: &str) -> Vec<Entry> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{}/", dir.trim_end_matches('/'))
        };
        let mut out: Vec<Entry> = Vec::new();
        for k in self.by_path.keys().filter(|k| k.starts_with(&prefix)) {
            let rest = &k[prefix.len()..];
            let (name, is_dir) = rest.find('/').map_or((rest, false), |i| (&rest[..i], true));
            if !out.iter().any(|e| e.name == name) {
                out.push(Entry {
                    name: name.to_string(),
                    is_dir,
                });
            }
        }
        out
    }
}

/// The table of levels of the folder; `fetch(k, offset, length)` gives the bytes of file `k` (`None`: unreadable).
pub fn run_with(
    files: &Files,
    fetch: &dyn Fn(usize, u64, u64) -> Option<Vec<u8>>,
) -> Result<Run, Error> {
    let read = |p: &str| {
        let &(k, size) = files.by_path.get(p)?;
        fetch(k, 0, size)
    };
    let list = |d: &str| files.list(d);
    let io = Io::new(&read, &list);
    table::run(&io, &files.root)
        .map_err(|e| Error::new("error.attributio").with("detail", e.to_string()))
}

/// The table in one of its renderings, in pieces of about 64 KiB written at increasing offsets of `write`.
pub fn render_to(run: &Run, kind: &str, write: &mut dyn FnMut(u64, &[u8]) -> bool) -> bool {
    let (mut buf, mut off, mut ok) = (String::new(), 0u64, true);
    let mut emit = |s: &str| {
        buf.push_str(s);
        if buf.len() >= 65536 && ok {
            ok = write(off, buf.as_bytes());
            off += buf.len() as u64;
            buf.clear();
        }
    };
    match kind {
        "csv" => run.csv_to(&mut emit),
        "xlsx" => run.xlsx_rows_to(&mut emit),
        _ => run.json_to(&mut emit),
    }
    if ok && !buf.is_empty() {
        ok = write(off, buf.as_bytes());
    }
    ok
}

fn js_err(e: Error) -> JsValue {
    let mut o = Out::default();
    o.raw("{\"error_key\":").str(&e.key).raw(",\"params\":{");
    for (i, (k, v)) in e.params.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    o.raw("}}");
    JsValue::from_str(&o.0)
}

fn js_fetch(read: &Function, k: usize, off: u64, len: u64) -> Option<Vec<u8>> {
    let v = read
        .call3(
            &JsValue::NULL,
            &(k as f64).into(),
            &(off as f64).into(),
            &(len as f64).into(),
        )
        .ok()?;
    Some(v.dyn_into::<Uint8Array>().ok()?.to_vec())
}

fn go(read: Function, names_json: &str, write: Function, kind: &str) -> Result<String, JsValue> {
    let files = Files::from_json(names_json).map_err(js_err)?;
    let run = run_with(&files, &|k, o, l| js_fetch(&read, k, o, l)).map_err(js_err)?;
    let mut put = |off: u64, b: &[u8]| {
        write
            .call2(&JsValue::NULL, &(off as f64).into(), &Uint8Array::from(b))
            .is_ok()
    };
    if !render_to(&run, kind, &mut put) {
        return Err(js_err(Error::new("error.io")));
    }
    Ok(run.summary())
}

/// The table as JSON `{"columns","rows"}` to `write`; answers `{n_features, by_level, fingerprint, settings, warnings}`.
#[wasm_bindgen]
pub fn annotate_to(read: Function, names_json: &str, write: Function) -> Result<String, JsValue> {
    go(read, names_json, write, "json")
}

/// The table as CSV (`;`, UTF-8) to `write`; answers the same summary.
#[wasm_bindgen]
pub fn table_csv(read: Function, names_json: &str, write: Function) -> Result<String, JsValue> {
    go(read, names_json, write, "csv")
}

/// The rows of the sheet `{"hojas":[{"nombre","filas"}]}` that the page zips into an XLSX, to `write`; answers the same summary.
#[wasm_bindgen]
pub fn table_xlsx_rows(
    read: Function,
    names_json: &str,
    write: Function,
) -> Result<String, JsValue> {
    go(read, names_json, write, "xlsx")
}
