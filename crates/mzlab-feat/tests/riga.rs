//! The row of the table for the page (`riga/1`) and its traces (`riga_tracce/1`): the values agree with the sheet of the export and with the summaries of the
//! `.mzfeat`; the traces come from the index of a synthetic series (skipped without python3 and numpy, `MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::json::{parse, Value};
use mzlab_feat::export::{xlsx_rows, Ctx};
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::riga;
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{
    Aligned, RowIdent, RowQ, CELL_ABSENT, CELL_DETECTED, CELL_FILLED, CELL_TARGETED, NO_FILE,
};
use mzlab_feat::trace::Params;

const STATES: [u8; 3] = [CELL_DETECTED, CELL_FILLED, CELL_TARGETED];

fn table() -> Aligned {
    let mut t = Aligned::with_capacity(3, 5);
    for r in 0..5usize {
        let (mz, rt) = (200.0 + 12.5 * r as f64, 3.0 + r as f64);
        t.mz.push(mz);
        t.rt.push(rt);
        for k in 0..3usize {
            let absent = (r + k) % 4 == 3;
            t.area.push(if absent {
                0.0
            } else {
                1e5 * (1 + r + 2 * k) as f64
            });
            t.height.push(1e4 * (1 + r) as f32);
            t.apex_rt.push((rt + 0.01 * k as f64) as f32);
            t.width.push(0.1 + 0.01 * k as f32);
            t.state.push(if absent {
                CELL_ABSENT
            } else {
                STATES[(r + k) % 3]
            });
        }
        let q = if r == 4 {
            RowQ {
                rt_lo: (rt - 0.1) as f32,
                rt_hi: (rt + 0.1) as f32,
                ..RowQ::NONE
            }
        } else {
            RowQ {
                rt_lo: (rt - 0.1) as f32,
                rt_hi: (rt + 0.2) as f32,
                charge: 1,
                q_shape: 0.9,
                q_snr: 12.5,
                q_fwhm: 6.0,
                qreason: if r == 2 { 5 } else { 0 },
                iso_m1: 0.25,
                ms2_file: (r % 3) as u32,
                ms2_purity: 0.75,
            }
        };
        t.meta.push(&q);
    }
    t.ident = RowIdent {
        neutral: (0..5).map(|r| 199.0 + 12.5 * r as f64).collect(),
        ion: vec![1, 1, 2, 0, 1],
        src: vec![1, 3, 5, 0, 1],
    };
    t
}

fn samples() -> Vec<Sample> {
    let f: Vec<(String, String)> = ["blank.mzML", "t10.mzML", "t20.mzML"]
        .iter()
        .map(|n| (n.to_string(), "00".repeat(32)))
        .collect();
    propose(&f)
}

fn num(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.f64())
}

#[test]
fn the_row_agrees_with_the_sheet_and_with_the_summaries() {
    let (t, s) = (table(), samples());
    let c = Ctx {
        table: &t,
        samples: &s,
        view: None,
        table_sha256: "ab",
        polarity: 1,
    };
    let sheet = parse(xlsx_rows(&c).as_bytes()).unwrap();
    let rows = sheet
        .get("hojas")
        .unwrap()
        .at(0)
        .unwrap()
        .get("filas")
        .unwrap();
    for r in 0..5usize {
        let id = t.id(r);
        let text = riga::row(&t, &s, id).unwrap();
        assert_eq!(
            text,
            riga::row(&t, &s, id).unwrap(),
            "canonical JSON: the same bytes every time"
        );
        let v = parse(text.as_bytes()).unwrap();
        assert_eq!(v.get("schema").unwrap().str(), Some("riga/1"));
        let line = rows.at(r + 1).unwrap();
        let cell = |i: usize| line.at(i).unwrap();
        assert_eq!(num(&v, "id"), cell(0).f64());
        assert_eq!(num(&v, "mz"), cell(1).f64());
        assert_eq!(num(&v, "rt"), cell(2).f64());
        assert_eq!(num(&v, "rt_lo"), cell(3).f64());
        assert_eq!(num(&v, "rt_hi"), cell(4).f64());
        assert_eq!(num(&v, "charge"), cell(5).f64().filter(|c| *c != 0.0));
        let cells = v.get("cells").unwrap().arr();
        assert_eq!(cells.len(), 3);
        for (k, e) in cells.iter().enumerate() {
            assert_eq!(e.get("file").unwrap().str(), Some(s[k].file.as_str()));
            assert_eq!(e.get("type").unwrap().str(), Some(s[k].kind.as_str()));
            assert_eq!(num(e, "area"), cell(6 + 2 * k).f64());
            let word = cell(7 + 2 * k).str().unwrap().to_lowercase();
            let want = if word == "unknown" {
                "absent"
            } else {
                word.as_str()
            };
            assert_eq!(e.get("state").unwrap().str(), Some(want));
            let c = t.cell(r, k);
            assert_eq!(
                e.get("height").unwrap().f64(),
                (want != "absent").then(|| f64::from(t.height[c]))
            );
            assert_eq!(
                e.get("apex_rt").unwrap().f64(),
                (want != "absent").then(|| f64::from(t.apex_rt[c]))
            );
        }
        let q = t.meta.get(r);
        match v.get("quality").unwrap() {
            n if n.is_null() => assert_eq!(r, 4, "only the row without summaries has no quality"),
            q2 => {
                assert_eq!(num(q2, "shape"), Some(f64::from(q.q_shape)));
                assert_eq!(num(q2, "snr"), Some(f64::from(q.q_snr)));
                assert_eq!(num(q2, "fwhm"), Some(f64::from(q.q_fwhm)));
                let flags: Vec<&str> = q2
                    .get("flags")
                    .unwrap()
                    .arr()
                    .iter()
                    .map(|f| f.str().unwrap())
                    .collect();
                assert_eq!(
                    flags,
                    if r == 2 {
                        vec!["shape", "points"]
                    } else {
                        vec![]
                    }
                );
            }
        }
        assert_eq!(num(&v, "iso_m1"), (r != 4).then_some(f64::from(q.iso_m1)));
        let ms2 = v.get("ms2").unwrap();
        if r == 4 {
            assert!(ms2.is_null());
        } else {
            assert_eq!(
                ms2.get("file").unwrap().str(),
                Some(s[q.ms2_file as usize].file.as_str())
            );
            assert_eq!(num(ms2, "purity"), Some(0.75));
        }
        let id = v.get("ident").unwrap();
        assert_eq!(num(id, "neutral"), Some(t.ident.neutral[r]));
    }
    let id = |r: usize| parse(riga::row(&t, &s, t.id(r)).unwrap().as_bytes()).unwrap();
    assert_eq!(
        id(0).get("ident").unwrap().get("adduct").unwrap().str(),
        Some("[M+H]+")
    );
    assert!(id(3).get("ident").unwrap().get("adduct").unwrap().is_null());
    let words = |r: usize| -> Vec<String> {
        id(r)
            .get("ident")
            .unwrap()
            .get("sources")
            .unwrap()
            .arr()
            .iter()
            .map(|w| w.str().unwrap().to_string())
            .collect()
    };
    assert_eq!(words(0), ["detected"]);
    assert_eq!(words(1), ["detected", "hypothesis", "rule"]);
    assert_eq!(words(3), Vec::<String>::new());
}

#[test]
fn a_row_without_summaries_or_identity_has_nulls_and_an_unknown_id_is_an_error() {
    let (mut t, s) = (table(), samples());
    t.meta = Default::default();
    t.ident = RowIdent::default();
    let v = parse(riga::row(&t, &s, 1).unwrap().as_bytes()).unwrap();
    for k in [
        "rt_lo", "rt_hi", "charge", "quality", "iso_m1", "ms2", "ident",
    ] {
        assert!(v.get(k).unwrap().is_null(), "{k}");
    }
    assert_eq!(v.get("cells").unwrap().arr().len(), 3);
    for id in [0, 6, 1000] {
        assert_eq!(riga::row(&t, &s, id).unwrap_err().key, "error.rowNotFound");
    }
    assert_eq!(riga::row(&t, &s[..2], 1).unwrap_err().key, "error.badView");
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_traces_come_from_the_index_of_every_file() {
    let out = std::env::temp_dir().join(format!("mzlab_feat_riga_{}", std::process::id()));
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
    let open = |_: usize| {
        let f = std::fs::File::open(out.join("identita.mzidx")).unwrap();
        let size = f.metadata().unwrap().len();
        Index::open(f, size)
    };
    let (f, _) = analyse(
        &mut open(0).unwrap(),
        &Params::default(),
        &Steps {
            split: false,
            ..Steps::default()
        },
    )
    .unwrap();
    let i = (0..f.len())
        .find(|&i| f.ms2_n[i] > 0)
        .expect("a feature with an MS2");
    let mut t = Aligned::with_capacity(1, 1);
    t.mz.push(f.mz[i]);
    t.rt.push(f64::from(f.rt[i]));
    t.area.push(f.area[i]);
    t.height.push(f.height[i]);
    t.apex_rt.push(f.rt[i]);
    t.width.push(f.right[i] - f.left[i]);
    t.state.push(CELL_DETECTED);
    t.meta.push(&RowQ {
        rt_lo: f.left[i],
        rt_hi: f.right[i],
        ms2_file: 0,
        ms2_purity: f.purity[i],
        ..RowQ::NONE
    });
    let s = &samples()[..1];
    let text = riga::traces(&t, s, 1, open, 5.0).unwrap();
    assert_eq!(
        text,
        riga::traces(&t, s, 1, open, 5.0).unwrap(),
        "canonical JSON"
    );
    let v = parse(text.as_bytes()).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("riga_tracce/1"));
    let xic = v.get("xic").unwrap().at(0).unwrap();
    let (x, y): (Vec<f64>, Vec<f64>) = (
        xic.get("rt")
            .unwrap()
            .arr()
            .iter()
            .filter_map(|a| a.f64())
            .collect(),
        xic.get("y")
            .unwrap()
            .arr()
            .iter()
            .filter_map(|a| a.f64())
            .collect(),
    );
    assert!(
        x.len() == y.len()
            && x.len() > 3
            && x.len() <= riga::MAX_POINTS
            && x.windows(2).all(|w| w[0] < w[1])
    );
    let top = (0..y.len()).max_by(|&a, &b| y[a].total_cmp(&y[b])).unwrap();
    assert!(
        y[top] > 0.0
            && x[top] >= f64::from(f.left[i]) - 1e-9
            && x[top] <= f64::from(f.right[i]) + 1e-9,
        "the maximum of the XIC lies inside the edges of the peak"
    );
    let ms1 = v.get("ms1").unwrap();
    let mz: Vec<f64> = ms1
        .get("mz")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|a| a.f64())
        .collect();
    assert!(
        !mz.is_empty()
            && mz
                .iter()
                .all(|m| (f.mz[i] - 1.0..=f.mz[i] + 4.0).contains(m))
    );
    let ms2 = v.get("ms2").unwrap();
    assert!(
        !ms2.get("mz").unwrap().arr().is_empty()
            && (num(ms2, "prec").unwrap() - f.mz[i]).abs() < 1.0
    );
    // no MS2 file: no MS2
    t.meta.ms2_file[0] = NO_FILE;
    assert!(parse(riga::traces(&t, s, 1, open, 5.0).unwrap().as_bytes())
        .unwrap()
        .get("ms2")
        .unwrap()
        .is_null());
    assert_eq!(
        riga::traces(&t, s, 2, open, 5.0).unwrap_err().key,
        "error.rowNotFound"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn the_xic_keeps_the_maximum_of_every_group() {
    let x: Vec<f64> = (0..1000).map(f64::from).collect();
    let mut y = vec![1.0; 1000];
    y[503] = 99.0;
    let (ox, oy) = riga::decimate(&x, &y);
    assert_eq!(ox.len(), riga::MAX_POINTS);
    assert!(oy.contains(&99.0) && ox[oy.iter().position(|&v| v == 99.0).unwrap()] == 503.0);
}
