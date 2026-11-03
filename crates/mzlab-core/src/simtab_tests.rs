//! `simtab::table_text` on synthetic traces sampled every 0.8 s with peaks 12-15 s wide at the base (about 15 points per peak, the SIM methods of a
//! triple quadrupole): rows and columns of both layouts, an isomer pair 25 s apart in one channel, a blank, a channel without Q1, columns keyed by
//! m/z across files, groups (row order, `group` column, channels without Q1 apart per group).

use super::*;

/// A trace of 20 min at 0.8 s with ripple noise and Gaussian peaks `(apex min, height)` of sigma 3.2 s (about 13 s at the base).
fn trace(id: i64, q1: Option<f64>, name: &str, peaks: &[(f64, f64)]) -> String {
    let (mut t, mut y) = (Vec::new(), Vec::new());
    for i in 0..1500 {
        let x = i as f64 * 0.8 / 60.0;
        t.push(format!("{x:.4}"));
        let s: f64 = peaks
            .iter()
            .map(|&(a, h)| h * (-0.5 * ((x - a) * 60.0 / 3.2).powi(2)).exp())
            .sum();
        y.push(format!("{:.1}", 50.0 + (i % 5) as f64 + s));
    }
    let q1 = q1.map_or("null".to_string(), |q| q.to_string());
    format!("{{\"id\":{id},\"q1\":{q1},\"q3\":null,\"ce\":null,\"name\":\"{name}\",\"dwell\":100,\"rt\":[{}],\"y\":[{}]}}", t.join(","), y.join(","))
}

fn files(list: &[(&str, Vec<String>)]) -> String {
    grouped(
        &list
            .iter()
            .map(|(n, tr)| (*n, "", tr.clone()))
            .collect::<Vec<_>>(),
    )
}

/// Files with a group ("" = none).
fn grouped(list: &[(&str, &str, Vec<String>)]) -> String {
    let parts: Vec<String> = list
        .iter()
        .map(|(n, g, tr)| {
            let g = if g.is_empty() {
                String::new()
            } else {
                format!(",\"group\":\"{g}\"")
            };
            format!("{{\"name\":\"{n}\",\"transitions\":[{}]{g}}}", tr.join(","))
        })
        .collect();
    format!("[{}]", parts.join(","))
}

fn set() -> String {
    files(&[
        (
            "t0.mzML",
            vec![
                trace(0, Some(237.1), "A", &[(5.0, 8e4)]),
                trace(1, Some(253.1), "B", &[(8.0, 5e4), (8.0 + 25.0 / 60.0, 3e4)]),
            ],
        ),
        (
            "blank.mzML",
            vec![
                trace(0, Some(237.1), "A", &[]),
                trace(1, Some(253.1), "B", &[]),
            ],
        ),
        (
            "s, 3h.mzml",
            vec![
                trace(1, Some(253.1), "B", &[(8.12, 2e4)]),
                trace(2, None, "", &[(12.0, 1e4)]),
            ],
        ),
    ])
}

#[test]
fn rows_are_samples_and_columns_channels() {
    let area = table_text(&set(), "area_sum", "samples").unwrap();
    let lines: Vec<&str> = area.lines().collect();
    assert_eq!(lines[0], "sample,237.1,253.1,ion 2");
    assert_eq!(lines.len(), 4, "{area}");
    assert!(
        lines[1].starts_with("t0,") && lines[1].ends_with(','),
        "{area}"
    );
    assert_eq!(
        lines[2], "blank,0.00000E+00,0.00000E+00,",
        "a blank: no peak, sum 0, no error"
    );
    assert!(lines[3].starts_with("\"s, 3h\",,"), "{area}");
    let n = table_text(&set(), "n_peaks", "samples").unwrap();
    assert_eq!(
        n.lines().nth(1).unwrap(),
        "t0,1,2,",
        "the isomer pair is two peaks of one channel"
    );
    let max = table_text(&set(), "area_max", "samples").unwrap();
    assert_eq!(max.lines().nth(2).unwrap(), "blank,,,");
    let sum: f64 = area
        .lines()
        .nth(1)
        .unwrap()
        .split(',')
        .nth(2)
        .unwrap()
        .parse()
        .unwrap();
    let top: f64 = max
        .lines()
        .nth(1)
        .unwrap()
        .split(',')
        .nth(2)
        .unwrap()
        .parse()
        .unwrap();
    assert!(sum > top * 1.4, "{sum} {top}");
    let rt = table_text(&set(), "apex_rt", "channels").unwrap();
    assert_eq!(
        rt.lines().next().unwrap(),
        "channel,name,t0,blank,\"s, 3h\""
    );
    assert_eq!(rt.lines().nth(2).unwrap(), "253.1,B,8.000,,8.120");
}

#[test]
fn the_peak_table_has_one_row_per_resolved_peak() {
    let p = table_text(&set(), "peaks", "samples").unwrap();
    let rows: Vec<&str> = p
        .lines()
        .filter(|l| l.starts_with("t0,253.1000,B,"))
        .collect();
    assert_eq!(rows.len(), 2, "{p}");
    let apex: Vec<f64> = rows
        .iter()
        .map(|l| l.split(',').nth(4).unwrap().parse().unwrap())
        .collect();
    assert!(
        (apex[0] - 8.0).abs() < 0.01 && (apex[1] - (8.0 + 25.0 / 60.0)).abs() < 0.01,
        "{apex:?}"
    );
    assert!(
        rows[0].split(',').nth(3) == Some("1") && rows[1].split(',').nth(3) == Some("2"),
        "peaks numbered in RT order"
    );
    assert!(p.contains("\nblank,237.1000,A,0,,,,,\n"), "{p}");
    let area = rows[0].split(',').nth(5).unwrap();
    assert!(
        area.len() == 11 && &area[7..9] == "E+" && area.parse::<f64>().is_ok(),
        "{area}"
    );
    assert!(p.contains("\n\"s, 3h\",,,1,12.000,"), "{p}");
}

#[test]
fn no_sim_channel_and_bad_options() {
    let mrm = "{\"id\":1,\"q1\":100,\"q3\":50,\"name\":\"\",\"rt\":[0,1,2],\"y\":[1,9,1]}";
    let only = format!("[{{\"name\":\"x\",\"transitions\":[{mrm}]}}]");
    assert_eq!(
        table_text(&only, "area_sum", "samples").unwrap_err().key,
        "err.simtab.noChannel"
    );
    assert_eq!(
        table_text(&set(), "area", "samples").unwrap_err().key,
        "err.simtab.output"
    );
    assert_eq!(
        table_text(&set(), "area_sum", "rows").unwrap_err().key,
        "err.simtab.output"
    );
}

#[test]
fn a_column_is_one_mz_across_files_whatever_its_place() {
    let set = files(&[
        ("a", vec![trace(1, Some(253.1), "X", &[(5.0, 8e4)])]),
        (
            "b",
            vec![
                trace(4, Some(196.1), "", &[(6.0, 8e4)]),
                trace(5, Some(253.1), "Y", &[(5.0, 4e4)]),
            ],
        ),
    ]);
    let t = table_text(&set, "n_peaks", "samples").unwrap();
    assert_eq!(t, "sample,253.1,196.1\na,1,\nb,1,1\n");
}

#[test]
fn groups_order_the_rows_and_keep_channels_without_q1_apart() {
    let set = grouped(&[
        ("a1", "List A", vec![trace(1, None, "", &[(5.0, 8e4)])]),
        ("free", "", vec![trace(1, Some(300.2), "", &[(5.0, 8e4)])]),
        ("b1", "List B", vec![trace(1, None, "", &[(6.0, 8e4)])]),
        ("a2", "List A", vec![trace(1, None, "", &[(5.0, 8e4)])]),
    ]);
    let t = table_text(&set, "n_peaks", "samples").unwrap();
    assert_eq!(
        t,
        "sample,group,ion 1 [List A],ion 1 [List B],300.2\na1,List A,1,,\na2,List A,1,,\nb1,List B,,1,\nfree,,,,1\n"
    );
    let p = table_text(&set, "peaks", "samples").unwrap();
    assert!(p.starts_with("sample,group,q1,name,peak,"), "{p}");
    assert!(p.contains("\nb1,List B,,,1,6.000,"), "{p}");
    let c = table_text(&set, "n_peaks", "channels").unwrap();
    assert_eq!(c.lines().next().unwrap(), "channel,name,a1,a2,b1,free");
}
