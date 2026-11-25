//! The roles proposed from file names with the lab conventions (`MAT_COND_Ttt` with underscores or dashes, dark controls, dark matrix blanks, standards, QC,
//! blanks and washes): a dark control is never a time zero.

use mzlab_core::samples::{propose, Sample};

fn table(names: &[&str]) -> Vec<Sample> {
    let files: Vec<_> = names
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    propose(&files)
}

#[test]
fn roles_of_the_lab_names() {
    #[rustfmt::skip]
    let rows: [(&str, &str, Option<f64>); 17] = [
        ("MQ-H2O2-T00", "t0", Some(0.0)), ("MQ_H2O2_T00", "t0", Some(0.0)),
        ("MQ_H2O2_T30", "sample", Some(30.0)), ("MQ-H2O2-T30", "sample", Some(30.0)),
        // dark controls: suffix D on the condition or apart; the time is kept, the role is not a time zero
        ("MQ_H2O2D_T00", "dark", Some(0.0)), ("MQ-H2O2D-T00", "dark", Some(0.0)), ("MQ_H2O2_D_T60", "dark", Some(60.0)),
        ("MQ-PMS-D-T60", "dark", Some(60.0)), ("TW_PMSD_T30", "dark", Some(30.0)),
        // dark matrix blanks
        ("TW_BLKD_T00", "dark", Some(0.0)), ("TW-BLKD-T00", "dark", Some(0.0)),
        ("MQ_STD05", "std", None), ("QC_MQ", "qc", None), ("MQ_BLK_1", "blank", None), ("MQ_Wash_1", "blank", None),
        ("MQ_dark_T10", "dark", Some(10.0)), ("MQ_T00_dark", "dark", Some(0.0)),
    ];
    for (name, kind, time) in rows {
        let c = table(&[name]).remove(0);
        assert_eq!((c.kind.as_str(), c.time_min), (kind, time), "{name}");
    }
}

#[test]
fn a_wash_is_a_blank_of_its_kind_and_a_dark_control_has_no_role_t0() {
    let t = table(&["MQ_Wash_1", "MQ_BLKD_T00", "MQ_H2O2_T00"]);
    assert_eq!(
        (t[0].kind.as_str(), t[0].blank_kind.as_deref()),
        ("blank", Some("wash"))
    );
    assert_eq!(t[1].role(), "dark");
    assert_eq!(t[2].role(), "t0");
}

#[test]
fn a_d_after_a_number_is_days_not_a_dark_mark() {
    for name in ["S 2 d", "S 3 d (2)", "S 1d", "S d1"] {
        let c = table(&[name]).remove(0);
        assert_eq!(c.kind, "sample", "{name}");
        assert!(c.time_min.is_some(), "{name}");
    }
}

#[test]
fn the_lone_d_is_read_from_the_stem_after_a_known_condition_only() {
    // an Agilent dataset folder, a folder called d, a d that follows a time or an unknown condition: none is a dark control, the t0 stays
    for name in [
        "MQ_H2O2_T00.d",
        "MQ_H2O2_T00.D",
        "data/d/MQ_H2O2_T00",
        "d/MQ_H2O2_T00.mzML",
    ] {
        let c = table(&[name]).remove(0);
        assert_eq!((c.kind.as_str(), c.time_min), ("t0", Some(0.0)), "{name}");
    }
    for name in ["mq_h2o2_t30_d", "x_d", "MQ_COND_D_T30", "S 2 d"] {
        let c = table(&[name]).remove(0);
        assert_ne!(c.kind, "dark", "{name}");
    }
    // the same rule for the role alone (the table roles of mzlab-feat and the finder) as for the table
    for (name, kind) in [
        ("MQ_H2O2_T00.d", "t0"),
        ("MQ_H2O2_D_T00", "dark"),
        ("MQ_COND_D_T00", "t0"),
    ] {
        assert_eq!(
            mzlab_core::samples::role_and_minutes(name).0,
            kind,
            "{name}"
        );
        assert_eq!(
            mzlab_core::samples::table_kind_of_name(name),
            kind,
            "{name}"
        );
    }
}

#[test]
fn the_evidence_of_a_lone_d_is_the_d_that_decided() {
    // a folder and a leading token `d1` hold a d too: the span is the d after the condition, in characters of the whole name
    let name = "D1_MQ_H2O2_D_T30";
    let files = [(name.to_string(), String::new(), String::new())];
    let p = &mzlab_core::samples::propose_explained(&files)[0];
    assert_eq!(p.sample.kind, "dark");
    let kind = p
        .positions
        .iter()
        .find(|x| x.field == "kind")
        .expect("kind position");
    assert_eq!(&name[kind.start..kind.end], "D");
    assert_eq!((kind.start, kind.end), (11, 12));
    let name = "d/MQ_H2O2_D_T30";
    let files = [(name.to_string(), String::new(), String::new())];
    let p = &mzlab_core::samples::propose_explained(&files)[0];
    let kind = p
        .positions
        .iter()
        .find(|x| x.field == "kind")
        .expect("kind position");
    assert_eq!((kind.start, kind.end), (10, 11));
}

/// The two dark words written as one token are a dark control with the whole token as evidence; a longer word that only contains them is not.
#[test]
fn a_dark_control_written_as_one_token_is_dark() {
    for (name, word) in [
        ("Smp_darkcontrol", "darkcontrol"),
        ("Smp_t30_darkCtrl", "darkCtrl"),
        ("Smp_Ctrl-dark_2", "Ctrl"),
        ("Smp_controldark", "controldark"),
    ] {
        let c = table(&[name]).remove(0);
        assert_eq!(c.kind, "dark", "{name}");
        let files = vec![(name.to_string(), String::new(), String::new())];
        let p = &mzlab_core::samples::propose_explained(&files)[0];
        let k = p.positions.iter().find(|x| x.field == "kind").expect(name);
        let chars: Vec<char> = name.chars().collect();
        assert_eq!(
            chars[k.start..k.end].iter().collect::<String>(),
            word,
            "{name}"
        );
    }
    for name in ["Smp_darkcontrolled", "Smp_undarkcontrol", "Smp_t30"] {
        assert_ne!(table(&[name]).remove(0).kind, "dark", "{name}");
    }
}

/// `order_for_roles`: the same files give the text of `order_like`; a sample the table lacks is left out; a table file with no sample takes its name's role.
#[test]
fn the_pairing_for_roles_never_stops_on_a_difference() {
    use mzlab_core::samples::{order_for_roles, order_like};
    let doc = mzlab_core::samples::to_json(&table(&["a_t0.mzML", "a_t10.mzML", "a_std.mzML"]));
    let same: Vec<String> = ["a_t10.mzML", "a_t0.mzML", "a_std.mzML"]
        .map(String::from)
        .to_vec();
    let p = order_for_roles(&doc, &same).unwrap();
    assert_eq!(
        (p.text, p.notes.len()),
        (order_like(&doc, &same).unwrap(), 0)
    );
    let fewer: Vec<String> = ["a_t0.mzML", "a_t10.mzML"].map(String::from).to_vec();
    assert!(order_like(&doc, &fewer).is_err());
    let p = order_for_roles(&doc, &fewer).unwrap();
    assert_eq!(
        p.notes
            .iter()
            .map(|n| (n.action, n.file.as_str(), n.role.as_str()))
            .collect::<Vec<_>>(),
        [("left_out", "a_std.mzML", "std")]
    );
    assert_eq!(
        mzlab_core::samples::from_json(p.text.as_bytes())
            .unwrap()
            .len(),
        2
    );
    let more: Vec<String> = [
        "a_t0.mzML",
        "a_t10.mzML",
        "a_std.mzML",
        "a_blank.mzML",
        "a_run.mzML",
    ]
    .map(String::from)
    .to_vec();
    let p = order_for_roles(&doc, &more).unwrap();
    let got: Vec<_> = p
        .notes
        .iter()
        .map(|n| {
            (
                n.action,
                n.file.as_str(),
                n.role.as_str(),
                n.evidence.as_str(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("proposed", "a_blank.mzML", "blank", "blank"),
            ("proposed", "a_run.mzML", "sample", "")
        ]
    );
    // a document that is not a samples/1 is still refused
    assert!(order_for_roles("{\"schema\":\"x\"}", &more).is_err());
}

/// The lenient pairing edits the document as it was written, like the strict `order_like`: a field the model does not know stays, in the document and in each row.
#[test]
fn the_lenient_pairing_keeps_the_fields_it_does_not_know() {
    use mzlab_core::json::parse;
    use mzlab_core::samples::{order_for_roles, to_json};
    let doc = to_json(&table(&["a_t0.mzML", "a_t10.mzML", "a_std.mzML"]))
        .replacen('{', "{\"lab_note\":\"kept\",", 1)
        .replacen("\"file\":", "\"free_text\":\"row\",\"file\":", 1);
    let names: Vec<String> = ["a_t10.mzML", "a_t0.mzML", "a_dark.mzML"]
        .map(String::from)
        .to_vec();
    let p = order_for_roles(&doc, &names).unwrap();
    let v = parse(p.text.as_bytes()).unwrap();
    assert_eq!(v.get("lab_note").and_then(|x| x.str()), Some("kept"));
    let rows = v.get("samples").unwrap().arr();
    let files: Vec<_> = rows
        .iter()
        .map(|r| r.get("file").and_then(|x| x.str()).unwrap())
        .collect();
    assert_eq!(files, ["a_t10.mzML", "a_t0.mzML", "a_dark.mzML"]);
    let kept = rows
        .iter()
        .filter(|r| r.get("free_text").is_some())
        .count();
    assert_eq!(kept, 1, "the extra field of the row that is still there");
    assert_eq!(rows[2].get("kind").and_then(|x| x.str()), Some("dark"));
}

/// The evidence of a proposal names the word that decided the role for every role a word gives, and the `pairing` array of a result has stable keys.
#[test]
fn the_pairing_array_has_stable_keys_and_evidence_for_every_word_role() {
    use mzlab_core::samples::{notes_json, order_for_roles, to_json, with_pairing};
    let doc = to_json(&table(&["a_t0.mzML"]));
    let names: Vec<String> = ["a_t0.mzML", "a_infusion.mzML", "a_cal.mzML"]
        .map(String::from)
        .to_vec();
    let p = order_for_roles(&doc, &names).unwrap();
    let got: Vec<_> = p
        .notes
        .iter()
        .map(|n| (n.role.as_str(), n.evidence.as_str()))
        .collect();
    assert_eq!(got, [("infusion", "infusion"), ("calibration", "cal")]);
    assert_eq!(
        notes_json(&p.notes),
        "[{\"action\":\"proposed\",\"file\":\"a_infusion.mzML\",\"role\":\"infusion\",\"evidence\":\"infusion\"},{\"action\":\"proposed\",\"file\":\"a_cal.mzML\",\"role\":\"calibration\",\"evidence\":\"cal\"}]"
    );
    // no note: the result keeps its bytes; notes: the key goes at the end
    assert_eq!(with_pairing("{\"a\":1}", &[]), "{\"a\":1}");
    assert_eq!(
        with_pairing("{\"a\":1}", &p.notes),
        format!("{{\"a\":1,\"pairing\":{}}}", notes_json(&p.notes))
    );
}
