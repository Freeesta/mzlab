//! `raw isotopes --figure PATH`: the file is the SVG the route `/api/rawiso?figure=1` answers (so the site, the app and the CLI give the same bytes), its
//! path and sha256 are in the answer, nothing changes without the flag, and a path that cannot be written is exit 6.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::tests::{file, json, WIN};
use crate::extract::tests::{page_routes, run};
use mzlab_core::budget::sha256_hex;
use mzlab_core::routes::Query;

fn base(f: &str) -> Vec<&str> {
    vec![
        "raw",
        "isotopes",
        f,
        "--mz",
        "181.07",
        "--rt",
        WIN,
        "--formula",
        "C6H12O6",
    ]
}

#[test]
fn the_figure_file_is_the_route_svg_and_its_hash_is_in_the_answer() {
    let (d, f) = file("isofig", 100.0, 7.0, 1.0);
    let svg = d.join("iso.svg");
    let mut a = base(&f);
    let p = svg.to_string_lossy().into_owned();
    a.extend(["--figure", &p]);
    let o = run(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let written = std::fs::read_to_string(&svg).unwrap();
    let route = page_routes(&f)
        .raw_isotopes_route(&Query::parse(
            "mz=181.07&rt=2.0,2.4&formula=C6H12O6&figure=1",
        ))
        .unwrap()
        .unwrap();
    assert_eq!(written, route, "the file is the route's bytes");
    let v = json(&o);
    let fig = &v.get("figures").unwrap().arr()[0];
    assert_eq!(fig.get("path").unwrap().str(), Some(p.as_str()));
    assert_eq!(
        fig.get("sha256").unwrap().str(),
        Some(sha256_hex(written.as_bytes()).as_str())
    );
    assert_eq!(fig.get("bytes").unwrap().f64(), Some(written.len() as f64));
    assert_eq!(
        v.get("query").unwrap().get("figure").unwrap().str(),
        Some(p.as_str())
    );
    // the numbers are the ones of the run without the flag
    let plain = json(&run(&base(&f)));
    assert_eq!(v.get("results"), plain.get("results"));
    assert!(plain.get("figures").is_none() && plain.get("query").unwrap().get("figure").is_none());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn several_results_number_their_files_and_a_bad_path_is_exit_6() {
    let (d, f) = file("isofig2", 100.0, 7.0, 1.0);
    let svg = d.join("two.svg").to_string_lossy().into_owned();
    let mut a = base(&f);
    a.push(&f);
    a.extend(["--figure", &svg]);
    let o = run(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        d.join("two-1.svg").is_file()
            && d.join("two-2.svg").is_file()
            && !d.join("two.svg").exists()
    );
    let v = json(&o);
    assert_eq!(v.get("figures").unwrap().arr().len(), 2);
    assert!(String::from_utf8_lossy(&o.stdout).contains("\"what\":\"figure\""));
    let missing = d.join("none").join("x.svg").to_string_lossy().into_owned();
    let mut a = base(&f);
    a.extend(["--figure", &missing]);
    let o = run(&a);
    assert_eq!(o.code, 6, "{}", o.stderr);
    assert!(o.stderr.contains("err.cli.out"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_dry_run_writes_no_figure() {
    let (d, f) = file("isofig3", 100.0, 7.0, 1.0);
    let svg = d.join("dry.svg").to_string_lossy().into_owned();
    let mut a = base(&f);
    a.extend(["--figure", &svg, "--dry-run"]);
    let o = run(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(!d.join("dry.svg").exists());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_picture_of_the_isotope_traces_and_ratios_finds_raw_isotopes_first() {
    for words in [
        "draw a picture of the M+1 and M+2 traces and ratios against a formula",
        "plot isotope ratios svg figure",
    ] {
        let mut a = vec!["commands".to_string(), "--find".to_string()];
        a.extend(words.split(' ').map(String::from));
        a.push("--json".to_string());
        let o = crate::execute(&a);
        let v = json(&o);
        let first = v.get("matches").unwrap().arr()[0]
            .get("name")
            .unwrap()
            .str()
            .unwrap()
            .to_string();
        assert_eq!(first, "raw isotopes", "{words}");
    }
}
