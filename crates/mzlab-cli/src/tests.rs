//! `mzlab-cli features` writes the same table as the page: the CSV of the CLI on three synthetic mzML files is compared, character by character, with the one
//! the pipeline of the app gives on indexes built in memory (`run::recipe_for` -> `run::detect` -> `run::align` -> the export "full" of the table). Synthetic data only.

use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_core::idxread::Index;
use mzlab_core::json::Out;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::store::Slice;
use mzlab_feat_table_wasm::core::{samples_json, table_file};
use mzlab_feat_wasm::run;

/// An mzML of Q1 scans every 3 s: a flat noise floor and two ions, one growing with the treatment time `t`.
fn mzml(t: f64) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: format!("t{t}"),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 120;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let g = |m: f64, c: f64, a: f64| (m, (a * (-0.5 * ((rt - c) / 0.07).powi(2)).exp()) as f32);
        let mut peaks = vec![
            g(241.3, 3.0, 3.0e6),
            g(257.3, 4.2, 1.0e5 * t),
            (150.0 + (i % 7) as f64, 2e3),
        ];
        peaks.retain(|p| p.1 > 100.0);
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment=1", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: "+ p ESI Q1MS [100.000-400.000]".into(),
            precursor: None,
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The CSV of the app on `files` (name, mzML bytes): the chain of the page, indexes in memory.
fn page_csv(files: &[(String, Vec<u8>)], params: &str) -> String {
    let idxs: Vec<Vec<u8>> = files
        .iter()
        .map(|(name, b)| {
            let mut src = b.clone();
            let p = prepare(&mut src, b.len() as u64, name).unwrap();
            build_mzml_to_vec(&p.table, b, 64 << 20).unwrap().0
        })
        .collect();
    let open = |k: usize| Index::open(idxs[k].clone(), idxs[k].len() as u64);
    let r = run::recipe_for(params, &mut open(0).unwrap()).unwrap();
    let det: Vec<Vec<u8>> = files
        .iter()
        .enumerate()
        .map(|(k, (n, _))| run::detect(&mut open(k).unwrap(), n, &r).unwrap())
        .collect();
    let (tables, origin) = run::read_files(&det).unwrap();
    let table = run::align(&tables, &origin, &r, open).unwrap();
    let mut list = Out::default();
    list.raw("{\"files\":[");
    for (k, (n, s)) in origin.inputs.iter().enumerate() {
        list.raw(if k > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(n)
            .raw(",\"sha256\":")
            .str(s)
            .raw("}");
    }
    list.raw("]}");
    let samples = samples_json(&list.0).unwrap();
    table_file(
        &mut Slice(&table),
        table.len() as u64,
        &samples,
        "",
        &sha256_hex(&table),
        1,
        "full",
    )
    .unwrap()
}

/// The command of `main.rs`, run in this process (a binary built for the test would be a second build of the engine with another panic profile).
fn cli(args: &[&str]) -> crate::Outcome {
    let a: Vec<String> = args.iter().map(|x| x.to_string()).collect();
    crate::execute(&a)
}

#[test]
fn the_csv_of_the_cli_is_the_one_of_the_page() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let files: Vec<(String, Vec<u8>)> = [0.0, 10.0, 30.0]
        .iter()
        .map(|&t| (format!("std_t{:03}min.mzML", t as u32), mzml(t)))
        .collect();
    let paths: Vec<String> = files
        .iter()
        .map(|(n, b)| {
            let p = dir.join(n);
            std::fs::write(&p, b).unwrap();
            p.to_string_lossy().into_owned()
        })
        .collect();
    let mut args: Vec<&str> = vec!["features"];
    args.extend(paths.iter().map(String::as_str));

    // default recipe, to a folder
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let mut a = args.clone();
    a.extend(["--out", out.to_str().unwrap()]);
    let o = cli(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let got = std::fs::read_to_string(out.join("features.csv")).unwrap();
    let want = page_csv(&files, "");
    assert_eq!(got, want, "default recipe");
    let lines: Vec<&str> = got.lines().collect();
    assert!(
        lines[0].starts_with("id,mz,rt,rt_range:min,rt_range:max,charge,datafile:std_t000min:area"),
        "{}",
        lines[0]
    );
    assert!(
        !lines[0].to_lowercase().split(',').any(|c| c == "formula"),
        "no bare formula column"
    );
    assert!(lines.len() > 2, "the ions are rows of the table");

    // the numbers of the panel from a JSON file, to a .csv file, standard output untouched
    let params = r#"{"min_height": 5e4, "fill": false}"#;
    let pj = dir.join("params.json");
    std::fs::write(&pj, params).unwrap();
    let csv = dir.join("series.csv");
    let mut a = args.clone();
    a.extend([
        "--params",
        pj.to_str().unwrap(),
        "--out",
        csv.to_str().unwrap(),
    ]);
    let o = cli(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stdout.is_empty());
    assert_eq!(
        std::fs::read_to_string(&csv).unwrap(),
        page_csv(&files, params),
        "params file"
    );

    // a polarity preset is the recipe `{"polarity": -1}`: no negative scan, no row
    let mut a = args.clone();
    a.extend(["--preset", "neg"]);
    let o = cli(&a);
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        page_csv(&files, "{\"polarity\":-1}")
    );

    // --help and the usage errors
    assert!(String::from_utf8_lossy(&cli(&["features", "--help"]).stdout).contains("--preset"));
    assert_eq!(cli(&["features"]).code, 2);
    assert_eq!(
        cli(&["features", "--preset", "x", paths[0].as_str()]).code,
        2
    );
    let _ = std::fs::remove_dir_all(Path::new(&dir));
}
