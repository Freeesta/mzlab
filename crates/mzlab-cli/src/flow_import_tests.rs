//! `import.raw` and `index.mzidx` on vendor datasets: the import hands the input on unchanged, a `.wiff` + `.wiff.scan` pair indexes one sample per
//! input `NAME@ID`, an unknown sample and a file no reader takes are errors with their key.

use crate::convert_fixtures::{folder, makers, wiff};
use crate::flow_run_tests::{flow_run, p};

const WIFF: &str = r#"{"format":"mzflow/2","inputs":[{"name":"c.wiff@1","type":"raw"},{"name":"c.wiff@2","type":"raw"}],"steps":[
{"id":"imp","step":"import.raw@1","in":{"files":["file:c.wiff@1","file:c.wiff@2"]}},
{"id":"idx","step":"index.mzidx@1","in":{"source":["imp.raw"]}}]}"#;

#[test]
fn a_wiff_with_two_samples_gives_one_index_per_sample() {
    let dir = folder("import_wiff");
    let src = dir.join("in");
    std::fs::create_dir_all(&src).unwrap();
    if !wiff(&src, "c", 2) {
        return;
    }
    let plan = dir.join("plan.json");
    std::fs::write(&plan, WIFF).unwrap();
    let out = dir.join("out");
    let o = flow_run(&[p(&plan), "--in", p(&src), "--out", p(&out)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let (a, b) = (
        std::fs::read(out.join("idx-0.mzidx")).unwrap(),
        std::fs::read(out.join("idx-1.mzidx")).unwrap(),
    );
    assert_ne!(a, b, "two samples, two indexes");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sample_the_dataset_lacks_and_a_file_no_reader_takes_are_errors() {
    let dir = folder("import_bad");
    let src = dir.join("in");
    std::fs::create_dir_all(&src).unwrap();
    if !wiff(&src, "c", 1) {
        return;
    }
    let plan = dir.join("plan.json");
    std::fs::write(&plan, WIFF.replace("c.wiff@2", "c.wiff@9")).unwrap();
    let o = flow_run(&[p(&plan), "--in", p(&src), "--out", p(&dir.join("o1"))]);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("failed imp"), "{}", o.stderr);

    let broken = makers(&dir, true);
    let plan = dir.join("plan2.json");
    let name = "rotto.raw";
    let text = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"{name}","type":"raw"}}],"steps":[{{"id":"imp","step":"import.raw@1","in":{{"files":["file:{name}"]}}}}]}}"#
    );
    std::fs::write(&plan, text).unwrap();
    let o = flow_run(&[p(&plan), "--in", p(&broken), "--out", p(&dir.join("o2"))]);
    assert_ne!(o.code, 0, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}
