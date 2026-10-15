use super::*;

// rewritten text (no real compound names): the layout is a header of column names and one row per entry, bars between the cells
const TEXT: &str = "OVERALL METHOD SETTINGS\nINCLUSION LIST\n| Mass [m/z] | Formula [M] | Species | CS [z] | Polarity | Start [min] | End [min] | (N)CE | Comment |\n| 301.1410 | C16H20O4 | [M+H]+ | 1 | Positive | 2.5 | 4,0 | 20, 35, 60 | compound one |\n| 415.2000 |  | [M+Na]+ | 1 | Negative | 5.0 | 7.5 | 30 |  |\n\nNEXT SECTION | with | bars\n";

#[test]
fn the_table_after_the_title_gives_one_entry_per_row() {
    let l = parse(TEXT).expect("a list");
    assert_eq!(l.len(), 2);
    assert_eq!(l[0].mz, 301.141);
    assert_eq!(l[0].formula, "C16H20O4");
    assert_eq!(l[0].species, "[M+H]+");
    assert_eq!(l[0].charge, Some(1.0));
    assert_eq!(l[0].polarity, Some("positive"));
    assert_eq!((l[0].start_min, l[0].end_min), (Some(2.5), Some(4.0)));
    assert_eq!(l[0].nce, vec![20.0, 35.0, 60.0]); // the steps, not their mean
    assert_eq!(l[0].name, "compound one");
    assert_eq!(
        (l[1].polarity, l[1].nce.clone()),
        (Some("negative"), vec![30.0])
    );
    assert!(l[1].formula.is_empty() && l[1].name.is_empty());
}

#[test]
fn tab_separated_cells_work_too_and_a_text_without_the_title_gives_none() {
    let t = "INCLUSION LIST\nMass [m/z]\tStart [min]\n100.5\t1\n";
    assert_eq!(parse(t).map(|l| l[0].start_min), Some(Some(1.0)));
    assert!(parse("Mass [m/z] | Start\n100 | 1 |\n").is_none());
    assert!(parse("INCLUSION LIST\nno table here\n").is_none());
}

#[test]
fn a_cell_that_is_not_a_number_ends_the_table() {
    let t = "INCLUSION LIST\n| Mass |\n| 100 |\n| text |\n| 200 |\n";
    assert_eq!(parse(t).map(|l| l.len()), Some(1));
}

#[test]
fn number_lists() {
    assert_eq!(number_list("20, 35, 60"), vec![20.0, 35.0, 60.0]);
    assert_eq!(number_list("27,5"), vec![27.5]); // a comma without a blank is a decimal comma
    assert_eq!(number_list("20; 35"), vec![20.0, 35.0]);
    assert!(number_list("abc").is_empty());
}
