// Ported from Attributio src/attributio/review/build.py (propose, precursor_in_ms2, review_sites, blank_ratios, largest_samples, network_neighbours, read_mgf_blocks).
//! The proposal of the review: the tier of a feature (a sorting aid, never a verdict) with the reasons, and the helpers around it that read the campaign:
//! the MS2 blocks of the FBMN MGF with their source scans, the sites of the review, the blank ratios, the sample where a feature is largest and its neighbours
//! in the GNPS2 molecular network.

use super::isotopes::round_to;
use super::matching::delta_formulas;
use super::settings::ReviewSettings;
use crate::attributio::annotate::Feature;
use crate::attributio::campaign::Campaign;
use crate::attributio::detection::{detected_mask, has_site};
use crate::attributio::frame::Frame;
use crate::attributio::review::sources::Edge;
use crate::attributio::sorbents::blank_abundance;
use std::collections::{BTreeMap, HashMap};

/// Sorting order of the tiers on the page.
pub const TIER_ORDER: [(&str, i32); 8] = [
    ("2b-strong", 0),
    ("2b-check", 1),
    ("2a->2b", 2),
    ("2a->3", 3),
    ("2a-unverified", 4),
    ("2a-robust", 5),
    ("3-keep", 6),
    ("1", 7),
];

pub fn tier_rank(tier: &str) -> i32 {
    TIER_ORDER.iter().find(|t| t.0 == tier).map_or(9, |t| t.1)
}

/// What `propose` reads of a feature (the dictionary `r` of the reference tool).
#[derive(Clone, Debug, Default)]
pub struct ProposeInput {
    pub level: i32,
    pub level_sub: String,
    pub adduct: Option<String>,
    /// NaN without an adduct.
    pub ion_ppm: f64,
    pub n_expl: i64,
    pub n_peaks: i64,
    pub frac_expl: f64,
    pub decoy_frac: f64,
    pub level_basis: String,
    pub lib_peaks: i64,
    pub iso_ok: Option<bool>,
    pub obs1: Option<f64>,
    pub th1: Option<f64>,
    pub obs2: Option<f64>,
    pub th2: Option<f64>,
    pub chim_n: Option<i64>,
    pub chim_max: Option<f64>,
    pub ref_cos_vis: Option<f64>,
    pub ref_matched_vis: Option<i64>,
    pub other_best: Option<f64>,
    pub other_refs: String,
}

/// The tier, the proposed level and the reasons.
#[derive(Clone, Debug, PartialEq)]
pub struct Tier {
    pub tier: &'static str,
    pub proposed: &'static str,
    pub reasons: Vec<String>,
}

fn out(tier: &'static str, proposed: &'static str, reasons: Vec<String>) -> Tier {
    Tier {
        tier,
        proposed,
        reasons,
    }
}

/// The reasons that do not depend on the tier.
fn base_reasons(
    r: &ProposeInput,
    s: &ReviewSettings,
    fm: bool,
    expl_ok: bool,
    iso_bad: bool,
) -> Vec<String> {
    let mut why = Vec::new();
    if !fm {
        why.push(if r.adduct.is_some() {
            format!(
                "precursor not explained by the named structure within {:.0} ppm",
                s.ppm_ion
            )
        } else {
            "no structure/formula to test".to_string()
        });
    }
    if fm && !expl_ok {
        why.push(format!(
            "fragments explained: {} of {} peaks, {:.0}% of intensity (decoy {:.0}%)",
            r.n_expl,
            r.n_peaks,
            100.0 * r.frac_expl,
            100.0 * r.decoy_frac
        ));
    }
    if iso_bad {
        let g = |v: Option<f64>| v.unwrap_or(f64::NAN);
        why.push(format!(
            "MS1 isotope pattern inconsistent (M+1 {:.2}/{:.2}, M+2 {:.2}/{:.2} obs/theo)",
            g(r.obs1),
            g(r.th1),
            g(r.obs2),
            g(r.th2)
        ));
    }
    if r.chim_n.unwrap_or(0) > 0 {
        why.push(format!(
            "chimeric: {} co-isolated ion(s) ≥ {:.0}% of the precursor (max {:.0}%)",
            r.chim_n.unwrap_or(0),
            100.0 * s.chim_rel,
            100.0 * r.chim_max.unwrap_or(0.0)
        ));
    }
    if r.level_basis == "isomers not discriminated" {
        why.push(
            "the same spectrum matches peaks at several retention times: the spectrum cannot separate the isomers, so 2b is not reachable from MS/MS"
                .into(),
        );
    } else if r.level_basis == "name conflict" {
        why.push(
            "valid sources name different structures: decide which one the fragments support"
                .into(),
        );
    }
    why
}

/// The proposal of the prototype: `(tier, proposed level, reasons)`.
pub fn propose(r: &ProposeInput, s: &ReviewSettings) -> Tier {
    if r.level == 1 {
        return out(
            "1",
            "1",
            vec!["Level 1: decided in level1_decisions.csv (attributio livello1)".into()],
        );
    }
    let fm = r.adduct.is_some() && r.ion_ppm.abs() <= s.ppm_ion;
    let expl_ok = fm
        && r.n_expl >= s.min_explained_peaks
        && r.frac_expl >= s.min_explained_frac
        && r.frac_expl - r.decoy_frac >= s.min_margin;
    let iso_bad = r.iso_ok == Some(false);
    let mut why = base_reasons(r, s, fm, expl_ok, iso_bad);
    let chim_cap = r.chim_max.unwrap_or(0.0) >= s.chim_cap;
    let isomers = r.level_basis == "isomers not discriminated";
    let amb = isomers || r.level_basis == "name conflict";
    let comp = r
        .other_best
        .is_some_and(|o| o >= s.ref_min_cos && r.ref_cos_vis.is_none_or(|c| o > c));
    if comp {
        why.push(format!(
            "another structure matches as well or better: {}",
            r.other_refs
        ));
    }
    let ref_ok = r.ref_matched_vis.is_some_and(|m| m >= s.ref_min_matched)
        && r.ref_cos_vis.is_some_and(|c| c >= s.ref_min_cos);
    if r.level == 3 && r.level_sub != "b" {
        return level3(
            r,
            s,
            why,
            (fm && expl_ok, iso_bad, isomers, chim_cap, amb, comp, ref_ok),
        );
    }
    let ref_present = r.ref_cos_vis.is_some();
    let (cos, matched) = (
        r.ref_cos_vis.unwrap_or(f64::NAN),
        r.ref_matched_vis.unwrap_or(0),
    );
    let robust = fm
        && !iso_bad
        && ref_present
        && cos >= s.l2a_min_cos_vis
        && matched >= s.l2a_min_matched_vis;
    if robust && !(chim_cap && cos < 0.85) {
        if !expl_ok {
            why.push(
                "reference match is solid but few fragments are explained by the formula".into(),
            );
        }
        return out("2a-robust", "2a", why);
    }
    if !ref_present && fm && expl_ok && !iso_bad && r.lib_peaks >= 4 {
        why.insert(
            0,
            format!(
                "reference spectrum not retrievable; pipeline match on {} ions",
                r.lib_peaks
            ),
        );
        return out("2a-unverified", "2a", why);
    }
    let weak_ref = ref_present && cos >= s.ref_min_cos && matched >= s.ref_min_matched;
    if fm && !iso_bad && (expl_ok || weak_ref) {
        if ref_present {
            why.insert(
                0,
                format!(
                    "reference cos_vis {:.2}, {} visible peaks (2a needs ≥ {:.2} and ≥ {})",
                    cos, matched, s.l2a_min_cos_vis, s.l2a_min_matched_vis
                ),
            );
        }
        return out("2a->2b", "2b", why);
    }
    out("2a->3", "3", why)
}

/// The Level 3 branch: (formula fits and fragments explained, isotopes bad, isomers, chimera cap, ambiguous, competing structure, reference ok).
fn level3(
    r: &ProposeInput,
    _s: &ReviewSettings,
    mut why: Vec<String>,
    f: (bool, bool, bool, bool, bool, bool, bool),
) -> Tier {
    let (fits, iso_bad, isomers, chim_cap, amb, comp, ref_ok) = f;
    if !fits || iso_bad || isomers {
        return out("3-keep", "3", why);
    }
    if ref_ok && !chim_cap && !amb && !comp {
        why.insert(
            0,
            format!(
                "same-structure reference: cos_vis {:.2}, {} visible peaks matched",
                r.ref_cos_vis.unwrap_or(f64::NAN),
                r.ref_matched_vis.unwrap_or(0)
            ),
        );
        return out("2b-strong", "2b", why);
    }
    if !ref_ok {
        why.insert(
            0,
            "fragments consistent with the formula; no same-structure reference spectrum: is a fragment specific of the structure shown?".into(),
        );
    }
    out("2b-check", "2b", why)
}

/// Is the precursor ion itself in the merged MS2 spectrum (before cleaning)?
#[derive(Clone, Debug, PartialEq)]
pub struct PrecursorInMs2 {
    pub present: bool,
    pub prec_obs: Option<f64>,
    /// Intensity relative to the base FRAGMENT peak (below precursor - 0.5), in %; above 100 when the precursor dominates.
    pub prec_rel: Option<f64>,
}

pub fn precursor_in_ms2(
    mz: &[f64],
    it: &[f64],
    prec: f64,
    ppm: f64,
    min_da: f64,
) -> PrecursorInMs2 {
    let absent = PrecursorInMs2 {
        present: false,
        prec_obs: None,
        prec_rel: None,
    };
    if mz.is_empty() {
        return absent;
    }
    let tol = f64::max(min_da, prec * ppm * 1e-6);
    let near: Vec<usize> = (0..mz.len())
        .filter(|&i| (mz[i] - prec).abs() <= tol)
        .collect();
    if near.is_empty() {
        return absent;
    }
    let base = (0..mz.len())
        .filter(|&i| mz[i] < prec - 0.5)
        .map(|i| it[i])
        .fold(0.0, f64::max);
    let k = near
        .iter()
        .fold(near[0], |b, &i| if it[i] > it[b] { i } else { b });
    PrecursorInMs2 {
        present: true,
        prec_obs: Some(round_to(mz[k], 5)),
        prec_rel: (base > 0.0).then(|| round_to(it[k] / base * 100.0, 1)),
    }
}

// ------------------------------------------------------------------------------ MGF

/// The largest MS2 block of a feature in the FBMN MGF, with its source scans.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MgfBlock {
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    pub prec: f64,
    pub file: Option<String>,
    /// `(file, scan)` of the scans merged into the block.
    pub srcs: Vec<(String, i64)>,
    /// Minutes, NaN when unknown.
    pub rt: f64,
    pub n_samples: Option<String>,
    pub iso: Option<String>,
    pub ce: Option<String>,
}

/// The strings of a JSON list of strings (`["a","b"]`); empty when the text is not one.
fn json_strings(t: &str) -> Vec<String> {
    let t = t.trim();
    if !(t.starts_with('[') && t.ends_with(']')) {
        return Vec::new();
    }
    let (mut out, mut cur, mut inq, mut esc) = (Vec::new(), String::new(), false, false);
    for c in t.chars() {
        match (inq, esc, c) {
            (true, true, c) => {
                cur.push(c);
                esc = false;
            }
            (true, false, '\\') => esc = true,
            (true, false, '"') => {
                inq = false;
                out.push(std::mem::take(&mut cur));
            }
            (true, false, c) => cur.push(c),
            (false, _, '"') => inq = true,
            _ => {}
        }
    }
    out
}

fn source_scans(hdr: &HashMap<String, String>) -> Vec<(String, i64)> {
    let mut srcs = Vec::new();
    for u in json_strings(hdr.get("SOURCE_SCAN_USI").map_or("[]", |s| s.as_str())) {
        let parts: Vec<&str> = u.split(':').collect();
        if parts.len() < 4 {
            break;
        }
        let name = if parts[2].ends_with(".mzML") {
            parts[2].to_string()
        } else {
            format!("{}.mzML", parts[2])
        };
        for x in parts[3].split(',') {
            let x = x.trim();
            if !x.is_empty() && x.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(n) = x.parse() {
                    srcs.push((name.clone(), n));
                }
            }
        }
    }
    srcs
}

fn block_of(hdr: &HashMap<String, String>, mz: &[f64], it: &[f64]) -> MgfBlock {
    let get = |k: &str| hdr.get(k).cloned();
    MgfBlock {
        mz: mz.to_vec(),
        it: it.to_vec(),
        prec: hdr
            .get("PEPMASS")
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0),
        file: get("FILENAME"),
        srcs: source_scans(hdr),
        rt: hdr
            .get("RTINSECONDS")
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(f64::NAN)
            / 60.0,
        n_samples: get("MERGED_ACROSS_N_SAMPLES"),
        iso: get("ISOLATION_WINDOW"),
        ce: get("COLLISION_ENERGY"),
    }
}

/// `{feature id: block}`: the largest MS2 block of each feature (the first of equal size), only for `wanted` ids when given.
pub fn read_mgf_blocks(text: &str, wanted: Option<&[i64]>) -> BTreeMap<i64, MgfBlock> {
    let mut out: BTreeMap<i64, MgfBlock> = BTreeMap::new();
    let (mut hdr, mut mzs, mut its): (HashMap<String, String>, Vec<f64>, Vec<f64>) =
        Default::default();
    for line in text.lines() {
        let line = line.trim();
        if line == "BEGIN IONS" {
            (hdr, mzs, its) = Default::default();
        } else if line == "END IONS" {
            let Some(fid) = hdr
                .get("FEATURE_ID")
                .and_then(|s| s.trim().parse::<f64>().ok())
                .map(|v| v as i64)
            else {
                continue;
            };
            let lvl = hdr.get("MSLEVEL").map_or("2", |s| s.trim());
            if lvl != "2" && !lvl.is_empty() {
                continue;
            }
            let want = wanted.is_none_or(|w| w.contains(&fid));
            if want && !mzs.is_empty() && out.get(&fid).is_none_or(|b| mzs.len() > b.mz.len()) {
                out.insert(fid, block_of(&hdr, &mzs, &its));
            }
        } else if line.contains('=') && !line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            if let Some((k, v)) = line.split_once('=') {
                hdr.insert(k.to_uppercase(), v.to_string());
            }
        } else {
            let p: Vec<&str> = line.split_whitespace().collect();
            if let (Some(a), Some(b)) = (
                p.first().and_then(|s| s.parse::<f64>().ok()),
                p.get(1).and_then(|s| s.parse::<f64>().ok()),
            ) {
                if p.len() >= 2 {
                    mzs.push(a);
                    its.push(b);
                }
            }
        }
    }
    out
}

// ------------------------------------------------------------------------------ campaign helpers

/// The sites of the review: those of `matrix` (or the `[review] matrix` of the campaign) when it names one, else every site. Returns (scope, sites).
pub fn review_sites(c: &Campaign, matrix: Option<&str>) -> (String, Vec<String>) {
    let m = matrix.map(String::from).or_else(|| {
        c.review
            .get("matrix")
            .and_then(|v| v.as_str())
            .map(String::from)
    });
    if let Some(m) = m {
        if let Some(sites) = c.matrix_sites(&m) {
            return (m, sites.to_vec());
        }
    }
    (
        "all sites".to_string(),
        c.site_keys().iter().map(|s| s.to_string()).collect(),
    )
}

/// `{(pol, id): {site: largest DETECTED area at the site / procedural-blank abundance}}` (`None` when the feature is absent from the blanks).
pub fn blank_ratios(
    c: &Campaign,
    pol: &str,
    table: &Frame,
    wanted: &[i64],
    sites: &[String],
) -> BTreeMap<i64, Vec<(String, Option<f64>)>> {
    let mut out = BTreeMap::new();
    let (Some(idc), true) = (table.col("id"), !table.is_empty()) else {
        return out;
    };
    let bcol = table.col("blank_subtraction");
    let blank: Vec<f64> = (0..table.len())
        .map(|r| {
            bcol.and_then(|k| table.str_na(r, k))
                .map_or(0.0, |t| blank_abundance(t).unwrap_or(0.0))
        })
        .collect();
    let mut per_site: Vec<(&String, Vec<f64>)> = Vec::new();
    for s in sites.iter().filter(|s| has_site(c, table, s, pol)) {
        if let Ok(a) = crate::attributio::detection::site_area(
            c,
            table,
            s,
            pol,
            None,
            crate::attributio::detection::Agg::Max,
        ) {
            per_site.push((s, a));
        }
    }
    for r in 0..table.len() {
        let Some(fid) = table.int(r, idc) else {
            continue;
        };
        if !wanted.contains(&fid) {
            continue;
        }
        let d = per_site
            .iter()
            .filter(|(_, a)| a[r] > 0.0)
            .map(|(s, a)| {
                (
                    (*s).clone(),
                    (blank[r] > 0.0).then(|| round_to(a[r] / blank[r], 1)),
                )
            })
            .collect();
        out.insert(fid, d);
    }
    out
}

/// `{id: (raw file, RT in that file)}`: the sample of the review scope where the feature is DETECTED with the largest area (the MS1 with the best isotope
/// statistics). Samples whose file name starts with one of `exclude` are skipped.
pub fn largest_samples(
    c: &Campaign,
    pol: &str,
    table: &Frame,
    wanted: &[i64],
    sites: &[String],
    exclude: &[String],
) -> BTreeMap<i64, (String, Option<f64>)> {
    let mut out = BTreeMap::new();
    let Some(idc) = table.col("id") else {
        return out;
    };
    let rows: Vec<usize> = (0..table.len())
        .filter(|&r| table.int(r, idc).is_some_and(|i| wanted.contains(&i)))
        .collect();
    let mut best: Vec<(f64, String, Option<f64>)> = vec![(0.0, String::new(), None); rows.len()];
    for s in sites.iter().filter(|s| has_site(c, table, s, pol)) {
        for sb in &c.sorbents {
            let col_a = c.col(s, sb, pol, "area");
            let parts: Vec<&str> = col_a.split(':').collect();
            if parts.len() < 3 || exclude.iter().any(|x| parts[1].starts_with(x.as_str())) {
                continue;
            }
            let (Some(ka), Ok(det)) = (table.col(&col_a), detected_mask(c, table, s, sb, pol))
            else {
                continue;
            };
            let kr = table.col(&c.col(s, sb, pol, "rt"));
            for (k, &r) in rows.iter().enumerate() {
                let a = if det[r] {
                    table.num(r, ka).unwrap_or(0.0)
                } else {
                    0.0
                };
                if a > best[k].0 {
                    best[k] = (a, parts[1].to_string(), kr.and_then(|kr| table.num(r, kr)));
                }
            }
        }
    }
    for (k, &r) in rows.iter().enumerate() {
        if !best[k].1.is_empty() {
            out.insert(
                table.int(r, idc).unwrap_or(0),
                (best[k].1.clone(), best[k].2),
            );
        }
    }
    out
}

/// A neighbour of a feature in the GNPS2 molecular network.
#[derive(Clone, Debug, PartialEq)]
pub struct Neighbour {
    pub k: String,
    pub id: i64,
    pub mz: f64,
    pub rt: f64,
    pub cos: Option<f64>,
    pub dm: f64,
    pub dm_formula: Option<String>,
    pub level: Option<i32>,
    pub level_sub: String,
    pub name: Option<String>,
}

/// `{feature key: [neighbours]}`: id, m/z, RT, edge cosine, delta m = m/z(neighbour) - m/z(feature) with its closest formula change, the neighbour's level
/// and name (when detected in the review scope). `tables` gives `(pol, id) -> (m/z, RT)` of every feature of the merged tables.
pub fn network_neighbours(
    edges: &[Edge],
    feats: &[(String, i64, f64)],
    annotated: &[Feature],
    tables: &HashMap<(String, i64), (f64, f64)>,
    s: &ReviewSettings,
) -> BTreeMap<String, Vec<Neighbour>> {
    let mut adj: HashMap<(String, i64), Vec<(i64, Option<f64>)>> = HashMap::new();
    for e in edges {
        adj.entry((e.pol.clone(), e.a))
            .or_default()
            .push((e.b, e.cosine));
        adj.entry((e.pol.clone(), e.b))
            .or_default()
            .push((e.a, e.cosine));
    }
    let mut out = BTreeMap::new();
    for (pol, id, mz) in feats {
        let mut nb: Vec<Neighbour> = Vec::new();
        for (other, cos) in adj.get(&(pol.clone(), *id)).into_iter().flatten() {
            let Some(&(omz, ort)) = tables.get(&(pol.clone(), *other)) else {
                continue;
            };
            let dm = omz - mz;
            let df = if dm.abs() >= s.analog_min_shift_da {
                delta_formulas(dm, s.delta_formula_mda, 1)
            } else {
                Vec::new()
            };
            let x = annotated.iter().find(|f| &f.pol == pol && f.id == *other);
            nb.push(Neighbour {
                k: format!("{pol}:{other}"),
                id: *other,
                mz: round_to(omz, 4),
                rt: round_to(ort, 2),
                cos: cos.map(|c| round_to(c, 2)),
                dm: round_to(dm, 4),
                dm_formula: df.first().map(|d| d.label.clone()),
                level: x.map(|f| f.level),
                level_sub: x.map_or(String::new(), |f| f.level_sub.clone()),
                name: x
                    .and_then(|f| f.disp_name.clone())
                    .filter(|n| !n.trim().is_empty() && n.to_lowercase() != "nan"),
            });
        }
        if !nb.is_empty() {
            nb.sort_by(|a, b| {
                b.cos
                    .unwrap_or(0.0)
                    .total_cmp(&a.cos.unwrap_or(0.0))
                    .then(a.id.cmp(&b.id))
            });
            out.insert(format!("{pol}:{id}"), nb);
        }
    }
    out
}
