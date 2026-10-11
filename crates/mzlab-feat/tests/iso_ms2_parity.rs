//! Isotope groups and MS2 links of `pipeline::analyse` against the plain-loop reference of `tools/feat_riferimento.py --solo identita`
//! (same indices, purity within 1e-5), and against the truth of the synthetic identity series: purity of the MS2 within 0.05 and
//! isotopologues grouped with their base ion. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::table::{FileFeatures, FLAG_ISOTOPE};
use mzlab_feat::trace::Params;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Without the cut of the maxima: the Python reference does not cut.
fn run(idx: &Path, p: &Params) -> (FileFeatures, mzlab_feat::ms2::Ms2Links) {
    let f = std::fs::File::open(idx).unwrap();
    let size = f.metadata().unwrap().len();
    let mut ix = Index::open(f, size).unwrap();
    let st = Steps {
        split: false,
        ..Steps::default()
    };
    analyse(&mut ix, p, &st).unwrap()
}

fn ints(v: &Value) -> Vec<i64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().unwrap())
        .collect()
}

/// Row of the feature within 5 ppm of `mz` and 0.1 min of `rt` (the nearest in m/z).
fn find(f: &FileFeatures, mz: f64, rt: f64) -> Option<usize> {
    (0..f.len())
        .filter(|&i| (f.mz[i] - mz).abs() <= mz * 5e-6 && (f.rt[i] as f64 - rt).abs() <= 0.1)
        .min_by(|&a, &b| (f.mz[a] - mz).abs().total_cmp(&(f.mz[b] - mz).abs()))
}

/// Names of the columns of two tables that differ.
fn columns_differing(a: &FileFeatures, b: &FileFeatures) -> Vec<&'static str> {
    let mut d = Vec::new();
    macro_rules! cmp {
        ($($c:ident),*) => { $( if a.$c != b.$c { d.push(stringify!($c)); } )* };
    }
    cmp!(
        mz, rt, left, right, apex, scan_lo, scan_hi, area, height, npts, flags, iso_group, charge,
        shape, snr, fwhm, asym, gaps, qreason, ms2_n, purity
    );
    d
}

fn check_reference(f: &FileFeatures, links: &mzlab_feat::ms2::Ms2Links, want: &Value) {
    assert_eq!(f.len(), want["n"].as_u64().unwrap() as usize, "features");
    for (i, w) in want["area_win"].as_array().unwrap().iter().enumerate() {
        let w = w.as_f64().unwrap();
        assert!(
            (f.area_win[i] - w).abs() <= 1e-9 * w.abs().max(1.0),
            "#{i}: area_win {} against {w}",
            f.area_win[i]
        );
    }
    let gid: Vec<i64> = f.iso_group.iter().map(|&g| g as i64).collect();
    assert_eq!(gid, ints(&want["iso_group"]), "iso_group");
    let ch: Vec<i64> = f.charge.iter().map(|&c| c as i64).collect();
    assert_eq!(ch, ints(&want["charge"]), "charge");
    let iso: Vec<i64> = f
        .flags
        .iter()
        .map(|&x| (x & FLAG_ISOTOPE != 0) as i64)
        .collect();
    assert_eq!(iso, ints(&want["isotope"]), "isotope flag");
    for i in 0..f.len() {
        let (scans, pur) = links.of(i);
        let ws = ints(&want["ms2_scans"][i]);
        assert_eq!(
            scans.iter().map(|&s| s as i64).collect::<Vec<_>>(),
            ws,
            "#{i}: MS2 scans"
        );
        for (k, p) in pur.iter().enumerate() {
            let w = want["ms2_purity"][i][k].as_f64().unwrap();
            assert!(
                (*p as f64 - w).abs() <= 1e-5,
                "#{i}: purity {p} against {w}"
            );
        }
    }
}

fn check_truth(f: &FileFeatures, dir: &Path) {
    let t: Value =
        serde_json::from_slice(&std::fs::read(dir.join("verita.json")).unwrap()).unwrap();
    let ions = t["ioni"].as_array().unwrap();
    let base = |fam: &str| ions.iter().find(|r| r["id"] == fam).unwrap();
    let (mut pur_ok, mut pur_n, mut iso_ok, mut iso_n) = (0, 0, 0, 0);
    for r in ions {
        let (Some(rt), Some(mz)) = (r["rt_obs"][0].as_f64(), r["mz"].as_f64()) else {
            continue;
        };
        let Some(i) = find(f, mz, rt) else { continue };
        if r["ruolo"] == "tp1" && r["purezza"].is_number() && f.ms2_n[i] > 0 {
            pur_n += 1;
            pur_ok +=
                (((f.purity[i] as f64) - r["purezza"].as_f64().unwrap()).abs() <= 0.05) as i32;
        }
        if r["ruolo"] == "isotopo" {
            let b = base(r["famiglia"].as_str().unwrap());
            if let Some(j) = find(f, b["mz"].as_f64().unwrap(), rt) {
                iso_n += 1;
                iso_ok += (f.iso_group[i] != 0 && f.iso_group[i] == f.iso_group[j]) as i32;
            }
        }
    }
    assert!(pur_n >= 4, "purity levels found: {pur_n}");
    assert_eq!(pur_ok, pur_n, "purity within 0.05");
    assert!(
        iso_n >= 10 && iso_ok as f64 >= 0.95 * iso_n as f64,
        "isotopologues grouped {iso_ok}/{iso_n}"
    );
}

#[test]
fn isotopes_and_ms2_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab_feat_iso_{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/feat_riferimento.py"))
        .arg(&out)
        .args(["--solo", "identita"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let want: Value =
        serde_json::from_slice(&std::fs::read(out.join("iso_ms2_identita.json")).unwrap()).unwrap();
    let idx = out.join("identita.mzidx");
    let p = Params::default();
    let (f, links) = run(&idx, &p);
    check_reference(&f, &links, &want);
    check_truth(&f, &out.join("identita"));
    let (small, links2) = run(
        &idx,
        &Params {
            budget: 1,
            ..p.clone()
        },
    );
    assert_eq!(
        columns_differing(&f, &small),
        Vec::<&str>::new(),
        "small slabs"
    );
    assert_eq!(links, links2, "small slabs: links");
    assert_eq!(f.to_bytes(), run(&idx, &p).0.to_bytes(), "second run");
    check_split(&idx, &p, f.len());
    let _ = std::fs::remove_dir_all(&out);
}

/// The cut of the maxima (`resolve`): more rows, in pieces of one ion by time, the same table with small slabs and on a second run.
fn check_split(idx: &Path, p: &Params, unsplit: usize) {
    let go = |p: &Params| {
        let f = std::fs::File::open(idx).unwrap();
        let size = f.metadata().unwrap().len();
        let mut ix = Index::open(f, size).unwrap();
        analyse(&mut ix, p, &Steps::default()).unwrap()
    };
    let (g, links) = go(p);
    let cut: Vec<usize> = (0..g.len())
        .filter(|&i| g.flags[i] & mzlab_feat::table::FLAG_RESOLVED != 0)
        .collect();
    assert!(!cut.is_empty() && g.len() > unsplit, "no feature was cut");
    for &i in &cut {
        let near =
            |j: usize| g.mz[j] == g.mz[i] && g.flags[j] & mzlab_feat::table::FLAG_RESOLVED != 0;
        assert!(
            (i > 0 && near(i - 1) && g.apex[i - 1] < g.apex[i])
                || (i + 1 < g.len() && near(i + 1) && g.apex[i + 1] > g.apex[i]),
            "#{i}: a piece without its neighbour"
        );
        assert!(g.scan_lo[i] <= g.apex[i] && g.apex[i] <= g.scan_hi[i]);
    }
    assert!(g.mz.windows(2).all(|w| w[0] <= w[1]), "sorted by m/z");
    let (small, links2) = go(&Params {
        budget: 1,
        ..p.clone()
    });
    assert_eq!(g.to_bytes(), small.to_bytes(), "split: small slabs");
    assert_eq!(links, links2, "split: links of small slabs");
    assert_eq!(g.to_bytes(), go(p).0.to_bytes(), "split: second run");
}
