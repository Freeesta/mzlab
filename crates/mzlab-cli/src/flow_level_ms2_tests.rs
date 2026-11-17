//! MEASURE (no default is changed): MS2 level gates on 50 small compounds (m/z < 200, 3 true fragments against a library entry of 3 strong peaks and 5 peaks at 1-2 %)
//! and 50 co-isolated ones (isobaric interferent at 40 %, purity 0.60), in a library of 300 entries. Level reached, failing gates, entropy drop and the decoy q-value
//! are pinned with the current thresholds; see `docs/agenti/livelli.md` (MISURA). Synthetic data, fixed seed.

use std::path::PathBuf;

use mzlab_core::json::{self, Value};
use mzlab_feat::flow_level::level_text;

use crate::flow_ident_tests::run;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

type Spectrum = Vec<(f64, f64)>;

/// Library entry and query spectrum of small compound `i`: 3 strong peaks and 5 at 1-2 % in the library, only the 3 strong ones in the query.
fn small(i: usize) -> (f64, Spectrum, Spectrum) {
    let prec = 120.0 + 1.3 * i as f64;
    let strong = [
        (prec - 20.0, 1000.0),
        (prec - 40.0, 600.0),
        (prec - 60.0, 300.0),
    ];
    let weak = (0..5).map(|j| (51.0 + 6.1 * j as f64 + 0.37, 10.0 + 2.5 * j as f64));
    let lib = strong.iter().copied().chain(weak).collect();
    (prec, lib, strong.to_vec())
}

/// Target and interferent of co-isolated compound `i` (same precursor, six fragments each, different m/z).
fn pair(i: usize) -> (f64, Spectrum, Spectrum) {
    let prec = 250.0 + 3.1 * i as f64;
    let inten = [1000.0, 800.0, 600.0, 400.0, 300.0, 200.0];
    let t = (0..6)
        .map(|j| (prec - 30.0 - 22.0 * j as f64, inten[j]))
        .collect();
    let o = (0..6)
        .map(|j| (prec - 41.0 - 19.0 * j as f64, inten[5 - j]))
        .collect();
    (prec, t, o)
}

fn norm(s: &Spectrum, w: f64) -> Spectrum {
    let sum: f64 = s.iter().map(|p| p.1).sum();
    s.iter().map(|p| (p.0, w * p.1 / sum * 1000.0)).collect()
}

fn msp(name: &str, prec: f64, peaks: &Spectrum) -> String {
    let mut t = format!(
        "Name: {name}\nPrecursorMZ: {prec:.4}\nPrecursor_type: [M+H]+\nIon_mode: Positive\nNum Peaks: {}\n",
        peaks.len()
    );
    for (m, i) in peaks {
        t += &format!("{m:.4} {i:.2}\n");
    }
    t + "\n"
}

fn mgf(id: &str, prec: f64, peaks: &Spectrum, rng: &mut Lcg) -> String {
    let mut t = format!("BEGIN IONS\nTITLE={id}\nPEPMASS={prec:.4}\nCHARGE=1+\nIONMODE=Positive\n");
    for (m, i) in peaks {
        let (m, i) = (m + (rng.next() - 0.5) * 0.006, i * (0.8 + 0.4 * rng.next()));
        t += &format!("{m:.4} {i:.2}\n");
    }
    t + "END IONS\n"
}

/// The library: the 50 small entries, the 50 targets of the pairs and 200 random fillers.
fn library() -> String {
    let mut t = String::new();
    for i in 0..50 {
        let (p, lib, _) = small(i);
        t += &msp(&format!("Small {i}"), p, &lib);
    }
    for i in 0..50 {
        let (p, tgt, _) = pair(i);
        t += &msp(&format!("Pair {i}"), p, &tgt);
    }
    let mut r = Lcg(5);
    for k in 0..200 {
        let p = 100.0 + 400.0 * r.next();
        let pk: Spectrum = (0..6)
            .map(|j| {
                (
                    50.0 + 12.0 * j as f64 + 10.0 * r.next(),
                    100.0 + 900.0 * r.next(),
                )
            })
            .collect();
        t += &msp(&format!("Filler {k}"), p, &pk);
    }
    t
}

/// What a set of queries gives: per query id (best entropy score, decoy q, level, failing gates).
struct Row {
    id: String,
    score: f64,
    q: Option<f64>,
    level: String,
    failed: Vec<String>,
}

/// Runs `spectra.match` (decoys on) and `annot.level` on the MGF `queries`.
fn measure(tag: &str, queries: &str) -> Vec<Row> {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("mzlab_cli_lvl_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("work")).unwrap();
    std::fs::write(dir.join("lib.msp"), library()).unwrap();
    std::fs::write(dir.join("q.mgf"), queries).unwrap();
    run(&dir, "spectral_entropy", r#"{"decoys":"spectrum"}"#);
    let hits = std::fs::read(dir.join("work/m.json")).unwrap();
    let levels = json::parse(level_text(&[&hits], &[]).unwrap().as_bytes()).unwrap();
    let doc = json::parse(&hits).unwrap();
    let rows = levels.get("rows").unwrap().arr().iter().map(|r| {
        let id = r.get("id").unwrap().str().unwrap().to_string();
        let first = doc
            .get("hits")
            .unwrap()
            .arr()
            .iter()
            .find(|h| h.get("query").unwrap().str() == Some(&id));
        let failed = r
            .get("gates")
            .unwrap()
            .arr()
            .iter()
            .filter(|g| g.get("ok").unwrap().bool() == Some(false))
            .map(|g| g.get("id").unwrap().str().unwrap().to_string())
            .collect();
        Row {
            score: first
                .and_then(|h| h.get("score"))
                .and_then(Value::f64)
                .unwrap_or(0.0),
            q: first.and_then(|h| h.get("decoy_q")).and_then(Value::f64),
            level: r.get("level").unwrap().str().unwrap().to_string(),
            failed,
            id,
        }
    });
    let out = rows.collect();
    let _ = std::fs::remove_dir_all(&dir);
    out
}

/// Queries of the three sets with `scans` spectra per feature (one per file): small compounds, co-isolated ones, and the targets alone.
fn queries(scans: usize) -> [String; 3] {
    let mut rng = Lcg(11);
    let (mut s, mut c, mut p) = (String::new(), String::new(), String::new());
    for k in 0..scans {
        for i in 0..50 {
            let (prec, _, q) = small(i);
            s += &mgf(&format!("s{i}_{k}"), prec, &q, &mut rng);
            let (prec, t, o) = pair(i);
            let mixed: Spectrum = norm(&t, 0.6).into_iter().chain(norm(&o, 0.4)).collect();
            c += &mgf(&format!("c{i}_{k}"), prec, &mixed, &mut rng);
            p += &mgf(&format!("p{i}_{k}"), prec, &norm(&t, 1.0), &mut rng);
        }
    }
    [s, c, p]
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// Counts of the levels and of the failing gates over `rows`.
fn summary(rows: &[Row]) -> String {
    let mut lv = std::collections::BTreeMap::new();
    let mut g = std::collections::BTreeMap::new();
    for r in rows {
        *lv.entry(r.level.clone()).or_insert(0) += 1;
        for f in &r.failed {
            *g.entry(f.clone()).or_insert(0) += 1;
        }
    }
    format!("levels {lv:?} failing {g:?}")
}

fn q_of(r: &[Row]) -> (f64, f64) {
    let q: Vec<f64> = r.iter().filter_map(|x| x.q).collect();
    (median(q.clone()), q.into_iter().fold(0.0, f64::max))
}

fn near(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() <= 0.01, "{what}: {got:.3} vs {want}");
}

/// Pinned: (scans per feature, levels and failing gates of small / co-isolated / pure targets, median entropy small / chimeric / pure, decoy q median and max).
#[test]
fn level_gates_measure() {
    let want = [
        (
            1,
            [
                r#"levels {"3a": 50} failing {"scans": 50, "shared": 50}"#,
                r#"levels {"3a": 50} failing {"scans": 50, "score": 27}"#,
                r#"levels {"3a": 50} failing {"scans": 50}"#,
            ],
            [0.902, 0.747, 0.999],
        ),
        (
            3,
            [
                r#"levels {"3a": 150} failing {"shared": 150}"#,
                r#"levels {"2a": 70, "3a": 80} failing {"score": 80}"#,
                r#"levels {"2a": 150} failing {}"#,
            ],
            [0.902, 0.749, 0.999],
        ),
    ];
    for (scans, text, ent) in want {
        let [s, c, p] = queries(scans);
        let sets = [measure("s", &s), measure("c", &c), measure("p", &p)];
        for (k, rows) in sets.iter().enumerate() {
            assert_eq!(summary(rows), text[k], "scans={scans} set {k}");
            near(
                median(rows.iter().map(|x| x.score).collect()),
                ent[k],
                "entropy",
            );
        }
        let (qs, qc) = (q_of(&sets[0]), q_of(&sets[1]));
        // the decoys do not separate here: every best hit has q = 0, the chimeric ones included (a lower bound, `lib.fdr.note`)
        assert_eq!((qs, qc), ((0.0, 0.0), (0.0, 0.0)), "scans={scans}");
    }
}
