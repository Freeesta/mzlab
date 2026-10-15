//! A made-up `Lettore` that carries every optional field a reader may give (precursor with and without a value, every polarity, empty scans,
//! Thermo-like filters, `extra_mzml` marks and userParams, 64-bit intensities, arrays of unequal length): the oracle of the rows built without text
//! (`mzidx_lettore`) and the source of the synthetic benchmark (`examples/mzidx_tempi.rs`).
#![allow(dead_code)]
#![allow(
    clippy::disallowed_methods,
    clippy::field_reassign_with_default,
    clippy::inconsistent_digit_grouping,
    clippy::too_many_arguments,
    clippy::useless_format
)]

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{
    Campione, Campo, Formato, Lettore, Posizione, VoceCromatogramma, VoceScansione,
};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

/// A tiny deterministic generator (xorshift), so the scans are the same on every run.
pub struct Caso(pub u64);

impl Caso {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// What a scan carries besides its peaks.
#[derive(Clone)]
pub struct Scheda {
    pub spettro: Spectrum,
    pub extra: SpectrumExtra,
}

pub struct Finto {
    pub formato: Formato,
    pub campioni: Vec<Campione>,
    pub indice: Vec<VoceScansione>,
    pub schede: Vec<Scheda>,
    pub testata: Option<Header>,
    pub tracce: Vec<Chromatogram>,
}

fn spettro(
    k: usize,
    level: u8,
    rt: f64,
    polarity: i8,
    profile: bool,
    filter: &str,
    precursor: Option<f64>,
    mz: Vec<f64>,
    it: Vec<f32>,
) -> Spectrum {
    Spectrum {
        index: k,
        id: format!("controllerType=0 controllerNumber=1 scan={}", k + 1),
        level,
        rt,
        polarity,
        profile,
        filter: filter.into(),
        precursor,
        mz,
        intensity: it,
        wide: None,
    }
}

impl Finto {
    /// `n` scans of one cycle MS1, MS2, MS2, MS3, empty, with every combination of the optional fields; `peaks` peaks in the survey scans.
    pub fn vario(n: usize, peaks: usize, seed: u64) -> Finto {
        let mut r = Caso(seed | 1);
        let mut schede = Vec::new();
        for k in 0..n {
            let (level, pol): (u8, i8) = match k % 5 {
                0 => (1, [1, -1, 0, 1][(k / 5) % 4]),
                1 | 2 => (2, [1, -1, 0][(k / 5) % 3]),
                3 => (3, 1),
                _ => (1, -1),
            };
            let np = if k % 7 == 6 {
                0
            } else {
                1 + (r.next() as usize % peaks.max(1))
            };
            // every third scan on a coarse grid: equal m/z across scans and groups (the order of equal m/z in a section)
            let mut mz: Vec<f64> = if k % 3 == 0 {
                (0..np)
                    .map(|_| 100.0 + (r.next() % 40) as f64 * 0.5)
                    .collect()
            } else {
                (0..np).map(|_| 50.0 + r.unit() * 1500.0).collect()
            };
            if k % 41 == 13 {
                // out of range, rounding to 32 bits across a bucket edge, not a number
                mz.extend([-5.0, 12345.6, 99.999_999_9, 9998.999_999_9, f64::NAN]);
            }
            mz.sort_by(|a, b| a.total_cmp(b));
            let np = mz.len();
            let it: Vec<f32> = (0..np).map(|_| (r.unit() * 1e6) as f32).collect();
            let filter = match k % 5 {
                0 => "FTMS + p ESI Full ms [100.0000-1000.0000]".to_string(),
                1 => format!("ITMS + c ESI d Full ms2 {:.4}@cid35.00 [50.0000-500.0000]", 300.0 + k as f64),
                2 => format!("FTMS + c ESI d Full ms2 {:.4}@hcd30.00 [50.0000-500.0000]", 400.5 + k as f64),
                3 => format!("ITMS + c ESI d Full ms3 500.1000@cid35.00 250.2000@hcd25.00 [50.0000-300.0000]"),
                _ => String::new(),
            };
            let precursor = match (level, k % 3) {
                (1, _) if k % 25 == 0 => Some(555.5),
                (1, _) => None,
                (_, 0) => None,
                (2, _) => Some(300.0 + k as f64 + r.unit()),
                _ => Some(250.2 + r.unit()),
            };
            let mut sp = spettro(
                k,
                level,
                k as f64 * 0.013 + r.unit() * 0.001,
                pol,
                k % 2 == 0,
                &filter,
                precursor,
                mz,
                it,
            );
            if k % 11 == 3 {
                sp.wide = Some(
                    (0..np)
                        .map(|i| f64::from(sp.intensity[i]) * 1.000_000_1)
                        .collect(),
                );
            }
            if k % 13 == 5 {
                sp.id = format!("a\"b<c>&d scan={k}");
            }
            let mut extra = SpectrumExtra::default();
            extra.sim = k % 19 == 4;
            extra.observed = k % 3 == 1;
            extra.window = (k % 4 != 3).then_some((50.0, 1500.0));
            if k % 2 == 0 {
                extra.params.push((
                    "opentfraw.resolution".into(),
                    format!("{}", 15000 + (k % 5) * 10000),
                ));
            }
            match k % 31 {
                2 => extra
                    .params
                    .push(("cv:MS:1000130".into(), "positive scan".into())),
                5 => extra
                    .params
                    .push(("cv:MS:1000045".into(), "collision energy".into())),
                8 => extra
                    .params
                    .push(("cv:MS:1000341".into(), "precursor ion spectrum".into())),
                _ => {}
            }
            if k % 29 == 4 {
                extra.params.push(("note".into(), "see MS:1000744".into()));
            }
            if k % 23 == 7 {
                extra.params.push(("scan \"mode\"".into(), "a<b&c".into()));
            }
            schede.push(Scheda { spettro: sp, extra });
        }
        Finto::da_schede(schede)
    }

    pub fn da_schede(schede: Vec<Scheda>) -> Finto {
        let indice = schede
            .iter()
            .enumerate()
            .map(|(k, s)| VoceScansione {
                id: s.spettro.id.clone(),
                rt: s.spettro.rt,
                livello: s.spettro.level,
                polarita: s.spettro.polarity,
                tic: 0.0,
                picco_base: None,
                filtro: s.spettro.filter.clone(),
                precursore: s.spettro.precursor,
                posizione: (k % 9 != 8).then_some(Posizione {
                    membro: (k % 2) as u16,
                    offset: 100 * k as u64,
                    lunghezza: 77,
                }),
            })
            .collect();
        let tracce = vec![
            Chromatogram {
                index: 0,
                id: "TIC".into(),
                kind: ChromKind::Tic,
                q1: None,
                q3: None,
                unit: String::new(),
                time: vec![0.0, 1.0],
                intensity: vec![5.0, 6.0],
            },
            Chromatogram {
                index: 1,
                id: "vuota".into(),
                kind: ChromKind::Bpc,
                q1: None,
                q3: None,
                unit: String::new(),
                time: vec![],
                intensity: vec![],
            },
            Chromatogram {
                index: 2,
                id: "pressione".into(),
                kind: ChromKind::Pressure,
                q1: None,
                q3: None,
                unit: String::new(),
                time: vec![0.0],
                intensity: vec![1.0],
            },
        ];
        Finto {
            formato: Formato::Altro("finto"),
            campioni: vec![Campione {
                id: 7,
                nome: "finto".into(),
                scansioni: schede.len(),
            }],
            indice,
            schede,
            testata: None,
            tracce,
        }
    }

    /// Some scans with one intensity more than m/z values (what `mzidx_lettore::spettro` cuts); the mzML of such a reader is not valid.
    pub fn con_lunghezze_diverse(mut self) -> Finto {
        for (k, s) in self.schede.iter_mut().enumerate() {
            if k % 17 == 9 && s.spettro.wide.is_none() {
                s.spettro.intensity.push(1.0);
            }
        }
        self
    }

    /// The header of a Q-TOF run with a serial number and an acquisition program that names the normalised collision energy.
    pub fn con_testata(mut self, h: Header) -> Finto {
        self.testata = Some(h);
        self
    }
}

impl Lettore for Finto {
    fn formato(&self) -> Formato {
        self.formato
    }
    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }
    fn indice(&mut self, _c: usize) -> Result<&[VoceScansione]> {
        Ok(&self.indice)
    }
    fn scansione(&mut self, _c: usize, k: usize) -> Result<Spectrum> {
        self.schede
            .get(k)
            .map(|s| s.spettro.clone())
            .ok_or_else(|| Error::new("error.badIndex"))
    }
    fn intestazione(&mut self, _c: usize, _k: usize) -> Result<Vec<Campo>> {
        Ok(Vec::new())
    }
    fn cromatogrammi(&mut self, _c: usize) -> Result<Vec<VoceCromatogramma>> {
        Ok(self
            .tracce
            .iter()
            .map(|t| VoceCromatogramma {
                id: t.id.clone(),
                tipo: t.kind,
                unita: String::new(),
            })
            .collect())
    }
    fn cromatogramma(&mut self, _c: usize, id: &str) -> Result<Chromatogram> {
        self.tracce
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex"))
    }
    fn metodo(&mut self, _c: usize) -> Result<String> {
        Ok(String::new())
    }
    fn testata_mzml(&mut self, _c: usize) -> Result<Option<Header>> {
        Ok(self.testata.clone())
    }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.schede
            .get(s.index)
            .map(|x| x.extra.clone())
            .unwrap_or_default()
    }
}
