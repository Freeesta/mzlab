//! The `mzlab://localhost/<route>?<query>` protocol: pure function of (path, query), so that the window (`main.rs`) and the tests call
//! the same code. Answers are bytes: little-endian `f64` arrays (`tic`, `xic`, `spectra`), file chunks (`chunk`) or small JSON
//! (`open`, `raw2mzml`, errors `{"error_key", "params"}` with status 400).
//!
//! Routes (all GET): `open?slot&id`, `close?slot`, `reset`, `tic?slot&level&px`, `xic?slot&level&mz&tol&px`,
//! `spectra?slot&level&i0&i1&bin`, `lookup?name&size` (the number of a file known by name and size), `raw2mzml?id`
//! (a converted mzML is a file like the others), `file?id&off&len`, and the folder of the libraries under `lib/` (see `lib_route`).

use std::collections::HashMap;

use mzlab_core::Error;

use crate::engine::State;

pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

fn f64_bytes(v: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 8);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

fn json(status: u16, text: String) -> Response {
    Response {
        status,
        content_type: "application/json",
        body: text.into_bytes(),
    }
}

fn error(e: &Error) -> Response {
    let params: Vec<String> = e
        .params
        .iter()
        .map(|(k, v)| format!("\"{}\":{:?}", k, v))
        .collect();
    json(
        400,
        format!(
            "{{\"error_key\":\"{}\",\"params\":{{{}}}}}",
            e.key,
            params.join(",")
        ),
    )
}

fn query(q: &str) -> HashMap<&str, &str> {
    q.split('&').filter_map(|kv| kv.split_once('=')).collect()
}

/// Percent-decoding of a query value (file names come with `encodeURIComponent`).
fn decode(v: &str) -> String {
    let b = v.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(h) = u8::from_str_radix(&v[i + 1..i + 3], 16) {
                out.push(h);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn handle(state: &State, path: &str, q: &str) -> Response {
    handle_body(state, path, q, &[])
}

/// The same with the body of a POST (only `lib/put` has one).
pub fn handle_body(state: &State, path: &str, q: &str, body: &[u8]) -> Response {
    let q = query(q);
    let u = |k: &str| q.get(k).and_then(|v| v.parse::<u32>().ok());
    let n = |k: &str| q.get(k).and_then(|v| v.parse::<u64>().ok());
    let f = |k: &str| q.get(k).and_then(|v| v.parse::<f64>().ok());
    let bad = || error(&Error::new("err.file.raw").with("detail", "bad request"));
    let bin = |r: Result<Vec<f64>, Error>| match r {
        Ok(v) => Response {
            status: 200,
            content_type: "application/octet-stream",
            body: f64_bytes(&v),
        },
        Err(e) => error(&e),
    };
    match path.trim_start_matches('/') {
        "open" => match (u("slot"), u("id")) {
            (Some(slot), Some(id)) => match state.open(slot, id) {
                Ok(s) => json(200, s),
                Err(e) => error(&e),
            },
            _ => bad(),
        },
        "close" => match u("slot") {
            Some(slot) => {
                state.close(slot);
                json(200, "{}".into())
            }
            None => bad(),
        },
        "reset" => {
            state.reset();
            json(200, "{}".into())
        }
        "tic" => match (u("slot"), u("level")) {
            (Some(slot), Some(level)) => bin(state.tic(slot, level as u8, u("px").unwrap_or(0))),
            _ => bad(),
        },
        "xic" => match (u("slot"), u("level"), f("mz"), f("tol")) {
            (Some(slot), Some(level), Some(mz), Some(tol)) => {
                bin(state.xic(slot, level as u8, mz, tol, u("px").unwrap_or(0)))
            }
            _ => bad(),
        },
        "spectra" => match (u("slot"), u("level"), u("i0"), u("i1")) {
            (Some(slot), Some(level), Some(i0), Some(i1)) => {
                bin(state.spectra(slot, level as u8, i0, i1, f("bin").unwrap_or(0.1)))
            }
            _ => bad(),
        },
        "raw2mzml" => match u("id") {
            Some(id) => match state.raw_to_mzml(id) {
                Ok((tid, size)) => json(200, format!("{{\"id\":{tid},\"size\":{size}}}")),
                Err(e) => error(&e),
            },
            None => bad(),
        },
        "rawprogress" => match u("id").and_then(|id| state.raw_progress(id)) {
            Some((total, read, written)) => json(
                200,
                format!("{{\"total\":{total},\"read\":{read},\"written\":{written}}}"),
            ),
            None => json(200, "{}".into()),
        },
        "lookup" => match (q.get("name"), n("size")) {
            (Some(name), Some(size)) => match state.find(&decode(name), size) {
                Some(id) => json(200, format!("{{\"id\":{id}}}")),
                None => error(&Error::new("err.file.raw").with("detail", "unknown file")),
            },
            _ => bad(),
        },
        "file" => match (u("id"), n("off"), n("len")) {
            (Some(id), Some(off), Some(len)) => match state.chunk(id, off, len) {
                Ok(body) => Response {
                    status: 200,
                    content_type: "application/octet-stream",
                    body,
                },
                Err(e) => error(&e),
            },
            _ => bad(),
        },
        r if r.starts_with("lib/") => lib_route(state, &r[4..], &q, body),
        _ => json(404, "{}".into()),
    }
}

/// The routes of the folder of the libraries: `lib/dir`, `lib/list?sub`, `lib/stat?name`, `lib/read?name&off&len`, `lib/put?name&off&total` (POST),
/// `lib/remove?name`, `lib/rename?from&to`. Names are relative to the folder (`librerie.rs` refuses anything else).
fn lib_route(state: &State, route: &str, q: &HashMap<&str, &str>, body: &[u8]) -> Response {
    let name = q.get("name").map(|v| decode(v));
    let n = |k: &str| q.get(k).and_then(|v| v.parse::<u64>().ok());
    let bad = || error(&Error::new("lib.err.io").with("detail", "bad request"));
    let ok = |r: crate::librerie::Result<()>| match r {
        Ok(()) => json(200, "{}".into()),
        Err(e) => error(&e),
    };
    let lib = &state.libs;
    match route {
        "dir" => json(
            200,
            serde_json::json!({ "dir": lib.root().to_string_lossy() }).to_string(),
        ),
        "list" => match lib.list(&q.get("sub").map(|v| decode(v)).unwrap_or_default()) {
            Ok(files) => json(200, serde_json::json!({ "files": files }).to_string()),
            Err(e) => error(&e),
        },
        "stat" => match name.as_deref().map(|r| lib.stat(r)) {
            Some(Ok(e)) => json(200, serde_json::to_string(&e).unwrap_or_default()),
            Some(Err(e)) => error(&e),
            None => bad(),
        },
        "read" => match (name.as_deref(), n("off"), n("len")) {
            (Some(r), Some(off), Some(len)) => match lib.read(r, off, len) {
                Ok(body) => Response {
                    status: 200,
                    content_type: "application/octet-stream",
                    body,
                },
                Err(e) => error(&e),
            },
            _ => bad(),
        },
        "put" => match (name.as_deref(), n("off"), n("total")) {
            (Some(r), Some(off), Some(total)) => ok(lib.put(r, off, total, body).map(|_| ())),
            _ => bad(),
        },
        "remove" => match name.as_deref() {
            Some(r) => ok(lib.remove(r)),
            None => bad(),
        },
        "rename" => match (q.get("from"), q.get("to")) {
            (Some(a), Some(b)) => ok(lib.rename(&decode(a), &decode(b))),
            _ => bad(),
        },
        _ => json(404, "{}".into()),
    }
}
