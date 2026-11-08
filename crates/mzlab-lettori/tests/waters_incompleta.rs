//! A Waters folder copied short gives one keyed notice that names the folder and the file: `_FUNCTNS.INF` left behind, `_FUNCTNS.INF` or an index
//! cut inside a record. Synthetic folders only.

mod comune_waters;

use comune_waters::{file_cartella, insieme};
use mzlab_core::lettori::Limiti;

/// The error of the synthetic folder after `cambia`, with its key and its `name` and `file`.
fn rifiuto(cambia: impl FnOnce(&mut Vec<(String, Vec<u8>)>)) -> (String, String, String) {
    let mut f = file_cartella();
    cambia(&mut f);
    let e = mzlab_lettori::apri_nudo(insieme(f), &Limiti::sito())
        .err()
        .expect("refused");
    let p = |n: &str| {
        e.params
            .iter()
            .find(|(k, _)| *k == n)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| panic!("{} has no {n}", e.key))
    };
    (e.key.to_string(), p("name"), p("file"))
}

fn senza(f: &mut Vec<(String, Vec<u8>)>, nome: &str) {
    f.retain(|(n, _)| n != nome);
}

fn accorcia(f: &mut [(String, Vec<u8>)], nome: &str, di: usize) {
    let b = &mut f.iter_mut().find(|(n, _)| n == nome).unwrap().1;
    b.truncate(b.len() - di);
}

#[test]
fn a_folder_without_its_function_list_is_incomplete() {
    let k = (
        "err.file.folderIncomplete".into(),
        "X.raw".into(),
        "_FUNCTNS.INF".into(),
    );
    assert_eq!(rifiuto(|f| senza(f, "_FUNCTNS.INF")), k);
}

#[test]
fn a_function_list_cut_inside_a_record_is_named() {
    let k = (
        "err.file.folderFileCut".into(),
        "X.raw".into(),
        "_FUNCTNS.INF".into(),
    );
    assert_eq!(rifiuto(|f| accorcia(f, "_FUNCTNS.INF", 10)), k);
}

#[test]
fn an_index_cut_inside_a_row_is_named() {
    let k = (
        "err.file.indexCut".into(),
        "X.raw".into(),
        "_FUNC001.IDX".into(),
    );
    assert_eq!(rifiuto(|f| accorcia(f, "_FUNC001.IDX", 5)), k);
}
