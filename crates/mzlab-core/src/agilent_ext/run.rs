//! A run as the `Run` that the rest of the program reads: one centroid spectrum per scan (empty ones left out) and the TIC from the index.

use std::collections::BTreeMap;
use std::io::Write;

use super::scan::ScanRec;
use super::{AgilentD, SCAN_FULL, SCAN_SIM};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::metodo::{self, Source, SrcFile};
use crate::mzml::{ChromKind, Chromatogram, Run, Spectrum};
use crate::mzmlwrite::{Header, MzmlWriter, SpectrumExtra};
use crate::window::Windowed;

impl AgilentD {
    /// Spectra and the TIC.
    pub fn run<R: ReadAt>(&self, peak: &mut R) -> Result<Run> {
        let mut run = Run::default();
        run.chromatograms = self.visit(peak, &mut |s| {
            run.spectra.push(s);
            Ok(())
        })?;
        Ok(run)
    }

    pub(crate) fn filter_of(&self, scan_type: i32, mz: &[f64]) -> String {
        let source = if self.is_gc() { "EI " } else { "" };
        match scan_type {
            SCAN_FULL => format!("{source}Scan"),
            SCAN_SIM => format!(
                "{source}SIM {}",
                mz.iter()
                    .map(|m| format!("{m:.1}"))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            super::lcms::SCAN_MRM => format!("{source}MRM"),
            super::lcms::SCAN_PRODUCT => format!("{source}Product ion"),
            n => format!("{source}type {n}"),
        }
    }

    /// The points of a scan: from `data.ms` when the run comes from it, else read from `MSPeak.bin`.
    pub(crate) fn points_of<R: ReadAt>(
        &self,
        peak: &mut R,
        r: &ScanRec,
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        match &self.inline {
            Some(v) => Ok(v.get(r.id as usize - 1).cloned().unwrap_or_default()),
            None => self.read_points_ext(peak, r),
        }
    }

    /// The spectrum number `n` (of the scans with points) from its record and its points.
    pub(crate) fn spectrum_of(
        &self,
        n: usize,
        r: &ScanRec,
        mz: Vec<f64>,
        it: Vec<f64>,
    ) -> Spectrum {
        let level = r.level.clamp(1, 255) as u8;
        let filter = self.filter_of(r.scan_type, &mz);
        let exact = it.iter().all(|&v| v as f32 as f64 == v);
        let profile = self.is_profile(r);
        Spectrum {
            index: n,
            id: format!("scan={}", r.id),
            level,
            rt: r.time_min,
            polarity: self.polarity_of(r),
            profile,
            filter,
            precursor: (level >= 2 && r.mz_of_interest > 0.0).then_some(r.mz_of_interest),
            mz,
            intensity: it.iter().map(|&v| v as f32).collect(),
            wide: (!exact).then_some(it),
        }
    }

    /// Gives the spectra one at a time to `on` (centroid; the scans without points are left out) and returns the TIC, the BPC and one SIC per SIM ion.
    pub fn visit<R: ReadAt>(
        &self,
        peak: &mut R,
        on: &mut dyn FnMut(Spectrum) -> Result<()>,
    ) -> Result<Vec<Chromatogram>> {
        let mut sic: BTreeMap<i64, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
        for (n, r) in self.records.iter().filter(|r| self.listed(r)).enumerate() {
            let (mz, it) = self.points_of(peak, r)?;
            if r.scan_type == SCAN_SIM {
                add_sic(&mut sic, r.time_min, &mz, &it);
            }
            on(self.spectrum_of(n, r, mz, it))?;
        }
        let mut out = self.survey_chromatograms();
        for (key, (time, intensity)) in sic {
            out.push(sic_chromatogram(out.len(), key, time, intensity));
        }
        out.extend(self.srm_chromatograms(peak, out.len())?);
        Ok(out)
    }

    /// The TIC and the BPC of the survey scans, from the records alone.
    pub(crate) fn survey_chromatograms(&self) -> Vec<Chromatogram> {
        let survey: Vec<_> = self.records.iter().filter(|r| r.level <= 1).collect();
        // a run of MRM visits has no survey scan: its TIC and BPC have a point per visit
        let survey: Vec<_> = if survey.is_empty() {
            self.records.iter().collect()
        } else {
            survey
        };
        let chrom = |index: usize, id: &str, kind, f: fn(&ScanRec) -> f64| Chromatogram {
            index,
            id: id.into(),
            kind,
            q1: None,
            q3: None,
            unit: "number of detector counts".into(),
            time: survey.iter().map(|r| r.time_min).collect(),
            intensity: survey.iter().map(|r| f(r)).collect(),
        };
        vec![
            chrom(0, "TIC", ChromKind::Tic, |r| r.tic),
            chrom(1, "BPC", ChromKind::Bpc, |r| r.base_value),
        ]
    }

    pub fn spectrum_count(&self) -> usize {
        self.records.iter().filter(|r| self.listed(r)).count()
    }

    /// The scan is listed as a spectrum: it has points (and, in an LC/MS run, is not an MRM visit).
    pub(crate) fn listed(&self, r: &ScanRec) -> bool {
        self.has_spectrum(r)
    }

    /// The `metodo/1` document for the files read (name, SHA-1).
    fn document(&self, sources: &[(String, String)]) -> String {
        self.metodo(&Source {
            format: "agilent-d",
            files: sources
                .iter()
                .map(|(name, sha1)| SrcFile {
                    name: name.clone(),
                    sha1: Some(sha1.clone()).filter(|h| !h.is_empty()),
                    bytes: 0,
                })
                .collect(),
            sample: None,
        })
    }

    pub(crate) fn header(&self, peak_sha1: &str) -> Header {
        let base = self
            .name
            .strip_suffix(".D")
            .or_else(|| self.name.strip_suffix(".d"))
            .unwrap_or(&self.name);
        let dev = self.device.as_ref();
        let name = dev.map_or("Agilent instrument model".to_string(), |d| {
            format!("Agilent {} {}", d.model, d.name).trim().to_string()
        });
        let low = dev.map(|d| d.name.to_lowercase()).unwrap_or_default();
        let mut components = Vec::new();
        let mut add = |kind: &str, acc: &str, n: &str| {
            components.push((kind.to_string(), acc.to_string(), n.to_string()))
        };
        if self.is_gc() {
            add("source", "MS:1000389", "electron ionization");
        }
        if low.contains("tof") {
            add("analyzer", "MS:1000084", "time-of-flight");
        } else if low.contains("quad") || low.contains("qqq") {
            add("analyzer", "MS:1000081", "quadrupole");
        }
        if self.is_gc() && low.contains("quad") {
            add("detector", "MS:1000253", "electron multiplier");
        }
        let mut sources: Vec<(String, String)> = self.file_sha1.clone();
        if self.inline.is_none() {
            let member = self.data_file;
            sources.insert(1.min(sources.len()), (member.into(), peak_sha1.into()));
        }
        let document = self.document(&sources);
        let mut content = vec![
            ("MS:1000127".into(), "centroid spectrum".into()),
            ("MS:1000235".into(), "total ion current chromatogram".into()),
            ("MS:1000628".into(), "basepeak chromatogram".into()),
        ];
        if self.records.iter().any(|r| self.is_profile(r)) {
            content[0] = ("MS:1000128".into(), "profile spectrum".into());
        }
        Header {
            run: base.to_string(),
            sources: sources.iter().map(|s| s.0.clone()).collect(),
            source_sha1: sources.iter().map(|s| s.1.clone()).collect(),
            source_cv: vec![
                ("MS:1001509".into(), "Agilent MassHunter format".into()),
                (
                    "MS:1001508".into(),
                    "Agilent MassHunter nativeID format".into(),
                ),
            ],
            acquired: self.acquired(),
            content,
            extra_software: self.acquisition_software().into_iter().collect(),
            serial: dev.map(|d| d.serial.clone()).filter(|s| !s.is_empty()),
            sample: self
                .sample_name()
                .map(|n| ("sample1".to_string(), n.to_string())),
            run_params: self
                .sample_type()
                .map(|t| ("sample type".to_string(), t.to_string()))
                .into_iter()
                .chain((!document.is_empty()).then(|| (metodo::USERPARAM.to_string(), document)))
                .collect(),
            instrument: Some(("MS:1000490".into(), name)),
            components,
            software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
            centroided: false,
        }
    }

    /// Writes the run as mzML, one spectrum at a time; `MSPeak.bin` is read front to back through a 1 MB window.
    pub fn to_mzml<R: ReadAt, W: Write>(&self, peak: &mut R, peak_len: u64, out: W) -> Result<()> {
        self.to_mzml_progress(peak, peak_len, out, &mut |_, _| {})
    }

    /// As `to_mzml`; `progress(done, total)` is called every 32 spectra with the spectra written and the spectra to write.
    pub fn to_mzml_progress<R: ReadAt, W: Write>(
        &self,
        peak: &mut R,
        peak_len: u64,
        out: W,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<()> {
        let mut peak = Windowed::new(peak, peak_len);
        let sha = if self.inline.is_some() {
            String::new()
        } else {
            sha1_of(&mut peak, peak_len)?
        };
        let mut w = MzmlWriter::new(out, &self.header(&sha))?;
        let count = self.spectrum_count();
        if count > 0 {
            w.open_spectra(count)?;
        }
        let mut done = 0;
        progress(0, count);
        let chroms = self.visit(&mut peak, &mut |s| {
            w.spectrum_with(&s, &self.extra_of(&s))?;
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
        if chroms.iter().any(|c| !c.time.is_empty()) {
            w.open_chromatograms(chroms.len())?;
            for c in &chroms {
                w.chromatogram(c)?;
            }
            w.close_chromatograms()?;
        }
        w.finish()?;
        Ok(())
    }
}

/// The SIC of the ion `key` (m/z × 100, rounded).
pub(crate) fn sic_chromatogram(
    index: usize,
    key: i64,
    time: Vec<f64>,
    intensity: Vec<f64>,
) -> Chromatogram {
    Chromatogram {
        index,
        id: format!("SIC {:.2}", key as f64 / 100.0),
        kind: ChromKind::Sic,
        q1: None,
        q3: None,
        unit: "number of detector counts".into(),
        time,
        intensity,
    }
}

/// Adds the points of a SIM scan to the trace of each ion (ions told apart at 0.01 m/z); two points of one ion in a scan are summed.
pub(crate) fn add_sic(
    sic: &mut BTreeMap<i64, (Vec<f64>, Vec<f64>)>,
    time: f64,
    mz: &[f64],
    it: &[f64],
) {
    for (m, &v) in mz.iter().zip(it) {
        let (t, y) = sic.entry((m * 100.0).round() as i64).or_default();
        match t.last() {
            Some(&last) if last == time => *y.last_mut().unwrap() += v,
            _ => {
                t.push(time);
                y.push(v);
            }
        }
    }
}

/// SHA-1 (hex) of the first `len` bytes of a reader, read in blocks of 1 MB.
pub(crate) fn sha1_of<R: ReadAt>(r: &mut R, len: u64) -> Result<String> {
    use sha1::{Digest as _, Sha1};
    let mut h = Sha1::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut at = 0u64;
    while at < len {
        let n = (len - at).min(buf.len() as u64) as usize;
        r.read_at(at, &mut buf[..n])?;
        h.update(&buf[..n]);
        at += n as u64;
    }
    Ok(h.finalize().iter().map(|x| format!("{x:02x}")).collect())
}

impl AgilentD {
    /// SIM mark, observed range and scan window of a spectrum: the window of the method for a scan run, the lowest and highest ion for SIM.
    pub(crate) fn extra_of(&self, s: &Spectrum) -> SpectrumExtra {
        let sim = s.filter.contains("SIM");
        let window = match (&self.method, s.mz.first(), s.mz.last()) {
            (Some(m), _, _) if !sim && m.low_mass.is_some() && m.high_mass.is_some() => {
                m.low_mass.zip(m.high_mass)
            }
            (_, Some(&a), Some(&b)) if sim => Some((a, b)),
            _ => None,
        };
        let mut params = Vec::new();
        params.extend(self.energies_of(s));
        SpectrumExtra {
            sim,
            observed: true,
            window,
            params,
        }
    }
}

impl AgilentD {
    /// The acquisition program as `Contents.xml` names it: ("MS:1000678", "MassHunter Data Acquisition", version), for MassHunter.
    pub(crate) fn acquisition_software(&self) -> Option<(String, String, String)> {
        let text = &self.contents.as_ref()?.software;
        let version = text
            .split_whitespace()
            .find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()) && w.contains('.'))?;
        text.contains("MassHunter").then(|| {
            (
                "MS:1000678".to_string(),
                "MassHunter Data Acquisition".to_string(),
                version.to_string(),
            )
        })
    }
}
