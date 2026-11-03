//! The channels of a triple quadrupole mzML: every SRM chromatogram (Q1 > Q3) and every SIM one (Q1 only, `mzmlhead` reads both as kind SRM), with the
//! polarity its id writes (`polarity=positive|negative`; none = not written, which matches either). The SRM reader of the engine (`srm::leggi`) keeps only
//! the traces with a Q3, so the SIM ions are read here and that reader stays as it is.

use mzlab_core::error::Result;
use mzlab_core::mzml::ChromKind;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::sorgente::{Tracce, TracceMzml};

/// One chromatogram of a transition or SIM ion: times in minutes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Channel {
    pub q1: f64,
    pub q3: Option<f64>,
    pub polarity: Option<i8>,
    pub t: Vec<f64>,
    pub y: Vec<f64>,
}

fn polarity_of(id: &str) -> Option<i8> {
    if id.contains("polarity=negative") {
        Some(-1)
    } else if id.contains("polarity=positive") {
        Some(1)
    } else {
        None
    }
}

/// The SRM and SIM chromatograms of the mzML `bytes`, in the order of the file.
pub fn channels(bytes: &[u8]) -> Result<Vec<Channel>> {
    let mut m = Mzml::new(bytes.to_vec(), bytes.len() as u64);
    let mut t = TracceMzml::new(&mut m, 0);
    let mut out = Vec::new();
    for v in t.cromatogrammi()? {
        if v.tipo != ChromKind::Srm {
            continue;
        }
        let c = t.cromatogramma(&v.id)?;
        let Some(q1) = c.q1 else {
            continue;
        };
        let n = c.time.len().min(c.intensity.len());
        out.push(Channel {
            q1,
            q3: c.q3,
            polarity: polarity_of(&c.id),
            t: c.time[..n].to_vec(),
            y: c.intensity[..n].to_vec(),
        });
    }
    Ok(out)
}
