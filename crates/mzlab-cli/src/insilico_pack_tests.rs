//! `insilico pack` on an invented PubChemLite-shaped CSV: pieces sorted by mass that each inflate alone, a catalogue whose ranges, sizes and SHA-256 are right,
//! the same bytes twice, and the rows (and the ranking) of the pieces of a mass window equal to those of the full list. No network, no lab data.

use std::path::{Path, PathBuf};

use super::*;
use mzlab_feat::attributio::ms2_engine::insilico::read_candidates;
use mzlab_feat::flow_insilico::{insilico_text as core_text, pieces_needed};
use mzlab_feat::flow_ms2::json_params;

const MGF: &str = "BEGIN IONS\nTITLE=q0\nPEPMASS=151.0754\nCHARGE=1+\nIONMODE=Positive\n105.0335 1000\n77.0386 400\n123.0441 300\n60.0000 50\nEND IONS\n";

/// PubChemLite columns (more than the package keeps), rows out of mass order, a name with a comma, a row without SMILES and one whose mass is read from its SMILES.
fn pubchemlite() -> String {
    let mut s = String::from("Identifier,FirstBlock,PubMed_Count,MolecularFormula,SMILES,InChI,InChIKey,MonoisotopicMass,CompoundName,Extra\n");
    let mut row = |id: &str, name: &str, smiles: &str, formula: &str, mass: &str| {
        s.push_str(&format!(
            "{id},FB{id},3,{formula},{smiles},InChI=1S/x,KEY{id},{mass},{name},z\n"
        ));
    };
    row(
        "A1",
        "Candidate A",
        "CCOC(=O)c1ccccc1",
        "C9H10O2",
        "150.06808",
    );
    row(
        "A2",
        "\"Candidate, B\"",
        "COC(=O)Cc1ccccc1",
        "C9H10O2",
        "150.06808",
    );
    row("Z9", "Far away", "CCCCCCCC", "C8H18", "114.14085");
    row(
        "B3",
        "Heavy one",
        "CCCCCCCCCCCCCCCCCCCC",
        "C20H42",
        "282.32865",
    );
    row(
        "M1",
        "Methyl benzoate",
        "COC(=O)c1ccccc1",
        "C8H8O2",
        "136.052429",
    );
    row("N0", "No structure", "", "C2H6", "30.04695");
    row("W5", "Wrong mass", "CCO", "", "oops");
    for i in 0..40 {
        row(
            &format!("F{i:02}"),
            "Filler",
            "CCCCCCCC",
            "C8H18",
            &format!("{}", 114.14085 + f64::from(i) * 0.5),
        );
    }
    s
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_insilico_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn read_all(d: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<_> = std::fs::read_dir(d)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    v.sort();
    v
}

/// A small `piece_raw` makes several pieces out of a short list.
fn packed(tag: &str) -> PathBuf {
    let d = dir(tag);
    pack_with(
        pubchemlite().as_bytes(),
        &d,
        "3.3.0",
        "10.5281/zenodo.0",
        600,
    )
    .unwrap();
    d
}

#[test]
fn pieces_are_sorted_by_mass_inflate_alone_and_the_catalogue_is_right() {
    let d = packed("sorted");
    let cat_text = std::fs::read(d.join("catalogo.json")).unwrap();
    let cat = json::parse(&cat_text).unwrap();
    assert_eq!(cat.get("format").and_then(Value::str), Some("insilico/1"));
    assert_eq!(cat.get("licence").and_then(Value::str), Some("CC-BY-4.0"));
    assert_eq!(
        cat.get("rows").and_then(Value::f64),
        Some(46.0),
        "6 rows + 40 fillers; the mass of W5 comes from its SMILES, as read_candidates does"
    );
    assert_eq!(
        cat.get("skipped_rows").and_then(Value::f64),
        Some(1.0),
        "the row without SMILES"
    );
    let pieces = cat.get("pieces").unwrap().arr();
    assert!(pieces.len() >= 3, "{} pieces", pieces.len());
    let (mut last, mut total_rows, mut sum_z) = (f64::MIN, 0.0, 0.0);
    for p in pieces {
        let file = p.get("file").and_then(Value::str).unwrap();
        let raw = read_piece(&d, &cat_text, file).unwrap();
        let rows = read_candidates(&String::from_utf8(raw).unwrap());
        assert_eq!(
            rows.len() as f64,
            p.get("rows").and_then(Value::f64).unwrap()
        );
        let (lo, hi) = (
            p.get("mass_min").and_then(Value::f64).unwrap(),
            p.get("mass_max").and_then(Value::f64).unwrap(),
        );
        assert!(
            rows.iter().all(|c| c.mass >= lo && c.mass <= hi)
                && rows.first().unwrap().mass == lo
                && rows.last().unwrap().mass == hi
        );
        assert!(lo >= last, "pieces in mass order");
        assert!(
            rows.windows(2)
                .all(|w| (w[0].mass, &w[0].id) <= (w[1].mass, &w[1].id)),
            "rows by mass then identifier"
        );
        (last, total_rows, sum_z) = (
            hi,
            total_rows + rows.len() as f64,
            sum_z + p.get("size").and_then(Value::f64).unwrap(),
        );
    }
    assert_eq!(total_rows, 46.0);
    assert_eq!(cat.get("size_compressed").and_then(Value::f64), Some(sum_z));
    // a changed byte is refused
    let mut z = std::fs::read(d.join("000.bin")).unwrap();
    z[10] ^= 1;
    std::fs::write(d.join("000.bin"), z).unwrap();
    assert_eq!(
        read_piece(&d, &cat_text, "000.bin").unwrap_err().key,
        "err.flow.insilico.piece"
    );
    assert_eq!(
        read_piece(&d, &cat_text, "../x").unwrap_err().key,
        "err.flow.insilico.piece"
    );
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    let (a, b) = (packed("same_a"), packed("same_b"));
    assert_eq!(read_all(&a), read_all(&b));
    assert!(
        String::from_utf8(std::fs::read(a.join("catalogo.json")).unwrap())
            .unwrap()
            .contains("\"version\":\"3.3.0\"")
    );
}

fn params() -> Vec<(String, Value)> {
    json_params(r#"{"db_ppm":5,"frag_ppm":10,"max_bonds":200,"top":5,"score_terms":false}"#)
        .unwrap()
}

#[test]
fn the_window_of_the_pieces_is_the_rows_of_the_full_list_and_the_ranking_is_the_same() {
    let d = packed("window");
    let cat = std::fs::read(d.join("catalogo.json")).unwrap();
    let p = params();
    let need = pieces_needed(&p, ("q.mgf", MGF.as_bytes()), &cat).unwrap();
    assert!(!need.is_empty() && need.len() < 3, "{need:?}");
    // the rows of those pieces that fall in the window are exactly the rows of the full list in the window
    let neutral = 151.0754 - 1.00727646677;
    let (lo, hi) = (neutral * (1.0 - 5e-6), neutral * (1.0 + 5e-6));
    let mut from_pieces: Vec<_> = need
        .iter()
        .flat_map(|f| {
            read_candidates(&String::from_utf8(read_piece(&d, &cat, f).unwrap()).unwrap())
        })
        .filter(|c| c.mass >= lo && c.mass <= hi)
        .collect();
    let mut full: Vec<_> = read_candidates(&pubchemlite())
        .into_iter()
        .filter(|c| c.mass >= lo && c.mass <= hi)
        .collect();
    let key = |c: &mzlab_feat::attributio::ms2_engine::insilico::Candidate| c.id.clone();
    from_pieces.sort_by_key(key);
    full.sort_by_key(key);
    assert_eq!(from_pieces, full);
    assert_eq!(from_pieces.len(), 2);
    // and the candidates/1 of the package equals the one of the full CSV
    let mut piece = |f: &str| read_piece(&d, &cat, f);
    let via_package = core_text(&p, ("q.mgf", MGF.as_bytes()), &cat, &mut piece).unwrap();
    let mut none = |_: &str| -> Result<Vec<u8>> { Err(Error::new("err.flow.insilico.piece")) };
    let via_csv = core_text(
        &p,
        ("q.mgf", MGF.as_bytes()),
        pubchemlite().as_bytes(),
        &mut none,
    )
    .unwrap();
    assert_eq!(via_package, via_csv);
    assert!(
        via_package.contains("\"id\":\"A1\"") && via_package.contains("\"id\":\"A2\""),
        "{via_package}"
    );
}

/// The plan reads the catalogue as the suspects file and gets the pieces from its folder: the bytes of the CSV run.
#[test]
fn flow_run_on_the_package_equals_flow_run_on_the_csv() {
    let d = packed("flow");
    std::fs::write(d.join("q.mgf"), MGF).unwrap();
    std::fs::write(d.join("list.csv"), pubchemlite()).unwrap();
    let run = |suspects: &str, work: &str| {
        let plan = format!(
            r#"{{"format":"mzflow/2","inputs":[{{"name":"q.mgf","type":"spectra"}},{{"name":"{suspects}","type":"candidates"}}],"steps":[
            {{"id":"i","step":"spectra.insilico@1","in":{{"spectra":"file:q.mgf","suspects":"file:{suspects}"}},"params":{{}}}}]}}"#
        );
        let w = d.join(work);
        std::fs::create_dir_all(&w).unwrap();
        let (r, prov) = crate::flow::run_plan(plan.as_bytes(), &d, &w, 1, None).unwrap();
        assert!(
            r.skipped.is_empty() && !prov.contains("\"error\""),
            "{prov}"
        );
        std::fs::read(w.join("i.json")).unwrap()
    };
    let (a, b) = (run("catalogo.json", "w_pkg"), run("list.csv", "w_csv"));
    assert_eq!(a, b);
    assert!(String::from_utf8_lossy(&a).contains("\"id\":\"A1\""));
}
