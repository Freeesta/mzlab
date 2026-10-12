//! The result of the hypotheses (L7) as bytes, so that the proofs (L8-L9) can run in another worker: each of the two phases lives in a worker of its own
//! and gives its memory back by closing it. Only what the proofs read is kept: the hypotheses (id, source, label, formula, masses, direction, RT of the
//! base), the polarity and the targeted rows with their cells; the outcomes, the adducts and the peaks of the files are not (they are not read there,
//! and the registry for the page is taken from the hypotheses worker). Little-endian; the first 8 bytes name the format.

use mzlab_core::error::{Error, Result};
use mzlab_feat::hypo::{Dir, Hypothesis, Registry};
use mzlab_feat::target::{Cell, TargetResult, TargetRow};

use crate::run::HypoRun;

const MAGIC: &[u8; 8] = b"MZHYPO01";

struct Put(Vec<u8>);

impl Put {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }
}

/// The state of a hypotheses run for the proofs.
pub fn dump(h: &HypoRun) -> Vec<u8> {
    let mut o = Put(MAGIC.to_vec());
    let reg = &h.registry;
    o.u32(reg.rules_version);
    o.u32(reg.polarity as u8 as u32);
    o.u32(reg.hyps.len() as u32);
    for x in &reg.hyps {
        o.u32(x.id);
        o.u32(u32::from(x.sources) | u32::from(x.polarity as u8) << 8 | (x.dir as u32) << 16);
        o.str(&x.label);
        o.str(&x.formula);
        o.f64(x.neutral);
        o.f64(x.mz);
        o.f32(x.rt_ref.unwrap_or(f32::NAN));
        o.u32(u32::from(x.rt_ref.is_some()));
    }
    let r = &h.result;
    o.u32(r.n_files as u32);
    o.u32(r.tried);
    o.u32(r.found);
    o.u32(r.rows.len() as u32);
    for row in &r.rows {
        o.u32(row.hyp);
        o.f64(row.mz);
        o.f64(row.rt);
        o.f32(row.window.0);
        o.f32(row.window.1);
        o.u32(u32::from(row.n_detected) | u32::from(row.n_targeted) << 16);
        o.u32(row.ms2_total);
        o.f64(row.coincidence);
        o.u32(row.cells.len() as u32);
        for c in &row.cells {
            o.u32(u32::from(c.state) | u32::from(c.ms2_n) << 16);
            o.f64(c.area);
            for v in [c.height, c.apex_rt, c.width, c.snr] {
                o.f32(v);
            }
            o.u32(c.feature);
        }
    }
    o.0
}

struct Get<'a>(&'a [u8]);

impl Get<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        if self.0.len() < n {
            return Err(Error::new("error.featState"));
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn str(&mut self) -> Result<String> {
        let n = self.u32()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| Error::new("error.featState"))
    }
}

fn hypothesis(g: &mut Get) -> Result<Hypothesis> {
    let id = g.u32()?;
    let packed = g.u32()?;
    let (label, formula) = (g.str()?, g.str()?);
    let (neutral, mz) = (g.f64()?, g.f64()?);
    let (rt, has) = (g.f32()?, g.u32()?);
    Ok(Hypothesis {
        id,
        sources: packed as u8,
        label,
        formula,
        neutral,
        mz,
        adduct: "",
        polarity: (packed >> 8) as u8 as i8,
        steps: 0,
        level: 0,
        dir: match packed >> 16 {
            0 => Dir::MorePolar,
            1 => Dir::LessPolar,
            _ => Dir::Free,
        },
        rt_ref: (has != 0).then_some(rt),
        alternatives: Vec::new(),
    })
}

fn row(g: &mut Get) -> Result<TargetRow> {
    let (hyp, mz, rt) = (g.u32()?, g.f64()?, g.f64()?);
    let window = (g.f32()?, g.f32()?);
    let counts = g.u32()?;
    let (ms2_total, coincidence) = (g.u32()?, g.f64()?);
    let n = g.u32()? as usize;
    let mut cells = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        let packed = g.u32()?;
        let area = g.f64()?;
        let (height, apex_rt, width, snr) = (g.f32()?, g.f32()?, g.f32()?, g.f32()?);
        cells.push(Cell {
            state: packed as u8,
            area,
            height,
            apex_rt,
            width,
            snr,
            ms2_n: (packed >> 16) as u16,
            feature: g.u32()?,
        });
    }
    Ok(TargetRow {
        hyp,
        mz,
        rt,
        window,
        cells,
        n_detected: counts as u16,
        n_targeted: (counts >> 16) as u16,
        ms2_total,
        coincidence,
    })
}

/// A run for the proofs from the bytes of `dump`.
pub fn load(bytes: &[u8]) -> Result<HypoRun> {
    let mut g = Get(bytes);
    if g.take(8)? != MAGIC {
        return Err(Error::new("error.featState"));
    }
    let (rules_version, polarity) = (g.u32()?, g.u32()? as u8 as i8);
    let n = g.u32()? as usize;
    let hyps = (0..n).map(|_| hypothesis(&mut g)).collect::<Result<Vec<_>>>()?;
    let (n_files, tried, found) = (g.u32()? as usize, g.u32()?, g.u32()?);
    let m = g.u32()? as usize;
    let rows = (0..m).map(|_| row(&mut g)).collect::<Result<Vec<_>>>()?;
    Ok(HypoRun {
        registry: Registry {
            hyps,
            outcomes: Vec::new(),
            rules_version,
            polarity,
            suspects_cut: 0,
        },
        result: TargetResult {
            n_files,
            rows,
            files: Vec::new(),
            tried,
            found,
            decoy_rate: Vec::new(),
        },
    })
}
