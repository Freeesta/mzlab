//! ANDI-MS read in windows: spectra that are not sorted by m/z, a TIC summed from the points, spectra longer than one window, a layout without record variables, and how many reads of the file a
//! spectrum takes (the file is built here, with the encoder of `comune::andi`).
#![cfg(feature = "andi")]

mod comune;

use comune::andi::*;
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};
use std::cell::Cell;
use std::rc::Rc;

/// Bytes in memory that count the reads asked of them.
struct Conta(Vec<u8>, Rc<Cell<usize>>);

impl ReadAt for Conta {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + 1);
        self.0.read_at(off, buf)
    }
}

/// An ANDI-MS file with `spettri` as its scans; `registri`: `point_number` is the unlimited dimension (record variables) or a fixed one; `tic`: the file carries `total_intensity`.
fn file(
    spettri: &[Vec<(f64, f64)>],
    tipo_mz: u32,
    tipo_it: u32,
    registri: bool,
    tic: bool,
) -> Vec<u8> {
    let conti: Vec<usize> = spettri.iter().map(Vec::len).collect();
    let totale: usize = conti.iter().sum();
    let mut inizi = vec![0usize];
    for &n in &conti[..conti.len() - 1] {
        inizi.push(inizi.last().unwrap() + n);
    }
    let n = spettri.len();
    let mz: Vec<f64> = spettri.iter().flatten().map(|p| p.0).collect();
    let it: Vec<f64> = spettri.iter().flatten().map(|p| p.1).collect();
    let dims = [
        ("point_number", if registri { 0u32 } else { totale as u32 }),
        ("scan_number", n as u32),
        ("instrument_number", 1),
        ("_32_byte_string", 32),
        ("range", 2),
    ];
    let per_scan = |nome: &str, ty: u32, x: Vec<f64>| var(nome, &[1], ty, x);
    let mut vm = var("mass_values", &[0], tipo_mz, mz);
    vm.att.push(("units".into(), t("M/Z")));
    let mut vi = var("intensity_values", &[0], tipo_it, it);
    vi.att
        .push(("units".into(), t("Arbitrary Intensity Units")));
    let mut vars = vec![
        per_scan(
            "scan_acquisition_time",
            6,
            (0..n).map(|k| 60.0 + 1.5 * k as f64).collect(),
        ),
        per_scan("mass_range_min", 6, vec![40.0; n]),
        per_scan("mass_range_max", 6, vec![400.0; n]),
        per_scan("scan_index", 4, inizi.iter().map(|&x| x as f64).collect()),
        per_scan("point_count", 4, conti.iter().map(|&x| x as f64).collect()),
        vm,
        vi,
        car("instrument_name", &[2, 3], "GCMS Test", 32),
    ];
    if tic {
        vars.insert(
            1,
            per_scan(
                "total_intensity",
                6,
                spettri
                    .iter()
                    .map(|s| s.iter().map(|p| p.1).sum())
                    .collect(),
            ),
        );
    }
    let g = vec![
        ("experiment_title".to_string(), t("run")),
        ("experiment_type".to_string(), t("Centroided Mass Spectrum")),
        (
            "test_ionization_polarity".to_string(),
            t("Positive Polarity"),
        ),
    ];
    cdf(
        false,
        &dims,
        &g,
        &vars,
        if registri { totale as u32 } else { 0 },
    )
}

fn apri(byte: Vec<u8>) -> (Box<dyn Lettore>, Rc<Cell<usize>>) {
    let letture = Rc::new(Cell::new(0));
    let fs = FileSet {
        membri: vec![Membro {
            nome: "campione.cdf".into(),
            dim: byte.len() as u64,
            dati: Box::new(Conta(byte, letture.clone())),
            sha1: None,
        }],
    };
    (mzlab_lettori::apri(fs, &Limiti::sito()).unwrap(), letture)
}

#[test]
fn points_out_of_order_are_sorted_and_ties_keep_the_file_order() {
    let s = vec![
        vec![(3.0, 30.0), (1.0, 10.0), (2.0, 20.0), (1.0, 11.0)],
        vec![(5.0, 1.0), (6.0, 2.0)],
        vec![(9.0, 7.0), (8.0, 6.0), (8.0, 5.0)],
    ];
    for registri in [true, false] {
        let (mut l, _) = apri(file(&s, 6, 6, registri, true));
        let a = l.scansione(0, 0).unwrap();
        assert_eq!(a.mz, vec![1.0, 1.0, 2.0, 3.0]);
        assert_eq!(
            (0..4).map(|i| a.intensity_f64(i)).collect::<Vec<_>>(),
            vec![10.0, 11.0, 20.0, 30.0]
        );
        let b = l.scansione(0, 1).unwrap();
        assert_eq!(b.mz, vec![5.0, 6.0]);
        let c = l.scansione(0, 2).unwrap();
        assert_eq!(c.mz, vec![8.0, 8.0, 9.0]);
        assert_eq!(
            (0..3).map(|i| c.intensity_f64(i)).collect::<Vec<_>>(),
            vec![6.0, 5.0, 7.0]
        );
    }
}

#[test]
fn the_tic_summed_from_the_points_is_that_of_the_scans_read() {
    let s: Vec<Vec<(f64, f64)>> = (0..5)
        .map(|k| {
            (0..40 + k)
                .map(|i| {
                    (
                        500.0 - i as f64 * 1.5,
                        0.1 + ((i * 7 + k) % 13) as f64 * 3.3,
                    )
                })
                .collect()
        })
        .collect();
    // total_intensity absent; the intensity type decides whether the points are f32 or wider
    for tipo_it in [4, 5, 6] {
        let (mut l, _) = apri(file(&s, 6, tipo_it, true, false));
        let indice = l.indice(0).unwrap().to_vec();
        for (k, v) in indice.iter().enumerate() {
            let sp = l.scansione(0, k).unwrap();
            let somma: f64 = (0..sp.mz.len()).map(|i| sp.intensity_f64(i)).sum();
            assert_eq!(v.tic, somma, "type {tipo_it} scan {k}");
        }
    }
}

#[test]
fn a_long_spectrum_crosses_windows_and_takes_few_reads() {
    let punti = |k: usize| -> Vec<(f64, f64)> {
        (0..40_000)
            .map(|i| {
                (
                    50.0 + i as f64 * 0.01 + k as f64,
                    1.0 + ((i * 31 + k) % 977) as f64,
                )
            })
            .collect()
    };
    let s = vec![punti(0), punti(1)];
    for registri in [true, false] {
        let (mut l, letture) = apri(file(&s, 6, 6, registri, true));
        let aperte = letture.get();
        for k in 0..2 {
            let sp = l.scansione(0, k).unwrap();
            assert_eq!(sp.mz.len(), 40_000);
            for (i, (mz, it)) in s[k].iter().enumerate() {
                assert_eq!(
                    (sp.mz[i], sp.intensity_f64(i)),
                    (*mz, *it),
                    "scan {k} point {i}"
                );
            }
        }
        // two arrays of 40 000 points, 16 bytes of record: a handful of windows, not 160 000 blocks
        assert!(
            letture.get() - aperte < 40,
            "{} reads",
            letture.get() - aperte
        );
    }
}

#[test]
fn every_type_of_the_arrays_reads_the_same_in_any_layout() {
    let s: Vec<Vec<(f64, f64)>> = (0..4)
        .map(|k| {
            (0..300)
                .map(|i| {
                    (
                        100.0 + (i * 3 % 100) as f64 + k as f64,
                        5.0 + ((i + k) % 50) as f64,
                    )
                })
                .collect()
        })
        .collect();
    for registri in [true, false] {
        for (tm, ti) in [(3, 3), (4, 4), (5, 5), (6, 6), (6, 4), (5, 3)] {
            let (mut l, _) = apri(file(&s, tm, ti, registri, true));
            for k in 0..4 {
                let sp = l.scansione(0, k).unwrap();
                let mut atteso = s[k].clone();
                atteso.sort_by(|a, b| a.0.total_cmp(&b.0));
                assert_eq!(
                    sp.mz,
                    atteso.iter().map(|p| p.0).collect::<Vec<_>>(),
                    "types {tm}/{ti} scan {k}"
                );
                for (i, p) in atteso.iter().enumerate() {
                    assert_eq!(
                        sp.intensity_f64(i),
                        p.1,
                        "types {tm}/{ti} scan {k} point {i}"
                    );
                }
            }
        }
    }
}
