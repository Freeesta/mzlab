//! `campioni/2`: the table of the names, the same minutes and roles as `project::guess_sample` on the generated names of `project_parity`,
//! the findings of `valida`, the migration of `da_legacy`, the indices of `per_tp`, the canonical JSON and its schema.

use mzlab_core::campioni::{
    self, da_legacy, from_json, per_tp, proponi, to_json, valida, Campione,
};
use mzlab_core::project::guess_sample;
use serde_json::Value;

fn one(name: &str, kind: &str) -> Campione {
    proponi(&[(name.to_string(), "ab".to_string(), kind.to_string())]).remove(0)
}

fn many(names: &[&str]) -> Vec<Campione> {
    let v: Vec<(String, String, String)> = names
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    proponi(&v)
}

/// (name, kind of the file, role, time as written, unit, minutes, replicate)
type Row = (
    &'static str,
    &'static str,
    &'static str,
    Option<f64>,
    &'static str,
    Option<f64>,
    u32,
);
#[rustfmt::skip]
const ROWS: [Row; 36] = [
        ("t0.mzML", "", "t0", Some(0.0), "min", Some(0.0), 1),
    ("t5.mzML", "", "sample", Some(5.0), "min", Some(5.0), 1),
    ("X_TiO2_t000min.mzML", "", "t0", Some(0.0), "min", Some(0.0), 1),
    ("X_TiO2_t005min.mzML", "", "sample", Some(5.0), "min", Some(5.0), 1),
    ("S_t5", "", "sample", Some(5.0), "min", Some(5.0), 1),
    ("S-t5", "", "sample", Some(5.0), "min", Some(5.0), 1),
    ("S-t30 (2).mzML", "", "sample", Some(30.0), "min", Some(30.0), 2),
    ("X_TiO2_t120min_(3)", "", "sample", Some(120.0), "min", Some(120.0), 3),
    ("S_30min", "", "sample", Some(30.0), "min", Some(30.0), 1),
    ("S_120min.raw", "", "sample", Some(120.0), "min", Some(120.0), 1),
    ("S_72h", "", "sample", Some(72.0), "h", Some(4320.0), 1),
    ("S_2d", "", "sample", Some(2.0), "d", Some(2880.0), 1),
    ("S_1,5h", "", "sample", Some(1.5), "h", Some(90.0), 1),
    ("S_2 ore", "", "sample", Some(2.0), "h", Some(120.0), 1),
    ("S_4gg", "", "sample", Some(4.0), "d", Some(5760.0), 1),
    ("S_90 s", "", "sample", Some(90.0), "s", Some(1.5), 1),
    ("S_90s.mzML", "", "sample", Some(90.0), "s", Some(1.5), 1),
    ("S_t7.5", "", "sample", Some(7.5), "min", Some(7.5), 1),
    ("X25ppm_t10", "", "sample", Some(10.0), "min", Some(10.0), 1),
    ("blank.mzML", "", "blank", None, "min", None, 1),
    ("blk_2", "", "blank", None, "min", None, 1),
    ("blank_t30", "", "blank", None, "min", None, 1),
    ("bianco_solv", "", "blank", None, "min", None, 1),
    ("X_blank_proc", "", "blank", None, "min", None, 1),
    ("X_dark-adsorption", "", "dark", None, "min", None, 1),
    ("X_buio_t30", "", "dark", Some(30.0), "min", Some(30.0), 1),
    ("X_adsorption", "", "dark", None, "min", None, 1),
    ("X_infusion", "", "infusion", None, "min", None, 1),
    ("X_infusione", "", "infusion", None, "min", None, 1),
    ("X_standard_direct-infusion_MSn", "", "infusion", None, "min", None, 1),
    ("X_FIA_1", "", "infusion", None, "min", None, 1),
    ("X_parent", "msn", "infusion", None, "min", None, 1),
    ("std_5ppm", "", "std", None, "min", None, 1),
    ("cal_10ppm", "", "calibration", None, "min", None, 1),
    ("QC_1", "", "qc", None, "min", None, 1),
    ("x", "", "sample", None, "min", None, 1),
];

#[test]
fn table_of_the_names() {
    for (name, kind, role, time, unit, minutes, rep) in ROWS {
        let c = one(name, kind);
        let got = (
            c.kind.as_str(),
            c.time,
            c.time_unit.as_str(),
            c.time_min,
            c.replicate,
        );
        assert_eq!(got, (role, time, unit, minutes, rep), "{name}");
        assert!(!c.time_confirmed, "{name}");
    }
    assert_eq!(one("std_5ppm", "").conc, Some(5.0));
    assert_eq!(one("std_5ppm", "").unit.as_deref(), Some("ppm"));
    assert_eq!(one("X25ppm_t10", "").conc, None);
    assert_eq!(one("blank", "").blank_kind, None);
    assert_eq!(
        one("bianco_solv", "").blank_kind.as_deref(),
        Some("solvent")
    );
    assert_eq!(
        one("X_blank_proc", "").blank_kind.as_deref(),
        Some("procedure")
    );
}

#[test]
fn series_from_the_prefix() {
    let s = |cs: Vec<Campione>| cs.into_iter().map(|c| c.series).collect::<Vec<_>>();
    assert_eq!(
        s(many(&[
            "X_TiO2_t000min",
            "X_TiO2_t005min",
            "X_dark-adsorption"
        ])),
        ["", "", ""]
    );
    assert_eq!(s(many(&["t0", "S-t5", "S-t30 (2)"])), ["", "", ""]);
    let got = s(many(&["A_t0", "A_t5", "B_t0", "B_t5", "A_dark", "blank"]));
    assert_eq!(got, ["A", "A", "B", "B", "A", ""]);
    let cs = many(&["S-t30", "S-t30 (2)", "S-t60"]);
    assert_eq!(
        cs.iter().map(|c| c.replicate).collect::<Vec<_>>(),
        [1, 2, 1]
    );
}

/// The names of `tools/project_riferimento.py` (the parity table of `guess_sample`), all of them.
fn generated() -> Vec<String> {
    let heads = [
        "",
        "s_",
        "Sample-",
        "std_",
        "STD",
        "stds ",
        "calib_",
        "standard-",
        "cal",
        "taratura_",
        "curva ",
        "blank",
        "BLK_",
        "bianco_",
        "mrm-",
        "full_",
        "x",
    ];
    let cores = [
        "t0", "t7.5", "t30", "t7,5", "5", "0.5", "10", "120min", "2h", "1d", "3days", "3day",
        "2 ore", "4gg", "1giorno", "2giorni", "30 min", "7_2ppm", "5ppm", "0.5ppm", "10mgL",
        "5_ug-L", "2 mg/L", "3p5ppb", "1ng/mL", "1,5 ppt", "5 g/l", "2mg/l-1", "4 mgml", "9µg/L",
        "8μg/l", "1d2", "t5m", "5ms", "20 xyz", "", "12", "3.", "3,",
    ];
    let tails = ["", "_1", "-2", " rep", "_t3", "ppm", "_A", "5", "mgL"];
    let exts = ["", ".mzML", ".MZML", ".raw", ".d", ".wiff"];
    let mut out = vec![];
    for h in heads {
        for c in cores {
            for t in tails {
                out.extend(exts.iter().map(|e| format!("{h}{c}{t}{e}")));
            }
        }
    }
    out
}

#[test]
fn same_roles_and_minutes_as_guess_sample() {
    let names = generated();
    assert!(names.len() > 30000);
    for n in &names {
        let (_, time, typ) = guess_sample(n);
        let c = one(n, "");
        let family = match c.kind.as_str() {
            "blank" => "blank",
            "std" | "calibration" => "standard",
            _ => "sample",
        };
        assert_eq!((family, c.time_min), (typ, time), "{n:?}");
    }
}

fn confirmed(mut cs: Vec<Campione>) -> Vec<Campione> {
    cs.iter_mut().for_each(|c| c.time_confirmed = true);
    cs
}

fn codes(cs: &[Campione]) -> Vec<(&'static str, Option<usize>, String)> {
    valida(cs)
        .into_iter()
        .map(|a| (a.code, a.file, a.series))
        .collect()
}

#[test]
fn findings() {
    let names = [
        "X_TiO2_t000min",
        "X_TiO2_t005min",
        "X_TiO2_t030min",
        "X_dark-adsorption",
        "X_standard_direct-infusion_MSn",
    ];
    let got = codes(&many(&names));
    assert!(got.contains(&("timeUnconfirmed", Some(1), String::new())));
    assert!(got.contains(&("noBlank", None, String::new())));
    assert!(!got
        .iter()
        .any(|g| g.0 == "noT0" || g.0 == "noDark" || g.0 == "noInfusion"));
    let ok = confirmed(many(&names));
    assert_eq!(codes(&ok), [("noBlank", None, String::new())]);
    assert!(campioni::valida_json(&valida(&ok)).ends_with("\"bloccante\":false}"));
    assert_eq!(
        codes(&confirmed(many(&["x", "t0"])))[0],
        ("timeMissing", Some(0), String::new())
    );
    let mut t0 = confirmed(many(&["t0", "t5", "t10"]));
    t0[0].set_time(Some(5.0), "min");
    t0[0].kind = "t0".into();
    assert!(codes(&t0).contains(&("t0WithTime", Some(0), String::new())));
    let only = |v: Vec<Campione>| codes(&v).into_iter().map(|c| c.0).collect::<Vec<_>>();
    assert_eq!(only(many(&["blank"])), ["noTreated", "noInfusion"]);
    assert_eq!(
        only(many(&["std_5ppm"])),
        ["noTreated", "noControls", "noInfusion"]
    );
    assert_eq!(
        only(confirmed(many(&["t5", "t10"]))),
        ["noControls", "noInfusion"]
    );
    assert_eq!(
        only(confirmed(many(&["t0", "t5", "blank", "X_dark"]))),
        ["onePoint", "noInfusion"]
    );
    assert_eq!(
        only(confirmed(many(&[
            "t0", "S-t5", "S-t5 (2)", "S-t10", "blank", "X_dark"
        ]))),
        ["replicates", "noInfusion"]
    );
}

#[test]
fn findings_by_series() {
    let cs = confirmed(many(&[
        "A_t0", "A_t5", "A_t10", "B_t5", "B_t10", "blank", "A_dark",
    ]));
    let got = codes(&cs);
    assert!(got.contains(&("noT0", None, "B".into())));
    assert!(got.contains(&("noDark", None, "B".into())));
    assert!(!got
        .iter()
        .any(|g| g.0 == "noControls" || (g.2 == "A" && g.0 != "noInfusion")));
}

#[test]
fn indices_for_the_tp_path() {
    let cs = confirmed(many(&[
        "X_t000min",
        "X_t005min",
        "X_t030min",
        "blank",
        "X_dark-adsorption",
        "X_infusion",
        "std_5ppm",
        "QC_1",
        "x",
    ]));
    let p = per_tp(&cs);
    assert_eq!(
        (p.t0, p.treated, p.blank, p.dark),
        (vec![0], vec![1, 2], vec![3], vec![4])
    );
    assert_eq!(
        (p.infusion, p.std, p.qc, p.untimed),
        (vec![5], vec![6], vec![7], vec![8])
    );
    let n = None;
    assert_eq!(
        p.minutes,
        [Some(0.0), Some(5.0), Some(30.0), n, n, n, n, n, n]
    );
    let j = campioni::route("per_tp", &to_json(&cs)).unwrap();
    assert!(
        j.ends_with("\"minuti\":[0,5,30,null,null,null,null,null,null]}"),
        "{j}"
    );
}

#[test]
fn migration_of_the_old_places() {
    let input = r#"{"session":[{"file":"a_t0.mzML","time":0,"type":"sample"},{"file":"a_t5.mzML","time":5,"type":"sample","rep":2},
      {"file":"d.mzML","time":null,"type":"sample"},{"file":"std.mzML","type":"standard","conc":5,"cunit":"mg/L"}],
      "qaqc":{"roles":{"d.mzML":"dark","q.mzML":"qc"}},
      "mzflow":{"files":[{"nome":"q.mzML","sha256":"ff","tipo":"sample","tempo":null}]},
      "campioni1":{"schema":"campioni/1","campioni":[{"file":"a_t5.mzML","sha256":"aa","tipo":"sample","tempo_min":7,"replica":1,"gruppo":"",
      "concentrazione":null,"unita":null,"ordine_iniezione":2,"istd":true,"note":"n"},{"file":"c1.mzML","sha256":"cc","tipo":"dark","tempo_min":60,
      "replica":1,"gruppo":"dark","concentrazione":null,"unita":null,"ordine_iniezione":3,"istd":false,"note":""}]}}"#;
    let cs = da_legacy(input.as_bytes()).unwrap();
    let files: Vec<&str> = cs.iter().map(|c| c.file.as_str()).collect();
    assert_eq!(
        files,
        [
            "a_t0.mzML",
            "a_t5.mzML",
            "d.mzML",
            "std.mzML",
            "q.mzML",
            "c1.mzML"
        ]
    );
    let kinds: Vec<&str> = cs.iter().map(|c| c.kind.as_str()).collect();
    assert_eq!(kinds, ["t0", "sample", "dark", "std", "qc", "dark"]);
    let a = &cs[1];
    assert_eq!(
        (
            a.sha256.as_str(),
            a.time_min,
            a.time_confirmed,
            a.replicate,
            a.istd,
            a.note.as_str()
        ),
        ("aa", Some(5.0), true, 2, true, "n")
    );
    assert!(cs[0].time_confirmed && cs[0].time_min == Some(0.0));
    assert_eq!(
        (cs[3].conc, cs[3].unit.as_deref()),
        (Some(5.0), Some("mg/L"))
    );
    assert_eq!(cs[4].sha256, "ff");
    assert_eq!((cs[5].time_min, cs[5].time_confirmed), (Some(60.0), false));
    assert_eq!(
        cs.iter().map(|c| c.order).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6]
    );
    assert!(da_legacy(b"{}").unwrap().is_empty());
}

/// The few keywords of JSON Schema that `docs/schemi/campioni-2.json` uses.
fn fits(v: &Value, s: &Value) -> bool {
    let ty = |t: &str| match t {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.is_i64() || v.is_u64(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        _ => false,
    };
    let checks = [
        s.get("type").is_none_or(|t| {
            t.as_str().map_or_else(
                || {
                    t.as_array()
                        .unwrap()
                        .iter()
                        .any(|x| ty(x.as_str().unwrap()))
                },
                ty,
            )
        }),
        s.get("const").is_none_or(|c| c == v),
        s.get("enum")
            .is_none_or(|e| e.as_array().unwrap().contains(v)),
        s.get("minimum")
            .is_none_or(|m| v.as_f64().unwrap() >= m.as_f64().unwrap()),
        s.get("required").is_none_or(|r| {
            r.as_array()
                .unwrap()
                .iter()
                .all(|k| v.get(k.as_str().unwrap()).is_some())
        }),
        s.get("properties").is_none_or(|p| {
            p.as_object()
                .unwrap()
                .iter()
                .all(|(k, ps)| v.get(k).is_none_or(|x| fits(x, ps)))
        }),
        s.get("items")
            .is_none_or(|i| v.as_array().unwrap().iter().all(|x| fits(x, i))),
    ];
    checks.iter().all(|c| *c)
}

#[test]
fn canonical_json_and_schema() {
    let mut cs = many(&[
        "X_TiO2_t000min",
        "X_TiO2_t005min",
        "bianco_solv",
        "X_dark",
        "X_infusion",
        "std_5ppm",
        "QC_1",
        "S_90 s",
    ]);
    cs[4].compound = "C13H24N4O3S".into();
    let j = to_json(&cs);
    assert_eq!(from_json(j.as_bytes()).unwrap(), cs);
    assert_eq!(campioni::digest(&cs).len(), 64);
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/schemi/campioni-2.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let doc: Value = serde_json::from_str(&j).unwrap();
    assert!(fits(&doc, &schema));
    let first = &j[..j.find('}').unwrap()];
    let mut keys: Vec<&str> = first
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|k| doc["campioni"][0].get(*k).is_some())
        .collect();
    keys.dedup();
    assert!(
        keys.windows(2).all(|w| w[0] < w[1]) && keys.len() == 16,
        "{keys:?}"
    );
    let mut bad = doc.clone();
    bad["campioni"][0]["tipo"] = "control".into();
    assert!(!fits(&bad, &schema));
    assert!(from_json(bad.to_string().as_bytes()).is_err());
    let input =
        r#"{"files":[{"name":"t0","sha256":"ab"},{"name":"X_parent","sha256":"cd","kind":"msn"}]}"#;
    let p = from_json(campioni::route("proponi", input).unwrap().as_bytes()).unwrap();
    assert_eq!((p[0].kind.as_str(), p[1].kind.as_str()), ("t0", "infusion"));
    assert!(campioni::route("valida", &j)
        .unwrap()
        .contains("\"codice\":\"timeUnconfirmed\""));
    assert_eq!(
        campioni::route("nope", "{}").unwrap_err().key,
        "error.badCampioni"
    );
}
