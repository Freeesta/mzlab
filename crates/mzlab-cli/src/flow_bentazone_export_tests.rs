//! The export of a table the plan detected itself: the bytes are the ones `table_file` (the entry the page asks of the table `.wasm`) writes for the
//! same table, samples and polarity; mzTab-M names the polarity the files decide when the plan leaves it out.

use crate::flow_bentazone_tests::{fixture, plan, run};
use mzlab_core::budget::sha256_hex;
use mzlab_feat::store::Slice;
use mzlab_feat_table_wasm::core::table_file;

fn exported(tag: &str, pol: i8) -> (std::path::PathBuf, [String; 2]) {
    let d = fixture(tag, pol, true);
    let base = plan(&|_| String::new(), false);
    let base = base.strip_suffix("]}").unwrap();
    let text = format!(
        r#"{base},{{"id":"mzt","step":"export.mztab@1","in":{{"table":"gap.table","samples":"file:samples.json"}}}},
        {{"id":"csv","step":"export.table_csv@1","in":{{"table":"gap.table","samples":"file:samples.json"}}}}]}}"#
    );
    run(&d, &text).unwrap();
    let read = |f: &str| std::fs::read_to_string(d.join(f)).unwrap();
    let out = [read("work/mzt.mzTab"), read("work/csv.csv")];
    (d, out)
}

#[test]
fn the_export_of_a_plan_is_the_file_of_the_page_and_names_the_polarity() {
    for (pol, word) in [(-1i8, "negative scan"), (1, "positive scan")] {
        let (d, [mzt, csv]) = exported(&format!("exp{pol}"), pol);
        let table = std::fs::read(d.join("work/gap.mzfeat")).unwrap();
        let samples = std::fs::read(d.join("samples.json")).unwrap();
        let samples = crate::flow_table::samples_text(&samples).unwrap();
        let sha = sha256_hex(&table);
        let page = |kind: &str| {
            table_file(
                &mut Slice(&table),
                table.len() as u64,
                &samples,
                "",
                &sha,
                i32::from(pol),
                kind,
            )
            .unwrap()
        };
        assert_eq!(csv, page("full"), "polarity {pol}");
        assert_eq!(mzt, page("mztab"), "polarity {pol}");
        assert!(mzt.contains(word), "polarity {pol}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
