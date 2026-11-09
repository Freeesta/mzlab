//! A sample as the `Run` that the rest of the program reads: scans become spectra (profile, empty ones left out as msconvert does)
//! and the chromatograms are those of `chromatograms.rs` (a TIC per experiment, SRM / SIM per transition or ion, BPC of a full scan).

#[cfg(not(feature = "estesi"))]
use super::chromatograms::Blocks;
use super::{scan, Wiff};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::methodinfo::{Kind, Transition};
#[cfg(not(feature = "estesi"))]
use crate::mzml::Chromatogram;
use crate::mzml::{Run, Spectrum};
use crate::mzmlwrite::Header;

impl Wiff {
    pub(super) fn mrm_transitions(&self, experiment: usize) -> Vec<Transition> {
        let kind = self
            .method
            .as_ref()
            .and_then(|m| m.experiments.iter().find(|x| x.index == experiment))
            .map(|e| &e.kind);
        match kind {
            Some(Kind::Mrm(t)) => t.clone(),
            _ => Vec::new(),
        }
    }

    /// The id of the SRM chromatogram of transition `j` of experiment `e` (0-based), as msconvert spells it (Q1, Q3, sample, period, experiment, transition),
    /// then the collision energy, dwell (ms) (target RT in min and window in s when scheduled) and name of the method when it has them; the mzML writer and `mzmlhead` read them back from it.
    pub(super) fn srm_id(&self, e: usize, j: usize, t: Option<&Transition>) -> String {
        let mut id = String::from("SRM SIC");
        if let Some(t) = t {
            id += &format!(" Q1={} Q3={}", t.q1, t.q3);
        }
        id += &format!(
            " sample={} period=1 experiment={} transition={j}",
            self.sample,
            e + 1
        );
        if let Some(t) = t {
            if let Some(ce) = t.ce {
                id += &format!(" ce={ce}");
            }
            match self.experiments.get(e).and_then(|x| x.window_s) {
                Some(w) => id += &format!(" rt={} window={w}", t.dwell),
                None => id += &format!(" dwell={}", t.dwell),
            }
            if !t.name.is_empty() {
                id += &format!(" name={}", t.name);
            }
        }
        id
    }

    /// The id of the SIM chromatogram of ion `j` of experiment `e` (0-based), as msconvert spells it (`SIM SIC Q1=<m/z> sample period experiment transition`),
    /// then the dwell (ms), the polarity of the experiment and the name of the method when it has them.
    pub(super) fn sim_id(&self, e: usize, j: usize, t: Option<&Transition>) -> String {
        let mut id = String::from("SIM SIC");
        if let Some(t) = t {
            id += &format!(" Q1={}", t.q1);
        }
        id += &format!(
            " sample={} period=1 experiment={} transition={j}",
            self.sample,
            e + 1
        );
        if let Some(t) = t {
            id += &format!(" dwell={}", t.dwell);
        }
        let pol = self
            .experiments
            .get(e)
            .map_or(self.polarity, |x| self.polarity_of(x));
        match pol {
            1 => id += " polarity=positive",
            -1 => id += " polarity=negative",
            _ => {}
        }
        if let Some(t) = t.filter(|t| !t.name.is_empty()) {
            id += &format!(" name={}", t.name);
        }
        id
    }

    /// Id, kind, Q1 and Q3 of the chromatogram of column `j` of the block experiment `e`: SRM for an MRM experiment, SIM (no Q3) for a Q1MI / Q3MI one.
    pub(super) fn trace_of(
        &self,
        e: usize,
        j: usize,
        t: Option<&Transition>,
    ) -> (String, crate::mzml::ChromKind, Option<f64>, Option<f64>) {
        if super::modes::sim(self.experiments.get(e).and_then(|x| x.code)) {
            let id = self.sim_id(e, j, t);
            return (id, crate::mzml::ChromKind::Sic, t.map(|t| t.q1), None);
        }
        let id = self.srm_id(e, j, t);
        (
            id,
            crate::mzml::ChromKind::Srm,
            t.map(|t| t.q1),
            t.map(|t| t.q3),
        )
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
    #[cfg(not(feature = "estesi"))]
    pub fn visit<R: ReadAt>(
        &self,
        scan: &mut R,
        on: &mut dyn FnMut(Spectrum) -> Result<()>,
    ) -> Result<Vec<Chromatogram>> {
        let mut blocks = Blocks::new(self.experiments.len().max(1));
        let mut n = 0;
        for r in self.records.iter().filter(|r| !r.cut(self.scan_len)) {
            if let Some(p) = scan::read_scan(scan, self.scan_len, r.at, r.end)? {
                if p.mz.is_empty() {
                    continue;
                }
                if self.is_full_scan(r.experiment) {
                    blocks.add_tic(r, p.intensity.iter().copied().reduce(f64::max));
                }
                on(self.spectrum_of(n, r, p))?;
                n += 1;
            } else if self.scan_experiment(r.experiment) || self.empty_sim_row(r) {
                continue;
            } else if let Some(v) = dense_mrm(scan, self.scan_len, r.at, r.size as usize / 4)? {
                blocks.add_dense(r, v);
            }
        }
        let mut chroms = self.chromatograms_of(blocks);
        if let Some(dad) = &self.dad {
            chroms.push(dad.twc_chromatogram(chroms.len()));
        }
        Ok(chroms)
    }

    /// The spectrum `n` (counting the non-empty scans) made of the block `r` and its points: the one `visit` gives and `lettore::WiffReader::scansione` serves.
    pub(super) fn spectrum_of(&self, n: usize, r: &super::Record, p: scan::Points) -> Spectrum {
        let e = self
            .experiments
            .get(r.experiment)
            .cloned()
            .unwrap_or_default();
        Spectrum {
            index: n,
            id: format!(
                "sample={} period=1 cycle={} experiment={}",
                self.sample,
                r.cycle + 1,
                r.experiment + 1
            ),
            level: e.level.max(1),
            rt: r.time_min,
            polarity: self.polarity_of(&e),
            profile: true,
            filter: String::new(),
            precursor: e.precursor,
            mz: p.mz,
            intensity: p.intensity.iter().map(|&v| v as f32).collect(),
            wide: Some(p.intensity),
        }
    }

    /// The chromatograms of the MRM blocks only (no scan is decoded). `counts`: the point count of each record's head when the reader already read them.
    #[cfg(not(feature = "estesi"))]
    pub fn chromatograms<R: ReadAt>(
        &self,
        scan: &mut R,
        counts: Option<&[Option<u32>]>,
    ) -> Result<Vec<Chromatogram>> {
        let mut blocks = Blocks::new(self.experiments.len().max(1));
        for (i, r) in self
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.cut(self.scan_len))
        {
            let count = match counts.and_then(|c| c.get(i)) {
                Some(&c) => c,
                None => scan::point_count(scan, self.scan_len, r.at, r.end)?,
            };
            if let Some(count) = count {
                if count > 0 && self.is_full_scan(r.experiment) {
                    blocks.add_tic(r, None);
                }
                continue;
            }
            if self.scan_experiment(r.experiment) || self.empty_sim_row(r) {
                continue;
            }
            if let Some(v) = dense_mrm(scan, self.scan_len, r.at, r.size as usize / 4)? {
                blocks.add_dense(r, v);
            }
        }
        let mut chroms = self.chromatograms_of(blocks);
        if let Some(dad) = &self.dad {
            chroms.push(dad.twc_chromatogram(chroms.len()));
        }
        Ok(chroms)
    }

    /// How many spectra `visit` will give (reads the head of each block only).
    pub fn spectrum_count<R: ReadAt>(&self, scan: &mut R) -> Result<usize> {
        let mut n = 0;
        for r in self.records.iter().filter(|r| !r.cut(self.scan_len)) {
            n += scan::point_count(scan, self.scan_len, r.at, r.end)?.is_some_and(|c| c > 0)
                as usize;
        }
        Ok(n)
    }

    /// The header of the mzML of `name` (the `.wiff`); `scan_sha1`: SHA-1 of the `.wiff.scan` when it could be read.
    pub(crate) fn header(&self, name: &str, scan_sha1: Option<String>, centroid: bool) -> Header {
        let (instrument, run_params) = self.instrument_terms();
        Header {
            run: name.trim_end_matches(".wiff").to_string(),
            sources: vec![name.to_string(), format!("{name}.scan")],
            source_sha1: self.wiff_sha1.iter().cloned().chain(scan_sha1).collect(),
            acquired: self.created.clone(),
            run_params,
            instrument: Some(instrument),
            components: [
                ("source", "MS:1000073", "electrospray ionization"),
                ("analyzer", "MS:1000081", "quadrupole"),
                ("analyzer", "MS:1000081", "quadrupole"),
                ("analyzer", "MS:1000078", "axial ejection linear ion trap"),
                ("detector", "MS:1000253", "electron multiplier"),
            ]
            .iter()
            .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
            .collect(),
            software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
            serial: self.serial.clone(),
            centroided: centroid,
            ..Default::default()
        }
    }

    /// The instrument of the header: the CV term of the model the Log names (the known models), otherwise the generic `instrument model` and
    /// the name as a userParam of the run; without a Log, the generic term alone.
    fn instrument_terms(&self) -> ((String, String), Vec<(String, String)>) {
        match self.model.as_deref() {
            // a file of another QTRAP (SCIEX OS) may have no Log either: never taken for a 3200
            None => (("MS:1000031".into(), "instrument model".into()), Vec::new()),
            Some(m) if m.replace(' ', "").contains("3200QTRAP") => {
                (("MS:1000651".into(), "3200 QTRAP".into()), Vec::new())
            }
            Some(m) => (
                ("MS:1000031".into(), "instrument model".into()),
                vec![("instrument model".into(), m.to_string())],
            ),
        }
    }
}

/// The cps of an MRM block, read dense. A scheduled (sMRM) block, which starts with `-(N + 0.01)`, is `err.file.wiff`: only the readers module
/// (`scan_extended.rs`) reads it, and its skip markers must never become intensities.
#[cfg(not(feature = "estesi"))]
fn dense_mrm<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    n: usize,
) -> Result<Option<Vec<f64>>> {
    let v = scan::read_mrm(scan, scan_len, at, n)?;
    if v.as_ref().and_then(|v| v.first()).is_some_and(|x| *x < 0.0) {
        return Err(crate::error::Error::new("err.file.wiff").with("detail", "sparse MRM block"));
    }
    Ok(v)
}

/// A dense MRM block: a point of the TIC of its experiment and one of each of its transitions.
#[cfg(not(feature = "estesi"))]
impl Blocks {
    fn add_dense(&mut self, r: &super::Record, v: Vec<f64>) {
        let Some(e) = self.experiments.get_mut(r.experiment) else {
            return;
        };
        e.time.push(r.time_min);
        e.tic.push(r.tic);
        if e.cols.len() < v.len() {
            e.cols.resize_with(v.len(), Default::default);
        }
        for (x, col) in v.into_iter().zip(&mut e.cols) {
            col.0.push(r.time_min);
            col.1.push(x);
        }
    }
}

#[cfg(feature = "estesi")]
#[path = "run_extended.rs"]
mod estesi_run;
