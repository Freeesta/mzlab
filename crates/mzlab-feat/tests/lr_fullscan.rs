//! The pipeline on the unit axis (`Recipe::lr`) on a synthetic full-scan series built here (centroids shifted by +0.30 Da like the real instrument: the parent
//! decays, two isomers of the +O product at one nominal mass grow with different kinetics, an ion grows with its 13C peak, a product of a product rises and
//! falls, one ion is flat in the treated files only, one is in every file): the ions are rows of the table within 0.35 Da and 0.25 min, the isomers are
//! two rows, the 13C peak is registered as an isotopologue, the Na adduct of the parent has a family, two runs give the same bytes. Synthetic data only.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::series::{align_and_fill, analyse_file};
use mzlab_feat::table::{Aligned, FileFeatures, CELL_DETECTED, FLAG_ISOTOPE, ROLE_UNDETERMINED};

const SHIFT: f64 = 0.30;
const TIMES: [f64; 5] = [0.0, 5.0, 15.0, 30.0, 60.0];

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let (a, b) = (self.next().max(1e-12), self.next());
        libm_sqrt(-2.0 * a.ln()) * (2.0 * std::f64::consts::PI * b).cos()
    }
}

fn libm_sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// (m/z, RT, sigma, amplitude) of what a file of treatment time `t` holds (`None` = the blank).
fn pool(t: Option<f64>) -> Vec<(f64, f64, f64, f64)> {
    let mut out = vec![(311.10, 3.3, 0.07, 4.0e5), (157.09, 3.0, 0.08, 1.5e5)];
    let Some(k) = t else { return out };
    out.push((241.0641, 9.0, 0.07, 3.0e6 * (-0.05 * k).exp()));
    out.push((263.0460, 9.0, 0.07, 6.0e5 * (-0.05 * k).exp()));
    out.push((257.0590, 7.4, 0.07, 1.2e6 * (1.0 - (-0.06 * k).exp())));
    out.push((257.0590, 8.3, 0.07, 6.0e5 * (1.0 - (-0.03 * k).exp())));
    let g = 8.0e5 * (1.0 - (-0.05 * k).exp());
    out.push((333.20, 6.8, 0.07, g));
    out.push((334.20, 6.8, 0.07, 0.11 * g));
    out.push((
        355.10,
        5.2,
        0.07,
        9.0e5 * ((-0.01 * k).exp() - (-0.15 * k).exp()),
    ));
    if k > 0.0 {
        out.push((289.10, 4.4, 0.07, 5.0e5));
    }
    out
}

/// The `.mzidx` of one file: Q1 scans every 3 s from 0.5 to 14 min, noise peaks and the ions of `pool`.
fn index_of(t: Option<f64>, seed: u64) -> Vec<u8> {
    let mut rng = Lcg(seed);
    let comps = pool(t);
    let name = format!("lr{seed}");
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: name.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 270;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let mut peaks: Vec<(f64, f32)> = (0..40)
            .map(|_| {
                (
                    100.0 + 300.0 * rng.next(),
                    (3e3 * (0.8 * rng.normal()).exp()) as f32,
                )
            })
            .collect();
        for &(m, c, s, a) in &comps {
            let v = a * (-0.5 * ((rt - c) / s).powi(2)).exp() * (1.0 + 0.04 * rng.normal());
            if v > 100.0 {
                peaks.push((m + SHIFT + 0.03 * rng.normal(), v as f32));
            }
        }
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment=1", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: "+ p ESI Q1MS [100.000-400.000]".into(),
            precursor: None,
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, &format!("{name}.mzML")).unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

fn series() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = TIMES
        .iter()
        .enumerate()
        .map(|(i, &t)| index_of(Some(t), 11 + i as u64))
        .collect();
    v.push(index_of(None, 99));
    v
}

fn open(idx: &[u8]) -> Index<Vec<u8>> {
    Index::open(idx.to_vec(), idx.len() as u64).unwrap()
}

/// Recipe, tables of the files and the aligned table of the series.
fn run(idxs: &[Vec<u8>]) -> (Recipe, Vec<FileFeatures>, Aligned) {
    let r = Recipe::for_index(&mut open(&idxs[1])).unwrap();
    let files: Vec<FileFeatures> = idxs
        .iter()
        .map(|b| analyse_file(&mut open(b), &r).unwrap())
        .collect();
    let (t, _) = align_and_fill(&files, &r, |k| Ok(open(&idxs[k]))).unwrap();
    (r, files, t)
}

fn rows_at(t: &Aligned, mz: f64, rt: f64) -> Vec<usize> {
    (0..t.len())
        .filter(|&r| (t.mz[r] - (mz + SHIFT)).abs() <= 0.35 && (t.rt[r] - rt).abs() <= 0.25)
        .collect()
}

#[test]
fn unit_profile_finds_the_ions_and_separates_the_isomers() {
    let idxs = series();
    let (r, files, t) = run(&idxs);
    assert!(
        r.detect.axis.is_da(),
        "the index of a unit-resolution file selects the unit profile"
    );
    for (mz, rt, n) in [
        (241.0641, 9.0, 1),
        (257.0590, 7.4, 1),
        (257.0590, 8.3, 1),
        (333.20, 6.8, 1),
        (355.10, 5.2, 1),
        (289.10, 4.4, 1),
        (311.10, 3.3, 1),
    ] {
        let rows = rows_at(&t, mz, rt);
        assert!(!rows.is_empty(), "{mz} at {rt} min is not in the table");
        assert!(
            rows.len() <= n + 1,
            "{mz} at {rt} min is {} rows",
            rows.len()
        );
    }
    let (a, b) = (rows_at(&t, 257.0590, 7.4), rows_at(&t, 257.0590, 8.3));
    assert!(
        a.iter().all(|x| !b.contains(x)),
        "the isomers of one nominal mass share a row"
    );
    // the parent is detected in every treated file and the product only after t0
    let parent = rows_at(&t, 241.0641, 9.0)[0];
    assert!((0..TIMES.len()).all(|k| t.state[t.cell(parent, k)] == CELL_DETECTED));
    let iso = rows_at(&t, 257.0590, 7.4)[0];
    assert!(
        t.state[t.cell(iso, 0)] != CELL_DETECTED,
        "the product is not in t0"
    );
    // the 13C peak of 333.20 is registered as an isotopologue in the files
    let k = 4;
    let j = (0..files[k].len())
        .find(|&i| {
            (files[k].mz[i] - (334.20 + SHIFT)).abs() < 0.2 && (files[k].rt[i] - 6.8).abs() < 0.2
        })
        .expect("the 13C peak");
    assert!(files[k].flags[j] & FLAG_ISOTOPE != 0 && files[k].iso_group[j] != 0);
}

#[test]
fn unit_profile_is_deterministic() {
    let idxs = series();
    let (_, _, a) = run(&idxs);
    let (_, _, b) = run(&idxs);
    assert_eq!(a.to_bytes(), b.to_bytes());
}

#[test]
fn unit_profile_does_not_name_an_adduct_without_ms2() {
    let idxs = series();
    let (_, files, _) = run(&idxs);
    let f = &files[1];
    let pos = |mz: f64, rt: f64| {
        (0..f.len())
            .find(|&i| {
                (f.mz[i] - (mz + SHIFT)).abs() < 0.2 && (f64::from(f.rt[i]) - rt).abs() < 0.15
            })
            .unwrap()
    };
    let (m, na) = (pos(241.0641, 9.0), pos(263.0460, 9.0));
    assert_eq!(
        f.family_id[m], f.family_id[na],
        "the sodium adduct is in the family of the parent"
    );
    assert_eq!(
        f.role[na], ROLE_UNDETERMINED,
        "the step of +22 is not named an adduct without an MS2"
    );
}

#[test]
fn unit_preview_proposes_the_window_and_the_tolerance_of_rt() {
    let idx = index_of(Some(30.0), 14);
    let mut ix = open(&idx);
    let r = Recipe::for_index(&mut ix).unwrap();
    let json = mzlab_feat::params::preview(&mut ix, &r.detect).unwrap();
    let v = mzlab_core::json::parse(json.as_bytes()).unwrap();
    let num = |a: &str, b: &str| v.get(a).and_then(|x| x.get(b)).and_then(|x| x.f64());
    let da = num("da", "value").expect("window of m/z");
    assert!((0.1..=0.5).contains(&da), "{da}");
    assert!(num("da", "n").unwrap() >= 20.0);
    let tol = v
        .get("rt_tol")
        .and_then(|x| x.f64())
        .expect("tolerance of RT");
    assert!((0.05..=1.0).contains(&tol), "{tol}");
    assert!(v
        .get("ppm")
        .and_then(|x| x.get("value"))
        .is_none_or(|x| x.f64().is_none()));
}

#[test]
fn detection_progress_is_monotone_ends_at_the_total_and_leaves_the_traces_alone() {
    let idxs = series();
    let r = Recipe::for_index(&mut open(&idxs[1])).unwrap();
    let plain = mzlab_feat::trace::detect(&mut open(&idxs[0]), &r.detect).unwrap();
    let mut seen = Vec::new();
    let with = mzlab_feat::trace::detect_with(&mut open(&idxs[0]), &r.detect, &mut |d, t| {
        seen.push((d, t))
    })
    .unwrap();
    assert_eq!(plain, with);
    assert!(!seen.is_empty() && seen.windows(2).all(|w| w[0].0 <= w[1].0));
    assert!(seen.last().is_some_and(|&(d, t)| d == t));
}

// ---- screening of a series at unit resolution: profile scans, m/z drift between files, duplicate rows (`lrscreen`) ----

/// Drift of each of the 8 samples (t0, t1..t5, two blanks), Da; the median is 0.
const DRIFT: [f64; 8] = [-0.2, -0.1, 0.0, 0.1, 0.2, -0.1, 0.1, 0.0];
/// (m/z, RT, amplitude): 4 ions apart, and a pair 1.0 Da apart at one RT whose areas move together (only the distance keeps them two rows).
const TRUE_IONS: [(f64, f64, f64); 5] = [
    (181.2, 3.0, 6.0e5),
    (223.3, 4.2, 9.0e5),
    (307.4, 6.8, 7.0e5),
    (265.1, 5.5, 8.0e5),
    (266.1, 5.5, 8.0e5),
];

/// Known noise rows: chemical background ions that every sample holds (they are also the anchors the drift of a file is measured on).
const BACKGROUND: [(f64, f64, f64); 8] = [
    (121.4, 1.5, 3.0e5),
    (139.7, 2.4, 2.5e5),
    (158.3, 3.6, 2.0e5),
    (203.6, 4.9, 3.5e5),
    (244.8, 6.1, 2.2e5),
    (287.2, 7.5, 2.8e5),
    (329.5, 8.4, 3.2e5),
    (371.9, 9.3, 2.4e5),
];

/// The `.mzidx` of sample `k` of the screening series: the same peak list as profile scans (0.1 Da grid, FWHM 0.7 Da) or as centroids.
fn screen_index(k: usize, profile: bool) -> Vec<u8> {
    let mut rng = Lcg(500 + k as u64);
    let name = format!("scr{k}{}", u8::from(profile));
    let mut w = MzmlWriter::new(Vec::new(), &Header { run: name.clone(), ..Default::default() }).unwrap();
    let n = 240;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let mut peaks: Vec<(f64, f64)> = (0..30)
            .map(|_| (100.0 + 300.0 * rng.next(), 1.5e3 * (0.5 * rng.normal()).exp()))
            .collect();
        for &(m, c, a) in TRUE_IONS.iter().chain(&BACKGROUND) {
            let v = a * (1.0 + 0.1 * k as f64) * (-0.5 * ((rt - c) / 0.07).powi(2)).exp();
            if v > 100.0 {
                peaks.push((m + DRIFT[k] + 0.02 * rng.normal(), v));
            }
        }
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mz, y): (Vec<f64>, Vec<f32>) = if profile {
            profile_points(&peaks)
        } else {
            peaks.iter().map(|p| (p.0, p.1 as f32)).unzip()
        };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile,
            filter: "+ p ESI Q1MS [100.000-400.000]".into(),
            precursor: None,
            mz,
            intensity: y,
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, &format!("{name}.mzML")).unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

/// The peaks rendered on a 0.1 Da grid as Gaussians of FWHM 0.7 Da (points within 1.5 Da of a peak; overlapping peaks add).
fn profile_points(peaks: &[(f64, f64)]) -> (Vec<f64>, Vec<f32>) {
    let sigma = 0.7 / 2.3548;
    let mut grid = std::collections::BTreeMap::<i64, f64>::new();
    for &(m, a) in peaks {
        let c = (m * 10.0).round() as i64;
        for g in c - 15..=c + 15 {
            let x = g as f64 / 10.0 - m;
            *grid.entry(g).or_default() += a * (-0.5 * (x / sigma).powi(2)).exp();
        }
    }
    grid.into_iter().map(|(g, v)| (g as f64 / 10.0, v as f32)).unzip()
}

fn screen_series(profile: bool) -> Vec<Vec<u8>> {
    (0..DRIFT.len()).map(|k| screen_index(k, profile)).collect()
}

/// The table of the screening series, through the recipe the engine builds for the first file (`adjusted` centroids it when asked).
fn screen_run(idxs: &[Vec<u8>], tweak: impl Fn(&mut Recipe)) -> (Recipe, Aligned) {
    let mut r = Recipe::for_index(&mut open(&idxs[1])).unwrap();
    tweak(&mut r);
    let r = mzlab_feat::lrscreen::adjusted(&mut open(&idxs[1]), r).unwrap();
    let files: Vec<FileFeatures> = idxs.iter().map(|b| analyse_file(&mut open(b), &r).unwrap()).collect();
    let (t, _) = align_and_fill(&files, &r, |k| Ok(open(&idxs[k]))).unwrap();
    (r, t)
}

fn near(t: &Aligned, mz: f64, rt: f64) -> Vec<usize> {
    (0..t.len()).filter(|&r| (t.mz[r] - mz).abs() <= 0.35 && (t.rt[r] - rt).abs() <= 0.25).collect()
}

#[test]
fn profile_scans_with_auto_give_the_rows_of_the_centroid_input_and_no_duplicates() {
    let (_, from_profile) = screen_run(&screen_series(true), |_| {});
    let (_, from_centroids) = screen_run(&screen_series(false), |_| {});
    assert_eq!(from_centroids.len(), TRUE_IONS.len() + BACKGROUND.len(), "the true ions and the known noise rows");
    assert_eq!(from_profile.len(), from_centroids.len(), "rows of the profile and of the centroid series");
    for t in [&from_profile, &from_centroids] {
        for a in 0..t.len() {
            for b in a + 1..t.len() {
                let close = (t.mz[a] - t.mz[b]).abs() <= 0.6 && (t.rt[a] - t.rt[b]).abs() <= 0.1;
                assert!(!close, "rows {a} and {b} are one ion: {} {} / {} {}", t.mz[a], t.rt[a], t.mz[b], t.rt[b]);
            }
        }
    }
}

#[test]
fn with_drift_every_true_ion_is_one_row_present_in_all_samples() {
    let (_, t) = screen_run(&screen_series(true), |_| {});
    for &(mz, rt, _) in &TRUE_IONS {
        let rows = near(&t, mz, rt);
        assert_eq!(rows.len(), 1, "{mz} at {rt} min is {} rows", rows.len());
        assert!((0..DRIFT.len()).all(|k| !mzlab_feat::table::no_peak(t.state[t.cell(rows[0], k)]) && t.area[t.cell(rows[0], k)] > 0.0));
    }
}

#[test]
fn recenter_recovers_the_drift_of_each_file_within_three_hundredths_of_a_dalton() {
    let idxs = screen_series(false);
    let r = Recipe::for_index(&mut open(&idxs[1])).unwrap();
    let files: Vec<FileFeatures> = idxs.iter().map(|b| analyse_file(&mut open(b), &r).unwrap()).collect();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let found = mzlab_feat::lrscreen_recenter::shifts(&refs, &r).unwrap();
    for (k, s) in found.iter().enumerate() {
        assert!((s.mz_da - DRIFT[k]).abs() <= 0.03, "file {k}: found {} for a drift of {}", s.mz_da, DRIFT[k]);
        assert_eq!(s.applied, DRIFT[k].abs() >= 0.05);
    }
}

#[test]
fn dedup_keeps_true_neighbours_one_dalton_apart_and_recenter_off_reproduces_the_split() {
    let (_, on) = screen_run(&screen_series(false), |_| {});
    assert_eq!(near(&on, 265.1, 5.5).len(), 1);
    assert_eq!(near(&on, 266.1, 5.5).len(), 1);
    // with every switch off, the profile points are detected as they are: today's path, thousands of rows
    use mzlab_feat::lrscreen::Switch::Off;
    let (_, today) = screen_run(&screen_series(true), |r| {
        r.screen.centroid = Off;
        r.screen.recenter = Off;
        r.screen.dedup = Off;
    });
    assert!(today.len() > 100 * on.len(), "{} rows", today.len());
}
