//! `manint::table_text` on traces the page supplies (chromatogram, XIC, DAD): the five extra columns, the grouping by the trace's own label, and the
//! bytes of a table without a panel staying as they were.

use super::tests::trace;
use super::*;

/// One page-supplied trace: `trace` with a panel, an own label and a unit added.
fn page_trace(panel: &str, label: &str, unit: &str, peak: (f64, f64)) -> String {
    let t = trace(None, None, "", &[peak]);
    t.replacen(
        '{',
        &format!("{{\"panel\":\"{panel}\",\"label\":\"{label}\",\"unit\":\"{unit}\","),
        1,
    )
}

fn files() -> String {
    let f = |name: &str, extra: &str, pos: &str| {
        format!(
            "{{\"name\":\"{name}\"{extra},\"transitions\":[{},{}]}}",
            page_trace("XIC 1", "m/z 100.1", "", (3.0, 8e4)),
            page_trace("DAD", "254 nm", "mAU·s", (5.0, 2e3)),
        )
        .replace("POS", pos)
    };
    format!(
        "[{},{}]",
        f("s1.mzML", ",\"conc\":2.5,\"conc_unit\":\"ppm\"", ""),
        f("s2.mzML", "", "")
    )
}

#[test]
fn extra_columns_follow_origin() {
    let w = "[{\"file\":0,\"tr\":0,\"a\":2.5,\"b\":3.5,\"baseline\":\"zero\",\"corr\":\"drift\",\"follow\":true}]";
    let t = table_text(&files(), w, true).unwrap();
    let head = t.lines().next().unwrap();
    assert_eq!(head, format!("{}{}", HEAD.trim_end(), EXT_HEAD));
    let rows: Vec<Vec<&str>> = t.lines().skip(1).map(|l| l.split(',').collect()).collect();
    // two samples on the one channel (same label across files), then its sum
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|r| r.len() == 23));
    assert_eq!(rows[0][0], "m/z 100.1");
    assert_eq!(
        &rows[0][17..],
        ["XIC 1", "2.5", "ppm", "drift", "0.00000E+00", ""]
    );
    assert_eq!(rows[1][17], "XIC 1");
    assert_eq!(rows[1][18], "");
    assert_eq!(rows[2][2], "");
    assert_eq!(rows[2][1], "sum");
}

#[test]
fn unit_and_edges_below() {
    let w = "[{\"file\":0,\"tr\":1,\"a\":4.5,\"b\":5.5,\"baseline\":\"edges\"}]";
    let t = table_text(&files(), w, false).unwrap();
    let r: Vec<&str> = t.lines().nth(1).unwrap().split(',').collect();
    assert_eq!(r[0], "254 nm");
    assert_eq!(r[17], "DAD");
    assert_eq!(r[21], "");
    assert_eq!(r[22], "mAU·s");
}
