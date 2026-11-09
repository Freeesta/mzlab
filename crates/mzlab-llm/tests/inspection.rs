//! `ispezione/1` on the synthetic files of `tools/dati_sintetici.py` (skipped without python3 and numpy; `MZLAB_GOLDEN_STRICT=1` fails instead): the sources
//! are made here with the engine of `mzlab-core` the way the page and the CLI make them, the kind of each file is the one the generator built it as
//! (table G01-G09 of `docs/studi/assistente-dati/06-valutazione.md`), the QC series gives its drift, no private field goes out and the same input gives
//! the same bytes. `MZLAB_UPDATE_LLM=1 cargo test -p mzlab-llm` rewrites the golden file.

mod ispez;

use ispez::*;
use mzlab_core::json::Out;
use mzlab_llm::inspect::{build, build_text, MAX_STR};
use std::path::Path;

#[test]
fn every_kind_of_file_is_named_as_it_was_made() {
    let Some(dir) = synthetic() else { return };
    let out = build(&sources(dir, &KINDS, &|_| String::new(), false), 30000).unwrap();
    let f = files_of(&out);
    let want: [(&str, &str, &str, &[&str]); 11] = [
        ("G01", "positive", "unit", &["full_scan"]),
        ("G02", "negative", "unit", &["full_scan"]),
        ("G03", "alternating", "unit", &["full_scan"]),
        ("G04", "positive", "unit", &["full_scan", "dda"]),
        ("G05", "positive", "unit", &["product_ion", "mrm"]),
        ("G06", "positive", "unknown", &["mrm"]),
        ("G07", "positive", "unit", &["product_ion"]),
        ("G08", "positive", "high", &["full_scan", "dda"]),
        ("G08", "positive", "high", &["full_scan", "dda"]),
        ("G08", "positive", "high", &["full_scan", "dda"]),
        ("DIA", "positive", "unit", &["full_scan", "dia"]),
    ];
    for (k, (id, pol, class, acq)) in want.iter().enumerate() {
        let one = &f[k];
        assert_eq!(text(one, "polarity"), *pol, "{id} {}", KINDS[k]);
        assert_eq!(text(one, "class"), *class, "{id} {}", KINDS[k]);
        assert_eq!(list(one, "acquisition"), *acq, "{id} {}", KINDS[k]);
    }
    // G03: two parts, one per polarity, the warning that the series must be split
    assert_eq!(f[2].get("parts").unwrap().arr().len(), 2);
    assert_eq!(list(&f[2], "warnings"), ["alternating_polarity"]);
    // G04: one survey scan and two product-ion scans per cycle
    assert_eq!(f[3].get("ms2_per_cycle").unwrap().f64(), Some(2.0));
    assert_eq!(f[3].get("levels").unwrap().arr().len(), 2);
    // G05: two transitions of the same precursor, scans of products beside them
    let mrm = f[4].get("mrm").unwrap();
    assert_eq!(mrm.get("transitions").unwrap().f64(), Some(2.0));
    assert_eq!(mrm.get("q1").unwrap().at(0).unwrap().f64(), Some(364.1));
    // G06: a standard named so, as a proposal and not a fact; no scans, so no class
    assert_eq!(text(&f[5], "role"), "std");
    assert_eq!(text(&f[5], "role_source"), "proposed");
    // G08: the analyzer and the resolving power are the file's, the product ions of the Fusion are unit resolution
    assert_eq!(text(&f[7], "model"), "Orbitrap Exploris 120");
    assert_eq!(
        f[7].get("resolving_power")
            .unwrap()
            .get("ms1")
            .unwrap()
            .f64(),
        Some(45000.0)
    );
    assert!(list(&f[7], "class_basis").contains(&"analyzer:FTMS".to_string()));
    assert_eq!(text(&f[8], "class_ms2"), "unit");
    // G09 (broken header): the model is not made up and no resolving power is invented, the class still comes from the mass decimals
    assert!(f[9].get("resolving_power").is_none());
    // DIA: eight fixed adjacent windows of 45 Da, eight product-ion scans per cycle
    let iso = f[10].get("isolation").unwrap();
    assert_eq!(iso.get("windows").unwrap().f64(), Some(8.0));
    assert_eq!(
        iso.get("width_mz").unwrap().at(0).unwrap().f64(),
        Some(45.0)
    );
    assert_eq!(f[10].get("ms2_per_cycle").unwrap().f64(), Some(8.0));
    assert!(f[0].get("cycle_ms1_s").unwrap().f64().unwrap() > 0.0);
}

#[test]
fn bytes_are_stable_and_the_golden_file_matches() {
    let Some(dir) = synthetic() else { return };
    let src = sources(dir, &KINDS, &|_| String::new(), false);
    let a = build(&src, 30000).unwrap();
    assert_eq!(a, build(&src, 30000).unwrap());
    assert!((a.len() as u64).div_ceil(3) <= 30000);
    // the golden file holds the files without the numbers that come from the detection of signals (they depend on the generator's floats)
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/ispezione_sintetici.json");
    let files: Vec<String> = files_of(&a)
        .iter()
        .map(|f| {
            let mut o = Out::default();
            mzlab_llm::tools::value(&mut o, f);
            o.0
        })
        .collect();
    let want = files.join("\n") + "\n";
    if std::env::var("MZLAB_UPDATE_LLM").is_ok() {
        std::fs::write(&golden, &want).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&golden).unwrap(),
        want,
        "MZLAB_UPDATE_LLM=1 rewrites the golden file"
    );
}

#[test]
fn an_unreadable_file_is_said_so() {
    let src = j(
        r#"{"profile":"ricerca","files":[{"name":"C:\\Users\\ana\\broken.mzML","error":{"error_key":"err.file.unreadable"}},{"name":"empty.mzML","items":[]}]}"#,
    );
    let out = j(&build(&src, 2000).unwrap());
    let f = out.get("files").unwrap().arr();
    assert_eq!(text(&f[0], "status"), "unreadable");
    assert_eq!(text(&f[0], "error_key"), "err.file.unreadable");
    assert_eq!(text(&f[0], "name"), "broken.mzML");
    assert_eq!(text(&f[1], "status"), "no_data");
    assert!(f[0].get("class").is_none() && f[0].get("acquisition").is_none());
}

#[test]
fn the_course_profile_has_no_assistant() {
    let e = build_text(r#"{"profile":"corso","files":[]}"#, 1000).unwrap_err();
    assert_eq!(e.key, "err.llm.profile.course");
}

#[test]
fn nothing_private_goes_out() {
    let item = r#"{"kind":"full","polarity":"positive","ms1":10,"ms2":0,"spectrum_mode":"centroid","prof1":{"hr":false,"an":"?","dec":1,"tol":0.5,"unit":"Da"},"acq":["MS1"],"instrument":"Test Q","sha256":"abc123secretsha","label":"patient_Ana_Pérez","file":"D:/lab/ana/x.mzML","precursors":[1],"rt_min":0.5,"rt_max":9}"#;
    let method = r#"{"operatore":{"utente":"anaperez","computer":"LAB-PC-ANA"},"strumento":{"produttore":"Acme","modello":"Q 3000","serie":"SN-0012345-SECRET","computer":"LAB-PC-ANA"},
      "origine":{"formato":"wiff","file_creato":"2026-03-04T10:11:12","file":[{"nome":"D:/lab/ana/secret.wiff","sha1":"deadbeef"}]},
      "campione":{"nome":"Ana-Perez-2026","commento":"phone 555-0123"},"campi":[{"clave":"k","valore":"top-secret-value"}],"grezzo":[{"testo":"RAW-LOG-SECRET"}],
      "lc":{"gradiente":[{"t_min":0,"B_pct":5},{"t_min":10,"B_pct":95}],"flusso_mL_min":0.3,"forno_C":40},"ms":{"sorgente":{"tipo":"ESI"}}}"#;
    let name = "Ana\u{202E}fdp.exe 123456789 \u{0007}file.mzML";
    let src = format!(
        r#"{{"profile":"ricerca","hide_names":false,"files":[{{"name":{},"items":[{item}],"method":{method},"sample":{{"tipo":"sample","ordine_iniezione":1,"note":"private-note"}}}}]}}"#,
        quoted(name)
    );
    let out = build_text(&src, 4000).unwrap();
    for secret in [
        "anaperez",
        "LAB-PC",
        "SN-0012345",
        "SECRET",
        "secret",
        "deadbeef",
        "Ana-Perez",
        "555-0123",
        "top-secret",
        "RAW-LOG",
        "D:/lab",
        "Pérez",
        "private-note",
        "2026-03-04",
        "abc123",
        "patient_",
    ] {
        assert!(!out.contains(secret), "{secret} went out: {out}");
    }
    assert!(out.contains("Acme") && out.contains("40") && out.contains("\"flow_uL_min\":300"));
    let f = files_of(&out);
    let shown = text(&f[0], "name");
    assert!(
        shown.chars().count() <= MAX_STR
            && !shown.contains('\u{202E}')
            && !shown.contains('\u{0007}')
            && !shown.contains("123456789"),
        "{shown}"
    );
    assert!(shown.contains("######"), "{shown}");
    // names hidden: aliases only
    let hidden = build_text(
        &src.replace("\"hide_names\":false", "\"hide_names\":true"),
        4000,
    )
    .unwrap();
    assert!(!hidden.contains("file.mzML") && files_of(&hidden)[0].get("name").is_none());
    assert_eq!(text(&files_of(&hidden)[0], "alias"), "f1");
}

#[test]
fn a_long_name_is_cut() {
    let long = "x".repeat(300);
    let src = format!(
        r#"{{"profile":"ricerca","files":[{{"name":"{long}.mzML","error":{{"error_key":"e"}}}}]}}"#
    );
    let f = files_of(&build_text(&src, 4000).unwrap());
    assert_eq!(text(&f[0], "name").chars().count(), MAX_STR);
}
