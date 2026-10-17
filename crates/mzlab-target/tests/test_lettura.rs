// Integration tests for target screening list parsing, validation and binary search index.
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;

use mzlab_target::{parse_rfc4180_str, parse_target_csv, validate_entries, AdductType, TargetList};

fn path_sostanze_csv() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/liste/sostanze_hrms.csv")
}

#[test]
fn test_caricamento_e_validazione_645_sostanze() {
    let path = path_sostanze_csv();
    assert!(path.exists(), "Target CSV file not found at {:?}", path);

    let file = File::open(&path).expect("Failed to open target CSV");
    let entries = parse_target_csv(file).expect("Failed to parse target CSV");

    assert_eq!(entries.len(), 645, "Expected exactly 645 target entries");

    let report = validate_entries(&entries);
    assert!(
        report.is_valid(),
        "Validation failed with errors: {:?}",
        report.errors
    );
    assert_eq!(report.total_entries, 645);
    assert_eq!(report.valid_entries, 645);
    assert_eq!(report.salts_or_hydrates, 2);
    assert_eq!(report.stereo_to_verify, 5);
    assert!(report.errors.is_empty());
}

#[test]
fn test_gestione_7_righe_segnalate() {
    let file = File::open(path_sostanze_csv()).expect("Failed to open target CSV");
    let entries = parse_target_csv(file).expect("Failed to parse target CSV");
    let list = TargetList::from_entries(entries);

    // Verify 2 salt/hydrate entries
    let dim = list
        .find_by_id("dimenhydrinate")
        .expect("dimenhydrinate not found");
    assert!(dim.check_flags.is_salt_or_hydrate);
    assert!(!dim.check_flags.stereo_to_verify);
    assert_eq!(dim.neutral_formula, "C17H21NO");
    assert!((dim.monoisotopic_mass - 255.162314).abs() < 1e-5);

    let hydro = list
        .find_by_id("hydrocodone-bitartrate-hemipentahydrate")
        .expect("hydrocodone bitartrate hemipentahydrate not found");
    assert!(hydro.check_flags.is_salt_or_hydrate);
    assert!(!hydro.check_flags.stereo_to_verify);
    assert_eq!(hydro.neutral_formula, "C18H21NO3");
    assert!((hydro.monoisotopic_mass - 299.152144).abs() < 1e-5);

    // Verify 5 stereo entries
    let stereo_ids = [
        "ergonovine",
        "eszopiclone",
        "ibogaine",
        "isopropyl-u-47700",
        "meclonazepam",
    ];
    for id in stereo_ids {
        let entry = list
            .find_by_id(id)
            .unwrap_or_else(|| panic!("{id} not found"));
        assert!(
            !entry.check_flags.is_salt_or_hydrate,
            "{id} should not be marked salt"
        );
        assert!(
            entry.check_flags.stereo_to_verify,
            "{id} should have stereo_to_verify flag"
        );
    }
}

#[test]
fn test_ricerca_indice_mz_e_addotti() {
    let file = File::open(path_sostanze_csv()).expect("Failed to open target CSV");
    let entries = parse_target_csv(file).expect("Failed to parse target CSV");
    let list = TargetList::from_entries(entries);

    // Search DMAA: mz_MH = 116.143376
    let dmaa_mh = list.find_by_mz(116.143376, 5.0, AdductType::MH);
    assert!(
        !dmaa_mh.is_empty(),
        "Expected DMAA to be found via [M+H]+ binary search"
    );
    assert!(dmaa_mh.iter().any(|e| e.id == "1-3-dimethylamylamine"));

    // Search DMAA: mz_MNa = 138.12532
    let dmaa_mna = list.find_by_mz(138.12532, 5.0, AdductType::MNa);
    assert!(
        !dmaa_mna.is_empty(),
        "Expected DMAA to be found via [M+Na]+ binary search"
    );
    assert!(dmaa_mna.iter().any(|e| e.id == "1-3-dimethylamylamine"));

    // Search DMAA: mz_MNH4 = 133.169923
    let dmaa_mnh4 = list.find_by_mz(133.169923, 5.0, AdductType::MNH4);
    assert!(
        !dmaa_mnh4.is_empty(),
        "Expected DMAA to be found via [M+NH4]+ binary search"
    );
    assert!(dmaa_mnh4.iter().any(|e| e.id == "1-3-dimethylamylamine"));

    // Search decoy m/z not present in screening list
    let decoy = list.find_by_mz(999.999, 5.0, AdductType::MH);
    assert!(decoy.is_empty(), "Expected empty result for decoy m/z");

    // Search by identifier
    assert!(list.find_by_id("1-3-dimethylamylamine").is_some());
    assert!(list.find_by_id("sostanza-inesistente").is_none());

    // Search by Neutral formula
    let matches = list.find_by_formula("C7H17N");
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|e| e.id == "1-3-dimethylamylamine"));
    assert!(list.find_by_formula("C999H999").is_empty());

    // Check mz_for_adduct method
    let dmaa = list.find_by_id("1-3-dimethylamylamine").expect("DMAA");
    assert_eq!(dmaa.mz_for_adduct(AdductType::MH), dmaa.mz_mh);
    assert_eq!(dmaa.mz_for_adduct(AdductType::MNa), dmaa.mz_mna);
    assert_eq!(dmaa.mz_for_adduct(AdductType::MNH4), dmaa.mz_mnh4);
}

#[test]
fn test_rfc4180_parsing_casi_limite() {
    let raw = "col1,col2,col3\r\n\"a,b\",\"c\nd\",\"e\"\"f\"\r\nx,y,z\n";
    let records = parse_rfc4180_str(raw).expect("Failed to parse RFC 4180 string");
    assert_eq!(records.len(), 3);
    assert_eq!(records[0], vec!["col1", "col2", "col3"]);
    assert_eq!(records[1], vec!["a,b", "c\nd", "e\"f"]);
    assert_eq!(records[2], vec!["x", "y", "z"]);

    // Test missing header detection
    let bad_csv = "id,name_en\n1,test\n";
    let res = parse_target_csv(Cursor::new(bad_csv));
    assert!(res.is_err());
}
