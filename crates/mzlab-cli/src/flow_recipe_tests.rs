//! One recipe rule for a unit-resolution series: each file is estimated on its own (`detect_job` does, the browser's `detect_file` does), and the alignment and
//! the gap filling read the floor each file estimated, so `flow run`, the desktop route and the browser runner write the same bytes.

use std::path::PathBuf;

use mzlab_core::idxread::Index;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let (a, b) = (self.next().max(1e-12), self.next());
        (-2.0 * a.ln()).sqrt() * (2.0 * std::f64::consts::PI * b).cos()
    }
}

/// Ions of the series: (m/z, apex RT, apex height).
const IONS: [(f64, f64, f64); 4] = [
    (301.10, 6.0, 2.0e5),
    (257.20, 7.0, 6.0e4),
    (233.15, 8.0, 2.5e4),
    (187.05, 9.0, 1.6e4),
];

/// A profile file (0.1 Da grid, calibration shift +0.3 Da) whose baseline noise has the scale `noise`, so the noise lump, and the floor estimated from it, differ
/// between files.
fn profile_mzml(seed: u64, noise: f64, weak: f64) -> Vec<u8> {
    let mut rng = Lcg(seed);
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: format!("p{seed}"),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 240;
    w.open_spectra(n).unwrap();
    let sigma = 0.7 / 2.3548;
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let (mut mz, mut y) = (Vec::new(), Vec::new());
        for g in 1000..4000 {
            let m = g as f64 / 10.0;
            let mut v = noise * (0.35 * rng.normal()).exp();
            for (j, &(c, r0, a)) in IONS.iter().enumerate() {
                // the last two ions are weak in the file `weak` scales: below the minimum height there, so their cells are gaps to fill
                let a = if j >= 2 { a * weak } else { a };
                let h = a * (-0.5 * ((rt - r0) / 0.07).powi(2)).exp();
                v += h * (-0.5 * ((m - (c + 0.3)) / sigma).powi(2)).exp();
            }
            mz.push(m);
            y.push(v as f32);
        }
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: true,
            filter: "+ p ESI Q1MS [100.000-400.000]".into(),
            precursor: None,
            mz,
            intensity: y,
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"p0.mzML","type":"raw"},{"name":"p1.mzML","type":"raw"},{"name":"p2.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:p0.mzML","file:p1.mzML","file:p2.mzML"]}},
{"id":"det","step":"features.detect@2","in":{"index":["idx.index"]}},
{"id":"ali","step":"features.align@1","in":{"peaks":["det.peaks"]}},
{"id":"gap","step":"features.gapfill@1","in":{"table":"ali.table","index":["idx.index"]}}]}"#;

/// The layers of the browser runner for the plan above with `centroid` off (the profile points are kept, so the floor of each file is its noise lump).
const LAYERS: &str = r#"[{"step":"features.detect","params":{"centroid":"off"}}]"#;

/// Noise scale of each file, and the share the weak ions keep there.
const SERIES: [(f64, f64); 3] = [(1500.0, 1.0), (600.0, 0.2), (150.0, 0.1)];

fn fixture(tag: &str) -> (PathBuf, Vec<(String, Vec<u8>)>) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_rec_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let files: Vec<(String, Vec<u8>)> = SERIES
        .iter()
        .enumerate()
        .map(|(k, &(noise, weak))| {
            (
                format!("p{k}.mzML"),
                profile_mzml(10 + k as u64, noise, weak),
            )
        })
        .collect();
    for (n, b) in &files {
        std::fs::write(dir.join("data").join(n), b).unwrap();
    }
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    (dir, files)
}

/// The gap-filled table of the browser runner's chain over the same files, as `mzlab_feat_wasm` builds it, with the floors of the files as they are
/// (`own`) or the one of the first file for all (`!own`, the rule before this one).
fn page_table(files: &[(String, Vec<u8>)], own: bool) -> Vec<u8> {
    use mzlab_core::mzidx::build_mzml_to_vec;
    use mzlab_core::upload::prepare;
    use mzlab_ops::run::feat::run;
    let idxs: Vec<Vec<u8>> = files
        .iter()
        .map(|(name, b)| {
            let mut src = b.clone();
            let p = prepare(&mut src, b.len() as u64, name).unwrap();
            build_mzml_to_vec(&p.table, b, 64 << 20).unwrap().0
        })
        .collect();
    let open = |k: usize| Index::open(idxs[k].clone(), idxs[k].len() as u64);
    let text = format!("{{\"layers\":{LAYERS}}}");
    let recipe_of = |k: usize| run::recipe_for(&text, &mut open(k).unwrap()).unwrap();
    let det: Vec<Vec<u8>> = files
        .iter()
        .enumerate()
        .map(|(k, (n, _))| {
            let mut rk = recipe_of(k);
            rk.fill_gaps = false;
            run::detect(&mut open(k).unwrap(), n, &rk).unwrap()
        })
        .collect();
    let mut r = recipe_of(0);
    if own {
        r.screen
            .set_file_floors((0..files.len()).map(|k| recipe_of(k).fill.floor).collect());
        assert!(
            !r.screen.file_floors.is_empty(),
            "the files estimate other floors"
        );
    }
    let (tables, origin) = run::read_files(&det).unwrap();
    run::align(&tables, &origin, &r, open).unwrap()
}

fn run_flow(dir: &std::path::Path, tag: &str) -> PathBuf {
    use crate::flow_run_tests::{flow_run, p};
    let out = dir.join(tag);
    let (plan, data) = (dir.join("plan.json"), dir.join("data"));
    let a = vec![
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        "--set",
        "det.centroid=off",
    ];
    let o = flow_run(&a);
    assert_eq!(o.code, 0, "{tag}: {}", o.stderr);
    out
}

#[test]
fn a_series_of_files_with_other_floors_gives_the_same_bytes_in_flow_run_the_app_and_the_browser_runner(
) {
    let (dir, files) = fixture("parity");
    let cli = run_flow(&dir, "cli");
    let table = std::fs::read(cli.join("gap.mzfeat")).unwrap();
    assert!(
        table == page_table(&files, true),
        "the browser runner's table differs from `flow run`"
    );
    // the files estimate other floors, and that changes the filled cells: the rule is not a no-op
    assert!(
        table != page_table(&files, false),
        "one floor for the series gives the same bytes: the fixture does not tell the rules apart"
    );
    // the desktop route runs the plan the CLI wrote
    let desk = dir.join("desk");
    std::fs::create_dir_all(&desk).unwrap();
    let as_run = std::fs::read(cli.join("plan.json")).unwrap();
    crate::flow::run_plan(&as_run, &dir.join("data"), &desk, 1, None).unwrap();
    assert_eq!(
        std::fs::read(desk.join("gap.mzfeat")).unwrap(),
        table,
        "desktop route"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_record_lists_the_floor_each_file_estimated() {
    let (dir, _) = fixture("record");
    let cli = run_flow(&dir, "cli");
    let prov = std::fs::read_to_string(cli.join("prov.json")).unwrap();
    assert!(
        prov.contains("profile floor of each file of the series"),
        "{prov}"
    );
    assert!(prov.contains("\"files\":["), "{prov}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_browser_record_lists_the_same_floors_as_flow_run() {
    use mzlab_core::step::json::registry_json;
    use mzlab_ops::run::feat::run::meta::{job_lineage_text, job_meta_series, meta_json};
    let (dir, _) = fixture("meta");
    let cli = run_flow(&dir, "cli");
    let prov = std::fs::read_to_string(cli.join("prov.json")).unwrap();
    let registry = registry_json(&crate::flow::steps());
    let reg = mzlab_core::step::registry_from_value(
        &mzlab_core::json::parse(registry.as_bytes()).unwrap(),
    )
    .unwrap();
    let plan = std::fs::read(cli.join("plan.json")).unwrap();
    let open = |k: usize| {
        let f = std::fs::File::open(cli.join(format!("idx-p{k}.mzidx"))).unwrap();
        let size = f.metadata().unwrap().len();
        Index::open(f, size)
    };
    let (mut first, mut own) = (open(0).unwrap(), open(0).unwrap());
    let meta = job_meta_series(
        &job_lineage_text(&plan, &reg, "gap").unwrap(),
        (&mut first, &mut own),
        (3, open),
        None,
        mzlab_core::budget_auto::native_auto_mb(),
    )
    .unwrap();
    let text = meta_json(&meta);
    let at = text
        .find("\"files\":[")
        .expect("the floors are in the meta");
    let floors = &text[at..text[at..].find(']').unwrap() + at + 1];
    assert!(
        prov.contains(floors),
        "{floors} is not in the record of flow run:\n{prov}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
