//! Times the table steps of the series on synthetic data: alignment, isotopes, RT correction, normalisation and the deconvolution of isomers.
//!
//!     cargo run --release -p mzlab-feat --example perf_lotto8 -- [features=50000] [files=40]
//!
//! Prints one line per operation (best of three) and the SHA-256 of its result, so two builds can be compared for time and for bytes.

use std::time::Instant;

use mzlab_feat::align::{align, AlignParams};
use mzlab_feat::deconvolve::{deconvolve_isomers, DeconvolveParams};
use mzlab_feat::isotopes::{group, IsoParams};
use mzlab_feat::norm::{factors, Method};
use mzlab_feat::rtmap::RtParams;
use mzlab_feat::rtwarp::{map_from_table, Method as Warp, WarpParams};
use mzlab_feat::samples::Sample;
use mzlab_feat::table::{Aligned, FileFeatures, FLAG_GOOD};
use sha2::{Digest, Sha256};

struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn best<T>(name: &str, mut run: impl FnMut() -> T, hash: impl Fn(&T) -> Vec<u8>) {
    let mut secs = f64::MAX;
    let mut last = None;
    for _ in 0..3 {
        let t = Instant::now();
        let out = run();
        secs = secs.min(t.elapsed().as_secs_f64());
        last = Some(out);
    }
    let sha = Sha256::digest(hash(&last.expect("ran")));
    println!(
        "{name:<34} {:>9.1} ms  {:02x}{:02x}{:02x}{:02x}",
        secs * 1e3,
        sha[0],
        sha[1],
        sha[2],
        sha[3]
    );
}

/// `n` ions (m/z, RT) seen in `files` files with a ppm of jitter, a drift per file, a few doubled peaks and a tenth of noise features.
fn series(n: usize, files: usize, drift: bool) -> Vec<FileFeatures> {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let ions: Vec<(f64, f64)> = (0..n)
        .map(|_| (100.0 + rng.unit() * 900.0, 0.5 + rng.unit() * 19.0))
        .collect();
    (0..files)
        .map(|k| {
            let mut rows: Vec<(f64, f32, f64)> = Vec::with_capacity(n + n / 10);
            for &(mz, rt) in &ions {
                let shift = if drift {
                    0.03 * k as f64 + 0.02 * k as f64 * (rt / 3.0).sin()
                } else {
                    0.0
                };
                let m = mz * (1.0 + (rng.unit() - 0.5) * 2e-6);
                let area = 1e5 + rng.unit() * 1e7;
                rows.push((m, (rt + shift + (rng.unit() - 0.5) * 0.01) as f32, area));
                if rng.unit() < 0.02 {
                    rows.push((m, (rt + shift + 0.04) as f32, area * 0.3));
                }
            }
            for _ in 0..n / 10 {
                rows.push((
                    100.0 + rng.unit() * 900.0,
                    (0.5 + rng.unit() * 19.0) as f32,
                    1e5 + rng.unit() * 1e6,
                ));
            }
            rows.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut f = FileFeatures::default();
            for (m, rt, area) in rows {
                f.mz.push(m);
                f.rt.push(rt);
                f.left.push(rt - 0.05);
                f.right.push(rt + 0.05);
                f.area.push(area);
                f.area_win.push(area);
                f.height.push((area / 10.0) as f32);
                f.flags.push(FLAG_GOOD);
                f.charge.push(0);
                f.iso_group.push(0);
            }
            f
        })
        .collect()
}

fn bytes(al: &Aligned) -> Vec<u8> {
    let mut v = Vec::new();
    for x in al.mz.iter().chain(&al.rt).chain(&al.area) {
        v.extend(x.to_bits().to_le_bytes());
    }
    for x in al.height.iter().chain(&al.apex_rt).chain(&al.width) {
        v.extend(x.to_bits().to_le_bytes());
    }
    v.extend(&al.state);
    v
}

fn f64s(v: &[f64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_bits().to_le_bytes()).collect()
}

fn bench_norm(refs: &[&FileFeatures], files: usize) {
    let al = align(refs, &AlignParams::default()).expect("align");
    let samples: Vec<Sample> = (0..files)
        .map(|k| Sample {
            file: format!("f{k}.mzML"),
            sha256: String::new(),
            kind: "sample".into(),
            time_min: None,
            replicate: 1,
            group: String::new(),
            conc: None,
            unit: None,
            order: k as u32 + 1,
            istd: false,
            note: String::new(),
        })
        .collect();
    for (name, method) in [("median", Method::Median), ("sum", Method::Sum)] {
        best(
            &format!("norm {name} {} rows", al.len()),
            || factors(&al, &samples, &method).expect("norm"),
            |v| f64s(v),
        );
    }
}

fn bench_isotopes(one: &FileFeatures, n: usize) {
    best(
        &format!("isotopes {n}"),
        || {
            let mut f = one.clone();
            group(&mut f, &IsoParams::default()).expect("isotopes");
            f
        },
        |f| {
            let mut v: Vec<u8> = f.flags.clone();
            v.extend(f.charge.iter().map(|&c| c as u8));
            v.extend(f.iso_group.iter().flat_map(|g| g.to_le_bytes()));
            v
        },
    );
}

fn bench_rtwarp() {
    let drifted = series(100_000, 6, true);
    let drefs: Vec<&FileFeatures> = drifted.iter().collect();
    let coarse = align(
        &drefs,
        &AlignParams {
            rt_gap: 0.5,
            ..AlignParams::default()
        },
    )
    .expect("align");
    for (name, method) in [("loess", Warp::Loess), ("ransac", Warp::Ransac)] {
        let w = WarpParams {
            method,
            ..WarpParams::default()
        };
        best(
            &format!("rtwarp {name} {} anchors", coarse.len()),
            || {
                map_from_table(
                    &coarse,
                    &RtParams {
                        min_height: 1e3,
                        ..RtParams::default()
                    },
                    &w,
                )
            },
            |m| f64s(&m.shift),
        );
    }
}

#[allow(clippy::needless_range_loop)]
fn bench_deconvolve() {
    // 200 scans of 30 co-eluting isomers with 60 fragments each
    let mut rng = Rng(7);
    let (scans, isomers) = (200usize, 30usize);
    let spectra: Vec<Vec<f64>> = (0..isomers)
        .map(|_| {
            (0..60)
                .map(|_| {
                    if rng.unit() < 0.4 {
                        rng.unit() * 100.0
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();
    let (mut mzs, mut ints, mut rts) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..scans {
        let t = i as f64 * 0.01;
        let (mut mz, mut it) = (Vec::new(), Vec::new());
        for f in 0..60 {
            let v: f64 = (0..isomers)
                .map(|j| {
                    spectra[j][f]
                        * (-0.5 * ((t - 0.3 - 1.4 * j as f64 / isomers as f64) / 0.05).powi(2))
                            .exp()
                })
                .sum();
            if v > 1.0 {
                mz.push(100.0 + f as f64 * 7.3);
                it.push(v);
            }
        }
        mzs.push(mz);
        ints.push(it);
        rts.push(t);
    }
    let xr: Vec<f64> = (0..scans).map(|i| i as f64 * 0.01).collect();
    let xy: Vec<f64> = xr
        .iter()
        .map(|&t| {
            (0..isomers)
                .map(|j| {
                    (-0.5 * ((t - 0.3 - 1.4 * j as f64 / isomers as f64) / 0.05).powi(2)).exp()
                })
                .sum()
        })
        .collect();
    for (name, k) in [("3 components", 3usize), ("30 isomers", 30)] {
        let p = DeconvolveParams {
            max_components: k,
            ..DeconvolveParams::default()
        };
        best(
            &format!("deconvolve {name}"),
            || deconvolve_isomers(&mzs, &ints, &rts, Some((&xr, &xy)), &p),
            |r| format!("{r:?}").into_bytes(),
        );
    }
}

fn main() {
    let arg = |i: usize, d: usize| {
        std::env::args()
            .nth(i)
            .and_then(|a| a.parse().ok())
            .unwrap_or(d)
    };
    let (n, files) = (arg(1, 50_000), arg(2, 40));

    let tables = series(n, files, false);
    let refs: Vec<&FileFeatures> = tables.iter().collect();
    best(
        &format!("align {n} x {files}"),
        || align(&refs, &AlignParams::default()).expect("align"),
        bytes,
    );
    bench_norm(&refs, files);
    bench_isotopes(&tables[0], n);
    bench_rtwarp();
    bench_deconvolve();
}
