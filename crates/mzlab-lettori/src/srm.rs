//! SRM/MRM stored as scans: a level-2 scan with an SRM (or CRM) filter whose points are the products of one precursor becomes, per transition, a chromatogram
//! with the id `SRM SIC Q1=<q1> Q3=<q3>` (`mzmlhead::srm_id` reads it), followed by ` ce=<eV>` when the collision energy is known and ` polarity=negative`
//! for the negative ones. The Q3 of a point is its m/z (the centre of the product window; ProteoWizard writes the same), the collision energy comes from the
//! `@cid25.0` of the filter or, when the filter has none (the TSQ filters never have), from the transition table of `metodo/1` (`ms…transizioni`), which also
//! gives the polarity of the transition (a TSQ scan carries a fixed polarity).
use mzlab_core::error::Result;
use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

/// Products closer than this (m/z) to the Q3 of a transition belong to it.
pub const TOLLERANZA_Q3: f64 = 0.005;
/// Precursors closer than this (m/z) to the Q1 of a transition belong to it.
pub const TOLLERANZA_Q1: f64 = 0.005;
const UNITA: &str = "number of detector counts";

fn filtro_srm(f: &str) -> bool {
    let f = f.to_ascii_lowercase();
    f.contains("srm") || f.contains("mrm") || f.contains("crm")
}

/// The collision energy of a filter: the number after `@cid` (or `@hcd`, …) of the first reaction.
fn ce_del_filtro(f: &str) -> Option<f64> {
    let dopo = f.get(f.find('@')? + 1..)?;
    let n = dopo.trim_start_matches(|c: char| c.is_ascii_alphabetic());
    let fine = n
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(n.len());
    n.get(..fine)?.parse().ok().filter(|v: &f64| *v > 0.0)
}

/// A transition of the table of the method: `(q1, q3, collision energy, polarity)`.
type DalMetodo = (f64, f64, Option<f64>, i8);

/// The number after `"key":` in `seg` (the document is written by `metodo.rs`, so keys are quoted and followed by a colon and, at most, one blank).
fn numero_di(seg: &str, key: &str) -> Option<f64> {
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
fn transizioni_del_metodo(m: &str) -> Vec<DalMetodo> {
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
        let (Some(q1), Some(q3)) = (numero_di(seg, "\"q1\":"), numero_di(seg, "\"q3\":")) else {
            continue;
        };
        let pol = match seg.find("\"polarita\":").and_then(|k| seg.get(k + 11..)) {
            Some(r) if r.trim_start().starts_with("\"positive\"") => 1,
            Some(r) if r.trim_start().starts_with("\"negative\"") => -1,
            _ => 0,
        };
        out.push((q1, q3, numero_di(seg, "\"ce_eV\":"), pol));
    }
    out
}

struct Transizione {
    q1: f64,
    q3: f64,
    ce: Option<f64>,
    polarita: i8,
    time: Vec<f64>,
    intensity: Vec<f64>,
}

/// A reader that also gives the SRM traces of the scans of the reader it wraps (unless that one has SRM chromatograms of its own).
pub struct SrmDaScansioni {
    inner: Box<dyn Lettore>,
    /// The traces of each sample asked for so far (a few at most: a list is enough).
    cache: Vec<(usize, Vec<Chromatogram>)>,
}

impl SrmDaScansioni {
    pub fn new(inner: Box<dyn Lettore>) -> Self {
        SrmDaScansioni {
            inner,
            cache: Vec::new(),
        }
    }

    fn tracce(&mut self, c: usize) -> Result<&Vec<Chromatogram>> {
        let i = match self.cache.iter().position(|(k, _)| *k == c) {
            Some(i) => i,
            None => {
                let t = self.costruisci(c)?;
                self.cache.push((c, t));
                self.cache.len() - 1
            }
        };
        Ok(&self.cache[i].1)
    }

    fn costruisci(&mut self, c: usize) -> Result<Vec<Chromatogram>> {
        if self
            .inner
            .cromatogrammi(c)?
            .iter()
            .any(|v| v.tipo == ChromKind::Srm)
        {
            return Ok(Vec::new());
        }
        let righe: Vec<VoceScansione> = self.inner.indice(c)?.to_vec();
        let mut tr = transizioni_dalle_scansioni(&mut *self.inner, c, &righe)?;
        if tr.iter().any(|t| t.ce.is_none()) {
            if let Ok(m) = self.inner.metodo(c) {
                completa_dal_metodo(&m, &mut tr);
            }
        }
        Ok(tr.into_iter().enumerate().map(cromatogramma_di).collect())
    }
}

/// One transition for every distinct (Q1, Q3, polarity, collision energy) of the SRM scans of `righe`.
fn transizioni_dalle_scansioni(
    inner: &mut dyn Lettore,
    c: usize,
    righe: &[VoceScansione],
) -> Result<Vec<Transizione>> {
    let mut tr: Vec<Transizione> = Vec::new();
    for (k, r) in righe.iter().enumerate().filter(|(_, r)| scansione_srm(r)) {
        let q1 = r.precursore.unwrap_or(0.0);
        let s = inner.scansione(c, k)?;
        let ce = ce_del_filtro(&r.filtro);
        for (i, &q3) in s.mz.iter().enumerate() {
            let y = s.intensity_f64(i);
            let j = match tr.iter().position(|t| {
                (t.q1 - q1).abs() <= TOLLERANZA_Q1
                    && (t.q3 - q3).abs() <= TOLLERANZA_Q3
                    && t.polarita == r.polarita
                    && t.ce == ce
            }) {
                Some(j) => j,
                None => {
                    tr.push(Transizione {
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

fn cromatogramma_di((index, t): (usize, Transizione)) -> Chromatogram {
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
fn completa_dal_metodo(m: &str, tr: &mut [Transizione]) {
    let tabella = transizioni_del_metodo(m);
    for t in tr.iter_mut() {
        let mut righe = tabella.iter().filter(|(q1, q3, _, _)| {
            (q1 - t.q1).abs() <= TOLLERANZA_Q1 && (q3 - t.q3).abs() <= TOLLERANZA_Q3
        });
        let (Some(&(_, _, ce, pol)), None) = (righe.next(), righe.next()) else {
            continue;
        };
        t.ce = t.ce.or(ce);
        if pol != 0 {
            t.polarita = pol;
        }
    }
}

/// `SRM SIC Q1=… Q3=…`, then ` ce=…` and ` polarity=negative` when they are known.
fn id_di(t: &Transizione) -> String {
    let mut id = format!("SRM SIC Q1={} Q3={}", t.q1, t.q3);
    if let Some(ce) = t.ce {
        id.push_str(&format!(" ce={ce}"));
    }
    if t.polarita < 0 {
        id.push_str(" polarity=negative");
    }
    id
}

fn scansione_srm(v: &VoceScansione) -> bool {
    v.livello == 2 && v.precursore.is_some() && filtro_srm(&v.filtro)
}

impl Lettore for SrmDaScansioni {
    fn formato(&self) -> Formato {
        self.inner.formato()
    }
    fn campioni(&self) -> &[Campione] {
        self.inner.campioni()
    }
    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.inner.indice(c)
    }
    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.inner.scansione(c, k)
    }
    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.inner.intestazione(c, k)
    }
    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        let mut v = self.inner.cromatogrammi(c)?;
        let extra = self.tracce(c)?;
        v.extend(extra.iter().map(|t| VoceCromatogramma {
            id: t.id.clone(),
            tipo: t.kind,
            unita: t.unit.clone(),
        }));
        Ok(v)
    }
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        if let Some(t) = self.tracce(c)?.iter().find(|t| t.id == id) {
            return Ok(t.clone());
        }
        self.inner.cromatogramma(c, id)
    }
    fn metodo(&mut self, c: usize) -> Result<String> {
        self.inner.metodo(c)
    }
    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.inner.testata_mzml(c)
    }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.inner.extra_mzml(s)
    }
    fn variante(&mut self, c: usize) -> bool {
        self.inner.variante(c)
    }
}

/// True when sample `c` of `l` has scans that are SRM transitions.
pub fn ha_scansioni_srm(l: &mut dyn Lettore, c: usize) -> bool {
    l.indice(c)
        .map(|i| i.iter().any(scansione_srm))
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "srm_prove.rs"]
mod tests;
