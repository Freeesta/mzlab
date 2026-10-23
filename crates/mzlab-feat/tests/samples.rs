//! `campioni/1`: the metadata proposed from the file names, the canonical JSON and its digest, the TSV, and the reader.

use mzlab_feat::samples::{digest, from_json, propose, to_json, to_tsv, KINDS};

fn files(names: &[&str]) -> Vec<(String, String)> {
    names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), format!("{i:064x}")))
        .collect()
}

const SERIES: [&str; 8] = [
    "TIM_TiO2_dark-adsorption.mzML",
    "TIM_TiO2_t000min.mzML",
    "TIM_TiO2_t005min.mzML",
    "TIM_TiO2_t005min_2.mzML",
    "TIM_TiO2_t030min.mzML",
    "blank_1.mzML",
    "QC_1.mzML",
    "STD_5ppm.mzML",
];

#[test]
fn types_times_and_replicates_come_from_the_names() {
    let s = propose(&files(&SERIES));
    let kinds: Vec<&str> = s.iter().map(|x| x.kind.as_str()).collect();
    assert_eq!(
        kinds,
        ["dark", "t0", "sample", "sample", "sample", "blank", "qc", "std"]
    );
    assert_eq!(s[1].time_min, Some(0.0));
    assert_eq!(s[2].time_min, Some(5.0));
    assert_eq!((s[2].replicate, s[3].replicate, s[4].replicate), (1, 2, 1));
    assert_eq!(s[7].conc, Some(5.0));
    assert_eq!(s[7].unit.as_deref(), Some("ppm"));
    assert!(s
        .iter()
        .enumerate()
        .all(|(i, x)| x.order == i as u32 + 1 && !x.istd));
    assert!(s.iter().all(|x| KINDS.contains(&x.kind.as_str())));
    assert_eq!(s[5].group, "blank");
}

#[test]
fn json_roundtrip_digest_and_edits() {
    let s = propose(&files(&SERIES));
    let text = to_json(&s);
    assert_eq!(from_json(text.as_bytes()).unwrap(), s);
    assert_eq!(to_json(&from_json(text.as_bytes()).unwrap()), text);
    assert_eq!(digest(&s), digest(&propose(&files(&SERIES))));
    let mut edited = s.clone();
    edited[2].kind = "qc".into();
    assert_ne!(digest(&edited), digest(&s));
}

#[test]
fn reader_refuses_wrong_documents() {
    let ok = to_json(&propose(&files(&SERIES[..2])));
    assert!(from_json(ok.replace("campioni/1", "campioni/9").as_bytes()).is_err());
    assert!(from_json(
        ok.replace("\"tipo\":\"t0\"", "\"tipo\":\"unknown\"")
            .as_bytes()
    )
    .is_err());
    assert!(from_json(ok.replace("\"replica\":1", "\"replica\":0").as_bytes()).is_err());
    assert!(from_json(b"not json").is_err());
}

#[test]
fn tsv_has_a_line_per_file() {
    let t = to_tsv(&propose(&files(&SERIES)));
    assert_eq!(t.lines().count(), SERIES.len() + 1);
    assert!(t.lines().all(|l| l.split('\t').count() == 11));
    assert!(t.starts_with("file\tsha256\ttipo\t"));
}
