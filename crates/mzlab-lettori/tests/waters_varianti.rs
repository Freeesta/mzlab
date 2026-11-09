//! A Waters folder with an empty `_FUNCTNS.INF`, or whose only function with data stores records of a width not read yet, gives one keyed notice
//! that names the folder (and the function and its type byte), never an empty reading; next to a readable function the folder still opens.

mod comune_waters;

use comune_waters::{file_set, folder_file, record_funzione};
use mzlab_core::lettori::Limits;

/// One row of `_FUNCnnn.IDX`, variant B (30 bytes, offset as `u64` at byte 22).
fn row_b(offset: u64, count: u32, tic: f32, rt_min: f32) -> Vec<u8> {
    let mut r = vec![0u8; 30];
    r[4..8].copy_from_slice(&(count | 0x1800_0000).to_le_bytes());
    r[8..12].copy_from_slice(&tic.to_le_bytes());
    r[12..16].copy_from_slice(&rt_min.to_le_bytes());
    r[22..30].copy_from_slice(&offset.to_le_bytes());
    r
}

/// Function 1 of type 0x10 with three scans of three 10-byte records (a width no decoder knows).
fn unknown_function() -> Vec<(String, Vec<u8>)> {
    let (mut idx, dat) = (Vec::new(), vec![7u8; 90]);
    for k in 0..3u64 {
        idx.extend(row_b(30 * k, 3, 100.0, 0.5 * (k + 1) as f32));
    }
    vec![
        (
            "_FUNCTNS.INF".into(),
            record_funzione(0x10, 0x71, 1.0, &[(50.0, 1000.0, 0.0)]),
        ),
        ("_FUNC001.DAT".into(), dat),
        ("_FUNC001.IDX".into(), idx),
    ]
}

fn param(e: &mzlab_core::error::Error, name: &str) -> String {
    e.params
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| panic!("{} has no {name}", e.key))
}

#[test]
fn an_empty_function_list_is_told_with_the_folder() {
    let mut f = folder_file();
    f.iter_mut()
        .find(|(n, _)| n == "_FUNCTNS.INF")
        .unwrap()
        .1
        .clear();
    let e = mzlab_lettori::apri_nudo(file_set(f), &Limits::sito())
        .err()
        .expect("refused");
    assert_eq!(e.key, "err.file.functionsEmpty");
    assert_eq!(param(&e, "name"), "X.raw");
}

#[test]
fn a_lone_function_of_unknown_records_is_told_with_its_number_and_type() {
    let e = mzlab_lettori::apri_nudo(file_set(unknown_function()), &Limits::sito())
        .err()
        .expect("refused");
    assert_eq!(e.key, "err.file.recordVariant");
    assert_eq!(param(&e, "name"), "X.raw");
    assert_eq!(param(&e, "function"), "1");
    assert_eq!(param(&e, "type"), "0x10");
}

#[test]
fn next_to_a_readable_function_the_folder_still_opens() {
    // Function 2 of the synthetic folder (MRM, two channels) beside the unknown function 1.
    let base = folder_file();
    let mut f = unknown_function();
    let mut inf = f.remove(0).1;
    inf.extend(&base[0].1[416..832]);
    f.insert(0, ("_FUNCTNS.INF".into(), inf));
    f.extend(base.into_iter().filter(|(n, _)| n.starts_with("_FUNC002")));
    let mut l = mzlab_lettori::apri_nudo(file_set(f), &Limits::sito()).expect("opens");
    assert!(l.chromatograms(0).expect("channels").len() >= 2);
}
