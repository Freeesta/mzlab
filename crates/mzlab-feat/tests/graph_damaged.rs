//! A damaged molfile of the drawing is `error.grafo.molfile`, never a panic: in the page a panic is a trap of the `.wasm` that takes the module down.
use mzlab_feat::chem::cut::cut_json;

const ETHANOL: &str = "\n  t\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    2.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\n  2  3  1  0\nM  END\n";

fn cut(molfile: &str) -> Result<String, &'static str> {
    let request = format!(
        "{{\"molfile\":{},\"bonds\":[[0,1]],\"mode\":\"homolytic\"}}",
        serde_json::to_string(molfile).unwrap()
    );
    cut_json(&request).map_err(|e| e.key)
}

#[test]
fn the_whole_file_is_cut() {
    assert!(cut(ETHANOL).unwrap().starts_with("{\"schema\":\"cut/1\""));
}

#[test]
fn negative_counts_are_a_molfile_error() {
    for counts in ["  3 -1", " -1  2", " -3 -2", "  3-99"] {
        let text = ETHANOL.replace("  3  2  0  0", &format!("{counts}  0  0"));
        assert_eq!(cut(&text), Err("error.grafo.molfile"), "{counts:?}");
    }
}
