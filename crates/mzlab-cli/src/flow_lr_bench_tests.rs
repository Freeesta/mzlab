//! Benchmark of the unit-resolution feature chain on a synthetic QTRAP-like Q1 profile with ground truth and decoys (`tools/lr_sintetico.py`, run into a
//! temp dir; skipped without python3 and numpy, `MZLAB_GOLDEN_STRICT=1` fails instead). Path A = `spectra.centroid` -> `index.mzidx` -> `features.detect`,
//! path B = the profile index as it is -> `features.detect`. The rows are scored against the truth (0.2 Da / 4 s); the floors hold what the chain does today.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_feat_wasm::run;

const TOL_DA: f64 = 0.2;
const TOL_S: f64 = 4.0;
/// Compounds below this apex (counts) are not expected to be found.
const MIN_APEX: f64 = 150.0;

struct Ion {
    compound: u64,
    kind: String,
    mz: f64,
    rt: f64,
    height: f64,
}

struct Truth {
    ions: Vec<Ion>,
    pairs: Vec<(usize, usize)>,
}

/// `(mz, rt in s)` of every row of a `.mzfeatf`.
type Rows = Vec<(f64, f64)>;

#[derive(Debug, Default)]
struct Score {
    rows: usize,
    recall: f64,
    precision: f64,
    pairs_both: usize,
    pairs: usize,
    split: f64,
    mean_dm: f64,
    merged: usize,
}

fn tools() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools")
}

/// The files of the generator in a new folder; `None` without python3 and numpy.
fn generate(tag: &str, jitter: f64) -> Option<PathBuf> {
    let d = std::env::temp_dir().join(format!("mzlab_lr_bench_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let ok = Command::new("python3")
        .arg(tools().join("lr_sintetico.py"))
        .arg(&d)
        .args(["--seed", "1", "--jitter", &jitter.to_string()])
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return None;
    }
    Some(d)
}

fn truth(d: &Path) -> Truth {
    let v =
        mzlab_core::json::parse(&std::fs::read(d.join("LR_bench-s1_truth.json")).unwrap()).unwrap();
    let ions: Vec<Ion> = v
        .get("ions")
        .unwrap()
        .arr()
        .iter()
        .map(|i| Ion {
            compound: i.get("compound").unwrap().f64().unwrap() as u64,
            kind: i.get("kind").unwrap().str().unwrap().to_string(),
            mz: i.get("mz").unwrap().f64().unwrap(),
            rt: i.get("rt_s").unwrap().f64().unwrap(),
            height: i.get("height").unwrap().f64().unwrap(),
        })
        .collect();
    let at = |c: u64| ions.iter().position(|i| i.compound == c).unwrap();
    let pairs = v
        .get("pairs")
        .unwrap()
        .arr()
        .iter()
        .map(|p| {
            (
                at(p.get("a").unwrap().f64().unwrap() as u64),
                at(p.get("b").unwrap().f64().unwrap() as u64),
            )
        })
        .collect();
    Truth { ions, pairs }
}

/// The rows the chain gives for `file` (path A with the centroid step, path B without); `tag` names the work folder.
fn rows(d: &Path, file: &str, centroid: bool, tag: &str) -> Rows {
    let (first, source) = if centroid {
        (
            format!(
                r#"{{"id":"cen","step":"spectra.centroid@1","in":{{"source":"file:{file}"}}}},"#
            ),
            "cen.mzml".to_string(),
        )
    } else {
        (String::new(), format!("file:{file}"))
    };
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"{file}","type":"raw"}}],"steps":[{first}
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":"{source}"}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}}]}}"#
    );
    let w = d.join(tag);
    std::fs::create_dir_all(&w).unwrap();
    let (run_, prov) = crate::flow::run_plan(plan.as_bytes(), d, &w, 2, None).unwrap();
    assert!(
        run_.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let bytes = std::fs::read_dir(&w)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "mzfeatf"))
        .map(|p| std::fs::read(p).unwrap())
        .unwrap();
    let (f, _) = run::read_files([bytes]).unwrap();
    (0..f[0].mz.len())
        .map(|k| (f[0].mz[k], f[0].rt[k] as f64 * 60.0))
        .collect()
}

fn near(r: (f64, f64), i: &Ion) -> bool {
    (r.0 - i.mz).abs() <= TOL_DA && (r.1 - i.rt).abs() <= TOL_S
}

fn score(t: &Truth, rows: &Rows) -> Score {
    let mut s = Score {
        rows: rows.len(),
        ..Score::default()
    };
    let monos: Vec<&Ion> = t
        .ions
        .iter()
        .filter(|i| i.kind == "mono" && i.height >= MIN_APEX)
        .collect();
    let matches = |i: &Ion| rows.iter().filter(|&&r| near(r, i)).count();
    s.recall = monos.iter().filter(|i| matches(i) > 0).count() as f64 / monos.len() as f64;
    s.split = monos.iter().filter(|i| matches(i) > 1).count() as f64 / monos.len() as f64;
    s.precision = rows
        .iter()
        .filter(|&&r| t.ions.iter().any(|i| near(r, i)))
        .count() as f64
        / rows.len().max(1) as f64;
    let dm: Vec<f64> = monos
        .iter()
        .filter_map(|i| {
            rows.iter()
                .filter(|&&r| near(r, i))
                .map(|r| (r.0 - i.mz).abs())
                .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))))
        })
        .collect();
    s.mean_dm = dm.iter().sum::<f64>() / dm.len().max(1) as f64;
    // both members found on two different rows
    s.pairs = t.pairs.len();
    s.pairs_both = t
        .pairs
        .iter()
        .filter(|&&(a, b)| {
            let (ra, rb): (Vec<usize>, Vec<usize>) = (
                (0..rows.len())
                    .filter(|&k| near(rows[k], &t.ions[a]))
                    .collect(),
                (0..rows.len())
                    .filter(|&k| near(rows[k], &t.ions[b]))
                    .collect(),
            );
            ra.iter().any(|x| rb.iter().any(|y| x != y))
        })
        .count();
    // a merged pair: a row on no true ion, between the two members of a pair (within the tolerance) and within 4 s of one of them
    s.merged = rows
        .iter()
        .filter(|&&r| {
            !t.ions.iter().any(|i| near(r, i))
                && t.pairs.iter().any(|&(a, b)| {
                    let (a, b) = (&t.ions[a], &t.ions[b]);
                    r.0 >= a.mz.min(b.mz) - TOL_DA
                        && r.0 <= a.mz.max(b.mz) + TOL_DA
                        && ((r.1 - a.rt).abs() <= TOL_S || (r.1 - b.rt).abs() <= TOL_S)
                })
        })
        .count();
    s
}

fn bench(tag: &str, jitter: f64) -> Option<(Score, Score, usize, usize)> {
    let d = generate(tag, jitter)?;
    let t = truth(&d);
    let f = "LR_bench-s1.mzML";
    let a = rows(&d, f, true, "a");
    let b = rows(&d, f, false, "b");
    let blank = "LR_bench-blank-s1.mzML";
    let (ba, bb) = (
        rows(&d, blank, true, "ba").len(),
        rows(&d, blank, false, "bb").len(),
    );
    let (sa, sb) = (score(&t, &a), score(&t, &b));
    println!("jitter {jitter}: blank rows A {ba}, B {bb}");
    println!("jitter {jitter}  A (centroid): {sa:?}");
    println!("jitter {jitter}  B (profile):  {sb:?}");
    let _ = std::fs::remove_dir_all(&d);
    Some((sa, sb, ba, bb))
}

/// The floors are today's numbers minus 10 % (ceilings plus 10 % for split rate and mean |dm|). Today path A (centroids) finds every compound but a third
/// of its rows are on no true ion (centroided noise; the blank gives 461 rows against 127 of path B), and path B loses some compounds to the per-point floor
/// of `trace.rs` on the profile points. Raising a floor is the proof of a fix; lowering one needs a reason in the PR.
#[test]
fn the_lr_profile_bench_at_the_nominal_jitter() {
    let Some((a, b, _, _)) = bench("n", 0.05) else {
        return;
    };
    assert!(a.rows > 0 && b.rows > 0);
    assert!(
        a.recall >= b.recall,
        "centroids must not find fewer compounds than the profile: {} < {}",
        a.recall,
        b.recall
    );
    assert!(a.recall >= 0.90 && b.recall >= 0.82, "recall {a:?} {b:?}");
    assert!(
        a.precision >= 0.16 && b.precision >= 0.33,
        "precision {a:?} {b:?}"
    );
    assert!(a.pairs_both >= 6 && b.pairs_both >= 9, "pairs {a:?} {b:?}");
    assert!(a.split <= 0.076 && b.split <= 0.72, "split {a:?} {b:?}");
    assert!(
        a.mean_dm <= 0.024 && b.mean_dm <= 0.053,
        "mean |dm| {a:?} {b:?}"
    );
    // merged pairs: rows between the two members of a decoy pair that sit on no true ion
    assert!(a.merged <= 6 && b.merged <= 7, "merged {a:?} {b:?}");
}

/// The 0.35 Da recipe against a jitter three times larger: numbers only, nothing asserted.
#[test]
fn the_lr_profile_bench_with_a_large_jitter_prints_its_numbers() {
    let _ = bench("j", 0.15);
}
