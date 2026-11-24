//! `samples::import` on the CSV files that real exports produce: a BOM, CRLF line ends, quoted fields with commas and line breaks, repeated headers,
//! blank lines and Windows paths as keys. Each case must give the same table as its plain form; a row is a record of the CSV (the header is row 1).

use super::import::{run, sample, what};

const FILES: [&str; 2] = ["run_a.mzML", "run_b.mzML"];

/// The sample names and roles of the two files after the import.
fn names(text: &str) -> [(String, String); 2] {
    let d = run(&FILES, text);
    FILES.map(|f| {
        let s = sample(&d, f);
        (
            s["sample_id"].as_str().unwrap().to_string(),
            s["kind"].as_str().unwrap().to_string(),
        )
    })
}

fn plain() -> [(String, String); 2] {
    names("file,sample,role\nrun_a.mzML,S-1,dark\nrun_b.mzML,S-2,std\n")
}

#[test]
fn a_utf8_bom_does_not_spoil_the_first_header() {
    let d = run(
        &FILES,
        "\u{feff}file,sample,role\nrun_a.mzML,S-1,dark\nrun_b.mzML,S-2,std\n",
    );
    assert_eq!(d["report"]["status"], "complete");
    assert!(what(&d, "ignored_column").is_empty());
    assert_eq!(
        names("\u{feff}file,sample,role\nrun_a.mzML,S-1,dark\nrun_b.mzML,S-2,std\n"),
        plain()
    );
}

#[test]
fn crlf_and_a_missing_final_newline_read_like_lf() {
    let crlf = "file,sample,role\r\nrun_a.mzML,S-1,dark\r\nrun_b.mzML,S-2,std";
    assert_eq!(names(crlf), plain());
    assert_eq!(run(&FILES, crlf)["report"]["status"], "complete");
}

#[test]
fn a_quoted_field_keeps_its_comma_and_its_line_break() {
    let text =
        "file,sample,role\nrun_a.mzML,\"S-1, rep 1\nsecond line\",dark\nrun_b.mzML,S-2,std\n";
    let d = run(&FILES, text);
    assert_eq!(
        sample(&d, "run_a.mzML")["sample_id"],
        "S-1, rep 1\nsecond line"
    );
    assert_eq!(d["report"]["rows"], 2);
    assert_eq!(d["report"]["status"], "complete");
    // a row is a record, not a line: the line break inside the quotes does not add one (the header is row 1)
    let dup = run(&FILES, &format!("{text}run_b.mzML,S-9,std\n"));
    assert_eq!(
        dup["report"]["duplicates"][0]["rows"],
        serde_json::json!([3, 4])
    );
}

#[test]
fn a_repeated_header_uses_the_first_column_and_logs_the_other() {
    let d = run(
        &FILES,
        "file,sample,sample,role\nrun_a.mzML,S-1,OTHER,dark\nrun_b.mzML,S-2,OTHER,std\n",
    );
    assert_eq!(sample(&d, "run_a.mzML")["sample_id"], "S-1");
    let ignored = what(&d, "ignored_column");
    assert_eq!(ignored.len(), 1);
    assert_eq!(ignored[0]["from"], "sample");
}

#[test]
fn empty_lines_are_skipped_and_do_not_count_as_rows() {
    let text =
        "file,sample,role\n\nrun_a.mzML,S-1,dark\n,,\n\nrun_b.mzML,S-2,std\nrun_x.mzML,S-3,std\n";
    let d = run(&FILES, text);
    assert_eq!(d["report"]["rows"], 3);
    // `,,` has no value: it is skipped too, but it is a record, so it counts (header 1, run_a 2, `,,` 3, run_b 4, run_x 5)
    assert_eq!(d["report"]["unmatched_rows"][0]["row"], 5);
    assert_eq!(d["report"]["unmatched_rows"][0]["file"], "run_x.mzML");
    assert_eq!(sample(&d, "run_b.mzML")["sample_id"], "S-2");
}

#[test]
fn a_windows_or_posix_path_as_key_names_the_file_by_its_last_part() {
    let text =
        "file,sample,role\nC:\\data\\batch 1\\run_a.mzML,S-1,dark\n/mnt/data/run_b,S-2,std\n";
    let d = run(&FILES, text);
    assert_eq!(d["report"]["status"], "complete");
    assert_eq!(names(text), plain());
    // the second key has no extension: the match is by stem and is logged as such
    assert_eq!(what(&d, "match:stem").len(), 1);
}
