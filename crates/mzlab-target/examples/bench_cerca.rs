// Timing and output fingerprint of the target screen on a synthetic run (1000 targets x 3 adducts, 3000 MS1 scans of ~1500 peaks).
// Run with `cargo run --release -p mzlab-target --example bench_cerca`; the printed hash must not change between versions.

use std::time::Instant;

use mzlab_core::mzml::Spectrum;
use mzlab_target::{
    cerca_targets, AdductType, CheckFlags, TargetEntry, TargetList, TargetSearchOptions,
};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn unit(&mut self) -> f64 {
        self.next() as f64 / (1u64 << 31) as f64
    }
}

fn entry(id: usize, f: &str, mass: f64) -> TargetEntry {
    TargetEntry {
        id: format!("t{id}"),
        name_en: format!("T{id}"),
        name_it: format!("T{id}"),
        neutral_formula: f.to_string(),
        monoisotopic_mass: mass,
        mz_mh: mass + AdductType::MH.delta_mass(),
        mz_mna: mass + AdductType::MNa.delta_mass(),
        mz_mnh4: mass + AdductType::MNH4.delta_mass(),
        smiles: String::new(),
        inchikey: String::new(),
        inchikey_connectivity: String::new(),
        cas: None,
        pubchem_cid: None,
        effect_class: String::new(),
        chemical_class: String::new(),
        role: String::new(),
        psychoactive: false,
        synonyms: Vec::new(),
        check: None,
        check_flags: CheckFlags::default(),
    }
}

fn main() {
    let mut r = Lcg(5);
    let n_targets = 1000;
    let n_scans = 3000;
    let mut entries = Vec::new();
    let mut peaks: Vec<(f64, f64, f64)> = Vec::new(); // mz, rt apex, height
    for k in 0..n_targets {
        let (c, h, n, o) = (
            6 + r.next() % 20,
            6 + r.next() % 30,
            r.next() % 5,
            r.next() % 6,
        );
        let formula = format!("C{c}H{h}N{n}O{o}");
        let mass = 12.0 * c as f64
            + 1.00782503223 * h as f64
            + 14.00307400443 * n as f64
            + 15.99491461957 * o as f64;
        let e = entry(k, &formula, mass);
        if k % 10 == 0 {
            peaks.push((e.mz_mh, 1.0 + r.unit() * 28.0, 1e5 + r.unit() * 1e6));
        }
        entries.push(e);
    }
    let mut spectra = Vec::new();
    for s in 0..n_scans {
        let rt = 0.5 + s as f64 * 0.01;
        let mut p: Vec<(f64, f32)> = (0..1500)
            .map(|_| (80.0 + r.unit() * 700.0, (500.0 + r.unit() * 20000.0) as f32))
            .collect();
        for &(mz, c, h) in &peaks {
            let g = h * (-0.5 * ((rt - c) / 0.08).powi(2)).exp();
            if g > 50.0 {
                p.push((mz, g as f32));
                p.push((mz + 1.003355, (g * 0.25) as f32));
            }
        }
        p.sort_by(|a, b| a.0.total_cmp(&b.0));
        spectra.push(Spectrum {
            index: s,
            id: format!("s{s}"),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: String::new(),
            precursor: None,
            mz: p.iter().map(|x| x.0).collect(),
            intensity: p.iter().map(|x| x.1).collect(),
            wide: None,
        });
    }
    let list = TargetList::from_entries(entries);
    let t = Instant::now();
    let m = cerca_targets(&list, &spectra, &TargetSearchOptions::default());
    let el = t.elapsed();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in format!("{m:?}").bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    println!(
        "matches {} hash {h:016x} time {:.0} ms",
        m.len(),
        el.as_secs_f64() * 1e3
    );
}
