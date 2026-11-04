use super::*;

fn p(kv: &[(&str, f64)]) -> Params {
    kv.iter()
        .map(|(k, v)| (k.to_string(), Value::Num(*v)))
        .collect()
}

#[test]
fn msn_graph_1_renames_and_moves_to_2() {
    let (v, out, m) = migrate_params(
        "spectra.msn_graph",
        1,
        &p(&[("formula", 1.0), ("mz_basso", 100.0), ("prove_min", 2.0)]),
    )
    .unwrap();
    assert_eq!(v, 2);
    assert_eq!(
        out,
        p(&[("formula", 1.0), ("low_mz", 100.0), ("min_evidence", 2.0)])
    );
    assert_eq!(
        m.renamed,
        [
            ("mz_basso".to_string(), "low_mz".to_string()),
            ("prove_min".to_string(), "min_evidence".to_string())
        ]
    );
}

#[test]
fn a_version_1_without_old_names_still_moves_to_2() {
    let (v, out, m) = migrate_params("spectra.msn_graph", 1, &p(&[("formula", 1.0)])).unwrap();
    assert_eq!((v, out.len(), m.renamed.len()), (2, 1, 0));
}

#[test]
fn version_2_with_old_names_is_renamed_and_the_new_name_wins() {
    let (v, out, m) = migrate_params(
        "spectra.msn_graph",
        2,
        &p(&[("isotopi", 0.0), ("isotopes", 1.0), ("regole", 1.0)]),
    )
    .unwrap();
    assert_eq!(v, 2);
    assert_eq!(out, p(&[("isotopes", 1.0), ("rules", 1.0)]));
    assert_eq!(m.renamed, [("regole".to_string(), "rules".to_string())]);
    assert!(migrate_params("spectra.msn_graph", 2, &p(&[("low_mz", 1.0)])).is_none());
    assert!(migrate_params("features.detect", 1, &p(&[("mz_basso", 1.0)])).is_none());
}

#[test]
fn names_of_a_set() {
    assert_eq!(
        migrate_name("spectra.msn_graph", 2, "toll_perdita_mda").as_deref(),
        Some("loss_tol_mda")
    );
    assert_eq!(
        migrate_name("spectra.msn_graph", 1, "scelte").as_deref(),
        Some("pick")
    );
    assert_eq!(migrate_name("spectra.msn_graph", 2, "formula"), None);
    assert_eq!(migrate_name("spectra.match", 1, "prove_min"), None);
}

#[test]
fn the_table_has_fourteen_distinct_english_names() {
    assert_eq!(MSN_GRAPH_1_2.len(), 14);
    let mut new: Vec<&str> = MSN_GRAPH_1_2.iter().map(|r| r.1).collect();
    new.sort_unstable();
    new.dedup();
    assert_eq!(new.len(), 14);
}

#[test]
fn table_filter_1_keeps_the_filled_blank_cells_and_2_is_left_alone() {
    let (v, out, _) = migrate_params("table.filter", 1, &p(&[("blank_ratio", 3.0)])).unwrap();
    assert_eq!(v, 2);
    assert!(out.contains(&("blank_cells".to_string(), Value::Str("all".to_string()))));
    // a version 2 plan that leaves the option out takes the default of the step, it is not rewritten
    assert!(migrate_params("table.filter", 2, &p(&[("blank_ratio", 3.0)])).is_none());
    // a version 1 plan that names the option keeps its value
    let given = vec![(
        "blank_cells".to_string(),
        Value::Str("detected".to_string()),
    )];
    let (_, out, _) = migrate_params("table.filter", 1, &given).unwrap();
    assert_eq!(out, given);
}
