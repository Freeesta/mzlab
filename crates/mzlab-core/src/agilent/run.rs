//! A run as the `Run` that the rest of the program reads: one centroid spectrum per scan (empty ones left out) and the TIC from the index.

use std::io::Write;

use super::peak::read_points;
use super::{AgilentD, SCAN_FULL, SCAN_SIM};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::mzml::{ChromKind, Chromatogram, Run, Spectrum};
use crate::mzmlwrite::{Header, MzmlWriter};
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
        Ok(vec![Chromatogram {
            index: 0,
            id: "TIC".into(),
            kind: ChromKind::Tic,
            q1: None,
            q3: None,
            unit: "number of detector counts".into(),
            time: survey.iter().map(|r| r.time_min).collect(),
            intensity: survey.iter().map(|r| r.tic).collect(),
        }])
    }

    pub fn spectrum_count(&self) -> usize {
        self.records.iter().filter(|r| r.points > 0).count()
    }

    fn header(&self) -> Header {
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
        Header {
            run: base.to_string(),
            sources: vec![self.name.clone()],
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
        let mut w = MzmlWriter::new(out, &self.header())?;
        let count = self.spectrum_count();
        if count > 0 {
            w.open_spectra(count)?;
        }
        let mut done = 0;
        progress(0, count);
        let chroms = self.visit(&mut peak, &mut |s| {
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
