//! wasm-bindgen bridge of `file pack` / `file unpack` / `file check-pack` (`mzlab_core::pack`, `pack_read`, docs/agenti/file-pack.md). The page gives
//! callbacks that read and write its OPFS files; the parts, the manifest and the JSON documents are the command line's, byte for byte. A `.wasm` of its own,
//! fetched on the first use (`mzlab/web/rust-pack.js`). Errors are JSON `{"error_key", "params"}` strings, as in the engine.
#![forbid(unsafe_code)]
use std::io::{self, Read};

use js_sys::{Function, Reflect, Uint8Array};
use mzlab_core::pack::{self, Manifest, Options, PartSink};
use mzlab_core::pack_read::{self, FileSink, PartSource, PartState};
use mzlab_core::{zipstream, Error, Result};
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "pack/1";

fn error_json(e: &Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// The message of a panic goes to a global of the worker (`__mzlabPanic`) before the abort, as in the engine.
#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        let _ = Reflect::set(
            &js_sys::global(),
            &"__mzlabPanic".into(),
            &info.to_string().into(),
        );
    }));
}

fn input(e: impl ToString) -> Error {
    Error::new("err.run.input").with("detail", e)
}
fn output(e: impl ToString) -> Error {
    Error::new("err.run.output").with("detail", e)
}
fn what(v: &JsValue) -> String {
    v.as_string()
        .or_else(|| {
            Reflect::get(v, &"message".into())
                .ok()
                .and_then(|m| m.as_string())
        })
        .unwrap_or_else(|| "callback failed".into())
}

/// Bytes `read(index, offset, length)` gives, as a stream; an empty answer is the end.
struct JsRead {
    f: Function,
    i: usize,
    pos: u64,
}

impl Read for JsRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let v = self
            .f
            .call3(
                &JsValue::NULL,
                &(self.i as f64).into(),
                &(self.pos as f64).into(),
                &(buf.len() as f64).into(),
            )
            .map_err(|e| io::Error::other(what(&e)))?;
        let a: Uint8Array = v
            .dyn_into()
            .map_err(|_| io::Error::other("read did not give a Uint8Array"))?;
        let n = (a.length() as usize).min(buf.len());
        if let Some(dst) = buf.get_mut(..n) {
            a.subarray(0, n as u32).copy_to(dst);
        }
        self.pos += n as u64;
        Ok(n)
    }
}

struct JsParts {
    size: Function,
    read: Function,
}

impl PartSource for JsParts {
    fn size(&mut self, part: usize) -> Result<Option<u64>> {
        let v = self
            .size
            .call1(&JsValue::NULL, &(part as f64).into())
            .map_err(|e| input(what(&e)))?;
        Ok(v.as_f64().filter(|n| *n >= 0.0).map(|n| n as u64))
    }
    fn open(&mut self, part: usize, from: u64) -> Result<Box<dyn Read>> {
        Ok(Box::new(JsRead {
            f: self.read.clone(),
            i: part,
            pos: from,
        }))
    }
}

struct JsPartSink {
    put: Function,
}

impl PartSink for JsPartSink {
    fn put(&mut self, part: usize, bytes: &[u8]) -> Result<()> {
        self.put
            .call2(
                &JsValue::NULL,
                &(part as f64).into(),
                &Uint8Array::from(bytes),
            )
            .map(|_| ())
            .map_err(|e| output(what(&e)))
    }
}

/// `{exists(path) -> bool, begin(path), write(bytes), end(keep)}`: the page's way of writing the files.
struct JsFiles(JsValue);

impl JsFiles {
    fn call(&self, name: &str, args: &[JsValue]) -> Result<JsValue> {
        let f: Function = Reflect::get(&self.0, &name.into())
            .map_err(|e| output(what(&e)))?
            .dyn_into()
            .map_err(|_| output(format!("no {name} callback")))?;
        let a = js_sys::Array::new();
        for x in args {
            a.push(x);
        }
        f.apply(&self.0, &a).map_err(|e| output(what(&e)))
    }
}

impl FileSink for JsFiles {
    fn exists(&mut self, path: &str) -> Result<bool> {
        Ok(self.call("exists", &[path.into()])?.is_truthy())
    }
    fn begin(&mut self, path: &str) -> Result<()> {
        self.call("begin", &[path.into()]).map(|_| ())
    }
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.call("write", &[Uint8Array::from(bytes).into()])
            .map(|_| ())
    }
    fn end(&mut self, keep: bool) -> Result<()> {
        self.call("end", &[keep.into()]).map(|_| ())
    }
}

fn manifest_of(text: &str) -> std::result::Result<Manifest, JsValue> {
    pack::parse_manifest(text.as_bytes()).map_err(|e| error_json(&e))
}

/// `[[path, size], ...]` of the page, in the order its callbacks number them.
fn list_of(text: &str) -> std::result::Result<Vec<(String, u64)>, JsValue> {
    let bad = || {
        error_json(
            &Error::new("err.run.input")
                .with("detail", "files must be a JSON list of [path, size]"),
        )
    };
    let v = mzlab_core::json::parse(text.as_bytes()).map_err(|_| bad())?;
    v.arr()
        .iter()
        .map(|e| match e.arr() {
            [p, s] => Some((p.str()?.to_string(), s.f64().filter(|n| *n >= 0.0)? as u64)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(bad)
}

fn options(name: &str, part_size: f64) -> std::result::Result<Options, JsValue> {
    if !pack::safe_path(name) || name.contains('/') || part_size.is_nan() || part_size < 1024.0 {
        return Err(error_json(&Error::new("err.run.input").with(
            "detail",
            "name must be a plain file name and part_size at least 1024",
        )));
    }
    Ok(Options {
        name: name.to_string(),
        part_size: part_size as u64,
        zip64_at: zipstream::ZIP64_AT,
    })
}

/// The files in path order, as the command line sorts them, each with the index the page gave it.
fn sorted(list: Vec<(String, u64)>) -> (Vec<(String, u64)>, Vec<usize>) {
    let mut order: Vec<usize> = (0..list.len()).collect();
    order.sort_by(|a, b| list[*a].0.cmp(&list[*b].0));
    (order.iter().map(|i| list[*i].clone()).collect(), order)
}

/// `pack-run/1` of a dry run: the plan of `files` (`[[path, size], ...]`), nothing read; refused for the names a real run refuses.
#[wasm_bindgen]
pub fn pack_plan(files: &str, name: &str, part_size: f64) -> std::result::Result<String, JsValue> {
    let (list, _) = sorted(list_of(files)?);
    let m = pack::plan_checked(&list, &options(name, part_size)?).map_err(|e| error_json(&e))?;
    Ok(pack::run_json(&m, true))
}

/// Writes the parts of `files` (`[[path, size], ...]`; `read(i, offset, length)` gives bytes of file `i` of that list, empty at the end;
/// `put(part, bytes)` appends to part `part`) and returns the manifest text. The `pack-run/1` document of it is `pack_doc`.
#[wasm_bindgen]
pub fn pack(
    files: &str,
    name: &str,
    part_size: f64,
    read: Function,
    put: Function,
) -> std::result::Result<String, JsValue> {
    let (list, order) = sorted(list_of(files)?);
    let o = options(name, part_size)?;
    let mut open = |k: usize| -> Result<Box<dyn Read>> {
        Ok(Box::new(JsRead {
            f: read.clone(),
            i: order.get(k).copied().unwrap_or(0),
            pos: 0,
        }))
    };
    let m =
        pack::pack(&list, &mut open, &mut JsPartSink { put }, &o).map_err(|e| error_json(&e))?;
    Ok(pack::manifest_json(&m))
}

/// `pack-run/1` of a manifest text.
#[wasm_bindgen]
pub fn pack_doc(manifest: &str) -> std::result::Result<String, JsValue> {
    Ok(pack::run_json(&manifest_of(manifest)?, false))
}

/// `verify-pack/1`: `size(part)` is the size of a part or -1 when it is not there; `read(part, offset, length)` gives its bytes.
/// `seen(part, status)` (optional) is told as each part is decided.
#[wasm_bindgen]
pub fn verify_pack(
    manifest: &str,
    size: Function,
    read: Function,
    seen: Option<Function>,
) -> std::result::Result<String, JsValue> {
    let m = manifest_of(manifest)?;
    let mut tell = |i: usize, s: PartState| {
        if let Some(f) = &seen {
            let _ = f.call2(&JsValue::NULL, &(i as f64).into(), &s.key().into());
        }
    };
    let parts = pack_read::verify_parts(&m, &mut JsParts { size, read }, &mut tell)
        .map_err(|e| error_json(&e))?;
    Ok(pack_read::verify_json(&m, &parts))
}

/// `unpack-run/1`: rebuilds the files through `files` (`{exists, begin, write, end}`); without `force` an existing file is `err.pack.exists`;
/// `dry_run` writes nothing. Parts are given as for `verify_pack`.
#[wasm_bindgen]
pub fn unpack(
    manifest: &str,
    size: Function,
    read: Function,
    files: JsValue,
    force: bool,
    dry_run: bool,
    seen: Option<Function>,
) -> std::result::Result<String, JsValue> {
    let m = manifest_of(manifest)?;
    let mut tell = |i: usize, s: PartState| {
        if let Some(f) = &seen {
            let _ = f.call2(&JsValue::NULL, &(i as f64).into(), &s.key().into());
        }
    };
    let (parts, done) = pack_read::unpack(
        &m,
        &mut JsParts { size, read },
        &mut JsFiles(files),
        force,
        !dry_run,
        &mut tell,
    )
    .map_err(|e| error_json(&e))?;
    Ok(pack_read::unpack_json(&m, &parts, &done, dry_run))
}
