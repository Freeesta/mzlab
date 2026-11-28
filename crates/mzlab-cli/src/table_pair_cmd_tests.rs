//! `mzlab-cli table pair` through `execute`: the CSV and the `pair/1` of the engine, the same bytes from CSV tables and from `.mzfeat` tables (the Flow
//! step), the evidence on stderr, and one exit code per kind of failure.

use std::path::PathBuf;

use mzlab_core::json::{parse, Value};

const H: f64 = 1.007276;

fn csv(rows: &[(&str, f64, f64)]) -> String {
    rows.iter()
        .fold(String::from("id,mz,rt\n"), |s, (id, mz, rt)| {
            s + &format!("{id},{mz:.5},{rt}\n")
        })
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_tpair_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    // neutral masses 180.06339 and 250.12 are paired; the negative decoys are the mass of 310.2 at 9.9 min and 310.3 at 3.12 min
    std::fs::write(
        d.join("p.csv"),
        csv(&[
            ("P1", 180.06339 + H, 5.0),
            ("P2", 250.12 + H, 8.2),
            ("P3", 310.2 + H, 3.1),
        ]),
    )
    .unwrap();
    std::fs::write(
        d.join("n.csv"),
        csv(&[
            ("N1", 180.06339 - H, 5.03),
            ("N2", 250.12 - H, 8.18),
            ("N3", 310.2 - H, 9.9),
            ("N4", 310.3 - H, 3.12),
        ]),
    )
    .unwrap();
    d
}

fn go(d: &PathBuf, extra: &[&str]) -> crate::Outcome {
    let mut a: Vec<String> = vec![
        "table".into(),
        "pair".into(),
        d.join("p.csv").to_string_lossy().into(),
        d.join("n.csv").to_string_lossy().into(),
    ];
    a.extend(extra.iter().map(|s| s.to_string()));
    crate::execute(&a)
}

#[test]
fn the_pairs_are_the_csv_and_the_evidence_is_on_stderr() {
    let d = dir("csv");
    let o = go(&d, &[]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        "pos_id,neg_id,rule,neutral_mass,err_da,err_ppm,rt_pos,rt_neg,drt,mutual"
    );
    assert!(
        lines[1].starts_with("P1,N1,[M+H]+ / [M-H]-,180.06339,"),
        "{text}"
    );
    assert!(lines[2].starts_with("P2,N2,"), "{text}");
    assert_eq!(lines.len(), 1 + 2 + 3, "{text}");
    assert!(lines.contains(&"P3,,unmatched,,,,3.1,,,"), "{text}");
    for needle in ["2 pairs", "[M+H]+", "[M-H]-", "5", "ppm"] {
        assert!(
            o.stderr.contains(needle),
            "{needle} missing from {}",
            o.stderr
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn json_has_the_keys_of_pair_1_and_set_reaches_the_engine() {
    let d = dir("json");
    let o = go(
        &d,
        &[
            "--json",
            "--set",
            "mass_tol=0.2",
            "--set",
            "mass_unit=da",
            "--set",
            "pos_adducts=[M+H]+,[M+Na]+",
        ],
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    assert_eq!(v.get("schema").and_then(Value::str), Some("pair/1"));
    for k in ["params", "counts", "pairs", "unmatched", "choices", "csv"] {
        assert!(v.get(k).is_some(), "{k}");
    }
    let p = v.get("params").unwrap();
    assert_eq!(p.get("mass_unit").and_then(Value::str), Some("da"));
    assert_eq!(
        p.get("pos_adducts").and_then(Value::str),
        Some("[M+H]+,[M+Na]+")
    );
    // a window of 0.2 Da takes the mass decoy N4 (0.1 Da off) at the right RT
    assert_eq!(
        v.get("counts").unwrap().get("pairs").and_then(Value::f64),
        Some(3.0)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_agent_gets_one_exit_code_per_kind_of_failure() {
    let d = dir("exit");
    assert_eq!(go(&d, &["--set", "rt_tol=-1"]).code, 2);
    assert_eq!(go(&d, &["--set", "pos_adducts=[M-H]-"]).code, 2);
    assert_eq!(go(&d, &["--set", "nope"]).code, 2);
    let one = crate::execute(&[
        "table".into(),
        "pair".into(),
        d.join("p.csv").to_string_lossy().into(),
    ]);
    assert_eq!(one.code, 2, "{}", one.stderr);
    std::fs::write(d.join("bad.csv"), "id,mz,rt\nX,abc,1\n").unwrap();
    let bad = crate::execute(&[
        "table".into(),
        "pair".into(),
        d.join("bad.csv").to_string_lossy().into(),
        d.join("n.csv").to_string_lossy().into(),
    ]);
    assert_eq!(bad.code, 3, "{}", bad.stderr);
    let missing = crate::execute(&[
        "table".into(),
        "pair".into(),
        d.join("none.csv").to_string_lossy().into(),
        d.join("n.csv").to_string_lossy().into(),
    ]);
    assert_eq!(missing.code, 4, "{}", missing.stderr);
    let out = d.join("no_such_dir").join("o.csv");
    assert_eq!(go(&d, &["--out", &out.to_string_lossy()]).code, 6);
    let help = crate::execute(&["table".into(), "pair".into(), "--help".into()]);
    assert_eq!(help.code, 0);
    assert!(String::from_utf8_lossy(&help.stdout).contains("Exit codes"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_same_bytes_as_the_flow_step_for_mzfeat_tables() {
    use mzlab_feat::recipe::Recipe;
    use mzlab_feat::store::{write_table, Origin};
    use mzlab_feat::table::{Aligned, CELL_DETECTED};
    let table = |rows: &[(f64, f64)]| {
        let mut t = Aligned::with_capacity(1, rows.len());
        for (mz, rt) in rows {
            t.mz.push(*mz);
            t.rt.push(*rt);
            t.area.push(6e5);
            t.height.push(1e5);
            t.apex_rt.push(*rt as f32);
            t.width.push(0.08);
            t.state.push(CELL_DETECTED);
        }
        let origin = Origin {
            inputs: vec![("c.mzML".to_string(), "00".repeat(32))],
            ..Origin::default()
        };
        write_table(&t, &Recipe::default(), &origin)
    };
    let d = dir("mzfeat");
    std::fs::write(
        d.join("p.mzfeat"),
        table(&[(180.06339 + H, 5.0), (310.2 + H, 3.1)]),
    )
    .unwrap();
    std::fs::write(
        d.join("n.mzfeat"),
        table(&[(180.06339 - H, 5.03), (310.3 - H, 3.12)]),
    )
    .unwrap();
    let a: Vec<String> = ["table", "pair"]
        .iter()
        .map(|s| s.to_string())
        .chain([d.join("p.mzfeat"), d.join("n.mzfeat")].map(|p| p.to_string_lossy().into_owned()))
        .collect();
    let o = crate::execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let (p, n) = (
        std::fs::read(d.join("p.mzfeat")).unwrap(),
        std::fs::read(d.join("n.mzfeat")).unwrap(),
    );
    let made = mzlab_feat::pair::pair_text(&[], ("p.mzfeat", &p), ("n.mzfeat", &n)).unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout), made.csv);
    assert!(
        made.csv
            .lines()
            .nth(1)
            .unwrap()
            .starts_with("1,1,[M+H]+ / [M-H]-,"),
        "{}",
        made.csv
    );
    let _ = std::fs::remove_dir_all(&d);
}
