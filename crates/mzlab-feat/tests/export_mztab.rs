//! The mzTab-M export read back by a small reader written here: sections, columns of every line against its header, the values against the table.

use std::collections::HashMap;

use mzlab_feat::export::Ctx;
use mzlab_feat::export_mztab::mztab;
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{Aligned, RowIdent, CELL_ABSENT, CELL_DETECTED, CELL_FILLED};

fn table() -> Aligned {
    let rows: [(f64, f64, [f64; 3], [u8; 3]); 2] = [
        (
            317.1642,
            5.5,
            [1e6, 2e6, 3e6],
            [CELL_DETECTED, CELL_FILLED, CELL_DETECTED],
        ),
        (
            300.2,
            7.0,
            [0.0, 0.0, 4e5],
            [CELL_ABSENT, CELL_ABSENT, CELL_DETECTED],
        ),
    ];
    let mut t = Aligned::with_capacity(3, 2);
    for (mz, rt, areas, states) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for (a, s) in areas.iter().zip(states) {
            t.area.push(*a);
            t.height.push((*a / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state.push(s);
        }
    }
    t
}

fn samples() -> Vec<Sample> {
    let f: Vec<(String, String)> = ["blank 1.mzML", "t10.mzML", "t20.mzML"]
        .iter()
        .map(|n| (n.to_string(), "00".repeat(32)))
        .collect();
    propose(&f)
}

/// The sections of a file: `MTD` key → value, and for each of SML, SMF, SME the rows keyed by the header names.
struct Read {
    mtd: HashMap<String, String>,
    sml: Vec<HashMap<String, String>>,
    smf: Vec<HashMap<String, String>>,
    sme: Vec<HashMap<String, String>>,
}

fn read(text: &str) -> Read {
    let mut r = Read {
        mtd: HashMap::new(),
        sml: vec![],
        smf: vec![],
        sme: vec![],
    };
    let mut heads: HashMap<&str, Vec<&str>> = HashMap::new();
    for l in text.lines().filter(|l| !l.is_empty()) {
        let f: Vec<&str> = l.split('\t').collect();
        match f[0] {
            "MTD" => {
                assert_eq!(f.len(), 3, "{l}");
                r.mtd.insert(f[1].to_string(), f[2].to_string());
            }
            h @ ("SMH" | "SFH" | "SEH") => {
                heads.insert(&h[..2], f);
            }
            k @ ("SML" | "SMF" | "SME") => {
                let key = if k == "SML" {
                    "SM"
                } else if k == "SMF" {
                    "SF"
                } else {
                    "SE"
                };
                let h = &heads[key];
                assert_eq!(f.len(), h.len(), "columns of {l}");
                let row: HashMap<String, String> = h
                    .iter()
                    .zip(&f)
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect();
                match k {
                    "SML" => r.sml.push(row),
                    "SMF" => r.smf.push(row),
                    _ => r.sme.push(row),
                }
            }
            other => panic!("unknown line {other}"),
        }
    }
    r
}

#[test]
fn round_trip_of_the_table() {
    let (mut t, s) = (table(), samples());
    t.ident = RowIdent {
        neutral: vec![316.1569, 0.0],
        ion: vec![1, 1],
        src: vec![3, 1],
    };
    let c = Ctx {
        table: &t,
        samples: &s,
        view: None,
        table_sha256: "abcdef0123456789ff",
        polarity: 1,
    };
    let text = mztab(&c);
    // `MZLAB_MZTAB_OUT=<path>` keeps the synthetic file for the official validator (run by hand).
    if let Ok(path) = std::env::var("MZLAB_MZTAB_OUT") {
        std::fs::write(path, &text).unwrap();
    }
    let r = read(&text);

    assert_eq!(r.mtd["mzTab-version"], "2.0.0-M");
    assert_eq!(r.mtd["ms_run[1]-location"], "file:///blank%201.mzML");
    assert_eq!(
        r.mtd["ms_run[3]-scan_polarity[1]"],
        "[MS, MS:1000130, positive scan, ]"
    );
    assert_eq!(r.mtd["assay[2]-ms_run_ref"], "ms_run[2]");
    assert!(r.mtd["study_variable[1]-assay_refs"].starts_with("assay[1]"));

    assert_eq!((r.sml.len(), r.smf.len(), r.sme.len()), (2, 2, 1));
    // Areas and states: absent cells are NaN, never zero.
    assert_eq!(r.sml[0]["abundance_assay[1]"], "1000000");
    assert_eq!(r.sml[1]["abundance_assay[1]"], "NaN");
    assert_eq!(r.sml[1]["abundance_assay[3]"], "400000");
    // Retention time in seconds, m/z, charge.
    assert_eq!(r.smf[0]["exp_mass_to_charge"], "317.1642");
    assert_eq!(r.smf[0]["retention_time_in_seconds"], "330");
    assert_eq!(r.smf[1]["retention_time_in_seconds"], "420");
    assert_eq!(r.smf[0]["charge"], "1");
    // Only the named row has evidence, with its adduct and the theoretical m/z; no level is raised.
    assert_eq!(r.smf[0]["SME_ID_REFS"], "1");
    assert_eq!(r.smf[1]["SME_ID_REFS"], "null");
    assert_eq!(r.sme[0]["adduct_ion"], "[M+H]+");
    assert_eq!(r.sme[0]["theoretical_mass_to_charge"], "317.16418");
    assert_eq!(r.sml[0]["theoretical_neutral_mass"], "316.1569");
    assert!(r.sml.iter().all(|m| m["reliability"] == "4"));
    assert_eq!(r.sml[1]["adduct_ions"], "null");
    // The default reliability scheme is not redeclared; the unit and the ontology terms are the ones of the PSI-MS CV.
    assert!(!r
        .mtd
        .contains_key("small_molecule-identification_reliability"));
    assert_eq!(
        r.mtd["small_molecule-quantification_unit"],
        "[MS, MS:1001844, MS1 feature area, ]"
    );
    // The evidence maps to the run with the largest area (the third file) and carries the mass error, not a level.
    assert_eq!(r.sme[0]["spectra_ref"], "ms_run[3]");
    assert_eq!(r.sme[0]["id_confidence_measure[1]"], "0.07");
    // No empty cell and no scientific notation anywhere in a table line.
    for l in text
        .lines()
        .filter(|l| l.starts_with("SM") || l.starts_with("SF") || l.starts_with("SE"))
    {
        assert!(l.split('\t').all(|f| !f.is_empty()), "{l}");
        assert!(
            !l.split('\t')
                .skip(1)
                .any(|f| f.parse::<f64>().is_ok() && f.contains(['e', 'E'])),
            "{l}"
        );
    }
}

#[test]
fn same_bytes_every_time_and_negative_polarity() {
    let (t, s) = (table(), samples());
    let c = Ctx {
        table: &t,
        samples: &s,
        view: None,
        table_sha256: "ab",
        polarity: -1,
    };
    assert_eq!(mztab(&c), mztab(&c));
    let r = read(&mztab(&c));
    assert_eq!(
        r.mtd["ms_run[1]-scan_polarity[1]"],
        "[MS, MS:1000129, negative scan, ]"
    );
    assert_eq!(r.smf[0]["charge"], "-1");
    assert!(r.sme.is_empty());
}
