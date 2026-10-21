//! `mzml_confronta A.mzML B.mzML [options]`: how equal two mzML files are, spectrum by spectrum. Prints only counts and the largest deviations, never a value of
//! the data. File A is held in memory (decoded), file B is streamed against it.
//!
//! Pairing: by the number after `scan=` in the spectrum id when every spectrum of A has a different one (`--per scan`), else by position (`--per indice`).
//! Compared: ms level, polarity, retention time, the precursors (m/z, charge), number of points, m/z and intensity of every point, and the chromatograms
//! (paired by type, Q1 and Q3, then by order).
//! Tolerances (declared, `docs/studi/convertitore/README.md` §4): m/z 0 ppm for the same source (`--f32`: 0.05 ppm, B writes 32-bit m/z), RT 1e-6 min,
//! relative intensity 1e-6 (`--numpress`: 2e-4, the declared error of the codecs). `--tol-mz-ppm`, `--tol-rt`, `--tol-int` set them by hand.
//! Exit code: 0 within tolerance and nothing unpaired, 1 otherwise, 2 a file that cannot be read or a wrong option.

use std::collections::HashMap;
use std::fs::File;

use mzlab_core::mzmlvalida::{analizza, ChromInfo, Item, SpecInfo};

#[derive(Default, Clone, Copy)]
struct Dev {
    /// Values compared, values over the tolerance, largest deviation.
    n: usize,
    over: usize,
    max: f64,
}

impl Dev {
    fn add(&mut self, d: f64, tol: f64) {
        self.n += 1;
        self.max = self.max.max(d);
        self.over += usize::from(d > tol);
    }

    fn show(&self, what: &str, unit: &str, tol: f64) -> String {
        format!(
            "{what}: {} compared, {} over the tolerance {tol:e} {unit}, largest {:.3e} {unit}\n",
            self.n, self.over, self.max
        )
    }
}

struct Tol {
    mz_ppm: f64,
    rt: f64,
    int: f64,
}

#[derive(Default)]
struct Stats {
    paired: usize,
    only_b: usize,
    level: usize,
    polarity: usize,
    rt: Dev,
    prec_count: usize,
    prec_mz: Dev,
    charge_differ: usize,
    charge_one_sided: usize,
    points: usize,
    mz: Dev,
    mz_spectra: usize,
    int: Dev,
    chrom_paired: usize,
    chrom_only_b: usize,
    chrom_points: usize,
    chrom_time: Dev,
    chrom_int: Dev,
}

fn ppm(a: f64, b: f64) -> f64 {
    if a == b {
        0.0
    } else {
        (a - b).abs() / a.abs().max(b.abs()).max(f64::MIN_POSITIVE) * 1e6
    }
}

fn rel(a: f64, b: f64) -> f64 {
    if a == b {
        0.0
    } else {
        (a - b).abs() / a.abs().max(b.abs())
    }
}

/// The number after `scan=` in a native id.
fn scan_of(id: &str) -> Option<u64> {
    let rest = id.split("scan=").nth(1)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

fn compare_spectrum(a: &SpecInfo, b: &SpecInfo, tol: &Tol, s: &mut Stats) {
    s.level += usize::from(a.level != b.level);
    s.polarity += usize::from(a.polarity != b.polarity);
    if let (Some(x), Some(y)) = (a.rt, b.rt) {
        s.rt.add((x - y).abs(), tol.rt);
    }
    if a.precursors.len() != b.precursors.len() {
        s.prec_count += 1;
    }
    for (p, q) in a.precursors.iter().zip(&b.precursors) {
        if let (Some(x), Some(y)) = (p.0, q.0) {
            s.prec_mz.add(ppm(x, y), tol.mz_ppm);
        }
        match (p.1, q.1) {
            (Some(x), Some(y)) => s.charge_differ += usize::from(x != y),
            (None, None) => {}
            _ => s.charge_one_sided += 1,
        }
    }
    if a.mz.len() != b.mz.len() || a.intensity.len() != b.intensity.len() {
        s.points += 1;
        return;
    }
    let before = s.mz.over;
    a.mz.iter()
        .zip(&b.mz)
        .for_each(|(x, y)| s.mz.add(ppm(*x, *y), tol.mz_ppm));
    s.mz_spectra += usize::from(s.mz.over > before);
    a.intensity
        .iter()
        .zip(&b.intensity)
        .for_each(|(x, y)| s.int.add(rel(*x, *y), tol.int));
}

/// Type, Q1 and Q3 (rounded to 1e-4) of a chromatogram, and its order among the ones with the same key.
fn chrom_key(c: &ChromInfo, seen: &mut HashMap<String, usize>) -> String {
    let q = |v: Option<f64>| v.map_or(String::new(), |x| format!("{:.4}", x));
    let base = format!("{}|{}|{}", c.kind, q(c.q1), q(c.q3));
    let n = seen.entry(base.clone()).or_insert(0);
    *n += 1;
    format!("{base}|{n}")
}

fn compare_chrom(a: &ChromInfo, b: &ChromInfo, tol: &Tol, s: &mut Stats) {
    if a.time.len() != b.time.len() || a.intensity.len() != b.intensity.len() {
        s.chrom_points += 1;
        return;
    }
    a.time
        .iter()
        .zip(&b.time)
        .for_each(|(x, y)| s.chrom_time.add((x - y).abs(), tol.rt));
    a.intensity
        .iter()
        .zip(&b.intensity)
        .for_each(|(x, y)| s.chrom_int.add(rel(*x, *y), tol.int));
}

struct Options {
    files: [String; 2],
    by_scan: Option<bool>,
    tol: Tol,
}

fn options() -> Result<Options, String> {
    let (mut files, mut by_scan) = (Vec::new(), None);
    let mut tol = Tol {
        mz_ppm: 0.0,
        rt: 1e-6,
        int: 1e-6,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut number = |k: &str| {
            args.next()
                .and_then(|v| v.parse::<f64>().ok())
                .ok_or(format!("{k} needs a number"))
        };
        match a.as_str() {
            "--f32" => tol.mz_ppm = 0.05,
            "--numpress" => tol.int = 2e-4,
            "--tol-mz-ppm" => tol.mz_ppm = number("--tol-mz-ppm")?,
            "--tol-rt" => tol.rt = number("--tol-rt")?,
            "--tol-int" => tol.int = number("--tol-int")?,
            "--per" => match args.next().as_deref() {
                Some("scan") => by_scan = Some(true),
                Some("indice") => by_scan = Some(false),
                _ => return Err("--per needs scan or indice".into()),
            },
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ => files.push(a),
        }
    }
    let [a, b] = <[String; 2]>::try_from(files).map_err(|_| "usage: mzml_confronta A.mzML B.mzML [--per scan|indice] [--f32] [--numpress] [--tol-mz-ppm N] [--tol-rt N] [--tol-int N]".to_string())?;
    Ok(Options {
        files: [a, b],
        by_scan,
        tol,
    })
}

fn load(path: &str) -> Result<(Vec<SpecInfo>, Vec<ChromInfo>), String> {
    let (mut specs, mut chroms) = (Vec::new(), Vec::new());
    let f = File::open(path).map_err(|e| format!("{path}: {e}"))?;
    analizza(f, &mut |i| match i {
        Item::Spectrum(s) => specs.push(s),
        Item::Chromatogram(c) => chroms.push(c),
    })
    .map_err(|e| format!("{path}: {e}"))?;
    Ok((specs, chroms))
}

fn main() {
    let code = match options().and_then(run) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            2
        }
    };
    std::process::exit(code);
}

fn run(o: Options) -> Result<i32, String> {
    let (specs, chroms) = load(&o.files[0])?;
    let scans: Vec<Option<u64>> = specs.iter().map(|s| scan_of(&s.id)).collect();
    let unique = scans.iter().all(Option::is_some)
        && scans
            .iter()
            .flatten()
            .collect::<std::collections::HashSet<_>>()
            .len()
            == scans.len();
    let by_scan = o.by_scan.unwrap_or(unique);
    let at: HashMap<u64, usize> = scans
        .iter()
        .enumerate()
        .filter_map(|(i, s)| Some((s.filter(|_| by_scan)?, i)))
        .collect();
    let mut chrom_at: HashMap<String, usize> = HashMap::new();
    let mut seen = HashMap::new();
    for (i, c) in chroms.iter().enumerate() {
        chrom_at.insert(chrom_key(c, &mut seen), i);
    }
    let (mut s, mut used, mut chrom_used, mut seen) = (
        Stats::default(),
        vec![false; specs.len()],
        vec![false; chroms.len()],
        HashMap::new(),
    );
    let mut n_b = (0, 0);
    let f = File::open(&o.files[1]).map_err(|e| format!("{}: {e}", o.files[1]))?;
    analizza(f, &mut |item| match item {
        Item::Spectrum(b) => {
            n_b.0 += 1;
            let i = if by_scan {
                scan_of(&b.id).and_then(|k| at.get(&k).copied())
            } else {
                Some(b.index).filter(|i| *i < specs.len())
            };
            match i.filter(|i| !used[*i]) {
                Some(i) => {
                    used[i] = true;
                    s.paired += 1;
                    compare_spectrum(&specs[i], &b, &o.tol, &mut s);
                }
                None => s.only_b += 1,
            }
        }
        Item::Chromatogram(b) => {
            n_b.1 += 1;
            match chrom_at.get(&chrom_key(&b, &mut seen)).copied() {
                Some(i) => {
                    chrom_used[i] = true;
                    s.chrom_paired += 1;
                    compare_chrom(&chroms[i], &b, &o.tol, &mut s);
                }
                None => s.chrom_only_b += 1,
            }
        }
    })
    .map_err(|e| format!("{}: {e}", o.files[1]))?;
    Ok(report(
        &o,
        &s,
        (specs.len(), chroms.len()),
        n_b,
        (&used[..], &chrom_used[..]),
        by_scan,
    ))
}

fn report(
    o: &Options,
    s: &Stats,
    a: (usize, usize),
    b: (usize, usize),
    used: (&[bool], &[bool]),
    by_scan: bool,
) -> i32 {
    let only_a = used.0.iter().filter(|u| !**u).count();
    let chrom_only_a = used.1.iter().filter(|u| !**u).count();
    let mut t = format!(
        "A: {} spectra, {} chromatograms. B: {} spectra, {} chromatograms.\n",
        a.0, a.1, b.0, b.1
    );
    t += &format!(
        "spectra paired by {}: {}; only in A: {only_a}; only in B: {}\n",
        if by_scan { "scan number" } else { "position" },
        s.paired,
        s.only_b
    );
    t += &format!(
        "ms level differs: {}; polarity differs: {}\n",
        s.level, s.polarity
    );
    t += &s.rt.show("retention time", "min", o.tol.rt);
    t += &format!("precursors: {} spectra with another number of precursors; charge differs {}, present in one file only {}\n", s.prec_count, s.charge_differ, s.charge_one_sided);
    t += &s.prec_mz.show("precursor m/z", "ppm", o.tol.mz_ppm);
    t += &format!("spectra with another number of points: {}\n", s.points);
    t += &s.mz.show("m/z", "ppm", o.tol.mz_ppm);
    t += &format!(
        "spectra with a point over the m/z tolerance: {}\n",
        s.mz_spectra
    );
    t += &s.int.show("intensity", "relative", o.tol.int);
    t += &format!("chromatograms paired: {}; only in A: {chrom_only_a}; only in B: {}; with another number of points: {}\n", s.chrom_paired, s.chrom_only_b, s.chrom_points);
    t += &s.chrom_time.show("chromatogram time", "min", o.tol.rt);
    t += &s
        .chrom_int
        .show("chromatogram intensity", "relative", o.tol.int);
    let bad = only_a
        + chrom_only_a
        + s.only_b
        + s.chrom_only_b
        + s.level
        + s.polarity
        + s.prec_count
        + s.charge_differ
        + s.points
        + s.chrom_points
        + [
            &s.rt,
            &s.prec_mz,
            &s.mz,
            &s.int,
            &s.chrom_time,
            &s.chrom_int,
        ]
        .iter()
        .map(|d| d.over)
        .sum::<usize>();
    t += if bad == 0 {
        "verdict: equal within the tolerance\n"
    } else {
        "verdict: DIFFERENT\n"
    };
    print!("{t}");
    i32::from(bad > 0)
}
