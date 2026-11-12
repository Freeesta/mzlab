//! The spectra of a folder: the index of the scans of the scanning functions in the order of time (`build_index`) and, once before the first read
//! through it, the check of their records (`ready`): a scan with a record no acquisition writes is left out, counted and named, the others are read.

use super::dat::whole;
use super::idx;
use super::reader::WatersReader;
use mzlab_core::error::Result;
use mzlab_core::readers::{Position, ScanEntry};
use std::collections::BTreeMap;

impl WatersReader {
    /// The spectra: every scan with points of a scanning function (the lock mass included, marked in its filter), in the order of time.
    pub(super) fn build_index(&mut self) {
        let mut v: Vec<(f32, usize, usize)> = Vec::new();
        for (i, f) in self.funs.iter().enumerate() {
            if f.f.mirato() || f.cod.is_none() || f.dat.is_none() {
                continue;
            }
            v.extend(
                f.righe
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.count > 0)
                    .map(|(k, r)| (r.rt, i, k)),
            );
        }
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        for &(_, i, k) in &v {
            let f = &self.funs[i];
            let r = f.righe[k];
            let precursor = (f.livello == 2)
                .then(|| {
                    f.sts
                        .as_ref()
                        .and_then(|t| t.value("Set Mass", k))
                        .filter(|&m| m > 0.0)
                        .or(f.ext.set_mass)
                })
                .flatten();
            self.indice.push(ScanEntry {
                id: format!("function={} process=0 scan={}", f.f.indice, k + 1),
                rt: f64::from(r.rt),
                livello: f.livello,
                polarita: f.polarita,
                tic: f64::from(r.tic),
                base_peak: None,
                filtro: if f.f.riferimento() {
                    format!("function {} lock mass", f.f.indice)
                } else {
                    format!("function {}", f.f.indice)
                },
                precursore: precursor,
                position: f.dat.map(|m| Position {
                    membro: u16::try_from(m).unwrap_or(0),
                    offset: r.offset,
                    lunghezza: u32::try_from(
                        u64::from(r.count) * f.cod.map_or(0, |c| c.width() as u64),
                    )
                    .unwrap_or(u32::MAX),
                }),
            });
            self.map.push((i, k));
        }
    }

    /// Every scan of the index read once and its records checked: a damaged one leaves the index, is counted in `bad` (`warn.waters.damaged`) and
    /// named per function in a notice. Runs before the first read through the index, so the index, the counts and the reads agree.
    pub(super) fn ready(&mut self) -> Result<()> {
        if self.checked {
            return Ok(());
        }
        let mut keep = Vec::with_capacity(self.map.len());
        let mut per: BTreeMap<u32, usize> = BTreeMap::new();
        for &(i, r) in &self.map {
            let f = &self.funs[i];
            let ok = match (f.cod, f.dat) {
                (Some(cod), Some(dat)) => {
                    match idx::extension(&f.righe, r, cod.width() as u64, f.dat_len) {
                        Ok((off, len)) => whole(cod, &self.c.read_from(dat, off, len)?),
                        Err(_) => false,
                    }
                }
                _ => true,
            };
            if !ok {
                *per.entry(f.f.indice).or_default() += 1;
            }
            keep.push(ok);
        }
        let mut k = keep.iter();
        self.indice.retain(|_| *k.next().unwrap_or(&true));
        let mut k = keep.iter();
        self.map.retain(|_| *k.next().unwrap_or(&true));
        self.bad = per.values().sum();
        for (n, x) in per {
            let a = format!("{}: function {n}: {x} damaged scans left out", self.c.name);
            self.warnings.push(("meta.nonDecodificato".into(), a));
        }
        if let Some(s) = self.samples.first_mut() {
            s.scansioni = self.indice.len();
        }
        self.checked = true;
        Ok(())
    }
}
