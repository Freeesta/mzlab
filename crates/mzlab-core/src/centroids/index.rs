//! An index with profile scans as an index of centroids, in memory: the engine of `spectra.centroid` (`estimate`, `reduce`) applied to the peaks of the
//! index instead of an mzML, so a detection can centroid first without a file in between. The scan rows are the ones of the source with the profile
//! flag cleared; the sections are rebuilt by the ordinary builder, so the result reads like an index made from the centroid mzML.

use std::io::Cursor;

use super::{estimate, Buf, Estimate, Over};
use crate::error::{Error, Result};
use crate::idxread::{Index, ReadAt};
use crate::json::{self, Out, Value};
use crate::mzidx::{self, ScanTable};
use crate::mzml::Spectrum;

/// The index of the centroids of `index` and the estimate they were made with.
pub struct Centroided {
    pub bytes: Vec<u8>,
    pub estimate: Estimate,
    /// Centroids flagged merged and saturated.
    pub flagged: (usize, usize),
}

/// How many scans of `level` are profile, of the first `probe` ones (`(scans, profile)`).
pub fn profile_share<R: ReadAt>(index: &Index<R>, level: u8, probe: usize) -> (usize, usize) {
    index
        .scans
        .iter()
        .filter(|s| s.level == level)
        .take(probe)
        .fold((0, 0), |(n, p), s| (n + 1, p + usize::from(s.profile)))
}

/// The section of each scan and its position in it (scans of one level and polarity, in table order).
fn places(index: &Index<impl ReadAt>) -> Result<Vec<(usize, usize)>> {
    let mut count = vec![0usize; index.sections.len()];
    let mut out = Vec::with_capacity(index.scans.len());
    for s in index.scans.iter() {
        let p = match s.polarity {
            1 => "+",
            -1 => "-",
            _ => "?",
        };
        let key = format!("{}/{p}", s.level);
        let si = index
            .keys
            .iter()
            .find(|(n, _)| *n == key)
            .map(|(_, i)| *i)
            .ok_or_else(|| Error::new("err.file.index").with("detail", "section"))?;
        out.push((si, count[si]));
        count[si] += 1;
    }
    if count
        .iter()
        .zip(index.sections.iter())
        .enumerate()
        .any(|(i, (&c, s))| s.n_scans != c && index.keys.iter().any(|(n, k)| *k == i && !n.ends_with("/all")))
    {
        return Err(Error::new("err.file.index").with("detail", "section"));
    }
    Ok(out)
}

/// The rows of `index` with the profile flag (column 9) cleared and the mzML offsets (columns 0 and 1) zero, as the rows of an mzML index read them.
fn rows_without_profile<R: ReadAt>(index: &mut Index<R>) -> Result<Vec<u8>> {
    let rows = index.rows_bytes()?;
    let mut out = Out(String::from("["));
    let mut first = true;
    json::each_element(&rows, |row| {
        let Value::Arr(cols) = row else {
            return Err(Error::new("error.badIndex"));
        };
        let mut cols = cols.clone();
        for (i, c) in cols.iter_mut().enumerate() {
            match i {
                0 | 1 => *c = Value::Num(0.0),
                9 => *c = Value::Bool(false),
                _ => {}
            }
        }
        if !first {
            out.raw(",");
        }
        first = false;
        json::write_value(&mut out, &Value::Arr(cols));
        Ok(())
    })?;
    out.raw("]");
    Ok(out.0.into_bytes())
}

/// The peaks of every section by position: `(m/z, intensity)` of each scan, the stream of `each_peak` gathered scan by scan.
type Peaks = Vec<Vec<(Vec<f64>, Vec<f64>)>>;

fn peaks_by_section<R: ReadAt>(index: &mut Index<R>) -> Result<Peaks> {
    let mut by_section: Peaks = vec![Vec::new(); index.sections.len()];
    let keyed: Vec<usize> = index
        .keys
        .iter()
        .filter(|(n, _)| !n.ends_with("/all"))
        .map(|(_, i)| *i)
        .collect();
    for si in keyed {
        let (n_scans, n_peaks) = (index.sections[si].n_scans, index.sections[si].n_peaks);
        let mut scans = vec![(Vec::new(), Vec::new()); n_scans];
        index.each_peak(si, 0, n_peaks, |pos, mz, y| {
            if let Some(s) = scans.get_mut(pos) {
                s.0.push(mz);
                s.1.push(y);
            }
        })?;
        by_section[si] = scans;
    }
    Ok(by_section)
}

/// Every scan as a spectrum, the profile ones reduced to their centroids with the estimate `e`; with the counts of centroids flagged merged and saturated.
fn reduced<R: ReadAt>(index: &Index<R>, places: &[(usize, usize)], by_section: &Peaks, e: &Estimate) -> (Vec<Spectrum>, (usize, usize)) {
    let mut flagged = (0, 0);
    let mut buf = Buf::default();
    let mut spectra: Vec<Spectrum> = Vec::with_capacity(index.scans.len());
    for (k, s) in index.scans.iter().enumerate() {
        let (si, pos) = places[k];
        let (mz, y) = by_section[si].get(pos).cloned().unwrap_or_default();
        let mut sp = Spectrum {
            index: k,
            level: s.level,
            rt: s.rt,
            polarity: s.polarity,
            profile: s.profile,
            intensity: y.iter().map(|&v| v as f32).collect(),
            wide: Some(y),
            mz,
            ..Spectrum::default()
        };
        if s.profile {
            super::convert::reduce(&mut sp, &e.params, &mut buf, &mut flagged);
        }
        spectra.push(sp);
    }
    (spectra, flagged)
}

/// The profile scans of `index` reduced to their centroids (`None` when it has no profile scan). `over` replaces the width and the threshold the
/// estimate finds, as in the `spectra.centroid` step. The scans that are centroids already, and every other scan, keep their points.
pub fn centroid_index<R: ReadAt>(index: &mut Index<R>, over: Over) -> Result<Option<Centroided>> {
    if !index.scans.iter().any(|s| s.profile) {
        return Ok(None);
    }
    let places = places(index)?;
    let by_section = peaks_by_section(index)?;
    // the estimate is made on at most 64 profile scans spread over the run, the way the conversion of an mzML does it
    let profile: Vec<usize> = (0..index.scans.len())
        .filter(|&k| index.scans[k].profile)
        .collect();
    let every = profile.len().div_ceil(64).max(1);
    let picked: Vec<&(Vec<f64>, Vec<f64>)> = profile
        .iter()
        .step_by(every)
        .filter_map(|&k| by_section[places[k].0].get(places[k].1))
        .filter(|s| !s.0.is_empty())
        .collect();
    if picked.is_empty() {
        return Ok(None);
    }
    let refs: Vec<(&[f64], &[f64])> = picked
        .iter()
        .map(|s| (s.0.as_slice(), s.1.as_slice()))
        .collect();
    let mut e = estimate(&refs, 0.7);
    if let Some(w) = over.fwhm {
        e.params.w = w;
        e.params.wm = None;
    }
    e.params.k = over.threshold.unwrap_or(e.params.k);
    let (spectra, flagged) = reduced(index, &places, &by_section, &e);
    let table = ScanTable {
        level: index.scans.iter().map(|s| s.level).collect(),
        polarity: index.scans.iter().map(|s| s.polarity).collect(),
        rt: index.scans.iter().map(|s| s.rt).collect(),
        tic: index.scans.iter().map(|s| s.tic).collect(),
        scans_json: rows_without_profile(index)?,
        source_json: json_of(&index.source),
        run_json: json_of(&run_with_chroms(&index.run)),
        hr1: false,
        declared: 0,
    };
    let mut out = Cursor::new(Vec::new());
    mzidx::build(
        &table,
        &mut |on| {
            spectra.iter().for_each(|s| on(s.clone()));
            Ok(())
        },
        &mut out,
        mzidx::DEFAULT_BUDGET,
        false,
    )?;
    e.merged = flagged.0;
    e.saturated = flagged.1;
    Ok(Some(Centroided {
        bytes: out.into_inner(),
        estimate: e,
        flagged: (flagged.0, flagged.1),
    }))
}

/// `run` as an mzML index keeps it: the index of a vendor file has no chromatogram list, and the rebuilt one (read as an mzML index) needs one. The
/// centroids are for untargeted detection, which does not read chromatograms, so the list is empty.
fn run_with_chroms(run: &Value) -> Value {
    let mut o = match run {
        Value::Obj(o) => o.clone(),
        _ => Vec::new(),
    };
    if !o.iter().any(|(k, _)| k == "chromatograms") {
        o.push(("chromatograms".into(), Value::Arr(Vec::new())));
    }
    Value::Obj(o)
}

fn json_of(v: &Value) -> String {
    let mut o = Out(String::new());
    json::write_value(&mut o, v);
    o.0
}
