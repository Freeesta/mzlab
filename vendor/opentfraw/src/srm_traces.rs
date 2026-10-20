//! One chromatogram per monitored transition of a TSQ SRM run whose events are one per scan (each event holds its Q1, the collision energy and
//! the product ranges): the points are the intensities of the peaks the scans give, matched to the ranges. Used by the mzML writer and by mzLab's reader.

use std::collections::HashMap;
use std::io::{Read, Seek};

use crate::RawFileReader;

/// The trace of one transition: times in minutes, intensities as the scans give them.
#[derive(Debug, Clone)]
pub struct SrmTrace {
    pub q1: f64,
    /// Middle of the product range.
    pub q3: f64,
    pub ce: Option<f64>,
    pub time_min: Vec<f64>,
    pub intensity: Vec<f32>,
}

impl SrmTrace {
    /// The id of the chromatogram: `SRM Q1=528.2960 Q3=343.2080 ce=25` (`ce` only when the event gives one).
    pub fn id(&self) -> String {
        let ce = self.ce.map(|e| format!(" ce={e}")).unwrap_or_default();
        format!("SRM Q1={:.4} Q3={:.4}{ce}", self.q1, self.q3)
    }
}

impl RawFileReader {
    /// The SRM traces of the file, in order of first appearance (Q1, then product range); empty when the events do not carry Q1 and ranges
    /// (the older TSQ layouts name their transitions through `srm_q1_by_event`). A scan whose peak is missing for a range adds no point to it.
    pub fn srm_traces<R: Read + Seek>(&self, source: &mut R) -> Vec<SrmTrace> {
        let mut out: Vec<SrmTrace> = Vec::new();
        let mut at: HashMap<(u64, usize), usize> = HashMap::new();
        if !self.flat_peaks || !self.srm_q1_by_event.is_empty() {
            return out;
        }
        let first = self.run_header.sample_info.first_scan_number;
        for (i, entry) in self.scan_index.iter().enumerate() {
            let Some(event) = self.scan_events.get(i) else {
                continue;
            };
            let Some(rx) = event.reactions.first() else {
                continue;
            };
            let Ok(peaks) = self.read_scan_peaks(source, first + i as u32) else {
                continue;
            };
            for (j, w) in event.fraction_collectors.iter().enumerate() {
                let half = (w.high_mz - w.low_mz) / 2.0;
                let mid = w.low_mz + half;
                let Some(p) = peaks.iter().find(|p| (p.mz - mid).abs() <= half + 0.01) else {
                    continue;
                };
                let k = *at.entry((rx.precursor_mz.to_bits(), j)).or_insert_with(|| {
                    out.push(SrmTrace {
                        q1: rx.precursor_mz,
                        q3: mid,
                        ce: Some(rx.energy).filter(|e| *e > 0.0),
                        time_min: Vec::new(),
                        intensity: Vec::new(),
                    });
                    out.len() - 1
                });
                out[k].time_min.push(entry.start_time);
                out[k].intensity.push(p.abundance);
            }
        }
        out
    }
}
