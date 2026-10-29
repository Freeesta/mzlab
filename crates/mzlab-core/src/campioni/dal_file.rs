//! What a file says about itself, put into `campioni/3` (`dal_file`): polarity, date and time of the acquisition, instrument model, method name,
//! range of m/z and of RT, number of scans (`DatiFile`, never edited by the user). The summary of a file comes from its scans (`riassunto_da_scansioni`,
//! the `.mzidx`) and from its `metodo/1`; the JSON binding takes the summary already made. Polarity and date replace what the names proposed, and
//! the replaced values are reported (`discordanze`).

use super::{tabella_da_value, to_json3, Campione, DatiFile, Tabella};
use crate::error::{Error, Result};
use crate::idxread::Scan;
use crate::json::{parse, Out, Value};

/// What is known of one file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RiassuntoFile {
    /// +1 or -1 when every scan has it; None when unknown or mixed.
    pub polarity: Option<i8>,
    pub rt: Option<(f64, f64)>,
    pub mz: Option<(f64, f64)>,
    pub scans: Option<u32>,
    pub acquired: Option<String>,
    pub instrument: String,
    pub method: String,
}

/// The `[low-high]` of a filter string (`FTMS + p ESI Full ms [100.0000-1000.0000]`).
fn filter_range(f: &str) -> Option<(f64, f64)> {
    let inner = f.split_once('[')?.1.split_once(']')?.0;
    let (a, b) = inner.split_once('-')?;
    let (a, b) = (a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?);
    (a.is_finite() && b.is_finite() && a <= b).then_some((a, b))
}

fn hull(a: Option<(f64, f64)>, b: (f64, f64)) -> (f64, f64) {
    a.map_or(b, |a| (a.0.min(b.0), a.1.max(b.1)))
}

/// The polarity, RT range, m/z range (from the filter strings) and number of the scans of an index.
pub fn riassunto_da_scansioni(scans: &[Scan]) -> RiassuntoFile {
    let mut r = RiassuntoFile::default();
    let mut pols: Vec<i8> = Vec::new();
    for s in scans {
        if s.polarity != 0 && !pols.contains(&s.polarity) {
            pols.push(s.polarity);
        }
        if s.rt.is_finite() {
            r.rt = Some(hull(r.rt, (s.rt, s.rt)));
        }
        if let Some(m) = filter_range(&s.filter) {
            r.mz = Some(hull(r.mz, m));
        }
    }
    if let [p] = pols[..] {
        r.polarity = Some(p.signum());
    }
    r.scans = (!scans.is_empty()).then_some(scans.len() as u32);
    r
}

/// An ISO date and time (`2024-05-03T14:22:01`, a space also accepted) as `YYYY-MM-DDTHH:MM[:SS]`; None for any other text.
fn iso(t: &str) -> Option<String> {
    let b: Vec<char> = t.trim().chars().collect();
    let digit = |i: usize| b.get(i).is_some_and(|c| c.is_ascii_digit());
    let shape = "dddd-dd-ddTdd:dd";
    let ok = shape.chars().enumerate().all(|(i, s)| match s {
        'd' => digit(i),
        'T' => matches!(b.get(i), Some('T') | Some(' ')),
        s => b.get(i) == Some(&s),
    });
    if !ok {
        return None;
    }
    let n = if b.get(16) == Some(&':') && digit(17) && digit(18) {
        19
    } else {
        16
    };
    let mut s: String = b[..n].iter().collect();
    s.replace_range(10..11, "T");
    Some(s)
}

fn text(v: &Value, path: &[&str]) -> Option<String> {
    let mut x = v;
    for k in path {
        x = x.get(k)?;
    }
    x.str().filter(|s| !s.is_empty()).map(str::to_string)
}

fn pair(v: &Value, lo: &str, hi: &str) -> Option<(f64, f64)> {
    v.get(lo)?
        .f64()
        .zip(v.get(hi)?.f64())
        .filter(|p| p.0 <= p.1)
}

/// The m/z range of the experiments of a `metodo/1`.
fn method_mz(m: &Value) -> Option<(f64, f64)> {
    let mut r = None;
    for p in m.get("ms")?.get("periodi")?.arr() {
        for e in p.get("esperimenti").map(|a| a.arr()).unwrap_or_default() {
            if let (Some(a), Some(b)) = (
                e.get("mz").and_then(|x| x.at(0)).and_then(|x| x.f64()),
                e.get("mz").and_then(|x| x.at(1)).and_then(|x| x.f64()),
            ) {
                r = Some(hull(r, (a.min(b), a.max(b))));
            }
        }
    }
    r
}

/// The summary of one entry of the binding: what it says outright wins over what its `metodo_json` (a `metodo/1`) gives.
fn summary(e: &Value) -> RiassuntoFile {
    let m = e.get("metodo_json");
    let pols: Vec<i8> = e
        .get("polarita")
        .map(|a| a.arr())
        .unwrap_or_default()
        .iter()
        .filter_map(|x| x.f64())
        .map(|x| x.signum() as i8)
        .filter(|p| *p != 0)
        .collect();
    let first = pols.first().copied();
    RiassuntoFile {
        polarity: first.filter(|f| pols.iter().all(|p| p == f)),
        rt: pair(e, "rt_min", "rt_max"),
        mz: pair(e, "mz_min", "mz_max").or_else(|| m.and_then(method_mz)),
        scans: e.get("scansioni").and_then(|x| x.f64()).map(|x| x as u32),
        acquired: text(e, &["acquisito"])
            .or_else(|| m.and_then(|m| text(m, &["origine", "file_creato"])))
            .and_then(|t| iso(&t)),
        instrument: text(e, &["modello"])
            .or_else(|| m.and_then(|m| text(m, &["strumento", "modello"])))
            .unwrap_or_default(),
        method: text(e, &["metodo"])
            .or_else(|| m.and_then(|m| text(m, &["campione", "metodo_file"])))
            .unwrap_or_default(),
    }
}

/// A change of a value the names had proposed.
struct Discordanza {
    index: usize,
    field: &'static str,
    before: String,
    after: String,
}

/// Puts what the file says into `c` (polarity and date replace the old ones; the rest goes to `file_data`); the replaced values are returned.
pub fn applica(
    c: &mut Campione,
    i: usize,
    r: &RiassuntoFile,
) -> Vec<(usize, &'static str, String, String)> {
    let mut out = Vec::new();
    let mut note = |field, before: Option<String>, after: &Option<String>| {
        if let (Some(b), Some(a)) = (before, after) {
            if b != *a {
                out.push((i, field, b, a.clone()));
            }
        }
    };
    if r.polarity.is_some() {
        note(
            "polarita",
            c.polarity.map(|p| p.to_string()),
            &r.polarity.map(|p| p.to_string()),
        );
        c.polarity = r.polarity;
    }
    if r.acquired.is_some() {
        note("acquisito", c.acquired.clone(), &r.acquired);
        c.acquired = r.acquired.clone();
    }
    c.file_data = Some(DatiFile {
        instrument: r.instrument.clone(),
        method: r.method.clone(),
        mz: r.mz,
        rt: r.rt,
        scans: r.scans,
    });
    out
}

fn find(t: &Tabella, e: &Value) -> Option<usize> {
    let sha = text(e, &["sha256"]);
    let name = text(e, &["nome"]);
    t.campioni.iter().position(|c| match (&sha, &name) {
        (Some(s), _) if !c.sha256.is_empty() => c.sha256 == *s,
        (_, Some(n)) => c.file == *n,
        _ => false,
    })
}

/// The binding: `{"campioni": <campioni/2 or /3>, "file":[{"nome"?,"sha256"?,"polarita":[..],"rt_min","rt_max","mz_min","mz_max","scansioni","acquisito"?,
/// "modello"?,"metodo"?,"metodo_json"?}]}` → `{"campioni":<campioni/3>,"discordanze":[{"indice","campo","prima","dopo"}],"senza_campione":[index of the entries]}`.
pub fn dal_file(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let mut t = tabella_da_value(
        v.get("campioni")
            .ok_or_else(|| Error::new("error.badCampioni"))?,
    )?;
    let mut disc: Vec<Discordanza> = Vec::new();
    let mut missing = Vec::new();
    for (n, e) in v
        .get("file")
        .map(|a| a.arr())
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let Some(i) = find(&t, e) else {
            missing.push(n);
            continue;
        };
        if t.campioni[i].sha256.is_empty() {
            t.campioni[i].sha256 = text(e, &["sha256"]).unwrap_or_default();
        }
        for (index, field, before, after) in applica(&mut t.campioni[i], i, &summary(e)) {
            disc.push(Discordanza {
                index,
                field,
                before,
                after,
            });
        }
    }
    let mut j = Out::default();
    j.raw("{\"campioni\":")
        .raw(&to_json3(&t))
        .raw(",\"discordanze\":[");
    for (k, d) in disc.iter().enumerate() {
        j.raw(if k > 0 { ",{\"campo\":" } else { "{\"campo\":" })
            .str(d.field);
        j.raw(",\"dopo\":")
            .str(&d.after)
            .raw(",\"indice\":")
            .int(d.index as i64);
        j.raw(",\"prima\":").str(&d.before).raw("}");
    }
    j.raw("],\"senza_campione\":[");
    for (k, n) in missing.iter().enumerate() {
        j.raw(if k > 0 { "," } else { "" }).int(*n as i64);
    }
    j.raw("]}");
    Ok(j.0)
}
