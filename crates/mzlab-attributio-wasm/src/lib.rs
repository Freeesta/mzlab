//! wasm-bindgen bridge of the table of levels of Attributio (`mzlab_feat::attributio::table`). The page gives the files of a campaign folder as
//! `{"root": name, "files": [{"path", "size"}]}` and `read(k, offset, length) -> Uint8Array` for the file `k` of that list (a `File` slice); only the files that
//! `campaign.toml` names are read. The table is written to `write(offset, bytes)` in order; the answer is a small JSON. Errors are JSON `{"error_key", "params"}`.
#![forbid(unsafe_code)]
use js_sys::{Function, Uint8Array};
use mzlab_core::error::Error;
use mzlab_core::json::{self, Out};
#[cfg(any(feature = "rev", feature = "export"))]
use mzlab_feat::attributio::export::run::Options;
use mzlab_feat::attributio::io::{Entry, Io};
use mzlab_feat::attributio::review::banco::review_json_with;
use mzlab_feat::attributio::review::save::{decision_text, Saved};
use mzlab_feat::attributio::table::{self, Run};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with. The build with the feature `rev` (the Banco 2b) is the
/// `.wasm` `attributio-rev`: it exports the review, the decision and the evidence export; the build with `export` is the `.wasm` `attributio-export`: the four spreadsheet exports only; the one without them exports the table only.
#[cfg(not(any(feature = "rev", feature = "export", feature = "ms2")))]
const BRIDGE: &str = "mzfeat-attributio/1";
#[cfg(all(feature = "rev", not(feature = "all")))]
const BRIDGE: &str = "mzfeat-attributio-rev/1";
#[cfg(all(feature = "export", not(feature = "all")))]
const BRIDGE: &str = "mzfeat-attributio-export/1";
#[cfg(all(feature = "ms2", not(feature = "all")))]
const BRIDGE: &str = "mzfeat-attributio-ms2/1";
#[cfg(feature = "all")]
const BRIDGE: &str = "mzfeat-attributio-all/1";

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

/// Runs `f` on the folder (`fetch` as in `run_with`).
fn with_io<T>(
    files: &Files,
    fetch: &dyn Fn(usize, u64, u64) -> Option<Vec<u8>>,
    f: &dyn Fn(&Io) -> mzlab_feat::attributio::error::Result<T>,
) -> Result<T, Error> {
    let read = |p: &str| {
        let &(k, size) = files.by_path.get(p)?;
        fetch(k, 0, size)
    };
    let list = |d: &str| files.list(d);
    let io = Io::new(&read, &list);
    f(&io).map_err(|e| Error::new("error.attributio").with("detail", e.to_string()))
}

/// The review of one feature (`pol`, `id`) of the folder as JSON: proposal, proofs, checklist, alternatives and the decisions already taken.
pub fn review_with(
    files: &Files,
    fetch: &dyn Fn(usize, u64, u64) -> Option<Vec<u8>>,
    pol: &str,
    id: i64,
    classes: &str,
) -> Result<String, Error> {
    let table = (!classes.is_empty()).then_some(classes);
    with_io(files, fetch, &|io| {
        review_json_with(io, &files.root, pol, id, table)
    })
}

/// The decisions file after the chemist's decision (JSON of the Banco) is added to the folder's.
pub fn decision_with(
    files: &Files,
    fetch: &dyn Fn(usize, u64, u64) -> Option<Vec<u8>>,
    decision_json: &str,
) -> Result<Saved, Error> {
    with_io(files, fetch, &|io| {
        decision_text(io, &files.root, decision_json)
    })
}

/// The export `kind` of the folder as the JSON of its files. The evidence export is the `.wasm` `attributio-rev` (it needs the review); the others the `.wasm`
/// `attributio`. `names_json` may carry `"matrix"` and `"generated"` (the date of the export).
#[cfg(any(feature = "rev", feature = "export"))]
pub fn export_with(
    files: &Files,
    fetch: &dyn Fn(usize, u64, u64) -> Option<Vec<u8>>,
    names_json: &str,
    kind: &str,
) -> Result<String, Error> {
    let v = json::parse(names_json.as_bytes()).map_err(|_| Error::new("error.badRequest"))?;
    let text = |k: &str| v.get(k).and_then(|x| x.str()).map(str::to_string);
    let o = Options {
        matrix: text("matrix"),
        generated: text("generated").unwrap_or_default(),
    };
    with_io(files, fetch, &|io| export_kind(io, &files.root, kind, &o))
}

#[cfg(all(feature = "export", not(feature = "all")))]
fn export_kind(
    io: &Io,
    root: &str,
    kind: &str,
    o: &Options,
) -> mzlab_feat::attributio::error::Result<String> {
    mzlab_feat::attributio::export::run::run(io, root, kind, o)
}

#[cfg(all(feature = "rev", not(feature = "all")))]
fn export_kind(
    io: &Io,
    root: &str,
    kind: &str,
    _o: &Options,
) -> mzlab_feat::attributio::error::Result<String> {
    if kind != "evidence" {
        return Err(mzlab_feat::attributio::error::CampaignError::value(
            format!("unknown export: {kind}"),
        ));
    }
    mzlab_feat::attributio::review::evidence::evidence_json(io, root)
}

/// The merged `.wasm`: the evidence export needs the review, the others the cascade of the exports.
#[cfg(feature = "all")]
fn export_kind(
    io: &Io,
    root: &str,
    kind: &str,
    o: &Options,
) -> mzlab_feat::attributio::error::Result<String> {
    if kind == "evidence" {
        mzlab_feat::attributio::review::evidence::evidence_json(io, root)
    } else {
        mzlab_feat::attributio::export::run::run(io, root, kind, o)
    }
}

/// `text` written to `write` in pieces of about 64 KiB at increasing offsets.
pub fn text_to(text: &str, write: &mut dyn FnMut(u64, &[u8]) -> bool) -> bool {
    let mut off = 0usize;
    while off < text.len() {
        let mut end = (off + 65536).min(text.len());
        while !text.is_char_boundary(end) {
            end += 1;
        }
        if !write(off as u64, &text.as_bytes()[off..end]) {
            return false;
        }
        off = end;
    }
    true
}

/// The summary of a saved decision: `{path, added, replaced, unchanged}`.
pub fn saved_summary(s: &Saved) -> String {
    let mut o = Out::default();
    o.raw("{\"path\":").str(&s.path);
    o.raw(",\"added\":").int(s.added as i64);
    o.raw(",\"replaced\":").int(s.replaced as i64);
    o.raw(",\"unchanged\":").int(s.unchanged as i64);
    o.raw("}");
    o.0
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
    o.raw("{\"error_key\":").str(e.key).raw(",\"params\":{");
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

#[cfg(any(
    not(any(feature = "rev", feature = "export", feature = "ms2")),
    feature = "all"
))]
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
#[cfg(any(
    not(any(feature = "rev", feature = "export", feature = "ms2")),
    feature = "all"
))]
#[wasm_bindgen]
pub fn annotate_to(read: Function, names_json: &str, write: Function) -> Result<String, JsValue> {
    go(read, names_json, write, "json")
}

/// The table as CSV (`;`, UTF-8) to `write`; answers the same summary.
#[cfg(any(
    not(any(feature = "rev", feature = "export", feature = "ms2")),
    feature = "all"
))]
#[wasm_bindgen]
pub fn table_csv(read: Function, names_json: &str, write: Function) -> Result<String, JsValue> {
    go(read, names_json, write, "csv")
}

/// The rows of the sheet `{"hojas":[{"nombre","filas"}]}` that the page zips into an XLSX, to `write`; answers the same summary.
#[cfg(any(
    not(any(feature = "rev", feature = "export", feature = "ms2")),
    feature = "all"
))]
#[wasm_bindgen]
pub fn table_xlsx_rows(
    read: Function,
    names_json: &str,
    write: Function,
) -> Result<String, JsValue> {
    go(read, names_json, write, "xlsx")
}

#[cfg(feature = "rev")]
fn js_write(write: &Function) -> impl FnMut(u64, &[u8]) -> bool + '_ {
    move |off, b| {
        write
            .call2(&JsValue::NULL, &(off as f64).into(), &Uint8Array::from(b))
            .is_ok()
    }
}

/// The review of one feature as JSON to `write` (a feature at a time: the proofs are computed when the Banco opens it); `classes` is the text of the chemical-class table (empty: none); answers `{"bytes": n}`.
#[cfg(feature = "rev")]
#[wasm_bindgen]
pub fn review_to(
    read: Function,
    names_json: &str,
    feature_id: f64,
    pol: &str,
    classes: &str,
    write: Function,
) -> Result<String, JsValue> {
    let files = Files::from_json(names_json).map_err(js_err)?;
    let text = review_with(
        &files,
        &|k, o, l| js_fetch(&read, k, o, l),
        pol,
        feature_id as i64,
        classes,
    )
    .map_err(js_err)?;
    if !text_to(&text, &mut js_write(&write)) {
        return Err(js_err(Error::new("error.io")));
    }
    Ok(format!("{{\"bytes\":{}}}", text.len()))
}

/// The decisions file after the decision `decision_json` to `write` (CSV, UTF-8); answers `{path, added, replaced, unchanged}`.
#[cfg(feature = "rev")]
#[wasm_bindgen]
pub fn decision_to(
    read: Function,
    names_json: &str,
    decision_json: &str,
    write: Function,
) -> Result<String, JsValue> {
    let files = Files::from_json(names_json).map_err(js_err)?;
    let saved = decision_with(&files, &|k, o, l| js_fetch(&read, k, o, l), decision_json)
        .map_err(js_err)?;
    if !text_to(&saved.text, &mut js_write(&write)) {
        return Err(js_err(Error::new("error.io")));
    }
    Ok(saved_summary(&saved))
}

/// The export `kind` (`schymanski`, `publication`, `quantify`, `gradient`, `sirius` in the `.wasm` `attributio`; `evidence` in `attributio-rev`) as the JSON of its files
/// `{"files":[{"name","text"}|{"name","rows"}],"warnings":[]}` to `write`; answers `{"bytes": n}`.
#[cfg(any(feature = "rev", feature = "export"))]
#[wasm_bindgen]
pub fn export_to(
    read: Function,
    names_json: &str,
    kind: &str,
    write: Function,
) -> Result<String, JsValue> {
    let files = Files::from_json(names_json).map_err(js_err)?;
    let text = export_with(
        &files,
        &|k, o, l| js_fetch(&read, k, o, l),
        names_json,
        kind,
    )
    .map_err(js_err)?;
    let mut put = |off: u64, b: &[u8]| {
        write
            .call2(&JsValue::NULL, &(off as f64).into(), &Uint8Array::from(b))
            .is_ok()
    };
    if !text_to(&text, &mut put) {
        return Err(js_err(Error::new("error.io")));
    }
    Ok(format!("{{\"bytes\":{}}}", text.len()))
}

/// A request of the MS2 engine (`mzlab_feat::attributio::ms2_engine::run`): libraries and candidate lists are files of the list under `__ms2/`, the campaign is the rest.
/// The answer JSON goes to `write`; answers `{"bytes": n}`.
#[cfg(feature = "ms2")]
#[wasm_bindgen]
pub fn ms2_to(
    read: Function,
    names_json: &str,
    request_json: &str,
    write: Function,
) -> Result<String, JsValue> {
    let files = Files::from_json(names_json).map_err(js_err)?;
    let fetch = |k, o, l| js_fetch(&read, k, o, l);
    let extra = |p: &str| {
        let &(k, size) = files.by_path.get(p)?;
        fetch(k, 0, size)
    };
    let text = with_io(&files, &fetch, &|io| {
        mzlab_feat::attributio::ms2_engine::run::ms2_json(io, &files.root, &extra, request_json)
    })
    .map_err(js_err)?;
    let mut put = |off: u64, b: &[u8]| {
        write
            .call2(&JsValue::NULL, &(off as f64).into(), &Uint8Array::from(b))
            .is_ok()
    };
    if !text_to(&text, &mut put) {
        return Err(js_err(Error::new("error.io")));
    }
    Ok(format!("{{\"bytes\":{}}}", text.len()))
}
