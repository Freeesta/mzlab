//! `Item.info()` of `mzlab/explore.py`: what the page learns about an open file (counts, ranges, polarity, experiments, mass profile,
//! acquisition scheme), worked out from the scan table and the `.mzidx` alone. The few facts that only the mzML header and its chromatograms
//! hold come in through `Facts`.

use crate::error::Result;
use crate::idxread::{Index, ReadAt, Scan};
use crate::json::{round, Out};

/// What only the mzML header / chromatogram list say.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub instrument: String,
    pub n_chromatograms: i64,
    pub srm: usize,
    pub pda: bool,
    /// "positive" | "negative" | "mixed" | "unknown", read from the header (used when a file has no scans).
    pub header_polarity: String,
    /// (min, max) retention time over the SRM chromatograms.
    pub srm_rt: Option<(f64, f64)>,
    /// The non-MS traces that are not the PDA (pressure, flow...): chromatogram index, kind, id, unit, first and last time, points.
    pub analog: Vec<Analog>,
    /// The `.raw` method the file carries (`metodo/1` in the `<run>` of a converted `.raw`) has an LC part (`None`: no `.raw` method).
    pub lc: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct Analog {
    pub i: i64,
    pub kind: String,
    pub id: String,
    pub unit: String,
    pub t0: f64,
    pub t1: f64,
    pub n: i64,
}

/// The sample-table fields of an item (they come from the session, not from the file).
#[derive(Debug, Clone, Default)]
pub struct Meta {
    pub file: String,
    pub label: String,
    pub sha256: String,
    pub time: Option<f64>,
    pub typ: String,
    pub conc: Option<f64>,
    pub cunit: Option<String>,
    pub rep: Option<String>,
}

/// One experiment of a MIXED file.
#[derive(Debug, Clone)]
pub struct Part {
    pub mode: &'static str, // "ms1" | "ms2" | "mrm"
    pub pol: Option<i8>,
    pub tag: String,
    /// The scan type of the MS2 scans of this part (`PREC`, `NL`, `NG`, `EPI`), `None` for the ordinary ones.
    pub kind: Option<&'static str>,
}

/// The scan types in the order they are listed, with the name shown for each.
const KINDS: [(&str, &str); 4] = [
    ("PREC", "Precursor ion"),
    ("NL", "Neutral loss"),
    ("NG", "Neutral gain"),
    ("EPI", "EPI"),
];

fn kind_name(k: &str) -> &'static str {
    KINDS
        .iter()
        .find(|(c, _)| *c == k)
        .map_or("MS2", |(_, n)| n)
}

/// The experiments of a mixed file (survey + product ions, MRM + EPI, positive + negative), or empty when the file has only one.
pub fn file_parts(scans: &[Scan], has_srm: bool) -> Vec<Part> {
    let pols: Vec<i8> = {
        let mut v: Vec<i8> = scans
            .iter()
            .map(|s| s.polarity)
            .filter(|p| *p == 1 || *p == -1)
            .collect();
        crate::sortc::slice_ord(&mut v);
        v.dedup();
        v
    };
    let split = pols.len() > 1;
    let mut parts = vec![];
    for (mode, name, level2) in [("ms1", "MS1", false), ("ms2", "MS2", true)] {
        let sel: Vec<&Scan> = scans
            .iter()
            .filter(|s| (s.level >= 2) == level2 && (level2 || s.level == 1))
            .collect();
        if sel.is_empty() {
            continue;
        }
        // the ordinary scans first, then one part per scan type present
        let kinds: Vec<Option<&'static str>> = std::iter::once(None)
            .chain(KINDS.iter().map(|(c, _)| Some(*c)))
            .filter(|k| (level2 || k.is_none()) && sel.iter().any(|s| s.kind.as_deref() == *k))
            .collect();
        for kind in kinds {
            let sel: Vec<&Scan> = sel
                .iter()
                .copied()
                .filter(|s| s.kind.as_deref() == kind)
                .collect();
            let name = kind.unwrap_or(name);
            let list: Vec<Option<i8>> = if split {
                let mut v: Vec<i8> = sel
                    .iter()
                    .map(|s| s.polarity)
                    .filter(|p| *p == 1 || *p == -1)
                    .collect();
                crate::sortc::slice_ord(&mut v);
                v.dedup();
                v.reverse();
                v.into_iter().map(Some).collect()
            } else {
                vec![None]
            };
            for pol in list {
                let suffix = if split {
                    if pol == Some(1) {
                        " pos"
                    } else {
                        " neg"
                    }
                } else {
                    ""
                };
                parts.push(Part {
                    mode,
                    pol,
                    tag: format!("{name}{suffix}"),
                    kind,
                });
            }
        }
    }
    if has_srm {
        parts.push(Part {
            mode: "mrm",
            pol: None,
            tag: "MRM".into(),
            kind: None,
        });
    }
    if parts.len() > 1 {
        parts
    } else {
        vec![]
    }
}

/// The name shown for a part of a mixed file: Full Scan, DDA (MS2 scans all tied to a precursor survey), MS2 or MRM, with the polarity suffix of the key.
pub fn part_label(part: &Part, scans: &[Scan]) -> String {
    let dda = scans.iter().any(|s| s.level == 1) && {
        let mut ms2 = scans.iter().filter(|s| s.level >= 2).peekable();
        ms2.peek().is_some() && ms2.all(|s| s.parent.is_some())
    };
    let (name, head) = match part.mode {
        "ms1" => ("Full Scan", "MS1"),
        "mrm" => ("MRM", "MRM"),
        _ if part.kind.is_some() => (kind_name(part.kind.unwrap_or("")), part.kind.unwrap_or("")),
        _ if dda => ("DDA", "MS2"),
        _ => ("MS2", "MS2"),
    };
    format!("{name}{}", part.tag.strip_prefix(head).unwrap_or(""))
}

pub fn matches(s: &Scan, part: Option<&Part>) -> bool {
    let Some(p) = part else { return true };
    if p.mode == "mrm"
        || (p.pol.is_some_and(|pp| pp != 0) && s.polarity != p.pol.unwrap() && s.polarity != 0)
    {
        return false;
    }
    if p.mode == "ms1" {
        s.level == 1
    } else {
        s.level >= 2 && s.kind.as_deref() == p.kind
    }
}

struct Profile {
    hr: bool,
    an: String,
    dec: i64,
    tol: f64,
    unit: &'static str,
}

/// `reader.profile.mass_profile`: how precisely m/z values are read, from the analyzer and resolving power of a group of scans.
fn mass_profile(scans: &[&Scan]) -> Profile {
    let low = |an: &str| Profile {
        hr: false,
        an: an.to_string(),
        dec: 1,
        tol: 0.5,
        unit: "Da",
    };
    if scans.is_empty() {
        return low("?");
    }
    let mut counts: Vec<(String, usize)> = vec![];
    for s in scans {
        let a =
            s.an.clone()
                .filter(|a| !a.is_empty())
                .unwrap_or_else(|| "?".into());
        match counts.iter_mut().find(|(k, _)| *k == a) {
            Some(c) => c.1 += 1,
            None => counts.push((a, 1)),
        }
    }
    let mut best = 0;
    for (i, c) in counts.iter().enumerate() {
        if c.1 > counts[best].1 {
            best = i;
        }
    }
    let an = counts[best].0.clone();
    match an.as_str() {
        "FTMS" => {
            return Profile {
                hr: true,
                an,
                dec: 4,
                tol: 5.0,
                unit: "ppm",
            }
        }
        "TOFMS" => {
            return Profile {
                hr: true,
                an,
                dec: 4,
                tol: 10.0,
                unit: "ppm",
            }
        }
        "ITMS" => {
            return Profile {
                hr: false,
                an,
                dec: 2,
                tol: 0.5,
                unit: "Da",
            }
        }
        _ => {}
    }
    let mut res: Vec<f64> = scans
        .iter()
        .filter_map(|s| s.res)
        .filter(|r| *r != 0.0)
        .collect();
    crate::sortc::by(&mut res, f64::total_cmp);
    if an == "?" && !res.is_empty() && res[res.len() / 2] >= 10000.0 {
        return Profile {
            hr: true,
            an,
            dec: 4,
            tol: 10.0,
            unit: "ppm",
        };
    }
    low(&an)
}

/// True when the scans of the survey (`survey`) or of the product ions are read as high resolution (`Item.profile(level)["hr"]`).
/// The analyzer `Item.profile(level)["an"]` of the survey or of the product-ion scans (the most frequent one, `?` when unknown).
pub fn group_an(scans: &[Scan], survey: bool) -> String {
    let g: Vec<&Scan> = scans.iter().filter(|s| (s.level == 1) == survey).collect();
    mass_profile(&g).an
}

pub fn group_is_hr(scans: &[Scan], survey: bool) -> bool {
    let g: Vec<&Scan> = scans.iter().filter(|s| (s.level == 1) == survey).collect();
    mass_profile(&g).hr
}

/// `Item.profile(level, hr)` of the survey scans (`survey`) or of the product ions: whether they are read as high resolution (`on`), the decimals
/// of their m/z and the tolerance in ppm. `hr` = false (the page switched high resolution off) gives the low-resolution profile.
pub struct HrProfile {
    pub on: bool,
    pub dec: usize,
    pub ppm: f64,
}

pub fn hr_profile(scans: &[Scan], survey: bool, hr: bool) -> HrProfile {
    let g: Vec<&Scan> = scans.iter().filter(|s| (s.level == 1) == survey).collect();
    let p = if hr {
        mass_profile(&g)
    } else {
        mass_profile(&[])
    };
    HrProfile {
        on: p.hr,
        dec: p.dec as usize,
        ppm: if p.unit == "ppm" { p.tol } else { 0.0 },
    }
}

fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    crate::sortc::by(v, f64::total_cmp);
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

/// What the file is, from its scans: MS1, SIM, DDA / DIA / AIF / PRM, MSn.
fn acq(sc1: &[&Scan], sc2: &[&Scan]) -> Vec<&'static str> {
    let mut out = vec![];
    if !sc1.is_empty() {
        out.push("MS1");
    }
    if sc1.iter().any(|s| {
        s.filter
            .to_lowercase()
            .split_whitespace()
            .any(|w| w == "sim")
    }) {
        out.push("SIM");
    }
    if sc2.is_empty() {
        return out;
    }
    // the scan types (precursor ion, neutral loss/gain, EPI) outrank the generic MS2; the ordinary scans left over are classified as before
    for (code, _) in KINDS {
        if sc2.iter().any(|s| s.kind.as_deref() == Some(code)) {
            out.push(code);
        }
    }
    let plain: Vec<&Scan> = sc2.iter().copied().filter(|s| s.kind.is_none()).collect();
    if plain.is_empty() {
        return out;
    }
    let sc2 = &plain[..];
    let wins: Vec<(f64, f64)> = sc2.iter().filter_map(|s| s.iso).collect();
    if sc2.iter().map(|s| s.level).max().unwrap() >= 3 {
        out.push("MSn");
        return out;
    }
    if wins.is_empty() {
        out.push("AIF");
    } else {
        let mut widths: Vec<f64> = wins.iter().map(|(lo, hi)| hi - lo).collect();
        let width = median(&mut widths).unwrap();
        let centres: Vec<f64> = wins
            .iter()
            .map(|(lo, hi)| round((lo + hi) / 2.0, 1))
            .collect();
        let n = centres.len() as f64;
        let mut uniq = centres.clone();
        crate::sortc::by(&mut uniq, f64::total_cmp);
        uniq.dedup();
        let d = uniq.len() as f64;
        if width >= 5.0 && n / d.max(1.0) >= 3.0 {
            out.push("DIA");
        } else if !sc1.is_empty() && sc2.iter().all(|s| s.parent.is_some()) {
            out.push("DDA");
        } else if n / d.max(1.0) >= 5.0 && sc2.iter().map(|s| s.level).max().unwrap() == 2 {
            out.push("PRM");
        } else {
            out.push("MS2");
        }
    }
    out
}

fn ms2_exps(ms2: &[&Scan]) -> Vec<(Option<f64>, Option<f64>, usize)> {
    let mut cnt: Vec<((Option<f64>, Option<f64>), usize)> = vec![];
    for s in ms2 {
        let k = (
            s.precursor.filter(|p| *p != 0.0).map(|p| round(p, 1)),
            s.collision_energy,
        );
        match cnt.iter_mut().find(|(q, _)| *q == k) {
            Some(c) => c.1 += 1,
            None => cnt.push((k, 1)),
        }
    }
    let key = |k: &(Option<f64>, Option<f64>)| (k.0.unwrap_or(0.0), k.1.unwrap_or(0.0));
    crate::sortc::slice_by(&mut cnt, |a, b| {
        key(&a.0)
            .0
            .total_cmp(&key(&b.0).0)
            .then(key(&a.0).1.total_cmp(&key(&b.0).1))
    });
    let mut precs: Vec<Option<u64>> = cnt.iter().map(|(k, _)| k.0.map(f64::to_bits)).collect();
    crate::sortc::slice_ord(&mut precs);
    precs.dedup();
    if precs.len() <= 8 {
        return cnt.into_iter().map(|((p, ce), n)| (p, ce, n)).collect();
    }
    struct G {
        lo: Option<f64>,
        ce: Option<f64>,
        n: usize,
        votes: Vec<(Option<f64>, usize)>,
    }
    let mut groups: Vec<G> = vec![];
    for ((pr, ce), n) in cnt {
        match groups.last_mut() {
            Some(g)
                if pr.is_some()
                    && g.lo.is_some()
                    && pr.unwrap() - g.lo.unwrap() <= 0.5
                    && g.ce == ce =>
            {
                g.n += n;
                match g.votes.iter_mut().find(|v| v.0 == pr) {
                    Some(v) => v.1 += n,
                    None => g.votes.push((pr, n)),
                }
            }
            _ => groups.push(G {
                lo: pr,
                ce,
                n,
                votes: vec![(pr, n)],
            }),
        }
    }
    groups
        .into_iter()
        .map(|g| {
            let mut b = 0;
            for (i, v) in g.votes.iter().enumerate() {
                if v.1 > g.votes[b].1 {
                    b = i;
                }
            }
            (g.votes[b].0, g.ce, g.n)
        })
        .collect()
}

fn opt_str(o: &mut Out, v: &Option<String>) {
    match v {
        Some(s) => o.str(s),
        None => o.raw("null"),
    };
}

/// The JSON of one item as `Session.info()` lists it.
pub fn item_info<R: ReadAt>(
    idx: &mut Index<R>,
    facts: &Facts,
    meta: &Meta,
    part: Option<&Part>,
) -> Result<String> {
    let has1 = idx.scans.iter().any(|s| matches(s, part) && s.level == 1);
    let has2 = idx.scans.iter().any(|s| matches(s, part) && s.level >= 2);
    let pol = part.and_then(|p| p.pol).filter(|p| *p == 1 || *p == -1);
    let mut ranges = [None, None];
    for (i, (level, present)) in [(1u8, has1), (2u8, has2)].into_iter().enumerate() {
        if present {
            if let Some(si) = idx.section_of(level, pol) {
                ranges[i] = idx.mz_range(si)?;
            }
        }
    }
    let (r1, r2) = (ranges[0], ranges[1]);
    let all: Vec<&Scan> = idx.scans.iter().collect();
    let sc: Vec<&Scan> = all.iter().copied().filter(|s| matches(s, part)).collect();
    let ms1: Vec<&Scan> = sc.iter().copied().filter(|s| s.level == 1).collect();
    let ms2: Vec<&Scan> = sc.iter().copied().filter(|s| s.level >= 2).collect();
    let mut rt_min = sc.iter().map(|s| s.rt).fold(f64::INFINITY, f64::min);
    let mut rt_max = sc.iter().map(|s| s.rt).fold(f64::NEG_INFINITY, f64::max);
    if sc.is_empty() {
        rt_min = 0.0;
        rt_max = 0.0;
    }
    let mut pols: Vec<i8> = sc.iter().map(|s| s.polarity).filter(|p| *p != 0).collect();
    crate::sortc::slice_ord(&mut pols);
    pols.dedup();
    let mut polarity = match pols.as_slice() {
        [1] => "positive",
        [-1] => "negative",
        [] => "unknown",
        _ => "mixed",
    }
    .to_string();
    let mut precs: Vec<f64> = ms2
        .iter()
        .filter_map(|s| s.precursor.filter(|p| *p != 0.0))
        .map(|p| round(p, 1))
        .collect();
    crate::sortc::by(&mut precs, f64::total_cmp);
    precs.dedup();
    let nprof = sc.iter().filter(|s| s.profile).count();
    let mode = if !sc.is_empty() && nprof == sc.len() {
        "profile"
    } else if nprof == 0 {
        "centroid"
    } else {
        "mixed"
    };
    let n1 = all.iter().filter(|s| s.level == 1).count();
    let msn = crate::infusione::infusione(&idx.scans, facts.lc, &meta.file);
    let kind = match part {
        Some(p) => match p.mode {
            "ms1" => "full",
            "ms2" => "ms2",
            _ => "mrm",
        },
        None if all.len() > n1 && msn.as_ref().is_some_and(|m| m.motivo != "rt_costante") => {
            "infusion"
        }
        None if all.len() - n1 > 0 => "ms2",
        None if n1 > 0 => "full",
        None if facts.srm > 0 => "mrm",
        None => "empty",
    };
    let phys1: Vec<&Scan> = all.iter().copied().filter(|s| s.level == 1).collect();
    let phys2: Vec<&Scan> = all.iter().copied().filter(|s| s.level > 1).collect();
    let (p1, p2) = (mass_profile(&phys1), mass_profile(&phys2));
    let res = |v: &[&Scan]| {
        median(
            &mut v
                .iter()
                .filter_map(|s| s.res)
                .filter(|r| *r != 0.0)
                .collect::<Vec<_>>(),
        )
    };
    let dda = !phys1.is_empty() && !phys2.is_empty() && phys2.iter().all(|s| s.parent.is_some());
    let mode_of = |v: &[&Scan]| {
        if v.iter().take(50).any(|s| s.profile) {
            Some("profile")
        } else if !v.is_empty() {
            Some("centroid")
        } else {
            None
        }
    };
    let mut o = Out::default();
    o.raw("{\"file\":")
        .str(&meta.file)
        .raw(",\"label\":")
        .str(&meta.label)
        .raw(",\"sha256\":")
        .str(&meta.sha256)
        .raw(",\"time\":")
        .opt(meta.time);
    o.raw(",\"type\":")
        .str(&meta.typ)
        .raw(",\"conc\":")
        .opt(meta.conc)
        .raw(",\"cunit\":");
    opt_str(&mut o, &meta.cunit);
    o.raw(",\"rep\":");
    opt_str(&mut o, &meta.rep);
    o.raw(",\"scans\":")
        .int(sc.len() as i64)
        .raw(",\"ms1\":")
        .int(ms1.len() as i64)
        .raw(",\"ms2\":")
        .int(ms2.len() as i64);
    if sc.is_empty() {
        if let Some((a, b)) = facts.srm_rt {
            rt_min = a;
            rt_max = b;
        }
        polarity = facts.header_polarity.clone();
    }
    o.raw(",\"rt_min\":")
        .num(rt_min)
        .raw(",\"rt_max\":")
        .num(rt_max);
    o.raw(",\"mz_min\":")
        .opt(r1.map(|r| r.0))
        .raw(",\"mz_max\":")
        .opt(r1.map(|r| r.1))
        .raw(",\"mz2_min\":")
        .opt(r2.map(|r| r.0))
        .raw(",\"mz2_max\":")
        .opt(r2.map(|r| r.1));
    o.raw(",\"polarity\":")
        .str(&polarity)
        .raw(",\"precursors\":")
        .floats(&precs)
        .raw(",\"spectrum_mode\":")
        .str(mode)
        .raw(",\"ms2_exps\":[");
    for (i, (p, ce, n)) in ms2_exps(&ms2).iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"prec\":")
            .opt(*p)
            .raw(",\"ce\":")
            .opt(*ce)
            .raw(",\"n\":")
            .int(*n as i64)
            .raw("}");
    }
    o.raw("],\"chromatograms\":")
        .int(facts.n_chromatograms)
        .raw(",\"kind\":")
        .str(kind)
        .raw(",\"srm\":")
        .int(facts.srm as i64)
        .raw(",\"pda\":")
        .raw(if facts.pda { "true" } else { "false" });
    o.raw(",\"analog\":[");
    for (i, a) in facts.analog.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"i\":")
            .int(a.i)
            .raw(",\"kind\":")
            .str(&a.kind)
            .raw(",\"id\":")
            .str(&a.id)
            .raw(",\"unit\":")
            .str(&a.unit);
        o.raw(",\"t0\":")
            .num(a.t0)
            .raw(",\"t1\":")
            .num(a.t1)
            .raw(",\"n\":")
            .int(a.n)
            .raw("}");
    }
    o.raw("]");
    let prof = |o: &mut Out, p: &Profile| {
        o.raw("{\"hr\":")
            .raw(if p.hr { "true" } else { "false" })
            .raw(",\"an\":")
            .str(&p.an)
            .raw(",\"dec\":")
            .int(p.dec)
            .raw(",\"tol\":")
            .num(p.tol)
            .raw(",\"unit\":")
            .str(p.unit)
            .raw("}");
    };
    o.raw(",\"prof1\":");
    prof(&mut o, &p1);
    o.raw(",\"prof2\":");
    prof(&mut o, &p2);
    o.raw(",\"instrument\":")
        .str(&facts.instrument)
        .raw(",\"res1\":")
        .opt(res(&phys1))
        .raw(",\"res2\":")
        .opt(res(&phys2));
    o.raw(",\"nce\":").raw(
        if idx.run.get("nce").and_then(|v| v.bool()).unwrap_or(false) {
            "true"
        } else {
            "false"
        },
    );
    o.raw(",\"dda\":")
        .raw(if dda { "true" } else { "false" })
        .raw(",\"infusion\":")
        .raw(if msn.is_some() { "true" } else { "false" });
    crate::infusione::json(&mut o, msn.as_ref());
    o.raw(",\"max_level\":")
        .int(all.iter().map(|s| s.level as i64).max().unwrap_or(1))
        .raw(",\"acq\":[");
    for (i, a) in acq(&phys1, &phys2).iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a);
    }
    o.raw("],\"mode1\":");
    match mode_of(&phys1) {
        Some(m) => o.str(m),
        None => o.raw("null"),
    };
    o.raw(",\"mode2\":");
    match mode_of(&phys2) {
        Some(m) => o.str(m),
        None => o.raw("null"),
    };
    if let Some(p) = part {
        o.raw(",\"part\":").str(&p.tag);
        if p.mode == "ms2" {
            o.raw(",\"ms2_events\":[");
            for (i, s) in ms2.iter().enumerate() {
                o.raw(if i > 0 { "," } else { "" })
                    .raw("[")
                    .num(round(s.rt, 4))
                    .raw(",")
                    .opt(s.precursor.filter(|p| *p != 0.0).map(|p| round(p, 1)))
                    .raw("]");
            }
            o.raw("]");
        }
    }
    o.raw("}");
    Ok(o.0)
}

/// The `Facts` of a file, read from its mzML: instrument, chromatograms (SRM, PDA, analog traces). `need_mrm` also reads what a file without
/// scans needs (the polarity of the header and the retention-time range of the SRM chromatograms): it can mean reading a whole small file.
pub fn facts_from<M: ReadAt>(
    m: &mut crate::mzmlhead::Mzml<M>,
    scans_end: u64,
    n_chromatograms: i64,
    need_mrm: bool,
) -> Result<Facts> {
    use crate::json::round;
    let chroms = m.chromatograms(scans_end)?;
    let mut f = Facts {
        instrument: m.metadata()?.instrument,
        n_chromatograms,
        srm: chroms.iter().filter(|c| c.kind == "srm").count(),
        pda: chroms.iter().any(|c| c.kind == "pda"),
        ..Default::default()
    };
    for c in chroms
        .iter()
        .filter(|c| c.kind == "pressure" || c.kind == "analog")
    {
        let (t, _) = m.chromatogram(c)?;
        if let (Some(a), Some(b)) = (t.first(), t.last()) {
            f.analog.push(Analog {
                i: c.index as i64,
                kind: c.kind.clone(),
                id: c.id.clone(),
                unit: m.chromatogram_unit(c)?,
                t0: round(*a, 4),
                t1: round(*b, 4),
                n: t.len() as i64,
            });
        }
    }
    f.lc = crate::infusione::raw_lc(m)?;
    // The header polarity and the RT range of the SRM traces matter only for a file without scans or with SRM parts: reading a whole file (up to 256 MB) for them is
    // refused as `error.memoryBudget` for a big profile file (HRMS), and a file with scans and no SRM never shows them.
    if need_mrm && (scans_end == 0 || f.srm > 0) {
        f.header_polarity = m.header_polarity()?;
        let (mut lo, mut hi): (Option<f64>, Option<f64>) = (None, None);
        for c in chroms.iter().filter(|c| c.kind == "srm") {
            let (t, _) = m.chromatogram(c)?;
            if !t.is_empty() {
                let (a, b) = (
                    t.iter().cloned().fold(f64::INFINITY, f64::min),
                    t.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                );
                lo = Some(lo.map_or(a, |x| x.min(a)));
                hi = Some(hi.map_or(b, |x| x.max(b)));
            }
        }
        f.srm_rt = lo.zip(hi);
    }
    Ok(f)
}
