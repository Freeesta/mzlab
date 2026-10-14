//! The SRM traces of a sample as plain numbers: read through `Tracce` (an mzML or a sample of a vendor file, `sorgente.rs`) and passed between
//! the engine and the mzFinder workers as bytes (`impacchetta` / `spacchetta`), so an MRM file is never written as an mzML to be read again.

use crate::error::{Error, Result};
use crate::mzml::ChromKind;
use crate::mzmlhead::srm_id;
use crate::sorgente::Tracce;

/// The chromatogram of one transition: times in minutes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Traccia {
    pub q1: f64,
    pub q3: f64,
    pub ce: Option<f64>,
    pub name: String,
    pub t: Vec<f64>,
    pub y: Vec<f64>,
}

/// Every SRM chromatogram with Q1 and Q3, in the order of the file.
pub fn leggi<T: Tracce>(t: &mut T) -> Result<Vec<Traccia>> {
    t.srm()
}

/// `Tracce::srm` for a reader of chromatograms: the list, then each of kind SRM.
pub(crate) fn da_voci<T: Tracce + ?Sized>(t: &mut T) -> Result<Vec<Traccia>> {
    let mut out = Vec::new();
    for v in t.cromatogrammi()? {
        if v.tipo != ChromKind::Srm {
            continue;
        }
        let c = t.cromatogramma(&v.id)?;
        let (_, _, ce, name) = srm_id(&c.id);
        let (Some(q1), Some(q3)) = (c.q1, c.q3) else {
            continue;
        };
        out.push(Traccia {
            q1,
            q3,
            ce,
            name,
            t: c.time,
            y: c.intensity,
        });
    }
    Ok(out)
}

/// `u32 n`, then per trace `q1 q3 ce(NaN: none)` as `f64`, `u32` name length and bytes, `u32` points, the times and the intensities as `f64` (little endian).
pub fn impacchetta(v: &[Traccia]) -> Vec<u8> {
    let mut b = (v.len() as u32).to_le_bytes().to_vec();
    for s in v {
        for x in [s.q1, s.q3, s.ce.unwrap_or(f64::NAN)] {
            b.extend(x.to_le_bytes());
        }
        b.extend((s.name.len() as u32).to_le_bytes());
        b.extend(s.name.as_bytes());
        let n = s.t.len().min(s.y.len());
        b.extend((n as u32).to_le_bytes());
        for x in s.t[..n].iter().chain(&s.y[..n]) {
            b.extend(x.to_le_bytes());
        }
    }
    b
}

struct Cursore<'a>(&'a [u8]);

impl Cursore<'_> {
    fn prendi(&mut self, n: usize) -> Result<&[u8]> {
        if self.0.len() < n {
            return Err(Error::new("error.badIndex"));
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn u32(&mut self) -> Result<usize> {
        Ok(u32::from_le_bytes(self.prendi(4)?.try_into().unwrap_or_default()) as usize)
    }
    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(
            self.prendi(8)?.try_into().unwrap_or_default(),
        ))
    }
    fn serie(&mut self, n: usize) -> Result<Vec<f64>> {
        (0..n).map(|_| self.f64()).collect()
    }
}

/// The traces `impacchetta` wrote.
pub fn spacchetta(b: &[u8]) -> Result<Vec<Traccia>> {
    let mut c = Cursore(b);
    let n = c.u32()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let (q1, q3, ce) = (c.f64()?, c.f64()?, c.f64()?);
        let len = c.u32()?;
        let name = String::from_utf8_lossy(c.prendi(len)?).into_owned();
        let p = c.u32()?;
        let (t, y) = (c.serie(p)?, c.serie(p)?);
        out.push(Traccia {
            q1,
            q3,
            ce: (!ce.is_nan()).then_some(ce),
            name,
            t,
            y,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod prove {
    use super::*;

    #[test]
    fn round_trip() {
        let v = vec![
            Traccia {
                q1: 301.1,
                q3: 150.0,
                ce: Some(25.0),
                name: "quant".into(),
                t: vec![0.0, 0.1],
                y: vec![1.0, 2.0],
            },
            Traccia::default(),
        ];
        assert_eq!(spacchetta(&impacchetta(&v)).unwrap(), v);
        assert!(spacchetta(&impacchetta(&v)[..20]).is_err());
    }
}
