//! `campioni::ricollega`: files renamed, moved and replaced are found by SHA-256; by name only when a SHA-256 is unknown, and flagged.

use mzlab_core::campioni::{self, proponi, ricollega, Campione};

fn series(files: &[(&str, &str)]) -> Vec<Campione> {
    let v: Vec<(String, String, String)> = files
        .iter()
        .map(|(n, h)| (n.to_string(), h.to_string(), String::new()))
        .collect();
    proponi(&v)
}

fn cand(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(p, h)| (p.to_string(), h.to_string()))
        .collect()
}

fn found(cs: &[Campione], c: &[(&str, &str)]) -> Vec<(usize, String, bool)> {
    ricollega(cs, &cand(c))
        .into_iter()
        .map(|a| (a.index, a.path, a.by_name))
        .collect()
}

#[test]
fn renamed_and_moved_files_are_found_by_sha() {
    let cs = series(&[
        ("a_t5.mzML", "aa"),
        ("b_t5.mzML", "bb"),
        ("c_t5.mzML", "cc"),
    ]);
    let got = found(
        &cs,
        &[
            ("/new/dir/x.mzML", "bb"),
            ("/d/a_t5.mzML", "aa"),
            ("/d/other.mzML", "ff"),
        ],
    );
    assert_eq!(
        got,
        [
            (0, "/d/a_t5.mzML".to_string(), false),
            (1, "/new/dir/x.mzML".to_string(), false)
        ]
    );
}

#[test]
fn a_replaced_file_is_never_matched_by_name() {
    let cs = series(&[("a_t5.mzML", "aa")]);
    assert_eq!(found(&cs, &[("/d/a_t5.mzML", "zz")]), []);
}

#[test]
fn by_name_only_when_a_sha_is_unknown_and_flagged() {
    let cs = series(&[("a_t5.mzML", ""), ("b_t5.mzML", "bb"), ("c_t5.mzML", "cc")]);
    let got = found(
        &cs,
        &[
            ("/d/a_t5.mzML", "11"),
            ("C:\\d\\b_t5.mzML", ""),
            ("/d/c_t5.mzML", ""),
        ],
    );
    assert_eq!(
        got,
        [
            (0, "/d/a_t5.mzML".to_string(), true),
            (1, "C:\\d\\b_t5.mzML".to_string(), true),
            (2, "/d/c_t5.mzML".to_string(), true)
        ]
    );
}

#[test]
fn copies_ambiguous_names_and_order() {
    let cs = series(&[("a_t5.mzML", "aa"), ("n.mzML", "")]);
    // two copies of the same file: the one with the same name wins, a candidate serves one file
    assert_eq!(
        found(&cs, &[("/x/copy.mzML", "aa"), ("/y/a_t5.mzML", "aa")])[0],
        (0, "/y/a_t5.mzML".to_string(), false)
    );
    // a name shared by two free candidates is not guessed
    assert_eq!(found(&cs, &[("/p/n.mzML", ""), ("/q/n.mzML", "")]), []);
    // the same answer for any order of the candidates
    let a = found(&cs, &[("/p/a_t5.mzML", "aa"), ("/q/a_t5.mzML", "aa")]);
    let b = found(&cs, &[("/q/a_t5.mzML", "aa"), ("/p/a_t5.mzML", "aa")]);
    assert_eq!(a, b);
}

#[test]
fn the_route_answers_in_json() {
    let cs = series(&[("a_t5.mzML", "aa")]);
    let input = format!(
        "{{\"campioni\":{},\"file\":[{{\"percorso\":\"/n/z.mzML\",\"sha256\":\"aa\"}}]}}",
        campioni::to_json(&cs)
    );
    assert_eq!(
        campioni::route("ricollega", &input).unwrap(),
        "{\"abbinati\":[{\"indice\":0,\"per_nome\":false,\"percorso\":\"/n/z.mzML\"}]}"
    );
}
