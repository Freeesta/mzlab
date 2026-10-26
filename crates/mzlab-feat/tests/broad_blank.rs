//! Broad-band rows against the blank: the rule `BroadBlank` needs the sample to be `ratio` times the blank only for rows marked `REASON_BROAD`;
//! the mark survives the table of the file and the quality summary does not count it as a failure.

use mzlab_feat::filters::{self, Rule};
use mzlab_feat::quality::REASON_BROAD;
use mzlab_feat::samples::propose;
use mzlab_feat::table::RowQ;
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

const NAMES: [&str; 3] = ["blank.mzML", "s1.mzML", "s2.mzML"];

/// Rows (areas of blank, s1, s2, broad): the band also in the blank, the band only in the samples, a narrow peak that looks like the first.
fn table(mark: bool) -> Aligned {
    let rows: [([f64; 3], bool); 3] = [
        ([4e5, 9e5, 8e5], true),
        ([1e5, 9e5, 8e5], true),
        ([4e5, 9e5, 8e5], false),
    ];
    let mut t = Aligned::with_capacity(3, rows.len());
    for (i, (areas, broad)) in rows.iter().enumerate() {
        t.mz.push(300.0 + i as f64);
        t.rt.push(5.0);
        for &a in areas {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(5.0);
            t.width.push(2.0);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
        if mark {
            t.meta.push(&RowQ {
                qreason: if *broad { REASON_BROAD } else { 0 },
                ..RowQ::NONE
            });
        }
    }
    t
}

fn samples() -> Vec<mzlab_feat::samples::Sample> {
    let files: Vec<(String, String)> = NAMES
        .iter()
        .map(|n| (n.to_string(), "00".repeat(32)))
        .collect();
    propose(&files)
}

#[test]
fn a_broad_band_that_is_also_in_the_blank_is_rejected() {
    let why = filters::apply(&table(true), &samples(), &[Rule::BroadBlank(3.0)]).unwrap();
    // 2.25 times the blank: out; 9 times: kept; the narrow peak is not judged by this rule
    assert_eq!(why, vec![1, 0, 0]);
    let loose = filters::apply(&table(true), &samples(), &[Rule::BroadBlank(2.0)]).unwrap();
    assert_eq!(loose, vec![0, 0, 0]);
}

#[test]
fn without_blank_or_summaries_nothing_changes() {
    let mut s = samples();
    s[0].kind = "sample".into();
    assert_eq!(
        filters::apply(&table(true), &s, &[Rule::BroadBlank(3.0)]).unwrap(),
        vec![0, 0, 0]
    );
    assert_eq!(
        filters::apply(&table(false), &samples(), &[Rule::BroadBlank(3.0)]).unwrap(),
        vec![0, 0, 0]
    );
}

#[test]
fn the_rule_round_trips_in_the_filtra_json() {
    let rules = vec![Rule::BroadBlank(3.0)];
    let text = filters::to_json(&rules);
    assert_eq!(text, "[{\"regola\":\"bianco_larga\",\"valore\":3}]");
    assert_eq!(
        filters::from_json(&mzlab_core::json::parse(text.as_bytes()).unwrap()).unwrap(),
        rules
    );
}
