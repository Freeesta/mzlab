//! The text of `screen.suspects@1`: the aligned feature table of a high-resolution run searched, per adduct, in the suspect lists of the user (`.mzsus` or CSV) by neutral
//! mass; hits with the same formula are one isomer group, never one name. Optional RT and MS2 fragment evidence; Level 5 (mass and formula), 4 when `formulas/1` supports the
//! formula. Takes bytes and returns the text (`screen/1`): `flow run` (`mzlab-cli`) and the browser (`mzlab-attributio-wasm`) run this one function. Candidates, never a verdict.

mod evidence;
mod source;

use crate::adducts::{NEG_ADDUCTS, POS_ADDUCTS, PPM};
use crate::store::{origin_from_meta, read_meta_mz_rt};
use evidence::{formula_set, fragments_of, matched, read_scans, supports};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{round, Out, Value};
pub use source::{Entry, Membership, Source};

/// A file the step reads: its name and its bytes.
pub type Named<'a> = (&'a str, &'a [u8]);

/// The default adducts: the five of the Attributio screening, cut to the polarity of the table.
pub const DEFAULT_ADDUCTS: &str = "[M+H]+,[M+NH4]+,[M+Na]+,[M-H]-,[M+Cl]-";

/// The parameters of the step (`None` = off).
pub struct Params {
    pub ppm: f64,
    pub adducts: String,
    pub rt_window_min: Option<f64>,
    pub min_fragments: Option<usize>,
    pub max_hits_per_feature: usize,
    pub ms2_rt_min: f64,
}

impl Params {
    /// From the resolved parameters of the step, or the object the page sends (the parameters merged with the defaults).
    pub fn from_params(p: &[(String, Value)]) -> Params {
        let get = |n: &str| p.iter().find(|(k, _)| k == n).map(|(_, v)| v);
        let num = |n: &str| get(n).and_then(Value::f64).filter(|v| v.is_finite());
        Params {
            ppm: num("ppm").unwrap_or(PPM),
            adducts: get("adducts").and_then(Value::str).filter(|s| !s.trim().is_empty()).unwrap_or(DEFAULT_ADDUCTS).to_string(),
            rt_window_min: num("rt_window_min"),
            min_fragments: num("min_fragments").map(|v| v.max(0.0) as usize),
            max_hits_per_feature: num("max_hits_per_feature").map_or(10, |v| v.max(1.0) as usize),
            ms2_rt_min: num("ms2_rt_min").unwrap_or(0.2),
        }
    }
}

/// The adducts to search: the listed ones that exist for the polarity of the table.
fn adducts_for(list: &str, polarity: i8) -> Result<Vec<(&'static str, f64)>> {
    let table = if polarity >= 0 { &POS_ADDUCTS[..] } else { &NEG_ADDUCTS[..] };
    let known = |n: &str| POS_ADDUCTS.iter().chain(NEG_ADDUCTS.iter()).any(|(k, _)| *k == n);
    let wanted: Vec<&str> = list.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()).collect();
    if let Some(bad) = wanted.iter().find(|n| !known(n)) {
        return Err(Error::new("err.flow.screen.adduct").with("adduct", bad));
    }
    let out: Vec<_> = table.iter().filter(|(n, _)| wanted.contains(n)).copied().collect();
    if out.is_empty() {
        return Err(Error::new("err.flow.screen.polarity").with("adducts", list));
    }
    Ok(out)
}

struct Cand {
    entry: Entry,
    files: Vec<usize>,
    delta: f64,
    rt_diff: Option<f64>,
    found: Vec<f64>,
}

struct Group {
    /// Index of the adduct and key of the group among the groups of the feature.
    slot: (usize, String),
    feature: usize,
    adduct: &'static str,
    formula: String,
    cands: Vec<Cand>,
}

impl Group {
    /// Skeleton and name of the first candidate: the last key of the order of the groups.
    fn tie(&self) -> (&str, &str) {
        self.cands.first().map_or(("", ""), |c| (c.entry.skeleton.as_str(), c.entry.name.as_str()))
    }

    fn delta(&self) -> f64 {
        self.cands.iter().map(|c| c.delta).min_by(|a, b| a.abs().total_cmp(&b.abs())).unwrap_or(0.0)
    }
}

/// A structure is told apart by its first block, else by name and SMILES.
fn cand_key(e: &Entry) -> String {
    if e.skeleton.is_empty() {
        format!("?{}|{}", e.name.to_ascii_lowercase(), e.smiles)
    } else {
        e.skeleton.clone()
    }
}

/// Runs the step on the alignment `table` (`.mzfeat`), the suspect files, the optional MGF of `spectra.ms2` and `formulas/1` files; returns the `screen/1` text.
pub fn screen_text(p: &Params, table: &[u8], suspects: &[Named], spectra: Option<&[u8]>, formulas: &[&[u8]]) -> Result<String> {
    if suspects.is_empty() {
        return Err(Error::new("err.flow.port.missing").with("port", "suspects"));
    }
    let sources: Vec<Source> = suspects.iter().map(|(n, b)| Source::open(n, b)).collect::<Result<_>>()?;
    let (meta, mzs, rts) = read_meta_mz_rt(table)?;
    let sign: i32 = origin_from_meta(&meta).facts.iter().map(|f| i32::from(f.polarity.signum())).sum();
    let polarity: i8 = if sign < 0 { -1 } else { 1 };
    let adducts = adducts_for(&p.adducts, polarity)?;
    let scans = spectra.map(read_scans);
    let supported = if formulas.is_empty() { None } else { Some(formula_set(formulas)?) };
    let mut groups: Vec<Group> = Vec::new();
    let mut dropped = 0;
    for row in 0..mzs.len() {
        let (mz, rt, id) = (mzs[row], rts[row], (row + 1).to_string());
        let own = feature_groups(p, &sources, &adducts, (row + 1, mz, rt), scans.as_deref().map(|s| fragments_of(s, &id, mz, rt, (p.ppm * mz * 1e-6).max(0.01), p.ms2_rt_min)));
        let mut own = own;
        own.sort_unstable_by(|a, b| a.delta().abs().total_cmp(&b.delta().abs()).then_with(|| a.adduct.cmp(b.adduct)).then_with(|| a.formula.cmp(&b.formula)).then_with(|| a.tie().cmp(&b.tie())));
        dropped += own.len().saturating_sub(p.max_hits_per_feature);
        own.truncate(p.max_hits_per_feature);
        groups.extend(own);
    }
    groups.sort_unstable_by(|a, b| {
        (a.feature.cmp(&b.feature)).then_with(|| a.adduct.cmp(b.adduct)).then_with(|| a.delta().total_cmp(&b.delta())).then_with(|| a.formula.cmp(&b.formula)).then_with(|| a.tie().cmp(&b.tie()))
    });
    Ok(write(p, polarity, &adducts, &sources, &mzs, (&groups, dropped), scans.as_ref().map(Vec::len), supported.as_deref()))
}

type Feature = (usize, f64, f64);

/// The isomer groups of one feature over the adducts and the files.
fn feature_groups(p: &Params, sources: &[Source], adducts: &[(&'static str, f64)], (id, mz, rt): Feature, peaks: Option<Vec<f64>>) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for (ai, (adduct, shift)) in adducts.iter().enumerate() {
        let neutral = mz - shift;
        if neutral <= 0.0 {
            continue;
        }
        // The window of Attributio is on the ion m/z; on the neutral mass it is wider by mz / neutral.
        let window = p.ppm * mz / neutral * 1.0000001;
        for (si, src) in sources.iter().enumerate() {
            for e in src.search(neutral, window) {
                let theo = e.mass + shift;
                let delta = (mz - theo) / theo * 1e6;
                if delta.abs() > p.ppm {
                    continue;
                }
                let rt_diff = e.rt.map(|r| rt - r);
                if p.rt_window_min.zip(rt_diff).is_some_and(|(w, d)| d.abs() > w) {
                    continue;
                }
                let found = peaks.as_deref().map(|pk| matched(&e.fragments, pk, p.ppm)).unwrap_or_default();
                if p.min_fragments.is_some_and(|k| found.len() < k) {
                    continue;
                }
                let key = cand_key(&e);
                let gkey = if e.formula.is_empty() { format!("?{key}") } else { e.formula.clone() };
                let at = match out.iter().position(|g| g.slot.0 == ai && g.slot.1 == gkey) {
                    Some(at) => at,
                    None => {
                        out.push(Group { slot: (ai, gkey), feature: id, adduct, formula: e.formula.clone(), cands: Vec::new() });
                        out.len() - 1
                    }
                };
                let g = &mut out[at];
                match g.cands.iter_mut().find(|c| cand_key(&c.entry) == key) {
                    Some(c) => {
                        c.files.push(si);
                        c.entry.name = if c.entry.name.is_empty() { e.name.clone() } else { c.entry.name.clone() };
                        c.rt_diff = c.rt_diff.or(rt_diff);
                    }
                    None => g.cands.push(Cand { entry: e, files: vec![si], delta, rt_diff, found }),
                }
            }
        }
    }
    for g in &mut out {
        g.cands.sort_unstable_by(|a, b| a.entry.skeleton.cmp(&b.entry.skeleton).then_with(|| a.entry.name.cmp(&b.entry.name)).then_with(|| a.entry.smiles.cmp(&b.entry.smiles)).then_with(|| a.entry.mass.total_cmp(&b.entry.mass)));
    }
    out
}

fn write(p: &Params, polarity: i8, adducts: &[(&str, f64)], sources: &[Source], mz: &[f64], (groups, dropped): (&[Group], usize), ms2: Option<usize>, supported: Option<&[(String, String)]>) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":\"screen/1\",\"polarity\":").int(i64::from(polarity)).raw(",\"params\":{\"ppm\":").num(p.ppm).raw(",\"adducts\":[");
    for (i, (a, _)) in adducts.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a);
    }
    o.raw("],\"rt_window_min\":").opt(p.rt_window_min).raw(",\"min_fragments\":").opt(p.min_fragments.map(|v| v as f64));
    o.raw(",\"max_hits_per_feature\":").int(p.max_hits_per_feature as i64).raw(",\"ms2_rt_min\":").num(p.ms2_rt_min).raw("},\"suspect_files\":[");
    for (i, s) in sources.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"name\":").str(&s.name).raw(",\"sha256\":").str(&s.sha256).raw(",\"kind\":").str(s.kind());
        o.raw(",\"rows\":").int(s.len() as i64).raw(",\"skipped_rows\":").int(s.skipped as i64);
        o.raw(",\"provenance\":").raw(&s.provenance).raw("}");
    }
    o.raw("],\"features\":").int(mz.len() as i64).raw(",\"ms2_scans\":").opt(ms2.map(|n| n as f64)).raw(",\"dropped_groups\":").int(dropped as i64).raw(",\"hits\":[");
    for (i, g) in groups.iter().enumerate() {
        let level = if supported.is_some_and(|s| supports(s, &g.feature.to_string(), &g.formula)) { 4 } else { 5 };
        o.raw(if i > 0 { "," } else { "" }).raw("{\"feature\":").int(g.feature as i64).raw(",\"mz\":").num(round(mz[g.feature - 1], 6));
        o.raw(",\"adduct\":").str(g.adduct).raw(",\"delta_ppm\":").num(round(g.delta(), 3)).raw(",\"formula\":").str(&g.formula).raw(",\"level\":").int(level).raw(",\"candidates\":[");
        for (k, c) in g.cands.iter().enumerate() {
            let e = &c.entry;
            o.raw(if k > 0 { "," } else { "" }).raw("{\"skeleton\":").str(&e.skeleton).raw(",\"inchikey\":").str(&e.inchikey).raw(",\"name\":").str(&e.name);
            o.raw(",\"smiles\":").str(&e.smiles).raw(",\"neutral_mass\":").num(round(e.mass, 6)).raw(",\"rt_diff_min\":").opt(c.rt_diff.map(|d| round(d, 3)));
            o.raw(",\"fragments_listed\":").int(e.fragments.len() as i64).raw(",\"fragments_matched\":").int(c.found.len() as i64).raw(",\"fragment_mz\":").floats(&c.found);
            o.raw(",\"files\":[");
            for (j, f) in c.files.iter().enumerate() {
                o.raw(if j > 0 { "," } else { "" }).int(*f as i64);
            }
            o.raw("],\"memberships\":[");
            for (j, m) in e.memberships.iter().enumerate() {
                o.raw(if j > 0 { "," } else { "" }).raw("{\"list\":").str(&m.list).raw(",\"entry_ref\":").str(&m.entry_ref).raw(",\"form\":").str(&m.form).raw(",\"group\":").raw(if m.group { "true" } else { "false" }).raw("}");
            }
            o.raw("]}");
        }
        o.raw("]}");
    }
    o.raw("]}");
    o.0
}
