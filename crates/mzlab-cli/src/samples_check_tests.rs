//! `sample list` against a pair that disagrees with itself (`samples_check.rs`): the evidence, the warning and the exit codes 1 and 8.

use super::{pairs, run};

/// Sets the modification time of both members of the pair `stem`.wiff in `d` to `secs_ago` seconds before now.
fn age_pair(d: &std::path::Path, stem: &str, secs_ago: u64) {
    let when = std::time::SystemTime::now() - std::time::Duration::from_secs(secs_ago);
    for n in [format!("{stem}.wiff"), format!("{stem}.wiff.scan")] {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open(d.join(n))
            .unwrap();
        f.set_modified(when).unwrap();
    }
}

/// A `.wiff` copied while the instrument was writing (the index lists the first samples, the `.wiff.scan` already holds more sections) is flagged with its
/// evidence, never listed as if it were whole: `consistency` in the JSON (sizes, sections, age, threshold), a warning on stderr, exit 8 when a member was
/// modified in the last `RECENT_S` seconds (still being written) and 1 when it was not (a finished but partial copy); the listing stays on stdout either way.
#[test]
fn a_pair_whose_scan_outruns_its_index_or_ends_before_it_is_flagged_with_its_evidence() {
    use mzlab_core::wiff::RECENT_S;
    let d = pairs("consistency");
    let at = |n: &str| d.join(format!("{n}.wiff")).to_str().unwrap().to_string();
    let doc = |out: &str| mzlab_core::json::parse(out.as_bytes()).unwrap();
    let consistency = |out: &str| {
        doc(out).get("files").unwrap().arr()[0]
            .get("consistency")
            .cloned()
    };
    // a whole pair carries no `consistency`: its bytes stay the ones of the site's listing
    age_pair(&d, "totale", 3600);
    let (code, out, err) = run(&["sample", "list", &at("totale"), "--json"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        consistency(&out).is_none() && !err.contains("warning"),
        "{out}{err}"
    );
    // unlisted sections, written a moment ago: still being written = exit 8, with the listing and the numbers
    let (code, out, err) = run(&["sample", "list", &at("unindexed"), "--json"]);
    assert_eq!(code, 8, "{err}");
    let c = consistency(&out).expect("consistency");
    let num = |k: &str| c.get(k).and_then(|v| v.f64()).unwrap() as u64;
    assert_eq!(
        c.get("status").and_then(|v| v.str()),
        Some("unlisted"),
        "{out}"
    );
    assert_eq!(
        (num("samples_listed"), num("sections_unlisted")),
        (2, 2),
        "{out}"
    );
    assert_eq!(num("recent_s"), RECENT_S);
    assert!(
        num("age_s") < RECENT_S && num("scan_bytes") > num("indexed_end") && num("wiff_bytes") > 0,
        "{out}"
    );
    assert_eq!(
        c.get("still_writing"),
        Some(&mzlab_core::json::Value::Bool(true))
    );
    assert_eq!(
        doc(&out).get("files").unwrap().arr()[0]
            .get("samples")
            .unwrap()
            .arr()
            .len(),
        2
    );
    assert!(
        err.contains("still being written, exit 8")
            && err.contains("2 samples")
            && err.contains("2 more section"),
        "{err}"
    );
    // the same pair, not touched for an hour: a partial copy, exit 1, same evidence, no longer "still writing"
    age_pair(&d, "unindexed", 3600);
    let (code, out, err) = run(&["sample", "list", &at("unindexed"), "--json"]);
    assert_eq!(code, 1, "{err}");
    let c = consistency(&out).unwrap();
    assert_eq!(
        c.get("still_writing"),
        Some(&mzlab_core::json::Value::Bool(false)),
        "{out}"
    );
    assert!(
        c.get("age_s").and_then(|v| v.f64()).unwrap() >= 3600.0 && err.contains("exit 1"),
        "{out}{err}"
    );
    // the table form says it too
    let (code, table, err) = run(&["sample", "list", &at("unindexed")]);
    assert_eq!((code, table.lines().count() > 1), (1, true), "{table}");
    assert!(
        err.contains("warning") && !table.contains("warning"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// A `.wiff.scan` cut short is `short` without `--probe` (exit 1); padding after the last section is not an alarm (exit 0, no `consistency`).
#[test]
fn a_cut_scan_is_short_and_padding_is_not_an_alarm() {
    let d = pairs("consistency_short");
    let at = |n: &str| d.join(format!("{n}.wiff")).to_str().unwrap().to_string();
    let doc = |out: &str| mzlab_core::json::parse(out.as_bytes()).unwrap();
    let consistency = |out: &str| {
        doc(out).get("files").unwrap().arr()[0]
            .get("consistency")
            .cloned()
    };
    for (name, cut) in [("short", true), ("padded", false)] {
        std::fs::copy(d.join("totale.wiff"), d.join(format!("{name}.wiff"))).unwrap();
        let mut scan = std::fs::read(d.join("totale.wiff.scan")).unwrap();
        if cut {
            scan.truncate(scan.len() - 5);
        } else {
            scan.extend([0u8; 64]);
        }
        std::fs::write(d.join(format!("{name}.wiff.scan")), scan).unwrap();
        age_pair(&d, name, 3600);
    }
    let (code, out, err) = run(&["sample", "list", &at("short"), "--json"]);
    assert_eq!(code, 1, "{err}");
    assert_eq!(
        consistency(&out)
            .unwrap()
            .get("status")
            .and_then(|v| v.str()),
        Some("short"),
        "{out}"
    );
    let (code, out, err) = run(&["sample", "list", &at("padded"), "--json"]);
    assert_eq!(code, 0, "{err}");
    assert!(consistency(&out).is_none(), "{out}");
    let _ = std::fs::remove_dir_all(&d);
}
