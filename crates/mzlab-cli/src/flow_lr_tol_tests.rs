//! The m/z tolerances of a plan on the unit axis are in Da (`tol_da`, `dup_da`): a ppm value there is `err.flow.param.axis`, never a silent overwrite of
//! the Da window of the recipe; on the ppm axis `tol_da` is the same error and the bytes of the ppm tolerances are untouched.

use std::path::{Path, PathBuf};

use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{read_table_from, write_table, Origin, Slice};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const NAMES: [&str; 5] = [
    "FullScan_t0.mzML",
    "FullScan_t5.mzML",
    "FullScan_t10.mzML",
    "FullScan_t15.mzML",
    "FullScan_t30.mzML",
];

fn fixture(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_lrtol_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi");
    for n in NAMES {
        std::fs::copy(src.join(n), d.join(n)).unwrap();
    }
    d
}

/// Detect and align the demo files with `params` on the alignment; the table bytes or the key of the error.
fn align(d: &Path, params: &str) -> Result<Vec<u8>, String> {
    let raw: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","type":"raw"}}"#))
        .collect();
    let src: Vec<String> = NAMES.iter().map(|n| format!(r#""file:{n}""#)).collect();
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{}]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}},"params":{params}}}]}}"#,
        raw.join(","),
        src.join(",")
    );
    let _ = std::fs::remove_file(d.join("work/ali.mzfeat"));
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 2, None)
        .map_err(|e| e.key.to_string())?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read(d.join("work/ali.mzfeat")).unwrap())
}

fn rows(table: &[u8]) -> usize {
    read_table_from(&mut Slice(table), table.len() as u64)
        .unwrap()
        .0
        .len()
}

#[test]
fn a_ppm_tolerance_on_a_unit_axis_file_is_an_error_and_tol_da_is_the_recipe_window() {
    let d = fixture("align");
    let base = align(&d, "{}").unwrap();
    for p in ["ppm", "max_ppm_span"] {
        let e = align(&d, &format!(r#"{{"{p}":5}}"#)).unwrap_err();
        assert!(e.contains("err.flow.param.axis"), "{p}: {e}");
        assert!(e.contains(p) && e.contains("Da"), "{e}");
    }
    // the Da window of the recipe written out is the recipe, byte for byte
    let same = align(&d, r#"{"tol_da":0.35}"#).unwrap();
    assert!(same == base, "tol_da 0.35 differs from the recipe default");
    // a narrower window splits more rows
    let tight = align(&d, r#"{"tol_da":0.05}"#).unwrap();
    assert!(
        rows(&tight) >= rows(&base),
        "{} < {}",
        rows(&tight),
        rows(&base)
    );
    assert!(tight != base);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_unit_axis_table_filters_duplicates_in_da_and_refuses_ppm() {
    let d = std::env::temp_dir().join(format!("mzlab_cli_lrtol_dup_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    // 300.1 and 300.3 (0.2 Da apart, same RT) are one ion twice; 310.2 is another
    let mut t = Aligned::with_capacity(1, 3);
    for (mz, area) in [(300.1, 5e5), (300.3, 9e5), (310.2, 7e5)] {
        t.mz.push(mz);
        t.rt.push(6.0);
        t.area.push(area);
        t.height.push((area / 10.0) as f32);
        t.apex_rt.push(6.0);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    let table = write_table(&t, &Recipe::lr(0.1, 3e3, 7e3), &Origin::default());
    std::fs::write(d.join("table.mzfeat"), &table).unwrap();
    let samples = mzlab_feat::samples::propose(&[("s.mzML".to_string(), "00".repeat(32))]);
    std::fs::write(
        d.join("samples.json"),
        mzlab_feat::samples::to_json(&samples),
    )
    .unwrap();
    let run = |params: &str| {
        let plan = format!(
            r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}},{{"name":"samples.json","type":"samples"}}],"steps":[
            {{"id":"f","step":"table.filter@1","in":{{"table":"file:table.mzfeat","samples":"file:samples.json"}},"params":{params}}}]}}"#
        );
        let (run, prov) = crate::flow::run_plan(plan.as_bytes(), &d, &d.join("work"), 1, None)
            .map_err(|e| e.key.to_string())?;
        if !run.skipped.is_empty() || prov.contains("\"error\"") {
            return Err(prov);
        }
        Ok(std::fs::read_to_string(d.join("work/f.json")).unwrap())
    };
    let kept = run(r#"{"dup_da":0.3}"#).unwrap();
    assert!(kept.contains("\"tenute\":2"), "{kept}");
    let none = run(r#"{"dup_da":0.1}"#).unwrap();
    assert!(none.contains("\"tenute\":3"), "{none}");
    let e = run(r#"{"dup_ppm":10}"#).unwrap_err();
    assert!(
        e.contains("err.flow.param.axis") && e.contains("dup_ppm"),
        "{e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
