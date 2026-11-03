//! `ispezione/1`: what the assistant is told about each open file and about the series, assembled from answers the engine already gives (`Item.info()`,
//! `metodo/1`, the estimate of `features.estimate`, `campioni/2`, TIC and BPC, the scan statistics of `scan_stats`). This module reads no file and
//! computes no spectrum: it names the kind of acquisition, the resolution class, the polarity and the cycle from facts, never from a file name, and it
//! lets through only the keys listed here (a whitelist, so a field added to a source later cannot leak). Used by `mzlab-cli llm` and the `llm` module.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::sortc;

use crate::context::{sig4, tic};
use crate::inspect_facts::{
    class_of, estimate_fields, gradient, mode_of, mrm_section, polarity_of, put_instrument,
    put_shape, signals, strs,
};
use crate::inspect_qc::{self, QcFile};
pub use crate::inspect_scan::scan_stats;
use crate::tools::value;

/// The longest string that goes out, in characters.
pub const MAX_STR: usize = 64;
/// The most points of a gradient.
pub(crate) const GRADIENT_POINTS: usize = 30;

/// A string as the assistant may see it: control and direction characters removed, runs of six digits or more masked (dates, serials), at most `MAX_STR`.
pub(crate) fn clean(s: &str) -> String {
    let kept: Vec<char> = s
        .chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{061C}' | '\u{FEFF}')
        })
        .collect();
    let mut out = String::new();
    let mut i = 0;
    while i < kept.len() {
        if kept[i].is_numeric() {
            let mut j = i;
            while j < kept.len() && kept[j].is_numeric() {
                j += 1;
            }
            if j - i >= 6 {
                out.extend(std::iter::repeat_n('#', j - i));
            } else {
                out.extend(&kept[i..j]);
            }
            i = j;
        } else {
            out.push(kept[i]);
            i += 1;
        }
    }
    out.chars().take(MAX_STR).collect()
}

pub(crate) fn path<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().try_fold(v, |v, k| v.get(k))
}

pub(crate) fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::f64).filter(|x| x.is_finite())
}

pub(crate) fn text(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::str)
        .map(clean)
        .filter(|s| !s.trim().is_empty())
}

pub(crate) fn n(x: f64) -> Value {
    Value::Num(sig4(x))
}

pub(crate) fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    sortc::by(v, f64::total_cmp);
    let k = v.len() / 2;
    Some(if v.len() % 2 == 1 {
        v[k]
    } else {
        (v[k - 1] + v[k]) / 2.0
    })
}

pub(crate) struct Obj(pub(crate) Vec<(String, Value)>);

impl Obj {
    pub(crate) fn put(&mut self, k: &str, v: Option<Value>) {
        if let Some(v) = v {
            self.0.push((k.to_string(), v));
        }
    }
}

/// What the rest of the series needs of one file.
pub(crate) struct Facts {
    pub qc: QcFile,
    pub class: String,
    pub polarity: String,
}

fn file_name(src: &Value) -> Option<String> {
    let raw = src.get("name")?.str()?;
    text(Some(&Value::Str(
        raw.rsplit(['/', '\\']).next().unwrap_or(raw).to_string(),
    )))
}

fn unreadable(alias: &str, name: Option<String>, status: &str, key: Option<String>) -> Value {
    let mut o = Obj(vec![("alias".into(), Value::Str(alias.into()))]);
    o.put("name", name.map(Value::Str));
    o.put("status", Some(Value::Str(status.into())));
    o.put("error_key", key.map(Value::Str));
    Value::Obj(o.0)
}

type Class = Option<(bool, Vec<String>, Option<f64>)>;

/// Resolution class of the second stage when it differs, the evidence for the first, and the resolving power of each stage.
fn put_class(
    o: &mut Obj,
    c1: &Class,
    c2: &Class,
    primary: Option<&(bool, Vec<String>, Option<f64>)>,
) {
    let label = |hr: bool| if hr { "high" } else { "unit" };
    if let (Some(a), Some(b)) = (c1, c2)
        && a.0 != b.0
    {
        o.put("class_ms2", Some(Value::Str(label(b.0).into())));
    }
    o.put("class_basis", primary.map(|c| strs(c.1.clone())));
    let rp: Vec<(&str, Value)> = [("ms1", c1), ("ms2", c2)]
        .into_iter()
        .filter_map(|(k, c)| Some((k, n(c.as_ref()?.2?))))
        .collect();
    if !rp.is_empty() {
        o.put(
            "resolving_power",
            Some(Value::Obj(
                rp.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            )),
        );
    }
}

/// Role, its source and the injection order from the sample table (else the rank by date); returns the role and the order.
fn put_role(
    o: &mut Obj,
    sample: &Value,
    rank: Option<usize>,
    confirmed: bool,
) -> (Option<String>, Option<f64>) {
    let role = text(sample.get("tipo"));
    let order = num(sample.get("ordine_iniezione")).or(rank.map(|r| r as f64));
    o.put("role", role.clone().map(Value::Str));
    if role.is_some() {
        o.put(
            "role_source",
            Some(Value::Str(
                if confirmed { "confirmed" } else { "proposed" }.into(),
            )),
        );
    }
    o.put("order", order.map(Value::Num));
    (role, order)
}

/// One file of the series: the object that goes out and what the series section needs of it.
fn file(
    src: &Value,
    k: usize,
    hide: bool,
    rank: Option<usize>,
    confirmed: bool,
) -> (Value, Option<Facts>) {
    let alias = format!("f{}", k + 1);
    let name = if hide { None } else { file_name(src) };
    if let Some(e) = src.get("error") {
        let key = text(e.get("error_key").or(Some(e)).filter(|v| v.str().is_some()));
        return (unreadable(&alias, name, "unreadable", key), None);
    }
    let items = src.get("items").map_or(&[][..], Value::arr);
    if items.is_empty() {
        return (unreadable(&alias, name, "no_data", None), None);
    }
    let null = Value::Null;
    let method = src.get("method").unwrap_or(&null);
    let stats = src.get("scan_stats").unwrap_or(&null);
    let est = src.get("estimate").unwrap_or(&null);
    let sample = src.get("sample").unwrap_or(&null);
    let polarity = polarity_of(items);
    let c1 = class_of(items, false);
    let c2 = class_of(items, true);
    let label = |hr: bool| if hr { "high" } else { "unit" };
    let primary = c1.as_ref().or(c2.as_ref());
    let class = primary.map_or("unknown", |c| label(c.0)).to_string();
    let mut o = Obj(vec![("alias".into(), Value::Str(alias.clone()))]);
    o.put("name", name.map(Value::Str));
    o.put("status", Some(Value::Str("ok".into())));
    put_instrument(&mut o, method, items);
    o.put("class", Some(Value::Str(class.clone())));
    put_class(&mut o, &c1, &c2, primary);
    o.put("polarity", Some(Value::Str(polarity.clone())));
    o.put("mode", Some(Value::Str(mode_of(items).into())));
    put_shape(&mut o, items, stats);
    o.put("mrm", mrm_section(src, items));
    for key in ["tic", "bpc"] {
        o.put(key, src.get(key).map(tic));
    }
    o.put("gradient", gradient(method));
    o.put(
        "flow_uL_min",
        num(path(method, &["lc", "flusso_mL_min"])).map(|f| n(f * 1000.0)),
    );
    o.put("oven_C", num(path(method, &["lc", "forno_C"])).map(n));
    estimate_fields(&mut o, est, class == "unit");
    let (role, order) = put_role(&mut o, sample, rank, confirmed);
    let mut warn: Vec<String> = Vec::new();
    if polarity == "alternating" {
        warn.push("alternating_polarity".into());
    }
    if class == "high" && mode_of(items) == "profile" {
        warn.push("profile_data_centroid_first".into());
    }
    o.put("warnings", Some(strs(warn)));
    let facts = Facts {
        qc: QcFile {
            alias,
            role: role.unwrap_or_default(),
            order,
            signals: signals(src),
            fwhm: num(est.get("fwhm_min")),
        },
        class,
        polarity,
    };
    (Value::Obj(o.0), Some(facts))
}

/// The injection rank of each file from the date the method says (`origine.file_creato`), when the sample table does not say: 1 for the earliest.
fn ranks(files: &[Value]) -> Vec<Option<usize>> {
    let dates: Vec<Option<&str>> = files
        .iter()
        .map(|f| path(f, &["method", "origine", "file_creato"]).and_then(Value::str))
        .collect();
    dates
        .iter()
        .map(|d| d.map(|d| dates.iter().flatten().filter(|o| **o < d).count() + 1))
        .collect()
}

fn series(facts: &[Option<Facts>], total: usize) -> Value {
    let mut o = Obj(vec![("files".into(), Value::Num(total as f64))]);
    let ok: Vec<&Facts> = facts.iter().flatten().collect();
    o.put("readable", Some(Value::Num(ok.len() as f64)));
    let mut pols: Vec<String> = Vec::new();
    let mut classes: Vec<String> = Vec::new();
    let mut roles: Vec<(String, f64)> = Vec::new();
    for f in &ok {
        if !pols.contains(&f.polarity) {
            pols.push(f.polarity.clone());
        }
        if !classes.contains(&f.class) {
            classes.push(f.class.clone());
        }
        if !f.qc.role.is_empty() {
            match roles.iter_mut().find(|r| r.0 == f.qc.role) {
                Some(r) => r.1 += 1.0,
                None => roles.push((f.qc.role.clone(), 1.0)),
            }
        }
    }
    sortc::slice_by(&mut pols, |a, b| a.cmp(b));
    sortc::slice_by(&mut classes, |a, b| a.cmp(b));
    sortc::slice_by(&mut roles, |a, b| a.0.cmp(&b.0));
    o.put("polarities", Some(strs(pols)));
    o.put("classes", Some(strs(classes)));
    o.put(
        "roles",
        Some(Value::Obj(
            roles.into_iter().map(|(k, v)| (k, Value::Num(v))).collect(),
        )),
    );
    let qc: Vec<&QcFile> = ok.iter().map(|f| &f.qc).collect();
    let high = ok.iter().any(|f| f.class == "high");
    let (qc_section, blanks) = inspect_qc::sections(&qc, high);
    o.put("qc", qc_section);
    o.put("blanks", blanks);
    Value::Obj(o.0)
}

fn render(budget: u64, cut: &[&str], files: &[Value], series: &Value) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"ispezione/1\",\"profile\":\"ricerca\",\"budget_tokens\":")
        .int(budget as i64);
    o.raw(",\"truncated\":[");
    for (k, c) in cut.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(c);
    }
    o.raw("],\"files\":");
    value(&mut o, &Value::Arr(files.to_vec()));
    o.raw(",\"series\":");
    value(&mut o, series);
    o.raw("}");
    o.0
}

fn drop_keys(files: &mut [Value], keys: &[&str]) {
    for f in files {
        if let Value::Obj(o) = f {
            o.retain(|(k, _)| !keys.contains(&k.as_str()));
        }
    }
}

/// The `ispezione/1` text for `sources` within `budget_tokens` (`ceil(bytes / 3)`, as `contesto/1`). `sources` is
/// `{profile, hide_names, roles_confirmed?, files:[{name, error?, items:[Item.info], scan_stats?, mrm?, method?, estimate?, sample?, tic?, bpc?, signals?}]}`.
/// Sections that do not fit are dropped, least useful first, and named in `truncated`; the same input gives the same bytes. The `corso` profile gets
/// `err.llm.profile.course`.
pub fn build(sources: &Value, budget_tokens: u64) -> Result<String> {
    if sources.get("profile").and_then(Value::str) != Some("ricerca") {
        return Err(Error::new("err.llm.profile.course"));
    }
    let hide = sources.get("hide_names").and_then(Value::bool) == Some(true);
    let list = sources.get("files").map_or(&[][..], Value::arr);
    let ranks = ranks(list);
    let (mut files, mut facts) = (Vec::new(), Vec::new());
    let confirmed = sources.get("roles_confirmed").and_then(Value::bool) == Some(true);
    for (k, src) in list.iter().enumerate() {
        let (f, fa) = file(src, k, hide, ranks[k], confirmed);
        files.push(f);
        facts.push(fa);
    }
    let mut series = series(&facts, list.len());
    let mut cut: Vec<&str> = Vec::new();
    let fits = |t: &str| (t.len() as u64).div_ceil(3) <= budget_tokens;
    for (name, keys) in [
        ("tic", &["tic", "bpc"][..]),
        ("gradient", &["gradient"][..]),
        ("isolation", &["isolation", "parts"][..]),
    ] {
        if fits(&render(budget_tokens, &cut, &files, &series)) {
            break;
        }
        drop_keys(&mut files, keys);
        cut.push(name);
    }
    if !fits(&render(budget_tokens, &cut, &files, &series)) {
        if let Value::Obj(o) = &mut series {
            o.retain(|(k, _)| k != "qc" && k != "blanks");
        }
        cut.push("qc");
    }
    let text = render(budget_tokens, &cut, &files, &series);
    if fits(&text) {
        Ok(text)
    } else {
        Err(Error::new("err.llm.context.budget").with("budget", budget_tokens))
    }
}

/// `build` from JSON text.
pub fn build_text(sources: &str, budget_tokens: u64) -> Result<String> {
    let v = parse(sources.as_bytes()).map_err(|_| Error::new("err.llm.context.format"))?;
    build(&v, budget_tokens)
}
