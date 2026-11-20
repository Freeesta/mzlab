//! A damaged molfile is an error with its reason, never a panic: in the page a panic is a trap of the `.wasm` that takes the module down.
use mzlab_mol::Mol;

const V2000: &str = "\n  t\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\nM  END\n";

const V3000: &str = "\n  t\n\n  0  0  0  0  0  0  0  0  0  0999 V3000\nM  V30 BEGIN CTAB\nM  V30 COUNTS 2 1 0 0 0\nM  V30 BEGIN ATOM\nM  V30 1 C 0 0 0 0\nM  V30 2 O 1 0 0 0\nM  V30 END ATOM\nM  V30 BEGIN BOND\nM  V30 1 1 1 2\nM  V30 END BOND\nM  V30 END CTAB\nM  END\n";

fn reason(text: &str) -> String {
    Mol::from_molfile(text).unwrap_err().to_string()
}

#[test]
fn the_two_files_read_when_whole() {
    assert_eq!(Mol::from_molfile(V2000).unwrap().atoms.len(), 2);
    assert_eq!(Mol::from_molfile(V3000).unwrap().atoms.len(), 2);
}

#[test]
fn negative_counts_are_refused() {
    for counts in [" -1  1", "  2 -1", " -1 -1", "-99-99"] {
        let text = V2000.replace(
            "  2  1  0  0  0  0  0  0  0  0999",
            &format!("{counts}  0  0  0  0  0  0  0  0999"),
        );
        assert!(reason(&text).contains("negative count"), "{counts:?}");
    }
}

#[test]
fn a_negative_count_of_a_property_line_is_refused_with_its_reason() {
    for line in [
        "M  CHG -1   1   1",
        "M  ISO -3   1  13",
        "M  RAD -1   2   2",
    ] {
        let text = V2000.replace("M  END", &format!("{line}\nM  END"));
        assert!(reason(&text).contains("negative count"), "{line:?}");
    }
    // a count of zero says nothing and is read
    let text = V2000.replace("M  END", "M  CHG  0\nM  END");
    assert_eq!(Mol::from_molfile(&text).unwrap().atoms.len(), 2);
}

#[test]
fn an_empty_line_inside_a_v3000_block_is_refused() {
    for block in ["BEGIN ATOM\n", "BEGIN BOND\n"] {
        let text = V3000.replace(block, &format!("{block}M  V30 \n"));
        assert!(reason(&text).contains("too short"), "{block:?}");
    }
    // an empty line of a collection block says nothing and is passed over
    let text = V3000.replace(
        "M  V30 END BOND\n",
        "M  V30 END BOND\nM  V30 BEGIN COLLECTION\nM  V30 \nM  V30 END COLLECTION\n",
    );
    assert_eq!(Mol::from_molfile(&text).unwrap().atoms.len(), 2);
}
