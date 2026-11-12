//! A Waters folder copied short gives one keyed notice that names the folder and the file: `_FUNCTNS.INF` left behind, `_FUNCTNS.INF` or an index
//! cut inside a record, every index cut inside a row. Synthetic folders only.

mod common_waters;

use common_waters::{file_set, folder_file};
use mzlab_core::readers::Limits;

/// The error of the synthetic folder after `cambia`, with its key and its `name` and `file`.
fn refusal(change: impl FnOnce(&mut Vec<(String, Vec<u8>)>)) -> (String, String, String) {
    let mut f = folder_file();
    change(&mut f);
    let e = mzlab_lettori::open_bare(file_set(f), &Limits::sito())
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

fn without(f: &mut Vec<(String, Vec<u8>)>, name: &str) {
    f.retain(|(n, _)| n != name);
}

fn shorten(f: &mut [(String, Vec<u8>)], name: &str, di: usize) {
    let b = &mut f.iter_mut().find(|(n, _)| n == name).unwrap().1;
    b.truncate(b.len() - di);
}

#[test]
fn a_folder_without_its_function_list_is_incomplete() {
    let k = (
        "err.file.folderIncomplete".into(),
        "X.raw".into(),
        "_FUNCTNS.INF".into(),
    );
    assert_eq!(refusal(|f| without(f, "_FUNCTNS.INF")), k);
}

#[test]
fn a_function_list_cut_inside_a_record_is_named() {
    let k = (
        "err.file.folderFileCut".into(),
        "X.raw".into(),
        "_FUNCTNS.INF".into(),
    );
    assert_eq!(refusal(|f| shorten(f, "_FUNCTNS.INF", 10)), k);
}

#[test]
fn an_index_cut_inside_a_row_is_named_when_nothing_else_reads() {
    let k = (
        "err.file.indexCut".into(),
        "X.raw".into(),
        "_FUNC001.IDX".into(),
    );
    assert_eq!(
        refusal(|f| {
            shorten(f, "_FUNC001.IDX", 5);
            shorten(f, "_FUNC002.IDX", 5);
        }),
        k
    );
}

/// With the SIR/MRM function whole, the scanning function of the cut index is left out and its whole rows counted; the channels read.
#[test]
fn an_index_cut_inside_a_row_leaves_the_rest_readable() {
    let mut f = folder_file();
    shorten(&mut f, "_FUNC001.IDX", 5);
    let mut l = mzlab_lettori::open_bare(file_set(f), &Limits::sito()).expect("opened");
    let x = l.losses(0).unwrap();
    assert_eq!((x.declared, x.read, x.cause), (2, 0, "warn.waters.damaged"));
    assert!(l
        .chromatograms(0)
        .unwrap()
        .iter()
        .any(|c| c.id.starts_with("SRM SIC")));
}
