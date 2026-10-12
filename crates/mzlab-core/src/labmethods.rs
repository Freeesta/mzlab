//! The methods (`.dam`) a student uploaded, as `App.methods`, `App.lab_methods`, `App.method` and `App.method_warnings` of `mzlab/app.py` use them.
//! Nothing is assumed when none is loaded.

use crate::error::{Error, Result};
use crate::json::{self, fmt_g, Out, Value};
use crate::methodcheck::{check_against, method_kind, write_rows};
use crate::methodinfo::{read_methods, Kind, Method};

impl Lab {
    /// Reads a `.dam`; a broken file never breaks the page (the reason is kept).
    pub fn read(name: &str, data: Vec<u8>) -> Lab {
        let res = match read_methods(data, name) {
            Ok(mut v) if !v.is_empty() => {
                let mut m = v.swap_remove(0);
                m.name = name.to_string();
                Ok(m)
            }
            Ok(_) => Err("no method in the file".to_string()),
            Err(e) => Err(why(&e)),
        };
        Lab { name: name.to_string(), res }
    }

    /// The method for `method_kind`: an unreadable file only has its name.
    fn as_method(&self) -> Method {
        match &self.res {
            Ok(m) => Method { name: self.name.clone(), ..m.clone() },
            Err(_) => Method { name: self.name.clone(), ..Default::default() },
        }
    }

    /// The row of `App.methods`: `{name, size, info}`.
    pub fn write_listed(&self, size: u64, o: &mut Out) {
        o.raw("{\"name\":").str(&self.name).raw(",\"size\":").int(size as i64).raw(",\"info\":");
        self.summary(o);
        o.raw("}");
    }

    /// One line for the list of methods, as a message for the page (`""` when there is nothing to say).
    fn summary(&self, o: &mut Out) {
        let msg = |o: &mut Out, key: &str, params: &[(&str, String)], ints: bool| {
            o.raw("{\"key\":").str(key).raw(",\"params\":{");
            for (i, (k, v)) in params.iter().enumerate() {
                o.raw(if i > 0 { "," } else { "" }).str(k).raw(":");
                if ints {
                    o.raw(v);
                } else {
                    o.str(v);
                }
            }
            o.raw("}}");
        };
        let Ok(m) = &self.res else { return msg(o, "load.method.unreadable", &[], false) };
        let ex = &m.experiments;
        if ex.is_empty() {
            o.raw("\"\"");
        } else if ex.iter().all(|e| matches!(e.kind, Kind::Mrm(_))) {
            let n: usize = ex.iter().map(|e| if let Kind::Mrm(t) = &e.kind { t.len() } else { 0 }).sum();
            msg(o, "load.method.mrm", &[("n", n.to_string())], true);
        } else if ex.len() != 1 {
            msg(o, "load.method.scans", &[("n", ex.len().to_string())], true);
        } else if let Kind::Scan { range: Some(r), .. } = &ex[0].kind {
            msg(o, "load.method.scanRange", &[("lo", fmt_g(r[0])), ("hi", fmt_g(r[1]))], false);
        } else {
            msg(o, "load.method.scan", &[], false);
        }
    }

    /// The method with the rows of its check against `data` (`Item.method()`) added.
    fn write_checked(&self, data: &Value, o: &mut Out) {
        match &self.res {
            Ok(m) => {
                let mut t = Out::default();
                m.write_json(&mut t);
                t.0.pop();
                o.raw(&t.0).raw(",\"check\":");
                write_rows(&check_against(data, m), o);
                o.raw("}");
            }
            Err(msg) => {
                o.raw("{\"name\":").str(&self.name).raw(",\"error\":").str(&format!("cannot read this file ({msg})"));
                o.raw(",\"error_key\":\"err.dam.unreadable\",\"params\":{\"message\":").str(msg).raw("},\"source\":[],\"compound\":[],\"file\":{},\"check\":[]}");
            }
        }
    }
}

/// One uploaded method: what `read_methods` found, or why the file cannot be read.
#[derive(Clone)]
pub struct Lab {
    pub name: String,
    pub res: std::result::Result<Method, String>,
}

fn why(e: &Error) -> String {
    if let (true, Some((_, n))) = (e.key == "err.file.notDam", e.params.first()) {
        return format!("{n} is not an OLE2 file");
    }
    e.params.iter().find(|(k, _)| *k == "message").map_or_else(|| e.to_string(), |(_, v)| v.clone())
}

/// `Path(name).stem.lower()`.
fn stem(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    match base.rfind('.') {
        Some(i) if i > 0 && i + 1 < base.len() => base[..i].to_lowercase(),
        _ => base.to_lowercase(),
    }
}

fn pick_index(labs: &[Lab], rows: &[Vec<&'static str>], kind: &str, file: &str) -> Option<usize> {
    let names: Vec<String> = labs.iter().map(|l| l.name.to_lowercase()).collect();
    let want = if ["full", "ms2", "mrm"].contains(&kind) { kind } else { "" };
    let mut hit: Vec<usize> = (0..names.len()).filter(|&i| !want.is_empty() && names[i].contains(want)).collect();
    if hit.is_empty() && kind == "ms2" {
        hit = (0..names.len()).filter(|&i| names[i].contains("ms2")).collect();
    }
    let mut pick = hit.last().copied().or(names.len().checked_sub(1));
    if labs.len() > 1 {
        let diffs: Vec<usize> = rows.iter().map(|r| r.iter().filter(|s| **s == "diff").count()).collect();
        let best = diffs.iter().copied().min().unwrap_or(0);
        if pick.is_some_and(|p| diffs[p] > best) {
            pick = (0..diffs.len()).filter(|&i| diffs[i] == best).last();
        }
    }
    let s = stem(file.split('#').next().unwrap_or(file));
    labs.iter().rposition(|l| stem(&l.name) == s).or(pick)
}

fn statuses(v: &Value, labs: &[Lab]) -> Vec<Vec<&'static str>> {
    labs.iter()
        .map(|l| match &l.res {
            Ok(m) => check_against(v, m).iter().map(|r| r.status).collect(),
            Err(_) => vec![],
        })
        .collect()
}

/// The method preselected for a file (`lab_pick` of `App.method`); `data` is the JSON of `Item.method()`.
pub fn pick(data: &str, labs: &[Lab], file: &str) -> Result<Option<usize>> {
    let v = json::parse(data.as_bytes())?;
    Ok(pick_index(labs, &statuses(&v, labs), v.get("kind").and_then(Value::str).unwrap_or(""), file))
}

/// `/api/method`: the JSON of `Item.method()` (`data`) with the loaded methods, each with its check, and the one preselected for this file.
pub fn method_json(data: &str, labs: &[Lab], file: &str) -> Result<String> {
    let v = json::parse(data.as_bytes())?;
    let mut o = Out::default();
    o.raw(data.strip_suffix('}').unwrap_or(data)).raw(",\"lab\":[");
    for (i, l) in labs.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        l.write_checked(&v, &mut o);
    }
    o.raw("],\"lab_pick\":");
    match pick_index(labs, &statuses(&v, labs), v.get("kind").and_then(Value::str).unwrap_or(""), file) {
        Some(p) => o.int(p as i64),
        None => o.raw("null"),
    };
    o.raw("}");
    Ok(o.0)
}

/// `/api/method_warnings`: the loaded methods whose experiment type is not among the types of the open files (`have`, sorted).
pub fn warnings_json(labs: &[Lab], have: &[String]) -> String {
    let name = |k: &str| match k {
        "full" => Some("Full Scan"),
        "ms2" => Some("MS2"),
        "mrm" => Some("MRM"),
        _ => None,
    };
    let mut o = Out::default();
    o.raw("{\"warnings\":[");
    let mut first = true;
    for l in labs {
        let Some(g) = method_kind(&l.as_method()).filter(|g| !have.iter().any(|h| h == g)) else { continue };
        o.raw(if first { "" } else { "," }).raw("{\"method\":").str(&l.name).raw(",\"expects\":").str(name(g).unwrap_or(g)).raw(",\"files\":[");
        for (i, h) in have.iter().filter_map(|h| name(h)).enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(h);
        }
        o.raw("]}");
        first = false;
    }
    o.raw("]}");
    o.0
}

/// The pump program of the method at `at` in `labs`.
pub fn lc_of(labs: &[Lab], at: Option<usize>) -> Option<&crate::methodinfo::Lc> {
    match &labs.get(at?)?.res {
        Ok(m) => m.lc.as_ref(),
        Err(_) => None,
    }
}
