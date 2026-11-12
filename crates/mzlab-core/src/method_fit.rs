//! A `metodo/1` document cut to the room the readers of an mzML give it: every reader of the document (site, assistant, the mzML reader of the
//! readers module, infusion) reads at most `DOC_MAX` bytes after `<run`, so a longer document would be lost whole. A document that fits is never
//! touched; one that does not loses the last entries of `campi`, then the last texts of `grezzo`, each cut named by a `meta.troncato` warning.

use crate::json::{self, Out, Value};

/// The bytes after `<run` that every reader of the document reads.
pub const DOC_MAX: usize = 1 << 20;

/// What the cuts leave for the warnings they add.
const WARN_ROOM: usize = 1024;

/// The members of the document, in order.
type Top = [(String, Value)];

/// `doc` when `size(doc) <= room`, else the same document cut to fit (`size` = bytes as written, e.g. after XML escaping).
pub fn fit(doc: &str, room: usize, size: &dyn Fn(&str) -> usize) -> String {
    if size(doc) <= room {
        return doc.to_string();
    }
    let Ok(Value::Obj(mut top)) = json::parse(doc.as_bytes()) else {
        return String::new();
    };
    let text = |top: &Top| {
        let mut o = Out(String::new());
        json::write_value(&mut o, &Value::Obj(top.to_vec()));
        o.0
    };
    let fits = |top: &Top| size(&text(top)) + WARN_ROOM <= room;
    // a later step starts only when the document, warnings and all, is still too long
    let over = |top: &Top| size(&text(top)) > room;
    // `campi` first: they repeat what the file says, while `grezzo` is the text of the file itself
    for key in ["campi", "grezzo"] {
        if over(&top) && keep_head(&mut top, key, &fits) {
            warn(&mut top, key);
        }
    }
    if over(&top) {
        // what is left is too long by itself: the document keeps only where it comes from and why it is empty
        top.retain(|(k, _)| k == "schema" || k == "origine");
        warn(&mut top, "metodo/1");
    }
    text(&top)
}

/// Keeps the longest head of the list `key` that `fits` (binary search on the count); whether anything was cut.
fn keep_head(top: &mut Top, key: &str, fits: &dyn Fn(&Top) -> bool) -> bool {
    let Some(at) = top
        .iter()
        .position(|(k, v)| k == key && matches!(v, Value::Arr(_)))
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

/// A `meta.troncato` warning whose `dove` is the part that was cut.
fn warn(top: &mut Vec<(String, Value)>, part: &str) {
    let w = Value::Obj(vec![
        ("chiave".into(), Value::Str("meta.troncato".into())),
        ("dove".into(), Value::Str(part.into())),
    ]);
    match top.iter_mut().find(|(k, _)| k == "avvisi") {
        Some((_, Value::Arr(a))) => a.push(w),
        _ => top.push(("avvisi".into(), Value::Arr(vec![w]))),
    }
}

#[cfg(test)]
#[path = "method_fit_tests.rs"]
mod tests;
