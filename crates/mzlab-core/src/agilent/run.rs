//! A run as the `Run` that the rest of the program reads: one centroid spectrum per scan (empty ones left out) and the TIC from the index.

use std::io::Write;

use super::peak::read_points;
use super::{AgilentD, SCAN_FULL, SCAN_SIM};
use crate::error::Result;
use crate::idxread::ReadAt;
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

    fn filter_of(&self, scan_type: i32, mz: &[f64]) -> String {
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
            n => format!("{source}type {n}"),
        }
    }

    /// Gives the spectra one at a time to `on` (centroid; the scans without points are left out) and returns the TIC chromatogram of the survey scans.
    pub fn visit<R: ReadAt>(
        &self,
        peak: &mut R,
        on: &mut dyn FnMut(Spectrum) -> Result<()>,
    ) -> Result<Vec<Chromatogram>> {
        for (n, r) in self.records.iter().filter(|r| r.points > 0).enumerate() {
            let (mz, it) = read_points(peak, r)?;
            let level = r.level.clamp(1, 255) as u8;
            let filter = self.filter_of(r.scan_type, &mz);
            let exact = it.iter().all(|&v| v as f32 as f64 == v);
            on(Spectrum {
                index: n,
                id: format!("scan={}", r.id),
                level,
                rt: r.time_min,
                polarity: self.polarity_of(r),
                profile: false,
                filter,
                precursor: (level >= 2 && r.mz_of_interest > 0.0).then_some(r.mz_of_interest),
                mz,
                intensity: it.iter().map(|&v| v as f32).collect(),
                wide: (!exact).then_some(it),
            })?;
        }
        let survey: Vec<_> = self.records.iter().filter(|r| r.level <= 1).collect();
        let bpc = Chromatogram {
            index: 1,
            id: "BPC".into(),
            kind: ChromKind::Bpc,
            q1: None,
            q3: None,
            unit: "number of detector counts".into(),
            time: survey.iter().map(|r| r.time_min).collect(),
            intensity: survey.iter().map(|r| r.base_value).collect(),
        };
        Ok(vec![
            Chromatogram {
                index: 0,
                id: "TIC".into(),
                kind: ChromKind::Tic,
                q1: None,
                q3: None,
                unit: "number of detector counts".into(),
                time: survey.iter().map(|r| r.time_min).collect(),
                intensity: survey.iter().map(|r| r.tic).collect(),
            },
            bpc,
        ])
    }

    pub fn spectrum_count(&self) -> usize {
        self.records.iter().filter(|r| r.points > 0).count()
    }

    fn header(&self, peak_sha1: &str) -> Header {
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
        } else if low.contains("quad") {
            add("analyzer", "MS:1000081", "quadrupole");
        }
        if self.is_gc() && low.contains("quad") {
            add("detector", "MS:1000253", "electron multiplier");
        }
        let mut sources: Vec<(String, String)> = self.file_sha1.clone();
        sources.insert(
            1.min(sources.len()),
            ("AcqData/MSPeak.bin".into(), peak_sha1.into()),
        );
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
            content: vec![
                ("MS:1000127".into(), "centroid spectrum".into()),
                ("MS:1000235".into(), "total ion current chromatogram".into()),
                ("MS:1000628".into(), "basepeak chromatogram".into()),
            ],
            extra_software: self.acquisition_software().into_iter().collect(),
            serial: dev.map(|d| d.serial.clone()).filter(|s| !s.is_empty()),
            sample: self
                .sample_name()
                .map(|n| ("sample1".to_string(), n.to_string())),
            run_params: self
                .sample_type()
                .map(|t| vec![("sample type".to_string(), t.to_string())])
                .unwrap_or_default(),
            instrument: Some(("MS:1000490".into(), name)),
            components,
            software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
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
        let sha = sha1_of(&mut peak, peak_len)?;
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

/// SHA-1 (hex) of the first `len` bytes of a reader, read in blocks of 1 MB.
fn sha1_of<R: ReadAt>(r: &mut R, len: u64) -> Result<String> {
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
    fn extra_of(&self, s: &Spectrum) -> SpectrumExtra {
        let sim = s.filter.contains("SIM");
        let window = match (&self.method, s.mz.first(), s.mz.last()) {
            (Some(m), _, _) if !sim && m.low_mass.is_some() && m.high_mass.is_some() => {
                m.low_mass.zip(m.high_mass)
            }
            (_, Some(&a), Some(&b)) if sim => Some((a, b)),
            _ => None,
        };
        SpectrumExtra {
            sim,
            observed: true,
            window,
            params: Vec::new(),
        }
    }
}

impl AgilentD {
    /// The acquisition program as `Contents.xml` names it: ("MS:1000678", "MassHunter Data Acquisition", version), for MassHunter.
    fn acquisition_software(&self) -> Option<(String, String, String)> {
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
