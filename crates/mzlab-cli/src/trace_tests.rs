//! `mzlab-cli trace chrom|xic`: the points are the number tokens of the page's routes (`api/chrom`, `api/xic`, `api/mrm`), the CSV is what `peak-*` read, and the
//! failures follow the exit-code table (5 for a channel the file does not have).

use std::path::Path;

use mzlab_core::json::parse;

use crate::extract::tests::{
    dir, doc_points, mzml_file, page_routes, route_points, run, synthetic,
};

fn text(o: crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn tic_points_are_the_bytes_of_the_page_route() {
    let (d, f) = mzml_file("trace_tic");
    let doc = text(run(&["trace", "chrom", &f, "--id", "tic"]));
    assert!(
        doc.starts_with("{\"schema\":\"trace/1\",\"id\":\"tic\","),
        "{doc}"
    );
    assert!(doc.ends_with("}\n"));
    let v = parse(doc.trim().as_bytes()).unwrap();
    assert_eq!(v.get("x_unit").unwrap().str(), Some("min"));
    assert_eq!(v.get("y_unit").unwrap().str(), Some("counts"));
    let mut r = page_routes(&f);
    let q = mzlab_core::routes::Query::parse("canale=tic&kind=tic");
    let want = r.handle("/api/chrom", &q).unwrap().unwrap();
    let got = doc_points(&doc);
    assert_eq!(got, route_points(&want, "rt"));
    assert_eq!(got.len(), 120);
    assert_eq!(v.get("n_points").unwrap().f64(), Some(120.0));
    assert!(doc.contains("\"route\":\"/api/chrom\""), "{doc}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn csv_has_the_header_the_peak_commands_read() {
    let (d, f) = mzml_file("trace_csv");
    let csv = text(run(&[
        "trace", "chrom", &f, "--id", "tic", "--format", "csv",
    ]));
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some("rt_min,intensity"));
    assert_eq!(lines.count(), 120);
    let out = d.join("t.csv");
    std::fs::write(&out, &csv).unwrap();
    let o = run(&["peak", "auto-lr", &out.to_string_lossy()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(
        run(&["trace", "chrom", &f, "--id", "tic", "--format", "xml"]).code,
        2
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_window_keeps_the_points_inside_it() {
    let (d, f) = mzml_file("trace_win");
    let doc = text(run(&["trace", "chrom", &f, "--id", "bpc", "--rt", "1,2"]));
    let pts = doc_points(&doc);
    assert!(
        !pts.is_empty()
            && pts
                .iter()
                .all(|(x, _)| (1.0..=2.0).contains(&x.parse::<f64>().unwrap()))
    );
    assert!(doc.contains("\"rt_window_min\":[1,2]"), "{doc}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn xic_is_the_page_route_and_names_its_tolerance() {
    let (d, f) = mzml_file("trace_xic");
    let mut r = page_routes(&f);
    let doc = text(run(&["trace", "xic", &f, "--mz", "241.3", "--da", "0.35"]));
    assert_eq!(
        doc_points(&doc),
        route_points(&r.xic(241.3, 0.35, 1).unwrap(), "rt")
    );
    assert!(
        doc.contains("\"tolerance_da\":0.35") && doc.contains("\"scans_in_section\":120"),
        "{doc}"
    );
    let apex = doc_points(&doc)
        .iter()
        .map(|p| p.1.parse::<f64>().unwrap())
        .fold(0.0, f64::max);
    assert!(apex > 1e6, "{apex}");
    // ppm: the half-window of the same ion in Da
    let ppm = text(run(&["trace", "xic", &f, "--mz", "241.3", "--ppm", "500"]));
    assert_eq!(
        doc_points(&ppm),
        route_points(&r.xic(241.3, 241.3 * 500.0 * 1e-6, 1).unwrap(), "rt")
    );
    assert!(ppm.contains("\"tolerance_ppm\":500"), "{ppm}");
    // a level the file has no scans of: an empty trace with its evidence, not an error
    let none = text(run(&[
        "trace",
        "xic",
        &f,
        "--mz",
        "241.3",
        "--da",
        "0.3",
        "--ms-level",
        "2",
    ]));
    assert!(
        none.contains("\"n_points\":0") && none.contains("\"scans_in_section\":0"),
        "{none}"
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn bad_requests_are_usage_errors_and_a_missing_channel_is_exit_5() {
    let (d, f) = mzml_file("trace_err");
    for a in [
        vec!["trace", "chrom", &f],
        vec!["trace", "chrom", "--id", "tic"],
        vec!["trace", "xic", &f, "--mz", "241.3"],
        vec![
            "trace", "xic", &f, "--mz", "241.3", "--ppm", "5", "--da", "0.1",
        ],
        vec!["trace", "xic", &f, "--da", "0.1"],
        vec![
            "trace",
            "xic",
            &f,
            "--mz",
            "241.3",
            "--da",
            "0.1",
            "--polarity",
            "x",
        ],
        vec!["trace", "chrom", &f, "--id", "tic", "--sample", "1"],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    let o = run(&["trace", "chrom", &f, "--id", "srm:1>2", "--json"]);
    assert_eq!(o.code, 5);
    let v = parse(&o.stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("error/1"));
    assert!(o.stderr.contains("srm:1>2"), "{}", o.stderr);
    assert_eq!(
        run(&["trace", "chrom", "/nonexistent/x.mzML", "--id", "tic"]).code,
        4
    );
    assert_eq!(run(&["trace", "chrom", "--help"]).code, 0);
    assert_eq!(run(&["trace", "xic", "--help"]).code, 0);
    let _ = std::fs::remove_dir_all(d);
}

/// The transitions of an MRM file as `api/mrm` gives them, and the same trace through `trace chrom`, then through `peak auto-lr`.
#[test]
fn an_srm_trace_is_the_mrm_route_and_feeds_peak_auto_lr() {
    let Some(d) = synthetic() else { return };
    let f = d.join("B_MRM.mzML");
    let file = std::fs::File::open(&f).unwrap();
    let size = file.metadata().unwrap().len();
    let mut m = mzlab_core::mzmlhead::Mzml::new(file, size);
    let chroms = m.chromatograms(0).unwrap();
    let mrm = mzlab_core::fileroutes::mrm_transitions(&mut m, &chroms).unwrap();
    let p = f.to_string_lossy().into_owned();
    let id = "srm:364.1>194.1@20";
    let doc = text(run(&["trace", "chrom", &p, "--id", id]));
    assert_eq!(doc_points(&doc), route_points(&mrm, "rt"));
    assert!(
        doc.contains("\"source\":\"mzml:chromatogram\"") && doc.contains("\"family\":\"srm\""),
        "{doc}"
    );
    let csv = text(run(&["trace", "chrom", &p, "--id", id, "--format", "csv"]));
    let t = d.join("srm.csv");
    std::fs::write(&t, &csv).unwrap();
    let o = run(&["peak", "auto-lr", &t.to_string_lossy()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(run(&["trace", "chrom", &p, "--id", "srm:999>1@20"]).code, 5);
    // the TIC the maker wrote is a chromatogram of its own
    let tic = text(run(&["trace", "chrom", &p, "--id", "tic"]));
    assert!(
        tic.contains("\"family\":\"tic\"") && !doc_points(&tic).is_empty(),
        "{tic}"
    );
    let _ = std::fs::remove_dir_all(d);
}

/// `uv:auto` answers the XWC band of `itemroutes::pda_propose` (what `api/uvpropose` answers), with and without `--rt`, under `uvband/1`.
#[test]
fn uv_auto_gives_the_band_the_engine_proposes() {
    let f = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi/FullScan_t10.mzML");
    let p = f.to_string_lossy().into_owned();
    let engine = |a: f64, b: f64| {
        let file = std::fs::File::open(&f).unwrap();
        let size = file.metadata().unwrap().len();
        mzlab_core::itemroutes::pda_propose(&mut mzlab_core::mzmlhead::Mzml::new(file, size), a, b)
            .unwrap()
    };
    for (extra, want) in [
        (vec![], engine(f64::NAN, f64::NAN)),
        (vec!["--rt", "1,2"], engine(1.0, 2.0)),
    ] {
        let mut a = vec!["trace", "chrom", &p, "--id", "uv:auto"];
        a.extend(&extra);
        let got = text(run(&a));
        assert!(
            got.starts_with("{\"schema\":\"uvband/1\",\"id\":\"uv:auto\","),
            "{got}"
        );
        let band = &want[1..want.len() - 1];
        assert!(got.contains(band), "{extra:?}: {got} vs {want}");
        assert_eq!(
            parse(got.trim().as_bytes())
                .unwrap()
                .get("bw")
                .and_then(|x| x.f64()),
            Some(4.0),
            "{got}"
        );
    }
}

fn b64(v: &[f64]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    let mut o = String::new();
    for c in raw.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            o.push(if k <= c.len() {
                A[(n >> (18 - 6 * k) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    o
}

/// One `<chromatogram>` of two points: `acc` is its kind, `unit` the attribute of its intensity array.
fn chrom(i: usize, id: &str, acc: &str, unit: &str, y: [f64; 2]) -> String {
    let arr = |acc: &str, unit: &str, v: &str| {
        format!("<binaryDataArray encodedLength=\"1\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"{acc}\" name=\"x\"{unit}/><binary>{v}</binary></binaryDataArray>")
    };
    format!("<chromatogram index=\"{i}\" id=\"{id}\" defaultArrayLength=\"2\"><cvParam accession=\"{acc}\" name=\"x\"/><binaryDataArrayList count=\"2\">{}{}</binaryDataArrayList></chromatogram>", arr("MS:1000595", "", &b64(&[0.0, 0.03])), arr("MS:1000515", unit, &b64(&y)))
}

/// An mzML with a TWC and a pump pressure trace: `trace chrom` reads them as the old routes do.
#[test]
fn uv_and_analog_traces_come_from_the_chromatograms() {
    let dir = dir("trace_uv");
    let xml = format!(
        "<mzML><run><chromatogramList count=\"2\">{}{}</chromatogramList></run></mzML>",
        chrom(0, "TWC", "MS:1000813", "", [1.0, 2.0]),
        chrom(
            1,
            "Pump Pressure",
            "MS:1003019",
            " unitName=\"bar\"",
            [1.0, 2.0]
        )
    );
    let f = dir.join("u.mzML");
    std::fs::write(&f, xml).unwrap();
    let p = f.to_string_lossy().into_owned();
    let twc = text(run(&["trace", "chrom", &p, "--id", "uv:twc"]));
    assert_eq!(
        doc_points(&twc),
        [
            ("0".to_string(), "1".to_string()),
            ("0.03".to_string(), "2".to_string())
        ]
    );
    assert!(twc.contains("\"y_unit\":\"mAU\""), "{twc}");
    let an = text(run(&["trace", "chrom", &p, "--id", "an:1", "--n", "50"]));
    assert!(
        an.contains("\"y_unit\":\"bar\"") && an.contains("\"max_points_per_extreme\":50"),
        "{an}"
    );
    // this file has no TIC
    assert_eq!(run(&["trace", "chrom", &p, "--id", "tic"]).code, 5);
    let _ = std::fs::remove_dir_all(dir);
}

/// A file whose only chromatograms are transitions has no TIC of its own: `tic` is their sum by time.
#[test]
fn the_tic_of_a_file_of_transitions_is_their_sum() {
    let d = dir("trace_sum");
    let xml = format!(
        "<mzML><run><chromatogramList count=\"2\">{}{}</chromatogramList></run></mzML>",
        chrom(
            0,
            "SRM SIC Q1=364.1 Q3=194.1 CE=20 sample=1",
            "MS:1001473",
            "",
            [1.0, 2.0]
        ),
        chrom(
            1,
            "SRM SIC Q1=364.1 Q3=152.1 CE=20 sample=1",
            "MS:1001473",
            "",
            [10.0, 20.0]
        ),
    );
    let f = d.join("s.mzML");
    std::fs::write(&f, xml).unwrap();
    let p = f.to_string_lossy().into_owned();
    let tic = text(run(&["trace", "chrom", &p, "--id", "tic"]));
    assert_eq!(
        doc_points(&tic),
        [
            ("0".to_string(), "11".to_string()),
            ("0.03".to_string(), "22".to_string())
        ],
        "{tic}"
    );
    assert!(tic.contains("\"source\":\"sum:srm\""), "{tic}");
    let one = text(run(&["trace", "chrom", &p, "--id", "srm:364.1>152.1"]));
    assert_eq!(
        doc_points(&one),
        [
            ("0".to_string(), "10".to_string()),
            ("0.03".to_string(), "20".to_string())
        ],
        "{one}"
    );
    let _ = std::fs::remove_dir_all(d);
}
