//! `report.methods@1` on a fixed `corsa/1` record: the Markdown is the golden file byte for byte, in both styles.

use super::{methods_text, placeholders, template, templated};
use mzlab_core::json::Value;
use std::path::Path;

fn golden(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/golden/feat")
            .join(name),
    )
    .unwrap()
}

fn style(s: &str) -> Vec<(String, Value)> {
    vec![("style".to_string(), Value::Str(s.to_string()))]
}

fn check(style_name: &str, file: &str) {
    let text = methods_text(&style(style_name), &golden("methods-corsa.json")).unwrap();
    if std::env::var("MZLAB_UPDATE_GOLDEN").is_ok() {
        std::fs::write(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/golden/feat")
                .join(file),
            &text,
        )
        .unwrap();
    }
    assert_eq!(
        text.as_bytes(),
        golden(file).as_slice(),
        "{style_name}\n{text}"
    );
}

#[test]
fn the_paragraph_is_the_golden_file() {
    check("paragraph", "methods-paragraph.md");
}

#[test]
fn the_list_is_the_golden_file() {
    check("list", "methods-list.md");
}

#[test]
fn the_default_style_is_the_paragraph() {
    let run = golden("methods-corsa.json");
    assert_eq!(
        methods_text(&[], &run).unwrap(),
        methods_text(&style("paragraph"), &run).unwrap()
    );
}

#[test]
fn a_text_that_is_not_a_run_record_is_refused() {
    assert!(methods_text(&[], b"{\"schema\":\"prov/1\"}").is_err());
    assert!(methods_text(&[], b"nonsense").is_err());
}

#[test]
fn the_algorithm_selects_the_sentence_and_a_step_without_template_gets_the_generic_one() {
    assert!(template("features.detect", "centwave@1")
        .unwrap()
        .contains("centWave"));
    assert!(template("features.detect", "masstrace")
        .unwrap()
        .contains("mass traces"));
    assert!(template("features.detect", "other").is_some());
    assert!(template("vendor.custom", "").is_none());
}

#[test]
fn every_placeholder_of_a_template_is_a_word_of_the_table() {
    for (step, alg) in templated() {
        let t = template(step, alg).unwrap();
        assert!(t.ends_with('.'), "{step}: {t}");
        for p in placeholders(t) {
            assert!(
                !p.is_empty()
                    && p.chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                "{step}: {p}"
            );
        }
    }
}

/// The isotope grouper of mzmine adds its sentence to the one of the detection; the averagine default adds nothing.
#[test]
fn the_isotope_grouper_adds_its_sentence_to_the_detection() {
    let run = |iso: &str| {
        let record = format!(
            r#"{{"schema":"corsa/1","steps":[{{"instance":"det","step":"features.detect@1","algorithm":"masstrace@1","status":"ok","params":[
{{"name":"floor","value":20000,"state":"fixed"}},{{"name":"isotopes","value":"{iso}","state":"fixed"}},{{"name":"iso_ppm","value":5,"state":"fixed"}},
{{"name":"iso_abs_da","value":0.0015,"state":"fixed"}},{{"name":"iso_rt_tol","value":0.05,"state":"fixed"}},{{"name":"iso_max_charge","value":2,"state":"fixed"}},
{{"name":"iso_representative","value":"most_intense","state":"fixed"}}]}}]}}"#
        );
        methods_text(&style("paragraph"), record.as_bytes()).unwrap()
    };
    let grouper = run("grouper");
    assert!(
        grouper.contains("isotope grouper of mzmine 3 (m/z tolerance 5 ppm or 0.0015 Da, RT tolerance 0.05 min, charges up to 2, group representative most_intense)"),
        "{grouper}"
    );
    assert!(!run("averagine").contains("isotope grouper"));
}

#[test]
fn the_isotope_finder_adds_its_sentence_to_the_detection() {
    let run = |iso: &str| {
        let record = format!(
            r#"{{"schema":"corsa/1","steps":[{{"instance":"det","step":"features.detect@1","algorithm":"masstrace@1","status":"ok","params":[
{{"name":"floor","value":20000,"state":"fixed"}},{{"name":"isotopes","value":"{iso}","state":"fixed"}},{{"name":"finder_ppm","value":15,"state":"fixed"}},
{{"name":"finder_abs_da","value":0.005,"state":"fixed"}},{{"name":"finder_max_charge","value":3,"state":"fixed"}},{{"name":"finder_elements","value":"H,C,N,O,S","state":"fixed"}},
{{"name":"finder_require_c13","value":true,"state":"fixed"}},{{"name":"finder_auto","value":false,"state":"fixed"}}]}}]}}"#
        );
        methods_text(&style("paragraph"), record.as_bytes()).unwrap()
    };
    let finder = run("finder");
    assert!(
        finder.contains("isotope finder of mzmine 4 on the apex MS1 spectrum of each feature (m/z tolerance 15 ppm or 0.005 Da, charges up to 3, elements H,C,N,O,S, 13C peak requirement on, heavy elements from the spectrum off)"),
        "{finder}"
    );
    assert!(!finder.contains("isotope grouper"));
    assert!(!run("averagine").contains("isotope finder"));
}

#[test]
fn the_window_area_adds_its_sentence_to_the_detection() {
    let run = |from: &str| {
        let record = format!(
            r#"{{"schema":"corsa/1","steps":[{{"instance":"det","step":"features.detect@1","algorithm":"masstrace@1","status":"ok","params":[
{{"name":"floor","value":20000,"state":"fixed"}},{{"name":"area_from","value":"{from}","state":"fixed"}}]}}]}}"#
        );
        methods_text(&style("paragraph"), record.as_bytes()).unwrap()
    };
    assert!(
        run("window").contains("Feature areas were summed over every point within the m/z window")
    );
    let channel = run("channel");
    assert!(
        channel.contains("Feature areas were summed over the points of the trace of the m/z channel"),
        "{channel}"
    );
    assert!(!channel.contains("every point within the m/z window"));
    let xic = run("xic");
    assert!(
        xic.contains("integrated by the trapezoidal rule on the real retention times"),
        "{xic}"
    );
    assert!(!xic.contains("Feature areas were summed"));
}

#[test]
fn the_wide_stitch_adds_its_sentence_to_the_detection() {
    let run = |how: &str| {
        let record = format!(
            r#"{{"schema":"corsa/1","steps":[{{"instance":"det","step":"features.detect@1","algorithm":"masstrace@1","status":"ok","params":[
{{"name":"floor","value":20000,"state":"fixed"}},{{"name":"stitch","value":"{how}","state":"fixed"}}]}}]}}"#
        );
        methods_text(&style("paragraph"), record.as_bytes()).unwrap()
    };
    assert!(run("wide").contains("Pieces of a trace in neighbouring m/z channels were joined"));
    let strict = run("strict");
    assert!(strict.contains("were joined when they overlapped by half of the shorter one"), "{strict}");
    assert!(!strict.contains("whatever their height"));
}

/// `max_gap` of the mass traces is named as the missed scans it counts, as in `features.gapfill`.
#[test]
fn the_mass_trace_sentence_names_the_missed_scans() {
    let record = r#"{"schema":"corsa/1","steps":[{"instance":"det","step":"features.detect@2","algorithm":"masstrace@1","status":"ok","params":[
{"name":"floor","value":20000,"state":"fixed"},{"name":"min_points","value":5,"state":"fixed"},{"name":"max_gap","value":1,"state":"fixed"}]}]}"#;
    let text = methods_text(&style("paragraph"), record.as_bytes()).unwrap();
    assert!(
        text.contains("minimum points 5, maximum missed scans between two points of a trace 1)"),
        "{text}"
    );
    assert!(!text.contains("maximum gap"), "{text}");
}
