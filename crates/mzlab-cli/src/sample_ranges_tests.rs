//! `sample select --ranges`: the byte ranges a chosen sample needs, on a synthetic `.wiff` of four samples (`tools/wiff_grande.py`). The proof is the copy itself: a
//! `.wiff.scan` that holds only those ranges (zeros elsewhere) converts the chosen samples to the very bytes of the whole pair. python3 is part of the test.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{execute, Outcome};

fn pair(name: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_ranges_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_grande.py"))
        .arg(dir.join("in"))
        .args(["--samples", "4", "--scans", "40", "--points", "50"])
        .output()
        .expect("python3 is needed by this test");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let wiff = dir.join("in/grande.wiff").to_string_lossy().into_owned();
    (dir, wiff)
}

fn run(a: &[&str]) -> Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

/// `(start, end)` pairs of the `.wiff.scan` member of a `sample-select/1` with `--ranges`.
fn scan_ranges(json: &str) -> Vec<(u64, u64)> {
    let v = mzlab_core::json::parse(json.as_bytes()).unwrap();
    let m = v
        .get("members")
        .unwrap()
        .arr()
        .iter()
        .find(|m| {
            m.get("name")
                .unwrap()
                .str()
                .unwrap()
                .ends_with(".wiff.scan")
        })
        .unwrap();
    m.get("ranges")
        .unwrap()
        .arr()
        .iter()
        .map(|r| {
            let r = r.arr();
            (r[0].f64().unwrap() as u64, r[1].f64().unwrap() as u64)
        })
        .collect()
}

#[test]
fn the_plan_names_the_ranges_and_the_exit_codes_differ() {
    let (dir, wiff) = pair("plan");
    let o = run(&[
        "sample", "select", &wiff, "--ranges", "--sample", "2", "--json",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let j = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        j.starts_with("{\"schema\":\"sample-select/1\",\"samples\":[2],"),
        "{j}"
    );
    let total = std::fs::metadata(dir.join("in/grande.wiff.scan"))
        .unwrap()
        .len();
    let r = scan_ranges(&j);
    let kept: u64 = r.iter().map(|x| x.1 - x.0).sum();
    assert!(!r.is_empty() && kept < total / 2, "{r:?} of {total}");
    // every sample: the whole file as one range
    let all = run(&[
        "sample", "select", &wiff, "--ranges", "--sample", "1-4", "--json",
    ]);
    assert_eq!(
        scan_ranges(&String::from_utf8_lossy(&all.stdout)),
        vec![(0, total)]
    );
    // the text form, an unknown id (5, ids named), a bad list (2), a missing file (4)
    let t = run(&["sample", "select", &wiff, "--ranges", "--sample", "2"]);
    assert!(
        String::from_utf8_lossy(&t.stdout).contains("grande.wiff.scan "),
        "{}",
        t.stderr
    );
    let o = run(&[
        "sample", "select", &wiff, "--ranges", "--sample", "1,9", "--json",
    ]);
    assert_eq!(o.code, 5);
    assert!(String::from_utf8_lossy(&o.stdout).contains("err.sample.unknown"));
    assert_eq!(
        run(&["sample", "select", &wiff, "--ranges", "--sample", "3-1"]).code,
        2
    );
    assert_eq!(run(&["sample", "select", &wiff, "--ranges"]).code, 2);
    let none = dir.join("none.wiff").to_string_lossy().into_owned();
    assert_eq!(
        run(&["sample", "select", &none, "--ranges", "--sample", "1"]).code,
        4
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_range_copy_converts_the_chosen_samples_to_the_bytes_of_the_whole_pair() {
    let (dir, wiff) = pair("copy");
    let o = run(&[
        "sample", "select", &wiff, "--ranges", "--sample", "2,4", "--json",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let ranges = scan_ranges(&String::from_utf8_lossy(&o.stdout));
    let scan = std::fs::read(dir.join("in/grande.wiff.scan")).unwrap();
    let mut part = vec![0u8; scan.len()];
    for (a, b) in &ranges {
        part[*a as usize..*b as usize].copy_from_slice(&scan[*a as usize..*b as usize]);
    }
    assert!(part != scan, "the copy must leave something out");
    std::fs::create_dir_all(dir.join("part")).unwrap();
    std::fs::copy(&wiff, dir.join("part/grande.wiff")).unwrap();
    std::fs::write(dir.join("part/grande.wiff.scan"), &part).unwrap();
    let part_wiff = dir.join("part/grande.wiff").to_string_lossy().into_owned();
    let (full, cut) = (dir.join("full_out"), dir.join("cut_out"));
    for (input, out) in [(&wiff, &full), (&part_wiff, &cut)] {
        let o = run(&[
            "file",
            "convert",
            input,
            "--out",
            &out.to_string_lossy(),
            "--sample",
            "2,4",
        ]);
        assert_eq!(o.code, 0, "{}", o.stderr);
    }
    let names = |d: &Path| {
        let mut v: Vec<_> = std::fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".mzML"))
            .collect();
        v.sort();
        v
    };
    let n = names(&full);
    assert_eq!(n.len(), 2, "{n:?}");
    assert_eq!(n, names(&cut));
    // Everything but the identity of the data file is equal. The SHA-1 of the `.wiff.scan` (and the checksum of the mzML that covers it) is taken over the
    // bytes given, so it differs for a partial copy: how a partial copy is identified is the owner's open question Q4 (`wiff-sample-choice.md`), not decided here.
    let lines = |p: PathBuf| {
        let t = String::from_utf8(std::fs::read(p).unwrap()).unwrap();
        t.replace("><", ">\n<")
            .lines()
            .map(String::from)
            .collect::<Vec<_>>()
    };
    let identity = |l: &String| l.contains("name=\"SHA-1\"") || l.contains("<fileChecksum>");
    for f in &n {
        let (a, b) = (lines(full.join(f)), lines(cut.join(f)));
        assert_eq!(a.len(), b.len(), "{f}");
        let differ: Vec<_> = a.iter().zip(&b).filter(|x| x.0 != x.1).collect();
        assert_eq!(differ.len(), 2, "{f}: {differ:?}");
        assert!(
            differ.iter().all(|x| identity(x.0) && identity(x.1)),
            "{f}: {differ:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
