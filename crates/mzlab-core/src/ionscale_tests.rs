use super::*;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

#[test]
fn a_trap_scan_takes_the_fill_time() {
    let f = Facts {
        fill_ms: Some(20.0),
        ..Facts::default()
    };
    for k in [ScanKind::Ems, ScanKind::Epi] {
        let x = factor(k, &f, None).unwrap();
        assert!(near(x.seconds, 0.02) && x.source == Source::Method && x.field == "fill_ms");
        assert!(near(x.cps_per_ion(), 50.0));
    }
}

#[test]
fn sim_takes_the_dwell_and_a_q1_scan_the_time_per_step() {
    let f = Facts {
        dwell_ms: Some(250.0),
        ..Facts::default()
    };
    assert!(near(factor(ScanKind::Sim, &f, None).unwrap().seconds, 0.25));
    // 100 Da in 2 s with 0.1 Da steps: 1000 steps, 2 ms each
    let q1 = Facts {
        scan_s: Some(2.0),
        step_da: Some(0.1),
        mass_lo: Some(100.0),
        mass_hi: Some(200.0),
        ..Facts::default()
    };
    let x = factor(ScanKind::Q1, &q1, None).unwrap();
    assert!(near(x.seconds, 0.002) && x.field == "scan_s,step_da,mass_lo,mass_hi");
}

#[test]
fn without_the_method_the_data_estimate_is_used_and_says_why() {
    let x = factor(ScanKind::Ems, &Facts::default(), Some(200.0)).unwrap();
    assert!(x.source == Source::Data && near(x.seconds, 0.005) && x.missing == Some("fill_ms"));
    assert!(x.ratio().is_none());
    let j = x.json();
    assert!(
        j.contains("\"source\":\"data\"")
            && j.contains("\"missing\":\"fill_ms\"")
            && j.contains("\"agree\":null"),
        "{j}"
    );
    // a kind the method cannot give and no quantum: no factor
    assert!(factor(ScanKind::Other, &Facts::default(), None).is_none());
    assert!(factor(
        ScanKind::Sim,
        &Facts {
            dwell_ms: Some(0.0),
            ..Facts::default()
        },
        None
    )
    .is_none());
}

#[test]
fn the_method_wins_and_the_data_is_shown_beside_it() {
    let f = Facts {
        dwell_ms: Some(5.0),
        ..Facts::default()
    };
    let same = factor(ScanKind::Sim, &f, Some(200.0)).unwrap();
    assert!(same.source == Source::Method && near(same.ratio().unwrap(), 1.0));
    assert!(same.json().contains("\"agree\":true"));
    let off = factor(ScanKind::Sim, &f, Some(100.0)).unwrap();
    assert!(near(off.ratio().unwrap(), 0.5) && off.json().contains("\"agree\":false"));
    assert!(
        near(off.seconds, 0.005),
        "the method time is kept, never replaced by the data one"
    );
}

#[test]
fn a_q1_range_that_is_not_a_range_is_missing() {
    let f = Facts {
        scan_s: Some(1.0),
        step_da: Some(0.1),
        mass_lo: Some(200.0),
        mass_hi: Some(200.0),
        ..Facts::default()
    };
    assert!(factor(ScanKind::Q1, &f, None).is_none());
    assert_eq!(
        factor(ScanKind::Q1, &f, Some(10.0)).unwrap().missing,
        Some("mass_hi")
    );
}

#[test]
fn the_kind_of_a_level_is_its_commonest_experiment() {
    let k = |v: &[(Option<&'static str>, &'static str)]| ScanKind::of(v.iter().copied());
    assert_eq!(
        k(&[(Some("EMS"), ""), (Some("EMS"), ""), (Some("EPI"), "")]),
        ScanKind::Ems
    );
    assert_eq!(k(&[(Some("PI"), ""), (Some("EPI"), "")]), ScanKind::Epi);
    assert_eq!(k(&[(Some("Q1"), "")]), ScanKind::Q1);
    assert_eq!(k(&[(None, "+ SIM ms [100-101]")]), ScanKind::Sim);
    assert_eq!(k(&[(None, "+ Full ms")]), ScanKind::Other);
    assert_eq!(ScanKind::parse("q1"), Some(ScanKind::Q1));
    assert_eq!(ScanKind::parse("MS"), None);
}
