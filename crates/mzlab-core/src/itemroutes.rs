//! The routes of one open item that read what the mzML says about its acquisition: `Item.method`, `Item.analog`, the PDA trace and the delay between the PDA and the
//! mass spectrometer (`/api/method`, `/api/analog`, `/api/chrom?kind=pda`, `/api/delay`; `mzlab/explore.py` and `mzlab/app.py`). A high-resolution item also carries
//! `scan_params` (`scanparams.rs`).

use crate::analog::{analog_json, detector_delay};
use crate::error::{Error, Result};
use crate::idxread::{Index, ReadAt, Scan};
use crate::iteminfo::{file_parts, matches, Part};
use crate::json::{round, Out};
use crate::mzmlhead::{Chrom, Mzml};
use crate::routes::Routes;

/// The scans of the item `file` (`name` or `name#tag`): the experiment of a mixed file, else all.
fn item_scans<'a, R: ReadAt>(idx: &'a Index<R>, chroms: &[Chrom], file: &str) -> (Option<Part>, Vec<&'a Scan>) {
    let part = file.split_once('#').and_then(|(_, tag)| file_parts(&idx.scans, chroms.iter().any(|c| c.kind == "srm")).into_iter().find(|p| p.tag == tag));
    let sc = idx.scans.iter().filter(|s| matches(s, part.as_ref())).collect();
    (part, sc)
}

fn median(v: &mut [f64]) -> f64 {
    crate::sortc::by(v, f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// The value of the first `<cvParam accession="acc" ... value="...">` of a scan header (`_cv` of `reader/mzml.py`).
pub(crate) fn cv_value(h: &str, acc: &str) -> Option<String> {
    let key = format!("accession=\"{acc}\"");
    let mut from = 0;
    while let Some(p) = h[from..].find(&key) {
        let rest = &h[from + p + key.len()..];
        let tag = &rest[..rest.find('>').unwrap_or(rest.len())];
        if let Some(v) = tag.find("value=\"") {
            let t = &tag[v + 7..];
            return t.find('"').map(|e| t[..e].to_string());
        }
        from += p + 1;
    }
    None
}

/// `Run.scan_window`: (lowest, highest) m/z of the scan windows of the first 50 scans.
fn scan_window<R: ReadAt, M: ReadAt>(idx: &Index<R>, mzml: &mut M) -> Result<Option<(f64, f64)>> {
    let mut win: Option<(f64, f64)> = None;
    for s in idx.scans.iter().take(50) {
        let h = crate::sorgente::testo_scansione(mzml, s)?;
        let (Some(a), Some(z)) = (cv_value(&h, "MS:1000501").filter(|v| !v.is_empty()), cv_value(&h, "MS:1000500").filter(|v| !v.is_empty())) else { continue };
        let (Ok(a), Ok(z)) = (a.trim().parse::<f64>(), z.trim().parse::<f64>()) else { continue };
        win = Some(win.map_or((a, z), |(lo, hi)| (lo.min(a), hi.max(z))));
    }
    Ok(win)
}

fn strings(o: &mut Out, v: &[String]) {
    o.raw("[");
    for (i, s) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(s);
    }
    o.raw("]");
}

fn polarity_of(sc: &[&Scan]) -> &'static str {
    let mut pols: Vec<i8> = sc.iter().map(|s| s.polarity).filter(|p| *p != 0).collect();
    crate::sortc::slice_ord(&mut pols);
    pols.dedup();
    match pols.as_slice() {
        [1] => "positive",
        [-1] => "negative",
        [] => "unknown",
        _ => "mixed",
    }
}

/// `Item.method()` of a low-resolution item: how the data were acquired, as far as the mzML tells. `file` is the name of the item (with `#tag` for a part).
pub fn item_method<R: ReadAt, M: ReadAt>(idx: &Index<R>, m: &mut Mzml<M>, file: &str) -> Result<String> {
    let chroms = m.chromatograms(idx.scans.last().map_or(0, |s| s.end))?;
    let (part, sc) = item_scans(idx, &chroms, file);
    let n1 = idx.scans.iter().filter(|s| s.level == 1).count();
    let kind = match &part {
        Some(p) => match p.mode {
            "ms1" => "full",
            "ms2" => "ms2",
            _ => "mrm",
        },
        None if idx.scans.len() > n1 => "ms2",
        None if n1 > 0 => "full",
        None if chroms.iter().any(|c| c.kind == "srm") => "mrm",
        None => "empty",
    };
    let md = m.metadata()?;
    let ms2: Vec<&&Scan> = sc.iter().filter(|s| s.level >= 2).collect();
    let rts: Vec<f64> = sc.iter().filter(|s| s.level == if kind == "full" { 1 } else { 2 }).map(|s| s.rt).collect();
    let cycle = (rts.len() > 2).then(|| median(&mut rts.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>()) * 60.0);
    let polarity = if kind == "mrm" || sc.is_empty() { m.header_polarity()? } else { polarity_of(&sc).to_string() };
    let win = scan_window(idx, &mut m.r)?;
    let mut filters: Vec<&str> = sc.iter().map(|s| s.filter.as_str()).filter(|f| !f.is_empty()).collect();
    crate::sortc::slice_ord(&mut filters);
    filters.dedup();
    let mut precs: Vec<f64> = ms2.iter().filter_map(|s| s.precursor.filter(|p| *p != 0.0)).map(|p| round(p, 2)).collect();
    crate::sortc::by(&mut precs, f64::total_cmp);
    precs.dedup();
    let mut ces: Vec<f64> = ms2.iter().filter_map(|s| s.collision_energy).collect();
    crate::sortc::by(&mut ces, f64::total_cmp);
    ces.dedup();
    let mut o = Out::default();
    o.raw("{\"file\":").str(file).raw(",\"kind\":").str(kind).raw(",\"instrument\":").str(&md.instrument).raw(",\"serial\":").str(&md.serial);
    o.raw(",\"components\":");
    strings(&mut o, &md.components);
    o.raw(",\"analyzers\":");
    strings(&mut o, &md.analyzers);
    o.raw(",\"source_files\":");
    strings(&mut o, &md.source_files);
    o.raw(",\"software\":");
    strings(&mut o, &md.software);
    let fold = |f: fn(f64, f64) -> f64, init: f64| if sc.is_empty() { 0.0 } else { sc.iter().map(|s| s.rt).fold(init, f) };
    o.raw(",\"polarity\":").str(&polarity).raw(",\"scans\":").int(sc.len() as i64);
    o.raw(",\"rt_min\":").num(fold(f64::min, f64::INFINITY)).raw(",\"rt_max\":").num(fold(f64::max, f64::NEG_INFINITY));
    o.raw(",\"pda\":").raw(if chroms.iter().any(|c| c.kind == "pda") { "true" } else { "false" }).raw(",\"cycle_s\":").opt(cycle).raw(",\"scan_window\":");
    match win {
        Some((a, b)) => o.raw("[").num(a).raw(",").num(b).raw("]"),
        None => o.raw("null"),
    };
    o.raw(",\"filters\":");
    strings(&mut o, &filters.iter().take(6).map(|s| s.to_string()).collect::<Vec<_>>());
    o.raw(",\"precursors\":").floats(&precs).raw(",\"ce\":").floats(&ces).raw(",\"transitions\":[");
    for (i, c) in chroms.iter().filter(|c| c.kind == "srm").enumerate() {
        o.raw(if i > 0 { ",{\"q1\":" } else { "{\"q1\":" }).opt(c.q1).raw(",\"q3\":").opt(c.q3).raw(",\"ce\":").opt(c.ce).raw(",\"name\":").str(&c.name).raw(",\"dwell\":").opt(c.dwell).raw("}");
    }
    o.raw("]");
    crate::scanparams::write(idx, &mut m.r, &md.instrument, &mut o)?;
    o.raw("}");
    Ok(o.0)
}

/// `/api/analog`: one pressure or analog trace, `n` points at most per extreme (10 to 20000).
pub fn analog<M: ReadAt>(m: &mut Mzml<M>, chroms: &[Chrom], i: i64, n: i64) -> Result<String> {
    let none = || Error::new("err.analog.none").with("i", i);
    let c = chroms.iter().find(|c| c.index as i64 == i && (c.kind == "pressure" || c.kind == "analog")).ok_or_else(none)?;
    let (t, y) = m.chromatogram(c)?;
    if t.is_empty() {
        return Err(none());
    }
    let unit = m.chromatogram_unit(c)?;
    Ok(analog_json(i, &c.kind, &c.id, &unit, &t, &y, n.clamp(10, 20000) as usize))
}

/// `/api/chrom?kind=pda`: the UV trace of the PDA (own time axis), empty when the file has none.
pub fn pda_trace<M: ReadAt>(m: &mut Mzml<M>, chroms: &[Chrom]) -> Result<String> {
    let (t, y) = match chroms.iter().find(|c| c.kind == "pda") {
        Some(c) => m.chromatogram(c)?,
        None => (vec![], vec![]),
    };
    let mut o = Out::default();
    o.raw("{\"rt\":").rounded(&t, 4).raw(",\"y\":").rounded(&y, 1).raw("}");
    Ok(o.0)
}

/// `/api/map?kind=pda`: the 2D DAD optical matrix (retention time vs wavelength) from the native reader when available.
pub fn pda_map<M: ReadAt>(m: &mut Mzml<M>) -> Result<String> {
    if let Some((l, c)) = m.r.lettore() {
        if let Some(json) = l.dad_map(c)? {
            return Ok(json);
        }
    }
    Err(Error::new("err.pda.none"))
}

/// `/api/chrom` of an MRM file (no scans): the first chromatogram of the mzML with that kind (`tic`, `bpc`, `srm`), empty when it has none. The m/z window, the
/// precursor and the filter do not apply to it.
pub fn mrm_chrom<M: ReadAt>(m: &mut Mzml<M>, chroms: &[Chrom], kind: &str) -> Result<String> {
    let (t, y) = match chroms.iter().find(|c| c.kind == kind) {
        Some(c) => m.chromatogram(c)?,
        None => (vec![], vec![]),
    };
    let mut o = Out::default();
    o.raw("{\"rt\":").rounded(&t, 4).raw(",\"y\":").rounded(&y, 1).raw("}");
    Ok(o.0)
}

impl<R: ReadAt> Routes<R> {
    /// `/api/delay` of the item `item` (`name` or `name#tag`): a part of a mixed file has no PDA of its own to compare, unless the file has one (then the part reads its own
    /// scans: not served, `err.route.unavailable`).
    pub fn delay_item<M: ReadAt>(&mut self, m: &mut Mzml<M>, chroms: &[Chrom], item: &str) -> Result<String> {
        if item.contains('#') && chroms.iter().any(|c| c.kind == "pda") {
            return Err(Error::new("err.route.unavailable"));
        }
        self.delay(m, chroms)
    }

    /// `/api/delay`: `{"delay": {delay, peaks, corr} | null}`; `kind_mrm` files read the TIC from their chromatograms.
    pub fn delay<M: ReadAt>(&mut self, m: &mut Mzml<M>, chroms: &[Chrom]) -> Result<String> {
        let mut o = Out::default();
        o.raw("{\"delay\":");
        let Some(pda) = chroms.iter().find(|c| c.kind == "pda") else { return Ok(o.raw("null}").0.clone()) };
        let (ta, ya) = m.chromatogram(pda)?;
        let (tb, yb) = if self.has_scans() {
            match self.idx.section_of(1, None) {
                Some(si) => {
                    let ar = self.idx.arrays(si)?;
                    (ar.rt.clone(), ar.sumint.clone())
                }
                None => (vec![], vec![]),
            }
        } else {
            match chroms.iter().find(|c| c.kind == "tic") {
                Some(c) => m.chromatogram(c)?,
                None => (vec![], vec![]),
            }
        };
        match detector_delay(&ta, &ya, &tb, &yb) {
            Some(d) => o.raw("{\"delay\":").num(d.delay).raw(",\"peaks\":").int(d.peaks as i64).raw(",\"corr\":").num(d.corr).raw("}}"),
            None => o.raw("null}"),
        };
        Ok(o.0)
    }
}
