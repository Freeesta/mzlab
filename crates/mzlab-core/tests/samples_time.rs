//! The time point of a sample name: the spellings the parser reads (`t0`, `3h`, `1 h 30`, ...) and the names that only look like a time (`H20`, `80-20`, blanks).

use mzlab_core::samples::{proponi, Sample};

fn one(name: &str, kind: &str) -> Sample {
    proponi(&[(name.to_string(), "ab".to_string(), kind.to_string())]).remove(0)
}

/// The time point of a name (minutes), the names being generic labels: the spellings the parser knows and the ones it must leave alone.
#[test]
fn time_points_in_sample_names() {
    #[rustfmt::skip]
    let rows: [(&str, Option<f64>); 40] = [
        ("RET t0 (2)", Some(0.0)), ("RET t0_2", Some(0.0)), ("RET 3h_2", Some(180.0)), ("RET 8h (2)", Some(480.0)),
        ("PERM t0", Some(0.0)), ("PERM 3h", Some(180.0)), ("PERM 8h", Some(480.0)),
        ("T0", Some(0.0)), ("t=0", Some(0.0)), ("S t 0", Some(0.0)), ("S 0h", Some(0.0)), ("S 3 h", Some(180.0)),
        ("S T=3h", Some(180.0)), ("S 3hr", Some(180.0)), ("S 3 hr", Some(180.0)), ("S 8hrs", Some(480.0)), ("S 3 hours", Some(180.0)),
        ("S 30min", Some(30.0)), ("S 30 min", Some(30.0)), ("S 15 mins", Some(15.0)), ("S 15min", Some(15.0)), ("S 24h", Some(1440.0)),
        ("S 1d", Some(1440.0)), ("S 2 d", Some(2880.0)), ("S 2 days", Some(2880.0)), ("S d1", Some(1440.0)), ("S D2 (2)", Some(2880.0)),
        ("S 1 h 30", Some(90.0)), ("S 1h30", Some(90.0)), ("S 1h 30min", Some(90.0)), ("S (3h)", Some(180.0)), ("S_t0_3h", Some(180.0)),
        // a replicate after the hours is not minutes
        ("S 3h_15", Some(180.0)), ("S 3h 2", Some(180.0)), ("S 3h (2)", Some(180.0)),
        // an index and a replicate around the time
        ("Smp_01_30h_2", Some(1800.0)), ("S t30m", Some(30.0)), ("S 90 s", Some(1.5)), ("t5", Some(5.0)), ("S 45sec", Some(0.75)),
    ];
    for (name, min) in rows {
        assert_eq!(one(name, "").time_min, min, "{name}");
    }
    // not a time: the whole token must be one, a number alone is not
    for name in [
        "H20 demi",
        "H20 ricircolo",
        "PERM",
        "RET",
        "S 80-20",
        "S_80_20_1",
        "Run_5",
        "S 3hx",
        "S 5hello",
        "S 1h305",
        "S_75x",
        "S d",
        "S d1234",
        "S H2O 3",
        "S mix12",
        "S 2h2o",
        "xt5y",
        "S th3",
    ] {
        assert_eq!(one(name, "").time_min, None, "{name}");
    }
    // blanks carry no time, whatever they say
    for name in ["blk 90-10", "blk 3h", "blank 80-20"] {
        let c = one(name, "");
        assert_eq!((c.kind.as_str(), c.time_min), ("blank", None), "{name}");
    }
    // a time written with an `m` only after a `t`
    assert_eq!(one("S 30m", "").time_min, None);
    // the day first never beats a real time
    assert_eq!(one("S d3 8h", "").time_min, Some(480.0));
    // the written value and unit of the compound
    let c = one("S 1 h 30", "");
    assert_eq!((c.time, c.time_unit.as_str()), (Some(90.0), "min"));
}
