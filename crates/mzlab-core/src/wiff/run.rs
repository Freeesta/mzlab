//! A sample as the `Run` that the rest of the program reads: scans become spectra (profile, empty ones left out as msconvert does)
//! and the MRM blocks become a TIC and one SRM chromatogram per transition.

use std::io::Write;

use super::window::Window;
use super::{scan, Wiff};
use crate::centroidi::{self, Buf};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::methodinfo::Kind;
use crate::metodo::sha1_hex;
use crate::mzml::{ChromKind, Chromatogram, Run, Spectrum};
use crate::mzmlwrite::{Header, MzmlWriter};

impl Wiff {
    fn mrm_transitions(&self, experiment: usize) -> Vec<(Option<f64>, Option<f64>)> {
        let kind = self.method.as_ref().and_then(|m| m.experiments.get(experiment)).map(|e| &e.kind);
        match kind {
            Some(Kind::Mrm(t)) => t.iter().map(|t| (Some(t.q1), Some(t.q3))).collect(),
            _ => Vec::new(),
        }
    }

    /// Spectra and chromatograms of the sample. Reads every block of the `.wiff.scan` once.
    pub fn run<R: ReadAt>(&self, scan: &mut R) -> Result<Run> {
        let mut run = Run::default();
        run.chromatograms = self.visit(scan, &mut |s| {
            run.spectra.push(s);
            Ok(())
        })?;
        Ok(run)
    }

    /// Gives the spectra one at a time to `on` (profile; the empty scans are left out) and returns the chromatograms of the MRM blocks.
    pub fn visit<R: ReadAt>(&self, scan: &mut R, on: &mut dyn FnMut(Spectrum) -> Result<()>) -> Result<Vec<Chromatogram>> {
        let mut mrm: Vec<Mrm> = (0..self.experiments.len().max(1)).map(|_| Mrm::default()).collect();
        let mut n = 0;
        for r in &self.records {
            if let Some(p) = scan::read_scan(scan, self.scan_len, r.at, r.end)? {
                if p.mz.is_empty() {
                    continue;
                }
                let e = self.experiments.get(r.experiment).cloned().unwrap_or_default();
                on(Spectrum {
                    index: n,
                    id: format!("sample={} period=1 cycle={} experiment={}", self.sample, r.cycle + 1, r.experiment + 1),
                    level: e.level.max(1),
                    rt: r.time_min,
                    polarity: self.polarity,
                    profile: true,
                    filter: String::new(),
                    precursor: e.precursor,
                    mz: p.mz,
                    intensity: p.intensity.iter().map(|&v| v as f32).collect(),
                    wide: Some(p.intensity),
                })?;
                n += 1;
            } else if let Some(v) = scan::read_mrm(scan, self.scan_len, r.at, r.size as usize / 4)? {
                let m = &mut mrm[r.experiment];
                m.time.push(r.time_min);
                m.tic.push(r.tic);
                m.cols.resize(v.len(), Vec::new());
                v.iter().zip(&mut m.cols).for_each(|(x, col)| col.push(*x));
            }
        }
        let mut chroms = Vec::new();
        self.add_chromatograms(&mut chroms, mrm);
        Ok(chroms)
    }

    /// How many spectra `visit` will give (reads the head of each block only).
    pub fn spectrum_count<R: ReadAt>(&self, scan: &mut R) -> Result<usize> {
        let mut n = 0;
        for r in &self.records {
            n += scan::point_count(scan, self.scan_len, r.at, r.end)?.is_some_and(|c| c > 0) as usize;
        }
        Ok(n)
    }

    /// Writes the sample as mzML (`name`: the `.wiff`), one spectrum at a time; the `.wiff.scan` is read block by block.
    pub fn to_mzml<R: ReadAt, W: Write>(&self, scan: &mut R, out: W, name: &str) -> Result<()> {
        self.to_mzml_progress(scan, out, name, false, &mut |_, _| {})
    }

    /// Parameters of the centroids from at most 64 equally spaced scans (read and dropped), as `centroidi::estimate`.
    fn centroid_params<R: ReadAt>(&self, scan: &mut R) -> Result<centroidi::Params> {
        let mut good = Vec::new();
        for r in &self.records {
            if scan::point_count(scan, self.scan_len, r.at, r.end)?.is_some_and(|c| c > 0) {
                good.push((r.at, r.end));
            }
        }
        let n = good.len().min(64);
        let pick = (0..n).filter_map(|i| good.get(i * good.len() / n).copied());
        let mut scans = Vec::new();
        for (at, end) in pick {
            if let Some(p) = scan::read_scan(scan, self.scan_len, at, end)? {
                scans.push((p.mz, p.intensity));
            }
        }
        let refs: Vec<(&[f64], &[f64])> = scans.iter().map(|(a, b)| (a.as_slice(), b.as_slice())).collect();
        Ok(centroidi::estimate(&refs, 0.7).params)
    }

    /// As `to_mzml`; with `centroid` every spectrum is reduced to one point per peak (`centroidi`, MS:1000035). `progress(done, total)` is called every few spectra with the spectra written and the spectra to write.
    pub fn to_mzml_progress<R: ReadAt, W: Write>(&self, scan: &mut R, out: W, name: &str, centroid: bool, progress: &mut dyn FnMut(usize, usize)) -> Result<()> {
        let mut scan = Window::new(scan, self.scan_len);
        let (instrument, run_params) = self.instrument_terms();
        let header = Header {
            run: name.trim_end_matches(".wiff").to_string(),
            sources: vec![name.to_string(), format!("{name}.scan")],
            source_sha1: self.wiff_sha1.iter().cloned().chain(sha1_hex(&mut scan, self.scan_len).ok()).collect(),
            acquired: self.created.clone(),
            run_params,
            instrument: Some(instrument),
            components: [("source", "MS:1000073", "electrospray ionization"), ("analyzer", "MS:1000081", "quadrupole"), ("analyzer", "MS:1000081", "quadrupole"), ("analyzer", "MS:1000078", "axial ejection linear ion trap"), ("detector", "MS:1000253", "electron multiplier")]
                .iter()
                .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
                .collect(),
            software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
            centroided: centroid,
            ..Default::default()
        };
        let cp = if centroid { Some(self.centroid_params(&mut scan)?) } else { None };
        let mut buf = Buf::default();
        let mut w = MzmlWriter::new(out, &header)?;
        let count = self.spectrum_count(&mut scan)?;
        if count > 0 {
            w.open_spectra(count)?;
        }
        let mut done = 0;
        progress(0, count);
        let chroms = self.visit(&mut scan, &mut |mut s| {
            if let Some(p) = &cp {
                let c = centroidi::centroids(&s.mz, s.wide.as_deref().unwrap_or(&[]), p, &mut buf);
                s.mz = c.iter().map(|c| c.mz).collect();
                let v: Vec<f64> = c.iter().map(|c| c.value).collect();
                s.intensity = v.iter().map(|&x| narrow(x)).collect();
                s.wide = Some(v);
                s.profile = false;
            }
            w.spectrum(&s)?;
            done += 1;
            if done % 32 == 0 {
                progress(done, count);
            }
            Ok(())
        })?;
        progress(count, count);
        if count > 0 {
            w.close_spectra()?;
        }
        if !chroms.is_empty() {
            w.open_chromatograms(chroms.len())?;
            for c in &chroms {
                w.chromatogram(c)?;
            }
            w.close_chromatograms()?;
        }
        w.finish()?;
        Ok(())
    }

    /// The instrument of the header: the CV term of the model the Log names (the known models), otherwise the generic `instrument model` and
    /// the name as a userParam of the run; without a Log, the 3200 QTRAP this reader was written for.
    fn instrument_terms(&self) -> ((String, String), Vec<(String, String)>) {
        match self.model.as_deref() {
            None => (("MS:1000651".into(), "3200 QTRAP".into()), Vec::new()),
            Some(m) if m.replace(' ', "").contains("3200QTRAP") => (("MS:1000651".into(), "3200 QTRAP".into()), Vec::new()),
            Some(m) => (("MS:1000031".into(), "instrument model".into()), vec![("instrument model".into(), m.to_string())]),
        }
    }

    fn add_chromatograms(&self, chroms: &mut Vec<Chromatogram>, mrm: Vec<Mrm>) {
        let unit = "number of detector counts";
        for (e, Mrm { time, tic, cols }) in mrm.into_iter().enumerate().filter(|(_, m)| !m.time.is_empty()) {
            if chroms.is_empty() {
                chroms.push(Chromatogram { index: 0, id: "TIC".into(), kind: ChromKind::Tic, q1: None, q3: None, unit: unit.into(), time: time.clone(), intensity: tic });
            }
            let known = self.mrm_transitions(e);
            for (j, col) in cols.into_iter().enumerate() {
                let (q1, q3) = known.get(j).copied().unwrap_or((None, None));
                let mass = match (q1, q3) {
                    (Some(a), Some(b)) => format!(" Q1={a} Q3={b}"),
                    _ => String::new(),
                };
                let id = format!("SRM SIC{mass} sample={} period=1 experiment={} transition={j}", self.sample, e + 1);
                chroms.push(Chromatogram { index: chroms.len(), id, kind: ChromKind::Srm, q1, q3, unit: unit.into(), time: time.clone(), intensity: col });
            }
        }
    }
}

/// The MRM blocks of one experiment: times (min), TIC, and one column per transition.
#[derive(Default)]
struct Mrm {
    time: Vec<f64>,
    tic: Vec<f64>,
    cols: Vec<Vec<f64>>,
}

/// The f32 copy of a value written as f64 (`Spectrum::intensity` next to `wide`).
#[allow(clippy::cast_possible_truncation)]
fn narrow(x: f64) -> f32 {
    x as f32
}
