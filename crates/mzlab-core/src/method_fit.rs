//! A `metodo/1` document cut to the room the readers of an mzML give it: every reader of the document (site, assistant, the mzML reader of the
//! readers module, infusion) reads at most `DOC_MAX` bytes after `<run`, so a longer document would be lost whole. A document that fits is never
//! touched; one that does not loses the end of `campi`, then the end of its `grezzo` texts (largest first), each cut named by a `meta.troncato` warning.

use crate::json::{self, Out, Value};

/// The bytes after `<run` that every reader of the document reads.
pub const DOC_MAX: usize = 1 << 20;

const WARN_ROOM: usize = 1024;

/// `doc` when `size(doc) <= room`, else the same document cut to fit (`size` = bytes as written, e.g. after XML escaping).
pub fn fit(doc: &str, room: usize, size: &dyn Fn(&str) -> usize) -> String {
    if size(doc) <= room {
        return doc.to_string();
    }
    let Ok(Value::Obj(mut top)) = json::parse(doc.as_bytes()) else {
        return String::new();
    };
    let text = |top: &[(String, Value)]| {
        let mut o = Out(String::new());
        json::write_value(&mut o, &Value::Obj(top.to_vec()));
        o.0
    };
    // the cuts leave room for the warnings they add (two or three short objects)
    let fits = |top: &[(String, Value)]| size(&text(top)) + WARN_ROOM <= room;
    // `campi` first: they repeat what the file says, while `grezzo` is the text of the file itself
    if cut_fields(&mut top, &fits) {
        warn(&mut top, "campi");
    }
    let over = |top: &[(String, Value)]| size(&text(top)) > room;
    if over(&top) && cut_raw_texts(&mut top, &fits) {
        warn(&mut top, "grezzo");
    }
    if over(&top) {
        // what is left is too long by itself: the document keeps only where it comes from and why it is empty
        top.retain(|(k, _)| k == "schema" || k == "origine");
        warn(&mut top, "documento");
    }
    text(&top)
}

fn section<'a>(top: &'a mut [(String, Value)], key: &str) -> Option<&'a mut Vec<Value>> {
    top.iter_mut()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            Value::Arr(a) => Some(a),
            _ => None,
        })
}

/// Cuts the `testo` of `grezzo`, the longest first, until `fits`; whether anything was cut.
fn cut_raw_texts(
    top: &mut Vec<(String, Value)>,
    fits: &dyn Fn(&[(String, Value)]) -> bool,
) -> bool {
    let mut cut = false;
    while !fits(top) {
        let Some(texts) = section(top, "grezzo") else {
            break;
        };
        let longest = texts
            .iter_mut()
            .filter_map(|t| match t {
                Value::Obj(f) => f.iter_mut().find(|(k, _)| k == "testo").map(|(_, v)| v),
                _ => None,
            })
            .filter_map(|v| match v {
                Value::Str(s) if !s.is_empty() => Some(s),
                _ => None,
            })
            .max_by_key(|s| s.len());
        let Some(s) = longest else { break };
        // half of the longest text at a time: a few passes even for megabytes
        let mut end = s.len() / 2;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
        cut = true;
    }
    cut
}

/// Keeps the longest head of `campi` that `fits` (binary search on the count); whether anything was cut.
fn cut_fields(top: &mut Vec<(String, Value)>, fits: &dyn Fn(&[(String, Value)]) -> bool) -> bool {
    let Some(at) = top
        .iter()
        .position(|(k, v)| k == "campi" && matches!(v, Value::Arr(_)))
    else {
        return false;
    };
    let Value::Arr(all) = std::mem::replace(&mut top[at].1, Value::Arr(Vec::new())) else {
        return false;
    };
    let (mut lo, mut hi) = (0, all.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        top[at].1 = Value::Arr(all[..mid].to_vec());
        if fits(top) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    top[at].1 = Value::Arr(all[..lo].to_vec());
    lo < all.len()
}

fn warn(top: &mut Vec<(String, Value)>, where_: &str) {
    let w = Value::Obj(vec![
        ("chiave".into(), Value::Str("meta.troncato".into())),
        (
            "dove".into(),
            Value::Str(format!(
                "metodo/1 {where_}: cut to the {DOC_MAX} bytes the readers of the mzML read"
            )),
        ),
    ]);
    match section(top, "avvisi") {
        Some(a) => a.push(w),
        None => top.push(("avvisi".into(), Value::Arr(vec![w]))),
    }
}

#[cfg(test)]
#[path = "method_fit_tests.rs"]
mod tests;
