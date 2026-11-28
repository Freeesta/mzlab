//! `spectrum get --bin` with `--minus` and `--sum`: each side is cut into bins (its offset taken off first) and the blank's bins are subtracted from the
//! sample's; a sum is cut as it is. The same fragment split between two integers on the two sides leaves no residue at the wrong bin.

use mzlab_core::json::{parse, Value};

use crate::extract::tests::{dir, run};

/// Profile points of 0.1 Da on 100.02 + 0.1 i, baseline 5, four scans of one minute. The fragment of six points starts at point `frag` (106.62 for 66);
/// `extra` is one more point of 300 at point 20 (102.02), an ion that only the sample has.
fn file(d: &std::path::Path, name: &str, frag: usize, extra: bool) -> String {
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(4).unwrap();
    let peak = [400.0f32, 800.0, 1000.0, 1000.0, 800.0, 400.0];
    for k in 0..4 {
        let (mz, y): (Vec<f64>, Vec<f32>) = (0usize..100)
            .map(|i: usize| {
                let mut v = 5.0f32
                    + i.checked_sub(frag)
                        .and_then(|k| peak.get(k))
                        .copied()
                        .unwrap_or(0.0);
                if extra && i == 20 {
                    v += 300.0;
                }
                (100.02 + 0.1 * i as f64, v)
            })
            .unzip();
        let s = mzlab_core::mzml::Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: true,
            filter: "ITMS + p ESI Full ms [100.00-110.00]".into(),
            mz,
            intensity: y,
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let f = d.join(name);
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    f.to_string_lossy().into_owned()
}

fn doc(args: &[&str]) -> Value {
    let o = run(args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    parse(&o.stdout).unwrap()
}

/// `(bin, counts)` of a `spectrum-bins/1` document.
fn bins(v: &Value) -> Vec<(f64, f64)> {
    v.get("bins")
        .unwrap()
        .arr()
        .iter()
        .map(|b| {
            (
                b.get("bin").and_then(Value::f64).unwrap(),
                b.get("counts").and_then(Value::f64).unwrap(),
            )
        })
        .collect()
}

#[test]
fn a_blank_that_splits_the_fragment_differently_is_subtracted_bin_by_bin() {
    let d = dir("bins_minus");
    // sample: the fragment on 106.62..107.12 (all in bin 107) and an ion of its own; blank: the same fragment 0.4 Da higher, its last point in bin 108
    let (s, b) = (
        file(&d, "sample.mzML", 66, true),
        file(&d, "blank.mzML", 70, false),
    );
    let v = doc(&[
        "spectrum", "get", &s, "--rt", "0,3", "--bin", "nominal", "--minus", &b,
    ]);
    assert_eq!(v.get("schema").unwrap().str(), Some("spectrum-bins/1"));
    // bin 107: the sample's 4400 + 10 baselines, the blank's 4000 + 10 baselines; 108 is the blank's alone (clipped); 102 is the sample's own ion
    let got = bins(&v);
    assert_eq!(
        got.iter().find(|x| x.0 == 107.0).unwrap().1,
        400.0,
        "{got:?}"
    );
    assert_eq!(
        got.iter().find(|x| x.0 == 102.0).unwrap().1,
        300.0,
        "{got:?}"
    );
    assert!(got.iter().all(|x| x.1 > 0.0 && x.0 != 108.0), "{got:?}");
    assert_eq!(got.len(), 2, "{got:?}");
    let m = v.get("evidence").unwrap().get("minus").unwrap();
    assert_eq!(m.get("order").unwrap().str(), Some("bin_then_subtract"));
    assert_eq!(m.get("mz_offset_da").and_then(Value::f64), Some(0.0));
    assert!(m.get("mz_offset").is_some() && m.get("scans").is_some());
    assert_eq!(v.get("mz_offset_da").and_then(Value::f64), Some(0.0));
    // the same two files subtracted on their points and cut afterwards leave residues in the neighbouring bins
    let lines = doc(&["spectrum", "get", &s, "--rt", "0,3", "--minus", &b]);
    let (mz, y): (Vec<f64>, Vec<f64>) = {
        let p = lines.get("points").unwrap().arr();
        let col = |i: usize| {
            p.iter()
                .map(|x| x.arr()[i].f64().unwrap())
                .collect::<Vec<_>>()
        };
        (col(0), col(1))
    };
    let late = mzlab_core::nominalbin::rebin(&mz, &y, mzlab_core::nominalbin::Rule::Nominal, 0.0);
    assert!(
        late.iter().any(|x| x.centre == 106.0 || x.centre == 108.0)
            || late.iter().find(|x| x.centre == 107.0).map(|x| x.y) != Some(400.0),
        "{late:?}"
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_offset_is_taken_off_both_sides_and_logged_for_each() {
    let d = dir("bins_minus_off");
    let (s, b) = (
        file(&d, "sample.mzML", 66, false),
        file(&d, "blank.mzML", 66, false),
    );
    let o = run(&[
        "spectrum",
        "get",
        &s,
        "--rt",
        "0,3",
        "--bin",
        "nominal",
        "--mz-offset",
        "0.24",
        "--minus",
        &b,
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    // the same file as blank: nothing is left
    let v = parse(&o.stdout).unwrap();
    assert_eq!(v.get("n_bins").and_then(Value::f64), Some(0.0));
    assert_eq!(v.get("mz_offset_da").and_then(Value::f64), Some(0.24));
    let m = v.get("evidence").unwrap().get("minus").unwrap();
    assert_eq!(m.get("mz_offset_da").and_then(Value::f64), Some(0.24));
    assert!(
        o.stderr.contains("mz offset sample.mzML: +0.24 Da (fixed)"),
        "{}",
        o.stderr
    );
    assert!(
        o.stderr.contains("mz offset blank.mzML: +0.24 Da (fixed)"),
        "{}",
        o.stderr
    );
    assert!(o.stderr.contains("bin_then_subtract"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_sum_is_cut_into_bins_with_its_evidence() {
    let d = dir("bins_sum");
    let s = file(&d, "sample.mzML", 66, false);
    let mean = doc(&["spectrum", "get", &s, "--rt", "0,3", "--bin", "nominal"]);
    let sum = doc(&[
        "spectrum", "get", &s, "--rt", "0,3", "--bin", "nominal", "--sum",
    ]);
    let (a, b) = (bins(&mean), bins(&sum));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(y.1, 4.0 * x.1, "{x:?} {y:?}");
    }
    let ev = sum.get("evidence").unwrap();
    assert_eq!(
        ev.get("sum").unwrap().get("n_scans").and_then(Value::f64),
        Some(4.0)
    );
    assert!(ev.get("query").unwrap().str().unwrap().contains("mode=sum"));
    assert!(mean.get("evidence").unwrap().get("sum").is_none());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn what_a_bin_cannot_take_is_still_a_usage_error() {
    let d = dir("bins_minus_bad");
    let (s, b) = (
        file(&d, "sample.mzML", 66, false),
        file(&d, "blank.mzML", 70, false),
    );
    for args in [vec!["--sum", "--minus", &b[..]], vec!["--peak", "auto"]] {
        let mut a = vec!["spectrum", "get", &s, "--rt", "0,3", "--bin", "nominal"];
        a.extend(args.iter().copied());
        a.push("--json");
        let o = run(&a);
        assert_eq!(o.code, 2, "{args:?}: {}", o.stderr);
    }
    let _ = std::fs::remove_dir_all(d);
}
