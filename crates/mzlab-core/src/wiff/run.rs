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
    /// then the collision energy (its magnitude, absent when zero), dwell (ms) (target RT in min and window in s when scheduled), the polarity of the experiment and the name of the method when it has them; the mzML writer and `mzmlhead` read them back from it.
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
            // a negative-mode method stores the energy with its sign; the id (and the mzML) carry its magnitude, as ProteoWizard does
            if let Some(ce) = t.ce.map(f64::abs).filter(|c| *c > 0.0) {
                id += &format!(" ce={ce}");
            }
            match self.experiments.get(e).and_then(|x| x.window_s) {
                Some(w) => id += &format!(" rt={} window={w}", t.dwell),
                None => id += &format!(" dwell={}", t.dwell),
            }
        }
        id += self.polarity_term(e);
        if let Some(t) = t.filter(|t| !t.name.is_empty()) {
            id += &format!(" name={}", t.name);
        }
        id
    }

    /// ` polarity=positive` or ` polarity=negative` for experiment `e` (its header, else the ion spray voltage), empty when neither tells.
    fn polarity_term(&self, e: usize) -> &'static str {
        let pol = self
            .experiments
            .get(e)
            .map_or(self.polarity, |x| self.polarity_of(x));
        match pol {
            1 => " polarity=positive",
            -1 => " polarity=negative",
            _ => "",
        }
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
        id += self.polarity_term(e);
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
        let (mut n, mut tally) = (0, scan::Tally::default());
        for r in self.records.iter().filter(|r| !r.cut(self.scan_len)) {
            let block = scan::read_block(scan, self.scan_len, r, Some(&mut tally))?;
            if let scan::Block::Scan(p) = block {
                if p.mz.is_empty() {
                    continue;
                }
                if self.is_full_scan(r.experiment) {
                    blocks.add_tic(r, p.intensity.iter().copied().reduce(f64::max));
                }
                on(self.spectrum_of(n, r, p))?;
                n += 1;
            } else if matches!(block, scan::Block::Damaged)
                || self.scan_experiment(r.experiment)
                || self.empty_sim_row(r)
            {
                continue;
            } else if let Some(v) =
                blocks.take(dense_mrm(scan, self.scan_len, r.at, r.size as usize / 4)?)
            {
                blocks.add_dense(r, v);
            }
        }
        tally.check()?;
        Ok(self.chromatograms_of(blocks))
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

    /// The chromatograms of the MRM blocks only (no scan is decoded), and the MRM cycles read and left out. `counts`: the point count of each record's
    /// head when the reader already read them.
    #[cfg(not(feature = "estesi"))]
    pub fn chromatograms<R: ReadAt>(
        &self,
        scan: &mut R,
        counts: Option<&[Option<u32>]>,
    ) -> Result<(Vec<Chromatogram>, super::chromatograms::Cycles)> {
        let mut blocks = Blocks::new(self.experiments.len().max(1));
        for (i, r) in self
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.cut(self.scan_len))
        {
            let count = match counts.and_then(|c| c.get(i)) {
                Some(&c) => c,
                None => block_count(scan, self.scan_len, r)?,
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
            if let Some(v) = blocks.take(dense_mrm(scan, self.scan_len, r.at, r.size as usize / 4)?)
            {
                blocks.add_dense(r, v);
            }
        }
        let cycles = blocks.cycles;
        Ok((self.chromatograms_of(blocks), cycles))
    }

    /// How many spectra `visit` will give (reads the head of each block only).
    pub fn spectrum_count<R: ReadAt>(&self, scan: &mut R) -> Result<usize> {
        let mut n = 0;
        for r in self.records.iter().filter(|r| !r.cut(self.scan_len)) {
            n += block_count(scan, self.scan_len, r)?.is_some_and(|c| c > 0) as usize;
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
/// (`scan_extended.rs`) reads it, and its skip markers must never become intensities. A block with a NaN or infinite value is damaged: left out and
/// counted, the other cycles are read.
#[cfg(not(feature = "estesi"))]
fn dense_mrm<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    n: usize,
) -> Result<super::chromatograms::Mrm<Vec<f64>>> {
    use super::chromatograms::Mrm;
    let Some(v) = scan::read_mrm(scan, scan_len, at, n)? else {
        return Ok(Mrm::Absent);
    };
    if v.first().is_some_and(|x| *x < 0.0 && x.is_finite()) {
        return Err(crate::error::Error::new("err.file.wiff").with("detail", "sparse MRM block"));
    }
    Ok(if v.iter().all(|x| x.is_finite()) {
        Mrm::Read(v)
    } else {
        Mrm::Damaged
    })
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

/// The point count of the block of `r` as the reader's index takes it (`scan::Block::count`).
pub(super) fn block_count(
    scan: &mut dyn ReadAt,
    scan_len: u64,
    r: &super::Record,
) -> Result<Option<u32>> {
    Ok(scan::read_block(scan, scan_len, r, None)?.count())
}

#[cfg(feature = "estesi")]
#[path = "run_extended.rs"]
mod estesi_run;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::error::Error;

    fn sample(model: Option<&str>) -> Wiff {
        let mut w = Wiff::refused(1, 0, Error::new("x"));
        w.model = model.map(str::to_string);
        w.wiff_sha1 = Some("aa".into());
        w.created = Some("2020-01-02T03:04:05Z".into());
        w.serial = Some("S1".into());
        w
    }

    #[test]
    fn the_mzml_header_names_both_files_their_digests_and_the_instrument() {
        let h = sample(Some("3200 QTRAP")).header("Run_1.wiff", Some("bb".into()), true);
        assert_eq!(h.run, "Run_1");
        assert_eq!(h.sources, ["Run_1.wiff", "Run_1.wiff.scan"]);
        assert_eq!(h.source_sha1, ["aa", "bb"]);
        assert_eq!(h.acquired.as_deref(), Some("2020-01-02T03:04:05Z"));
        assert_eq!(h.serial.as_deref(), Some("S1"));
        assert!(h.centroided && h.run_params.is_empty());
        assert_eq!(
            h.instrument,
            Some(("MS:1000651".into(), "3200 QTRAP".into()))
        );
        assert_eq!(h.components.len(), 5);
        assert_eq!(
            h.components.first().map(|c| c.1.as_str()),
            Some("MS:1000073")
        );
        assert_eq!(h.software.0, "mzLab");
        assert!(!sample(None).header("a.wiff", None, false).centroided);
    }

    #[test]
    fn an_unknown_model_is_the_generic_term_with_its_name_as_a_parameter() {
        let generic = ("MS:1000031".to_string(), "instrument model".to_string());
        let h = sample(Some("Other Q")).header("a.wiff", None, false);
        assert_eq!(h.instrument.as_ref(), Some(&generic));
        assert_eq!(
            h.run_params,
            [("instrument model".to_string(), "Other Q".to_string())]
        );
        let h = sample(None).header("a.wiff", None, false);
        assert_eq!(h.instrument, Some(generic));
        assert!(h.run_params.is_empty() && h.source_sha1 == ["aa"]);
    }
}
