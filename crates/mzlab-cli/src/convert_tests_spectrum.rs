//! `file convert --spectrum auto|centroid|profile`: sparse ion counts that a reader flags as profile are written as centroid (MS:1000127), a continuous
//! trace stays profile (MS:1000128), the user's word wins, the file read back says what was written, the decision is on stderr and in the report, and the
//! bytes are those of the route of the site (`Conversion` with the options of the download).

use std::path::{Path, PathBuf};

use mzlab_core::json::{parse, Value};
use mzlab_core::mzml::{self, Item};
use mzlab_core::readers::Limits;
use mzlab_core::readers_mzml::Conversion;
use mzlab_core::mzmlwrite::Options;

use super::{convert, mzmls};
use crate::convert_fixtures::{file_set, folder, mzxml};
use crate::flow_run_tests::p;

/// `in/sparse.mzXML` (nine isolated counts per scan at irregular masses) and `in/trace.mzXML` (a gaussian every 0.1 Da), both flagged `centroided="0"`
/// the way a reader that calls the whole instrument family profile does.
fn inputs(dir: &Path) -> PathBuf {
    let src = dir.join("in");
    std::fs::create_dir_all(&src).unwrap();
    let scan = |num: u32, sparse: bool| {
        let (mz, it): (Vec<f64>, Vec<f64>) = if sparse {
            (0..9)
                .map(|i| {
                    let m = 100.0 + f64::from(i) * 7.31 + (f64::from(i) * 1.7).sin() * 2.0;
                    (m, 1.0 + f64::from(i % 3))
                })
                .unzip()
        } else {
            (0..200)
                .map(|i| {
                    let m = 100.0 + f64::from(i) * 0.1;
                    (m, 800.0 * (-((m - 110.0) / 0.35).powi(2)).exp())
                })
                .unzip()
        };
        mzxml::Sc {
            mz,
            it,
            ..mzxml::sc(num, 1, f64::from(num) * 0.5, 0, None)
        }
    };
    let o = mzxml::Opz {
        zlib: false,
        ..mzxml::FLAT
    };
    for (name, sparse) in [("sparse", true), ("trace", false)] {
        let scans: Vec<_> = (1..=6).map(|n| scan(n, sparse)).collect();
        let doc = String::from_utf8(mzxml::mzxml(&scans, o)).unwrap();
        // an mzXML has no "profile" word: the scans say centroided="0"
        std::fs::write(
            src.join(format!("{name}.mzXML")),
            doc.replace("centroided=\"1\"", "centroided=\"0\""),
        )
        .unwrap();
    }
    src
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The spectra of a written file as the reader of mzML reads them.
fn read_back(bytes: &[u8]) -> Vec<mzml::Spectrum> {
    let mut v = Vec::new();
    mzml::parse(bytes, |i| {
        if let Item::Spectrum(s) = i {
            v.push(s);
        }
    })
    .unwrap();
    v
}

fn out_of(dir: &Path, name: &str, sub: &str) -> Vec<u8> {
    std::fs::read(dir.join(sub).join(format!("{name}.mzML"))).unwrap()
}

#[test]
fn auto_writes_sparse_counts_as_centroid_and_a_continuous_trace_as_profile() {
    let dir = folder("spectrum_auto");
    let src = inputs(&dir);
    let o = convert(&[p(&src), "--out", p(&dir.join("o"))]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let (sparse, trace) = (out_of(&dir, "sparse", "o"), out_of(&dir, "trace", "o"));
    let (s, t) = (text(&sparse), text(&trace));
    assert!(s.contains("MS:1000127") && !s.contains("MS:1000128"));
    assert!(t.contains("MS:1000128") && !t.contains("MS:1000127"));
    // read back: the points are the file's, the term is what was written
    let (bs, bt) = (read_back(&sparse), read_back(&trace));
    assert!(bs.len() == 6 && bs.iter().all(|x| !x.profile && x.mz.len() == 9));
    assert!(bt.len() == 6 && bt.iter().all(|x| x.profile && x.mz.len() == 200));
    // the decision is told on stderr, and recorded in the file only where it changed what the reader says
    assert!(
        o.stderr.contains("spectrum type: spectrum=auto; written=centroid; basis=sparse"),
        "{}",
        o.stderr
    );
    assert!(o.stderr.contains("written=profile; basis=continuous points"), "{}", o.stderr);
    assert!(s.contains("mzLab spectrum type") && !t.contains("mzLab spectrum type"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_users_word_wins_and_the_short_form_is_the_parameter() {
    let dir = folder("spectrum_asked");
    let src = inputs(&dir);
    let run = |extra: &[&str], out: &str| {
        let target = dir.join(out);
        let mut a = vec![p(&src), "--out", p(&target)];
        a.extend(extra);
        convert(&a)
    };
    assert_eq!(run(&["--spectrum", "x"], "bad").code, 2);
    let (a, b) = (
        run(&["--spectrum", "profile"], "a"),
        run(&["--set", "conv.spectrum=profile"], "b"),
    );
    assert!(a.code == 0 && b.code == 0, "{} {}", a.stderr, b.stderr);
    assert_eq!(mzmls(&dir.join("a")), mzmls(&dir.join("b")));
    let s = text(&out_of(&dir, "sparse", "a"));
    assert!(s.contains("MS:1000128") && !s.contains("MS:1000127"));
    assert!(s.contains("spectrum=profile; written=profile; basis=asked"));
    let c = run(&["--spectrum", "centroid"], "c");
    assert_eq!(c.code, 0, "{}", c.stderr);
    let t = out_of(&dir, "trace", "c");
    assert!(read_back(&t).iter().all(|x| !x.profile && x.mz.len() == 200));
    // the cache key holds the choice: a second run into the same folder with another type converts again
    let again = run(&["--spectrum", "centroid"], "a");
    assert_eq!(again.code, 0, "{}", again.stderr);
    assert!(text(&out_of(&dir, "sparse", "a")).contains("MS:1000127"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_report_carries_the_decision_of_every_file() {
    let dir = folder("spectrum_report");
    let src = inputs(&dir);
    let rep = dir.join("r.json");
    let o = convert(&[p(&src), "--out", p(&dir.join("o")), "--report", p(&rep)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let doc = parse(&std::fs::read(&rep).unwrap()).unwrap();
    let items = doc.get("items").map(Value::arr).unwrap();
    let kind = |name: &str| {
        let it = items
            .iter()
            .find(|i| i.get("input").and_then(Value::str).is_some_and(|n| n.contains(name)))
            .unwrap();
        it.get("spectrum_type").and_then(Value::str).map(str::to_string)
    };
    assert!(kind("sparse").is_some_and(|t| t.contains("written=centroid")));
    assert!(kind("trace").is_some_and(|t| t.contains("written=profile")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_file_is_the_one_the_site_writes() {
    // `to_mzml_with` of the site: `Conversion` with the options of the download and `steps` empty, the type left to `auto`
    let dir = folder("spectrum_site");
    let src = inputs(&dir);
    let o = convert(&[p(&src), "--out", p(&dir.join("o")), "--workers", "1"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    for name in ["sparse", "trace"] {
        let bytes = std::fs::read(src.join(format!("{name}.mzXML"))).unwrap();
        let mut l = mzlab_lettori::open(file_set(&format!("{name}.mzXML"), bytes), &Limits::desktop()).unwrap();
        let site = Conversion {
            steps: Some(&[]),
            opz: Options::default(),
            ..Conversion::default()
        }
        .write(&mut *l, 0, Vec::new(), &mut |_, _| {})
        .unwrap();
        assert!(site == out_of(&dir, name, "o"), "{name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
