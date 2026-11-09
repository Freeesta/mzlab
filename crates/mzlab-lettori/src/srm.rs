//! SRM/MRM stored as scans: a level-2 scan with an SRM (or CRM) filter whose points are the products of one precursor becomes, per transition, a chromatogram
//! with the id `SRM SIC Q1=<q1> Q3=<q3>` (`mzmlhead::srm_id` reads it), followed by ` ce=<eV>` when the collision energy is known and ` polarity=negative`
//! for the negative ones. The Q3 of a point is its m/z (the centre of the product window; ProteoWizard writes the same), the collision energy comes from the
//! `@cid25.0` of the filter or, when the filter has none (the TSQ filters never have), from the transition table of `metodo/1` (`ms…transizioni`), which also
//! gives the polarity of the transition (a TSQ scan carries a fixed polarity).
use mzlab_core::error::Result;
use mzlab_core::lettori::{ChromatogramEntry, Field, Format, Reader, Sample, ScanEntry};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

/// Products closer than this (m/z) to the Q3 of a transition belong to it.
pub const Q3_TOLERANCE: f64 = 0.005;
/// Precursors closer than this (m/z) to the Q1 of a transition belong to it.
pub const Q1_TOLERANCE: f64 = 0.005;
const UNITA: &str = "number of detector counts";

fn srm_filter(f: &str) -> bool {
    let f = f.to_ascii_lowercase();
    f.contains("srm") || f.contains("mrm") || f.contains("crm")
}

/// The collision energy of a filter: the number after `@cid` (or `@hcd`, …) of the first reaction.
fn ce_of_the_filter(f: &str) -> Option<f64> {
    let after = f.get(f.find('@')? + 1..)?;
    let n = after.trim_start_matches(|c: char| c.is_ascii_alphabetic());
    let fine = n
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(n.len());
    n.get(..fine)?.parse().ok().filter(|v: &f64| *v > 0.0)
}

/// A transition of the table of the method: `(q1, q3, collision energy, polarity)`.
type FromMethod = (f64, f64, Option<f64>, i8);

/// The number after `"key":` in `seg` (the document is written by `metodo.rs`, so keys are quoted and followed by a colon and, at most, one blank).
fn number_of(seg: &str, key: &str) -> Option<f64> {
    let k = seg.find(key)?;
    let rest = seg.get(k + key.len()..)?.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(rest.len());
    rest.get(..end)?
        .parse()
        .ok()
        .filter(|v: &f64| v.is_finite())
}

/// The transitions of the `metodo/1` document `m` (`ms.periodi[].esperimenti[].transizioni[]`), read without a JSON parser (the module is size-capped): every
/// `"q1":` opens a transition that goes on to the next one. Nothing when `m` has none or cannot be read.
fn transitions_of_the_method(m: &str) -> Vec<FromMethod> {
    let Some(from) = m.find("\"transizioni\"") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut starts: Vec<usize> = m
        .match_indices("\"q1\":")
        .map(|(i, _)| i)
        .filter(|i| *i > from && m.get(i.saturating_sub(1)..*i) != Some("\\"))
        .collect();
    starts.push(m.len());
    for w in starts.windows(2) {
        let Some(seg) = m.get(w[0]..w[1]) else {
            continue;
        };
        let (Some(q1), Some(q3)) = (number_of(seg, "\"q1\":"), number_of(seg, "\"q3\":")) else {
            continue;
        };
        let pol = match seg.find("\"polarita\":").and_then(|k| seg.get(k + 11..)) {
            Some(r) if r.trim_start().starts_with("\"positive\"") => 1,
            Some(r) if r.trim_start().starts_with("\"negative\"") => -1,
            _ => 0,
        };
        out.push((q1, q3, number_of(seg, "\"ce_eV\":"), pol));
    }
    out
}

struct Transition {
    q1: f64,
    q3: f64,
    ce: Option<f64>,
    polarita: i8,
    time: Vec<f64>,
    intensity: Vec<f64>,
}

/// A reader that also gives the SRM traces of the scans of the reader it wraps (unless that one has SRM chromatograms of its own).
pub struct SrmDaScansioni {
    inner: Box<dyn Reader>,
    /// The traces of each sample asked for so far (a few at most: a list is enough).
    cache: Vec<(usize, Vec<Chromatogram>)>,
}

impl SrmDaScansioni {
    pub fn new(inner: Box<dyn Reader>) -> Self {
        SrmDaScansioni {
            inner,
            cache: Vec::new(),
        }
    }

    fn traces(&mut self, c: usize) -> Result<&Vec<Chromatogram>> {
        let i = match self.cache.iter().position(|(k, _)| *k == c) {
            Some(i) => i,
            None => {
                let t = self.build(c)?;
                self.cache.push((c, t));
                self.cache.len() - 1
            }
        };
        Ok(&self.cache[i].1)
    }

    fn build(&mut self, c: usize) -> Result<Vec<Chromatogram>> {
        if self
            .inner
            .chromatograms(c)?
            .iter()
            .any(|v| v.tipo == ChromKind::Srm)
        {
            return Ok(Vec::new());
        }
        let rows: Vec<ScanEntry> = self.inner.index_without_tic(c)?.to_vec();
        let mut tr = transitions_from_the_scans(&mut *self.inner, c, &rows)?;
        if tr.iter().any(|t| t.ce.is_none())
            && let Ok(m) = self.inner.method(c)
        {
            complete_from_method(&m, &mut tr);
        }
        Ok(tr.into_iter().enumerate().map(cromatogramma_di).collect())
    }
}

/// One transition for every distinct (Q1, Q3, polarity, collision energy) of the SRM scans of `righe`.
fn transitions_from_the_scans(
    inner: &mut dyn Reader,
    c: usize,
    rows: &[ScanEntry],
) -> Result<Vec<Transition>> {
    let mut tr: Vec<Transition> = Vec::new();
    for (k, r) in rows.iter().enumerate().filter(|(_, r)| scansione_srm(r)) {
        let q1 = r.precursore.unwrap_or(0.0);
        let s = inner.scan(c, k)?;
        let ce = ce_of_the_filter(&r.filtro);
        for (i, &q3) in s.mz.iter().enumerate() {
            let y = s.intensity_f64(i);
            let j = match tr.iter().position(|t| {
                (t.q1 - q1).abs() <= Q1_TOLERANCE
                    && (t.q3 - q3).abs() <= Q3_TOLERANCE
                    && t.polarita == r.polarita
                    && t.ce == ce
            }) {
                Some(j) => j,
                None => {
                    tr.push(Transition {
                        q1,
                        // the m/z of a point is stored as f32: 246.108 would otherwise print as 246.10800170898438
                        q3: (q3 * 1e3).round() / 1e3,
                        ce,
                        polarita: r.polarita,
                        time: Vec::new(),
                        intensity: Vec::new(),
                    });
                    tr.len() - 1
                }
            };
            tr[j].time.push(r.rt);
            tr[j].intensity.push(y);
        }
    }
    Ok(tr)
}

fn cromatogramma_di((index, t): (usize, Transition)) -> Chromatogram {
    Chromatogram {
        index,
        id: id_di(&t),
        kind: ChromKind::Srm,
        q1: Some(t.q1),
        q3: Some(t.q3),
        unit: UNITA.into(),
        time: t.time,
        intensity: t.intensity,
    }
}

/// The collision energy and the polarity of the transitions that the scans do not give, from the table of the method. Transitions that the
/// table has under two polarities (or none) keep what the scans say.
fn complete_from_method(m: &str, tr: &mut [Transition]) {
    let table = transitions_of_the_method(m);
    for t in tr.iter_mut() {
        let mut rows = table.iter().filter(|(q1, q3, _, _)| {
            (q1 - t.q1).abs() <= Q1_TOLERANCE && (q3 - t.q3).abs() <= Q3_TOLERANCE
        });
        let (Some(&(_, _, ce, pol)), None) = (rows.next(), rows.next()) else {
            continue;
        };
        t.ce = t.ce.or(ce);
        if pol != 0 {
            t.polarita = pol;
        }
    }
}

/// `SRM SIC Q1=… Q3=…`, then ` ce=…` and ` polarity=negative` when they are known.
fn id_di(t: &Transition) -> String {
    let mut id = format!("SRM SIC Q1={} Q3={}", t.q1, t.q3);
    if let Some(ce) = t.ce {
        id.push_str(&format!(" ce={ce}"));
    }
    if t.polarita < 0 {
        id.push_str(" polarity=negative");
    }
    id
}

fn scansione_srm(v: &ScanEntry) -> bool {
    v.livello == 2 && v.precursore.is_some() && srm_filter(&v.filtro)
}

impl Reader for SrmDaScansioni {
    fn format(&self) -> Format {
        self.inner.format()
    }
    fn samples(&self) -> &[Sample] {
        self.inner.samples()
    }
    fn missing(&mut self, c: usize) -> usize {
        self.inner.missing(c)
    }
    // The account `conv/1` gives of the mzML is the reader's own: the wrapper adds traces, not gaps.
    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        self.inner.not_carried()
    }
    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.inner.index(c)
    }
    fn index_without_tic(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.inner.index_without_tic(c)
    }
    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.inner.scan(c, k)
    }
    fn scan_data(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.inner.scan_data(c, k)
    }
    // the profile points of a Thermo scan: without this the wrapper answers `None` and `--peaks profile` refuses a file that has them
    fn scan_profile(&mut self, c: usize, k: usize) -> Result<Option<Spectrum>> {
        self.inner.scan_profile(c, k)
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        self.inner.scan_header(c, k)
    }
    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        let mut v = self.inner.chromatograms(c)?;
        let extra = self.traces(c)?;
        v.extend(extra.iter().map(|t| ChromatogramEntry {
            id: t.id.clone(),
            tipo: t.kind,
            unita: t.unit.clone(),
        }));
        Ok(v)
    }
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        if let Some(t) = self.traces(c)?.iter().find(|t| t.id == id) {
            return Ok(t.clone());
        }
        self.inner.chromatogram(c, id)
    }
    fn dad_map(&mut self, c: usize) -> Result<Option<String>> {
        self.inner.dad_map(c)
    }
    fn dad_spectrum(&mut self, c: usize, rt0: f64, rt1: f64) -> Result<Option<String>> {
        self.inner.dad_spectrum(c, rt0, rt1)
    }
    fn dad_wavelength(&mut self, c: usize, nm0: f64, nm1: f64) -> Result<Option<String>> {
        self.inner.dad_wavelength(c, nm0, nm1)
    }
    #[cfg(any(not(target_family = "wasm"), feature = "sciex"))]
    fn dad_data(&mut self, c: usize) -> Result<Option<mzlab_core::wiff::dad::DadData>> {
        self.inner.dad_data(c)
    }
    fn method(&mut self, c: usize) -> Result<String> {
        self.inner.method(c)
    }
    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.inner.mzml_header(c)
    }
    // the light header of the scan table (a Thermo `.raw` does not hash the whole file for it)
    fn table_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.inner.table_header(c)
    }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.inner.extra_mzml(s)
    }
    fn scan_extra(&mut self, c: usize, k: usize, s: &Spectrum) -> SpectrumExtra {
        self.inner.scan_extra(c, k, s)
    }
    #[cfg(feature = "sciex")]
    fn variant(&mut self, c: usize) -> bool {
        self.inner.variant(c)
    }
}

/// True when sample `c` of `l` has scans that are SRM transitions.
pub fn ha_scansioni_srm(l: &mut dyn Reader, c: usize) -> bool {
    l.index_without_tic(c)
        .map(|i| i.iter().any(scansione_srm))
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "srm_prove.rs"]
mod tests;
