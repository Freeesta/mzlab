//! Tests for `meta.compare@1` on aligned feature tables (`.mzfeat`) and per-file feature lists (`.mzfeatf`).

use mzlab_core::flow::{Artifact, Instance, Job, Resolved};
use mzlab_core::json::{parse, Value};
use mzlab_core::step::DataType;
use mzlab_feat::axis::MassAxis;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_file, write_table, Origin};
use mzlab_feat::table::{Aligned, FileFeatures, CELL_ABSENT, CELL_DETECTED, CELL_NA};

fn table_bytes(rows: &[(f64, f64, f64, bool)], unit: bool) -> Vec<u8> {
    let mut recipe = Recipe::default();
    if unit {
        recipe.detect.axis = MassAxis::Da { defect: 0.0 };
    }
    let mut t = Aligned::with_capacity(2, rows.len());
    for &(mz, rt, total_area, has_gap) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        if has_gap {
            t.area.extend([total_area, f64::NAN]);
            t.state.extend([CELL_DETECTED, CELL_NA]);
        } else {
            t.area.extend([total_area * 0.6, total_area * 0.4]);
            t.state.extend([CELL_DETECTED, CELL_DETECTED]);
        }
        t.height.extend([total_area as f32, total_area as f32]);
        t.apex_rt.extend([rt as f32, rt as f32]);
        t.width.extend([0.1, 0.1]);
    }
    write_table(&t, &recipe, &Origin::default())
}

fn peaks_bytes(rows: &[(f64, f32, f64)], sample: &str, unit: bool) -> Vec<u8> {
    let mut recipe = Recipe::default();
    if unit {
        recipe.detect.axis = MassAxis::Da { defect: 0.0 };
    }
    let n = rows.len();
    let mut f = FileFeatures {
        dt: 0.01,
        left: vec![0.0; n],
        right: vec![0.1; n],
        apex: vec![0; n],
        scan_lo: vec![0; n],
        scan_hi: vec![1; n],
        height: vec![1.0; n],
        npts: vec![5; n],
        flags: vec![0; n],
        iso_group: vec![0; n],
        charge: vec![1; n],
        shape: vec![1.0; n],
        snr: vec![10.0; n],
        fwhm: vec![0.05; n],
        asym: vec![1.0; n],
        gaps: vec![0; n],
        qreason: vec![0; n],
        ms2_n: vec![0; n],
        purity: vec![1.0; n],
        area_win: vec![0.0; n],
        family_id: vec![0; n],
        role: vec![0; n],
        alt: vec![0; n],
        ..Default::default()
    };
    for &(mz, rt, area) in rows {
        f.mz.push(mz);
        f.rt.push(rt);
        f.area.push(area);
    }
    let origin = Origin {
        inputs: if sample.is_empty() {
            Vec::new()
        } else {
            vec![(sample.to_string(), String::new())]
        },
        ..Default::default()
    };
    write_file(&f, &recipe, &origin)
}

#[test]
fn compares_two_aligned_tables_with_matched_and_unique_features() {
    let a = table_bytes(
        &[
            (200.0, 2.0, 1000.0, true),
            (300.0, 4.0, 5000.0, false),
            (400.0, 6.0, 2000.0, false),
        ],
        false,
    );
    let b = table_bytes(
        &[
            (200.0004, 2.02, 1200.0, false),
            (300.0006, 4.01, 4800.0, false),
            (500.0, 8.0, 3000.0, false),
        ],
        false,
    );
    let out =
        crate::flow_compare::compare_text("features.align@1", &["A", "B"], &[&a, &b]).unwrap();
    let v = parse(out.as_bytes()).unwrap();
    assert_eq!(v.get("format").unwrap().str(), Some("comparison/1"));
    assert_eq!(v.get("type").unwrap().str(), Some("table"));
    assert_eq!(v.get("by").unwrap().str(), Some("feature"));
    let p = &v.get("pairs").unwrap().arr()[0];
    let num = |k: &str| p.get(k).unwrap().f64().unwrap();
    assert_eq!(
        (
            num("count_a"),
            num("count_b"),
            num("both"),
            num("only_a"),
            num("only_b")
        ),
        (3.0, 3.0, 2.0, 1.0, 1.0)
    );
    assert_eq!(num("mean_overlap"), 0.5);
    assert!(num("mean_rank_corr").is_finite());
    assert!(num("median_dmz_ppm").is_finite());
    assert!(num("median_drt_min").is_finite());
}

fn run_dir_job(dir: &std::path::Path, da: std::path::PathBuf, db: std::path::PathBuf) -> Value {
    let work = dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let mut runner = crate::flow::FileRunner::new(dir, &work, None);
    runner.inputs.insert("dir_a".into(), da);
    runner.inputs.insert("dir_b".into(), db);
    let step = crate::flow_compare::compare_step();
    let inst = Instance {
        id: "cmp".into(),
        step: "meta.compare".into(),
        version: 1,
        algorithm: None,
        preset: None,
        label: None,
        ins: Vec::new(),
        params: Vec::new(),
    };
    let art = |label: &str| Artifact {
        label: Some(label.into()),
        sha256: String::new(),
        bytes: 0,
        ty: DataType::Peaks,
    };
    let job = Job {
        instance: &inst,
        step: &step,
        resolved: &Resolved {
            algorithm: None,
            preset: None,
            params: vec![
                ("step".into(), Value::Str("features.detect@2".into())),
                ("labels".into(), Value::Str("A,B".into())),
            ],
            noresult: Vec::new(),
            written: None,
        },
        item: None,
        item_name: None,
        item_root: None,
        inputs: vec![("variants".into(), vec![art("dir_a"), art("dir_b")])],
    };
    let out = crate::flow_compare::compare_job(&runner, &job).unwrap();
    parse(&std::fs::read(runner.path(&out)).unwrap()).unwrap()
}

#[test]
fn compares_peaks_directories_by_file_stem_and_aggregates() {
    let dir = std::env::temp_dir().join(format!("mzlab_cmp_peaks_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (da, db) = (dir.join("dir_a"), dir.join("dir_b"));
    std::fs::create_dir_all(&da).unwrap();
    std::fs::create_dir_all(&db).unwrap();
    std::fs::write(
        da.join("s1.mzfeatf"),
        peaks_bytes(
            &[
                (200.0, 2.0, 1000.0),
                (300.0, 4.0, 5000.0),
                (400.0, 6.0, 2000.0),
            ],
            "",
            false,
        ),
    )
    .unwrap();
    std::fs::write(
        da.join("s2.mzfeatf"),
        peaks_bytes(&[(150.0, 1.5, 800.0), (250.0, 3.5, 1600.0)], "", false),
    )
    .unwrap();
    std::fs::write(
        db.join("s1.mzfeatf"),
        peaks_bytes(
            &[
                (200.0003, 2.01, 1100.0),
                (300.0005, 4.02, 4900.0),
                (500.0, 7.0, 3000.0),
            ],
            "",
            false,
        ),
    )
    .unwrap();
    std::fs::write(
        db.join("s2.mzfeatf"),
        peaks_bytes(
            &[(150.0002, 1.51, 850.0), (250.0004, 3.49, 1550.0)],
            "",
            false,
        ),
    )
    .unwrap();

    let v = run_dir_job(&dir, da, db);
    assert_eq!(v.get("format").unwrap().str(), Some("comparison/1"));
    assert_eq!(v.get("type").unwrap().str(), Some("peaks"));
    let p = &v.get("pairs").unwrap().arr()[0];
    let num = |k: &str| p.get(k).unwrap().f64().unwrap();
    assert_eq!(
        (
            num("count_a"),
            num("count_b"),
            num("both"),
            num("only_a"),
            num("only_b")
        ),
        (5.0, 5.0, 4.0, 1.0, 1.0)
    );
    assert_eq!(num("mean_overlap"), 0.75);
    assert!(num("mean_rank_corr").is_finite());
    let pf = p.get("per_file").unwrap().arr();
    assert_eq!(pf.len(), 2);
    assert_eq!(pf[0].get("file").unwrap().str(), Some("s1"));
    assert_eq!(pf[1].get("file").unwrap().str(), Some("s2"));
    assert_eq!(pf[0].get("overlap").unwrap().f64(), Some(0.5));
    assert_eq!(pf[1].get("overlap").unwrap().f64(), Some(1.0));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tight_tolerances_and_unit_resolution_da_tables() {
    let a = table_bytes(
        &[
            (200.0, 2.0, 1000.0, false),
            (300.0, 4.0, 5000.0, false),
            (400.0, 6.0, 2000.0, false),
        ],
        false,
    );
    let b = table_bytes(
        &[
            (200.0004, 2.02, 1200.0, false),
            (300.0006, 4.01, 4800.0, false),
            (500.0, 8.0, 3000.0, false),
        ],
        false,
    );
    let tight = crate::flow_compare::Tol {
        ppm: 1.0,
        da: 0.3,
        rt: 0.01,
    };
    let ga: &[(&str, &[u8])] = &[("a.mzfeat", &a)];
    let gb: &[(&str, &[u8])] = &[("b.mzfeat", &b)];
    let out =
        crate::flow_compare::compare_groups("features.align@1", &["A", "B"], &[ga, gb], tight)
            .unwrap();
    let v = parse(out.as_bytes()).unwrap();
    let p = &v.get("pairs").unwrap().arr()[0];
    assert_eq!(p.get("both").unwrap().f64(), Some(0.0));
    assert_eq!(p.get("mean_overlap").unwrap().f64(), Some(0.0));
    assert_eq!(p.get("mean_rank_corr"), Some(&Value::Null));

    // Unit-resolution tables honor mz_tolerance_da instead of ppm
    let ua = table_bytes(
        &[(200.0, 2.0, 1000.0, false), (300.0, 4.0, 5000.0, false)],
        true,
    );
    let ub = table_bytes(
        &[(200.2, 2.02, 1200.0, false), (300.2, 4.01, 4800.0, false)],
        true,
    );
    let out_u =
        crate::flow_compare::compare_text("features.align@1", &["A", "B"], &[&ua, &ub]).unwrap();
    assert_eq!(
        parse(out_u.as_bytes()).unwrap().get("pairs").unwrap().arr()[0]
            .get("both")
            .unwrap()
            .f64(),
        Some(2.0)
    );
}

#[test]
fn empty_and_mismatched_table_or_peaks_inputs() {
    // Absent cells in a table have zero area; empty tables yield zero counts and null metrics
    let e = table_bytes(&[], false);
    let mut absent_t = Aligned::with_capacity(1, 1);
    absent_t.mz.push(200.0);
    absent_t.rt.push(2.0);
    absent_t.area.push(999.0);
    absent_t.height.push(999.0);
    absent_t.apex_rt.push(2.0);
    absent_t.width.push(0.1);
    absent_t.state.push(CELL_ABSENT);
    let eb = write_table(&absent_t, &Recipe::default(), &Origin::default());
    let ve = parse(
        crate::flow_compare::compare_text("features.align@1", &["A", "B"], &[&e, &eb])
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    let pe = &ve.get("pairs").unwrap().arr()[0];
    assert_eq!(pe.get("count_a").unwrap().f64(), Some(0.0));
    assert_eq!(pe.get("count_b").unwrap().f64(), Some(1.0));
    assert_eq!(pe.get("mean_overlap").unwrap().f64(), Some(0.0));

    let p1 = peaks_bytes(&[(200.0, 2.0, 1000.0)], "s1.mzML", false);
    let p2 = peaks_bytes(&[(300.0, 4.0, 2000.0)], "s2.mzML", false);
    let pga: &[(&str, &[u8])] = &[("", &p1)];
    let pgb: &[(&str, &[u8])] = &[("", &p1), ("", &p2)];
    let vp = parse(
        crate::flow_compare::compare_groups(
            "features.detect@2",
            &["A", "B"],
            &[pga, pgb],
            crate::flow_compare::Tol::default(),
        )
        .unwrap()
        .as_bytes(),
    )
    .unwrap();
    let pp = &vp.get("pairs").unwrap().arr()[0];
    assert_eq!(
        (
            pp.get("both").unwrap().f64(),
            pp.get("only_a").unwrap().f64(),
            pp.get("only_b").unwrap().f64()
        ),
        (Some(1.0), Some(0.0), Some(1.0))
    );
    let t_one: &[(&str, &[u8])] = &[("a.mzfeat", &eb)];
    let mixed: &[(&str, &[u8])] = &[("p.mzfeatf", &p1)];
    let err = crate::flow_compare::compare_groups(
        "features.align@1",
        &["A", "B"],
        &[t_one, mixed],
        crate::flow_compare::Tol::default(),
    )
    .unwrap_err();
    assert_eq!(err.key, "err.flow.compare.type");
}

#[test]
fn meta_compare_runs_features_detect_and_align_end_to_end() {
    let dir = std::env::temp_dir().join(format!("mzlab_cmp_flow_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("work")).unwrap();
    for (i, name) in ["a.mzML", "b.mzML"].iter().enumerate() {
        std::fs::write(
            dir.join(name),
            crate::flow_bentazone_tests::mzml(i, 1, true),
        )
        .unwrap();
    }
    let flow = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzML","type":"raw"},{"name":"b.mzML","type":"raw"}],"steps":[
        {"id":"idx","step":"index.mzidx@1","in":{"source":["file:a.mzML","file:b.mzML"]}},
        {"id":"cmp_det","step":"meta.compare@1","in":{"index":["idx.index"]},"params":{"step":"features.detect@2","variants":[{"algorithm":"masstrace","params":{"min_height":50000}},{"label":"hi","algorithm":"masstrace","params":{"min_height":200000}}]}},
        {"id":"det","step":"features.detect@2","in":{"index":["idx.index"]}},
        {"id":"cmp_aln","step":"meta.compare@1","in":{"peaks":["det.peaks"]},"params":{"step":"features.align@1","mz_tolerance_ppm":5,"rt_tolerance_min":0.2,"variants":[{"algorithm":"groups"},{"algorithm":"join"}]}}
    ]}"#;
    crate::flow::run_plan(flow.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    let cd = parse(&std::fs::read(dir.join("work/cmp_det.json")).unwrap()).unwrap();
    assert_eq!(cd.get("type").unwrap().str(), Some("peaks"));
    assert_eq!(
        cd.get("pairs").unwrap().arr()[0]
            .get("per_file")
            .unwrap()
            .arr()
            .len(),
        2
    );
    let ca = parse(&std::fs::read(dir.join("work/cmp_aln.json")).unwrap()).unwrap();
    assert_eq!(ca.get("type").unwrap().str(), Some("table"));
    assert!(
        ca.get("pairs").unwrap().arr()[0]
            .get("both")
            .unwrap()
            .f64()
            .unwrap()
            >= 1.0
    );
    let _ = std::fs::remove_dir_all(&dir);
}
