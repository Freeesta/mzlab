//! Synthetic Shimadzu containers for the tests: invented numbers, written the way the real files lay them out (docs/agenti/shimadzu.md).
#![allow(dead_code)]

use super::ole::ole;
use std::io::Write;

fn u16s(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}
fn u32s(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}
fn u64s(v: &[u64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// A `File Property` of a `.qgd` with invented text; `operator` is a made-up identity the tests look for.
pub fn file_property() -> Vec<u8> {
    let mut b = vec![0u8; 2000];
    let mut put = |at: usize, s: &str| b[at..at + s.len()].copy_from_slice(s.as_bytes());
    put(4, "1.0");
    put(20, "operatorXyz");
    put(148, "4.53");
    put(204, "sample A");
    put(300, "operatorXyz");
    put(580, "C:\\Users\\personQ\\data\\run1.qgd");
    // 2021-03-04T05:06:07
    let ft = (1_614_834_367u64 + 11_644_473_600) * 10_000_000;
    b[52..60].copy_from_slice(&ft.to_le_bytes());
    b[508..516].copy_from_slice(&ft.to_le_bytes());
    b
}

fn scans(pesi: &[(u16, &[(u16, u64)])]) -> (Vec<u8>, Vec<u32>) {
    let mut raw = Vec::new();
    let mut off = Vec::new();
    for (w, pts) in pesi {
        off.push(raw.len() as u32);
        let mut r = vec![0u8; 32];
        r[20..22].copy_from_slice(&w.to_le_bytes());
        r[22..24].copy_from_slice(&(pts.len() as u16).to_le_bytes());
        for (mz, y) in *pts {
            r.extend_from_slice(&mz.to_le_bytes());
            r.extend_from_slice(&y.to_le_bytes()[..usize::from(*w)]);
        }
        raw.extend(r);
    }
    (raw, off)
}

/// A `.qgd` of `n` EI scans of m/z 50–51 (x20 in the file) and a SIM of three ions repeated in the last five.
pub fn qgd(n: usize) -> Vec<u8> {
    let sim: &[(u16, u64)] = &[(1000, 10), (1500, 20), (2000, 30)];
    let spec: Vec<Vec<(u16, u64)>> = (0..n)
        .map(|i| {
            vec![
                (1000, 100 + i as u64),
                (1020, 7),
                (1100, 3000 + i as u64),
                (1200 + i as u16, 1),
            ]
        })
        .collect();
    let mut def: Vec<(u16, &[(u16, u64)])> = spec.iter().map(|s| (2u16, s.as_slice())).collect();
    def.extend((0..5).map(|_| (2u16, sim)));
    let (raw, off) = scans(&def);
    let tic: Vec<u64> = spec
        .iter()
        .map(|s| s.iter().map(|p| p.1).sum())
        .chain((0..5).map(|_| 60))
        .collect();
    let m = def.len();
    ole(&[
        ("File Property", file_property()),
        (
            "GCMS Configuration/System",
            b"Shimadzu GCMS-QP2010 Ultra\0".to_vec(),
        ),
        ("GCMS Raw Data/MS Raw Data", raw),
        ("GCMS Raw Data/Spectrum Index", u32s(&off)),
        (
            "GCMS Raw Data/Retention Time",
            u32s(&(0..m as u32).map(|i| 1000 + 500 * i).collect::<Vec<_>>()),
        ),
        ("GCMS Raw Data/TIC Data", u64s(&tic)),
    ])
}

/// A `.qgd` of MRM records (kind 102): two transitions in each of `n` records.
pub fn qgd_mrm(n: usize) -> Vec<u8> {
    let mut raw = Vec::new();
    let mut off = Vec::new();
    for i in 0..n {
        off.push(raw.len() as u32);
        let mut r = vec![0u8; 32];
        r[20..22].copy_from_slice(&102u16.to_le_bytes());
        r[22..24].copy_from_slice(&2u16.to_le_bytes());
        for (q1, q3, y) in [(4000u16, 2000u16, 10 + i as u16), (4000, 2400, 20)] {
            r.extend(u16s(&[q1, q3, y]));
        }
        raw.extend(r);
    }
    let tic = vec![30u64; n];
    ole(&[
        ("GCMS Raw Data/MS Raw Data", raw),
        ("GCMS Raw Data/Spectrum Index", u32s(&off)),
        (
            "GCMS Raw Data/Retention Time",
            u32s(&(0..n as u32).map(|i| 600 * i).collect::<Vec<_>>()),
        ),
        ("GCMS Raw Data/TIC Data", u64s(&tic)),
    ])
}

/// An LCMS-2020 `.lcd`: `n` scans of event 0 (a scan) then `n` of event 1 (three ions, SIM).
pub fn lcd2020(n: usize) -> Vec<u8> {
    let mut raw = Vec::new();
    let mut off = Vec::new();
    let mut tic = Vec::new();
    for i in 0..2 * n {
        off.push(raw.len() as u32);
        let sim = i >= n;
        let pts: Vec<(u16, u32)> = if sim {
            vec![(6000, 5), (6400, 6), (7000, 7)]
        } else {
            (0..8)
                .map(|j| (2000 + 100 * j, 10 * (j as u32 + 1) + i as u32))
                .collect()
        };
        let mut r = vec![0u8; 64];
        r[8..10].copy_from_slice(&u16::from(sim).to_le_bytes());
        r[52..54].copy_from_slice(&4u16.to_le_bytes());
        r[54..56].copy_from_slice(&(pts.len() as u16).to_le_bytes());
        for (mz, y) in &pts {
            r.extend(u16s(&[*mz]));
            r.extend(y.to_le_bytes());
        }
        tic.push(pts.iter().map(|p| u64::from(p.1)).sum::<u64>());
        raw.extend(r);
    }
    let m = 2 * n as u32;
    ole(&[
        ("Mass Raw Data/MS Raw Data", raw),
        ("Mass Raw Data/Spectrum Index", u32s(&off)),
        (
            "Mass Raw Data/Retention Time",
            u32s(&(0..m).map(|i| 300 * i).collect::<Vec<_>>()),
        ),
        ("Mass Raw Data/TIC Data", u64s(&tic)),
        ("LSS Raw Data/x", vec![1, 2, 3]),
    ])
}

fn tlm_record(neg: bool, pts: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut r = vec![0u8; 44];
    r[33] = if neg { 0xc0 } else { 0 };
    r[40..44].copy_from_slice(&(pts.len() as u32).to_le_bytes());
    for (a, b, y) in pts {
        r.extend(u32s(&[*a, *b, *y]));
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    z.write_all(&r).unwrap();
    let z = z.finish().unwrap();
    let mut out = u32s(&[0xffff_ffff, r.len() as u32, z.len() as u32]);
    out.extend(z);
    out
}

/// A triple-quadrupole `.lcd`: `n` records with two MRM transitions (m/z 176.1 > 159.1 and 200.5 > 100.2).
pub fn lcd_tlm(n: usize) -> Vec<u8> {
    let mut raw = Vec::new();
    let mut idx = Vec::new();
    for i in 0..n {
        idx.extend(u32s(&[0, i as u32, raw.len() as u32, 0, 0, 0]));
        raw.extend(tlm_record(
            false,
            &[(17610, 15910, 50 + i as u32), (20050, 10020, 7)],
        ));
    }
    ole(&[
        ("TLM Raw Data/MS Raw Data", raw),
        ("TLM Raw Data/Spectrum Index", idx),
        (
            "TLM Raw Data/Retention Time",
            u32s(&(0..n as u32).map(|i| 600 * i).collect::<Vec<_>>()),
        ),
        ("TLM Raw Data/TIC Data", u64s(&vec![57u64; n])),
    ])
}

/// A Q-TOF `.lcd` (only its storage name matters) and a container with only the PDA part.
pub fn lcd_qtof() -> Vec<u8> {
    ole(&[("QTFL RawData/MS Raw Data", vec![1, 2, 3, 4])])
}
pub fn lcd_solo_pda() -> Vec<u8> {
    ole(&[("LSS Raw Data/x", vec![1, 2, 3])])
}
