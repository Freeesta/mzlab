//! `simtab::table_text` on synthetic traces sampled every 0.8 s with peaks 12-15 s wide at the base (about 15 points per peak, the SIM methods of a
//! triple quadrupole): rows and columns of both layouts, an isomer pair 25 s apart in one channel, a blank, a channel without Q1, columns keyed by
//! m/z across files, groups (row order, `group` column, channels without Q1 apart per group).

use super::*;

/// The table against the line between the edges (the numbers these tests were written for).
fn edges(files: &str, output: &str, layout: &str) -> Result<String> {
    table_text_with(files, output, layout, "edges")
}

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
    let area = edges(&set(), "area_sum", "samples").unwrap();
    let lines: Vec<&str> = area.lines().collect();
    assert_eq!(lines[0], "sample,237.1,253.1,ion 2,baseline");
    assert_eq!(lines.len(), 4, "{area}");
    assert!(
        lines[1].starts_with("t0,") && lines[1].ends_with(",,edges"),
        "{area}"
    );
    assert_eq!(
        lines[2], "blank,0.00000E+00,0.00000E+00,,edges",
        "a blank: no peak, sum 0, no error"
    );
    assert!(lines[3].starts_with("\"s, 3h\",,"), "{area}");
    let n = edges(&set(), "n_peaks", "samples").unwrap();
    assert_eq!(
        n.lines().nth(1).unwrap(),
        "t0,1,2,,edges",
        "the isomer pair is two peaks of one channel"
    );
    let max = edges(&set(), "area_max", "samples").unwrap();
    assert_eq!(max.lines().nth(2).unwrap(), "blank,,,,edges");
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
    let rt = edges(&set(), "apex_rt", "channels").unwrap();
    assert_eq!(
        rt.lines().next().unwrap(),
        "channel,name,t0,blank,\"s, 3h\",baseline"
    );
    assert_eq!(rt.lines().nth(2).unwrap(), "253.1,B,8.000,,8.120,edges");
}

#[test]
fn the_peak_table_has_one_row_per_resolved_peak() {
    let p = edges(&set(), "peaks", "samples").unwrap();
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
    assert!(p.contains("\nblank,237.1000,A,0,,,,,,edges\n"), "{p}");
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
        edges(&only, "area_sum", "samples").unwrap_err().key,
        "err.simtab.noChannel"
    );
    assert_eq!(
        edges(&set(), "area", "samples").unwrap_err().key,
        "err.simtab.output"
    );
    assert_eq!(
        edges(&set(), "area_sum", "rows").unwrap_err().key,
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
    let t = edges(&set, "n_peaks", "samples").unwrap();
    assert_eq!(t, "sample,253.1,196.1,baseline\na,1,,edges\nb,1,1,edges\n");
}

#[test]
fn groups_order_the_rows_and_keep_channels_without_q1_apart() {
    let set = grouped(&[
        ("a1", "List A", vec![trace(1, None, "", &[(5.0, 8e4)])]),
        ("free", "", vec![trace(1, Some(300.2), "", &[(5.0, 8e4)])]),
        ("b1", "List B", vec![trace(1, None, "", &[(6.0, 8e4)])]),
        ("a2", "List A", vec![trace(1, None, "", &[(5.0, 8e4)])]),
    ]);
    let t = edges(&set, "n_peaks", "samples").unwrap();
    assert_eq!(
        t,
        "sample,group,ion 1 [List A],ion 1 [List B],300.2,baseline\na1,List A,1,,,edges\na2,List A,1,,,edges\nb1,List B,,1,,edges\nfree,,,,1,edges\n"
    );
    let p = edges(&set, "peaks", "samples").unwrap();
    assert!(p.starts_with("sample,group,q1,name,peak,"), "{p}");
    assert!(p.contains("\nb1,List B,,,1,6.000,"), "{p}");
    let c = edges(&set, "n_peaks", "channels").unwrap();
    assert_eq!(
        c.lines().next().unwrap(),
        "channel,name,a1,a2,b1,free,baseline"
    );
}

#[test]
fn from_zero_is_the_analytic_area_and_says_so() {
    // no background: the area from zero over the detector's edges is the Gaussian's, h sigma sqrt(2 pi) (counts x s), the height its apex
    let (h, s) = (6e4, 3.2);
    let (t, y): (Vec<String>, Vec<String>) = (0..1500)
        .map(|i| {
            let x = i as f64 * 0.8 / 60.0;
            (
                format!("{x:.4}"),
                format!("{:.3}", h * (-0.5 * ((x - 5.0) * 60.0 / s).powi(2)).exp()),
            )
        })
        .unzip();
    let tr = format!(
        "{{\"id\":0,\"q1\":300.1,\"q3\":null,\"name\":\"G\",\"rt\":[{}],\"y\":[{}]}}",
        t.join(","),
        y.join(",")
    );
    let one = files(&[("g.mzML", vec![tr])]);
    let peaks = table_text_with(&one, "peaks", "samples", "zero").unwrap();
    let row: Vec<&str> = peaks.lines().nth(1).unwrap().split(',').collect();
    let (area, height): (f64, f64) = (row[5].parse().unwrap(), row[6].parse().unwrap());
    let exact = h * s * (2.0 * std::f64::consts::PI).sqrt();
    assert!((area - exact).abs() < 0.01 * exact, "{area} {exact}");
    assert_eq!((height, *row.last().unwrap()), (h, "zero"));
    assert!(peaks.lines().next().unwrap().ends_with(",fwhm_s,baseline"));
    // on a background of about 50 counts the area from zero is larger than the one against the edge line, in every output and layout
    let (e, z) = (
        edges(&set(), "area_max", "samples").unwrap(),
        table_text(&set(), "area_max", "samples").unwrap(),
    );
    let first = |t: &str| -> f64 {
        t.lines()
            .nth(1)
            .unwrap()
            .split(',')
            .nth(1)
            .unwrap()
            .parse()
            .unwrap()
    };
    assert!(first(&z) > first(&e), "{z}\n{e}");
    for (o, l) in [
        ("area_sum", "samples"),
        ("n_peaks", "channels"),
        ("peaks", "samples"),
    ] {
        let z = table_text_with(&set(), o, l, "zero").unwrap();
        assert!(z.lines().next().unwrap().ends_with(",baseline"), "{z}");
        assert!(z.lines().skip(1).all(|r| r.ends_with(",zero")), "{z}");
        // no baseline named = zero; edges is still a choice and says so too
        assert_eq!(z, table_text(&set(), o, l).unwrap());
        let e = edges(&set(), o, l).unwrap();
        assert!(e.lines().skip(1).all(|r| r.ends_with(",edges")), "{e}");
    }
    for b in ["line", "valley"] {
        let e = table_text_with(&set(), "area_sum", "samples", b).unwrap_err();
        assert!(
            e.to_json().contains("err.simtab.baseline"),
            "{}",
            e.to_json()
        );
    }
}
