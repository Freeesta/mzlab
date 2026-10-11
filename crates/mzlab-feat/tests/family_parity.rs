//! Families against the frozen answers of the Python rules (`tools/feat_famiglie.py`, `tests/golden/feat/famiglie_*.json`):
//! the label and the evidence of `edge_evidence` on a grid of steps, MS2 and ratios; and, on peaks of the synthetic identity series with their
//! traces and MS2 spectra, the same pairs, arcs, roles, families and alternatives as the plain Python loops.

use std::path::{Path, PathBuf};

use mzlab_feat::family::evidence::{edge_evidence, EdgeKind, Pair, ProofKind, Ratio, Rules, Spec};
use mzlab_feat::family::rel::{RKind, Tables};
use mzlab_feat::family::{
    build_families, candidates, correlate_traces, FamParams, Spectra, Traces,
};
use mzlab_feat::table::{FileFeatures, FLAG_BACKGROUND, FLAG_GOOD, FLAG_ISOTOPE};
use serde_json::Value;

fn golden(name: &str) -> Value {
    let p: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/feat")
        .join(name);
    serde_json::from_slice(&std::fs::read(&p).unwrap_or_else(|e| panic!("{p:?}: {e}"))).unwrap()
}

fn f64s(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn spec(r: &Rules, v: &Value) -> Option<Spec> {
    (!v.is_null()).then(|| r.strongest(&f64s(&v["mz"]), &f64s(&v["it"])).unwrap())
}

fn kind_name(k: EdgeKind) -> &'static str {
    match k {
        EdgeKind::Adduct => "adduct",
        EdgeKind::Isf => "isf",
        EdgeKind::Undetermined => "undetermined",
        EdgeKind::Other => "other",
    }
}

fn read_name(k: RKind) -> &'static str {
    match k {
        RKind::Adduct => "adduct",
        RKind::Loss => "loss",
        RKind::Dimer => "dimer",
        RKind::Isotope => "isotope",
    }
}

fn proof_name(k: ProofKind) -> &'static str {
    match k {
        ProofKind::Step => "dm",
        ProofKind::Coelution => "coelution",
        ProofKind::FragmentInMs2 => "frag_ms2",
        ProofKind::Ratio => "ratio",
        ProofKind::Skeleton => "skeleton",
        ProofKind::TypicalLoss => "typical_loss",
        ProofKind::NoFragmentation => "no_fragmentation",
    }
}

fn readings_of(e: &mzlab_feat::family::evidence::Edge) -> Vec<(String, String)> {
    e.readings
        .iter()
        .map(|r| (read_name(r.kind).to_string(), r.name.clone()))
        .collect()
}

fn proofs_of(e: &mzlab_feat::family::evidence::Edge) -> Vec<(String, i64)> {
    e.proofs
        .iter()
        .map(|p| (proof_name(p.kind).to_string(), p.sign as i64))
        .collect()
}

fn pairs_of(v: &Value) -> Vec<(String, i64)> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| (p[0].as_str().unwrap().to_string(), p[1].as_i64().unwrap()))
        .collect()
}

fn names_of(v: &Value) -> Vec<(String, String)> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p[0].as_str().unwrap().to_string(),
                p[1].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn edge_evidence_like_python() {
    let doc = golden("famiglie_coppie.json");
    let r = Rules::default();
    let (mut n, mut other) = (0, 0);
    for c in doc["cases"].as_array().unwrap() {
        let pol = c["pol"].as_i64().unwrap() as i8;
        let ratio = if c["areas_lo"].is_null() {
            Ratio::Unavailable
        } else {
            mzlab_feat::family::evidence::ratio_of(&f64s(&c["areas_lo"]), &f64s(&c["areas_up"]), &r)
        };
        let (lo, up) = (spec(&r, &c["ms2_lo"]), spec(&r, &c["ms2_up"]));
        let e = edge_evidence(
            &Tables::new(pol),
            &r,
            &Pair {
                mz_lo: c["mz_lo"].as_f64().unwrap(),
                mz_up: c["mz_up"].as_f64().unwrap(),
                ms2_lo: lo.as_ref(),
                ms2_up: up.as_ref(),
                ratio,
                drt_min: c["drt_min"].as_f64(),
            },
        );
        let w = &c["want"];
        let what = format!(
            "{} pol {pol} ms2 {:?} drt {}",
            c["step"],
            c["ms2_up"].is_null(),
            c["drt_min"]
        );
        if w["kind"] == "other" {
            // a step in no table: Rust reads it as a loss only when the MS2 of the heavier ion shows the fragment
            assert!(
                e.kind == EdgeKind::Other || e.readings.iter().any(|x| x.name == r.unlisted),
                "{what}"
            );
            other += 1;
            continue;
        }
        assert_eq!(
            kind_name(e.kind),
            w["kind"].as_str().unwrap(),
            "kind of {what}"
        );
        assert_eq!(
            readings_of(&e),
            names_of(&w["readings"]),
            "readings of {what}"
        );
        assert_eq!(proofs_of(&e), pairs_of(&w["proofs"]), "proofs of {what}");
        n += 1;
    }
    assert!(n > 150 && other > 0, "cases compared: {n}, other: {other}");
}

fn table(rows: &Value) -> FileFeatures {
    let n = rows["mz"].as_array().unwrap().len();
    let col = |k: &str| f64s(&rows[k]);
    let mut f = FileFeatures {
        mz: col("mz"),
        dt: rows["dt"].as_f64().unwrap(),
        ..Default::default()
    };
    f.rt = col("rt").iter().map(|&x| x as f32).collect();
    f.apex = col("apex").iter().map(|&x| x as u32).collect();
    f.fwhm = col("fwhm").iter().map(|&x| x as f32).collect();
    f.height = col("height").iter().map(|&x| x as f32).collect();
    f.iso_group = col("iso_group").iter().map(|&x| x as u32).collect();
    for i in 0..n {
        let mut fl = FLAG_GOOD;
        if rows["isotope"][i] == true {
            fl |= FLAG_ISOTOPE;
        }
        if rows["excluded"][i] == true {
            fl |= FLAG_BACKGROUND;
        }
        f.flags.push(fl);
    }
    f
}

#[test]
fn families_like_python() {
    let doc = golden("famiglie_identita.json");
    let p = FamParams::default();
    let f = table(&doc["rows"]);
    let n = f.len();
    let sp: Vec<(usize, Spec)> = doc["spectra"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.parse().unwrap(), spec(&p.rules, v).unwrap()))
        .collect();
    let spectra = Spectra::from_parts(n, sp);
    let tr: Vec<(usize, u32, Vec<f64>)> = doc["traces"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| {
            (
                k.parse().unwrap(),
                v[0].as_u64().unwrap() as u32,
                f64s(&v[1]),
            )
        })
        .collect();
    let traces = Traces::from_parts(n, &tr);
    let cand = candidates(&f, &spectra, 1, &p);
    let pairs = correlate_traces(&f, cand, &traces, &p);
    let want = &doc["want"];
    let wp = want["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), wp.len(), "pairs");
    for (g, w) in pairs.iter().zip(wp) {
        assert_eq!(
            (g.0 as u64, g.1 as u64),
            (w[0].as_u64().unwrap(), w[1].as_u64().unwrap())
        );
        assert!(
            (g.2 as f64 - w[2].as_f64().unwrap()).abs() < 1e-6,
            "correlation {g:?} against {w}"
        );
    }
    let fam = build_families(&f, &pairs, &spectra, |_, _| Ratio::Unavailable, 1, &p);
    let wa = want["arcs"].as_array().unwrap();
    assert_eq!(fam.arcs.len(), wa.len(), "arcs");
    for (a, w) in fam.arcs.iter().zip(wa) {
        let e = mzlab_feat::family::evidence::Edge {
            kind: a.kind,
            readings: a.readings.clone(),
            proofs: a.proofs.clone(),
        };
        let what = format!("arc {}-{}", a.lo, a.up);
        assert_eq!(kind_name(a.kind), w["kind"].as_str().unwrap(), "{what}");
        assert_eq!(readings_of(&e), names_of(&w["readings"]), "{what}");
        assert_eq!(
            proofs_of(&e)
                .into_iter()
                .filter(|p| p.0 != "ratio")
                .collect::<Vec<_>>(),
            pairs_of(&w["proofs"]),
            "{what}"
        );
    }
    let ints = |k: &str| {
        want[k]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_i64().unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        fam.role.iter().map(|&x| x as i64).collect::<Vec<_>>(),
        ints("role"),
        "roles"
    );
    assert_eq!(
        fam.alt.iter().map(|&x| x as i64).collect::<Vec<_>>(),
        ints("alt"),
        "alternatives open"
    );
    assert_eq!(
        fam.family_id.iter().map(|&x| x as i64).collect::<Vec<_>>(),
        ints("family"),
        "families"
    );
    assert_eq!(
        fam.parent
            .iter()
            .map(|&x| if x == u32::MAX { -1 } else { x as i64 })
            .collect::<Vec<_>>(),
        ints("parent"),
        "parents"
    );
    let wl = want["alternatives"].as_array().unwrap();
    assert_eq!(fam.alternatives.len(), wl.len(), "neutral alternatives");
    for (a, w) in fam.alternatives.iter().zip(wl) {
        assert_eq!(
            (a.family as u64, a.row as u64, a.reading as u64),
            (
                w[0].as_u64().unwrap(),
                w[1].as_u64().unwrap(),
                w[3].as_u64().unwrap()
            )
        );
        assert!(
            (a.mass - w[2].as_f64().unwrap()).abs() < 1e-9
                && (a.points as f64 - w[4].as_f64().unwrap()).abs() < 1e-6
        );
    }
    assert!(fam.count() < n && fam.arcs.len() >= 5);
}

/// Whole pipeline on the synthetic identity series: two runs and small slabs give the same bytes.
#[test]
fn families_are_deterministic() {
    use mzlab_core::idxread::Index;
    use mzlab_feat::family::analyse_families;
    use mzlab_feat::pipeline::{analyse, Steps};
    use mzlab_feat::trace::Params;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("mzlab_feat_fam_{}", std::process::id()));
    let ok = std::process::Command::new("python3")
        .arg(root.join("tools/feat_riferimento.py"))
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
        return;
    }
    let idx = out.join("identita.mzidx");
    let run = |p: &Params| {
        let file = std::fs::File::open(&idx).unwrap();
        let size = file.metadata().unwrap().len();
        let mut ix = Index::open(file, size).unwrap();
        let (mut f, links) = analyse(&mut ix, p, &Steps::default()).unwrap();
        analyse_families(&mut ix, &mut f, &links, p, &FamParams::default()).unwrap();
        f.to_bytes()
    };
    let p = Params::default();
    let a = run(&p);
    assert_eq!(a, run(&p), "second run");
    assert_eq!(
        a,
        run(&Params {
            budget: 1,
            ..p.clone()
        }),
        "small slabs"
    );
    let _ = std::fs::remove_dir_all(&out);
}
