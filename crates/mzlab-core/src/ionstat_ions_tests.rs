//! `peak noise --ions` (`ionstat_ions`) on synthetic ion-count traces of a known quantum (2000 cps per ion): every scan type gives the factor of its
//! method fields, the data estimate stands in only when the method gives nothing, and the cps members never change.

use super::*;
use crate::ionstat::floor_json;
use crate::json::parse;

/// 300 points every 0.02 min: counts `bg` ions at every 7th point, a peak of 1, 2 and 1 ions at 150..153, times `q` cps per ion.
fn trace(q: f64) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..300).map(|i| i as f64 * 0.02).collect();
    let mut c = vec![0.0; 300];
    for i in (0..300).step_by(7) {
        c[i] = 1.0;
    }
    c[150..153].copy_from_slice(&[1.0, 2.0, 1.0]);
    (rt, c.iter().map(|k| k * q).collect())
}

fn ask() -> FloorAsk {
    FloorAsk {
        from: None,
        to: None,
        peak: Some((2.9, 3.1)),
    }
}

fn run(q: f64, kind: Option<ScanKind>, facts: Facts) -> (FloorResult, IonsOut) {
    let (rt, y) = trace(q);
    let (r, i) = measure_ions(
        &rt,
        &y,
        ask(),
        FloorParams::default(),
        Some(IonsAsk { kind, facts }),
    )
    .unwrap();
    (r, i.unwrap())
}

/// The fields of each scan type that give 0.0005 s per ion = 1 / 2000 cps.
fn facts_of(kind: ScanKind) -> Facts {
    match kind {
        ScanKind::Ems | ScanKind::Epi => Facts {
            fill_ms: Some(0.5),
            ..Facts::default()
        },
        ScanKind::Sim => Facts {
            dwell_ms: Some(0.5),
            ..Facts::default()
        },
        ScanKind::Q1 => Facts {
            scan_s: Some(0.5),
            step_da: Some(0.1),
            mass_lo: Some(100.0),
            mass_hi: Some(200.0),
            ..Facts::default()
        },
        ScanKind::Other => Facts::default(),
    }
}

#[test]
fn each_scan_type_gives_the_factor_of_its_method_fields() {
    for kind in [ScanKind::Ems, ScanKind::Epi, ScanKind::Sim, ScanKind::Q1] {
        let (_, i) = run(2000.0, Some(kind), facts_of(kind));
        let f = i.factor.as_ref().unwrap();
        assert_eq!(f.source, ionscale::Source::Method, "{kind:?}");
        assert!((f.seconds - 0.0005).abs() < 1e-12, "{kind:?} {}", f.seconds);
        assert_eq!(
            f.field,
            match kind {
                ScanKind::Ems | ScanKind::Epi => "fill_ms",
                ScanKind::Sim => "dwell_ms",
                _ => "scan_s,step_da,mass_lo,mass_hi",
            }
        );
        // the data estimate agrees: the trace is made of 2000-cps ions
        assert_eq!(f.ratio().map(|r| round(r, 4)), Some(1.0), "{kind:?}");
        let pk = i.at_factor.as_ref().unwrap().peak.as_ref().unwrap();
        assert!((pk.height * f.seconds - 2.0).abs() < 1e-9, "{kind:?}");
    }
}

#[test]
fn a_method_time_is_never_replaced_by_the_data_one() {
    // 1 ms per ion = 1000 cps per ion, the data say 2000: the method wins, the data estimate and the ratio are next to it
    let f = Facts {
        fill_ms: Some(1.0),
        ..Facts::default()
    };
    let (r, i) = run(2000.0, Some(ScanKind::Ems), f);
    let fac = i.factor.as_ref().unwrap();
    assert_eq!(fac.source, ionscale::Source::Method);
    assert_eq!(fac.seconds, 0.001);
    assert_eq!(fac.ratio().map(|r| round(r, 4)), Some(2.0));
    // the floor at 1000 cps per ion is half the one at 2000: the S/N at the factor doubles, the cps S/N is the data one
    let at = i.at_factor.as_ref().unwrap();
    assert_eq!(at.floor.one_ion, Some(1000.0));
    assert_eq!(r.floor.one_ion, Some(2000.0));
    let (s_cps, s_ions) = (
        r.peak.as_ref().unwrap().sn.unwrap(),
        at.peak.as_ref().unwrap().sn.unwrap(),
    );
    assert!((s_ions / s_cps - 2.0).abs() < 1e-9, "{s_cps} {s_ions}");
}

#[test]
fn without_method_values_the_data_estimate_is_used_and_logged() {
    let (_, i) = run(2000.0, None, Facts::default());
    let f = i.factor.as_ref().unwrap();
    assert_eq!(
        (f.source, f.field, f.missing),
        (ionscale::Source::Data, "ion_quantum", Some("scan_type"))
    );
    assert!((f.seconds - 0.0005).abs() < 1e-12);
    assert!(ions_log(&i).contains("data estimate"), "{}", ions_log(&i));
    let (_, j) = run(2000.0, Some(ScanKind::Ems), Facts::default());
    assert_eq!(j.factor.as_ref().unwrap().missing, Some("fill_ms"));
}

#[test]
fn no_method_and_no_accepted_quantum_leave_the_ion_scale_null_and_say_so() {
    // integer analog data in the thousands: the quantum is rejected (level > 100 steps), no method value either
    let rt: Vec<f64> = (0..300).map(|i| i as f64 * 0.02).collect();
    let y: Vec<f64> = (0..300).map(|i| 1000.0 + (i % 5) as f64 * 37.0).collect();
    let (r, i) = measure_ions(
        &rt,
        &y,
        ask(),
        FloorParams::default(),
        Some(IonsAsk::default()),
    )
    .unwrap();
    let i = i.unwrap();
    assert!(i.factor.is_none() && i.at_factor.is_none());
    let j = parse(floor_json_ions(&r, Some(&i)).as_bytes()).unwrap();
    let o = j.get("ions").unwrap();
    assert!(o.get("ion_scale").unwrap().is_null() && o.get("noise_ions").unwrap().is_null());
    assert!(o
        .get("ion_scale_choice")
        .unwrap()
        .get("evidence")
        .unwrap()
        .str()
        .unwrap()
        .starts_with("none:"));
    assert!(
        ions_log(&i).starts_with("ion scale: none"),
        "{}",
        ions_log(&i)
    );
}

#[test]
fn the_cps_members_keep_their_bytes_and_the_ions_member_is_appended() {
    let (rt, y) = trace(2000.0);
    let p = FloorParams::default();
    let (r, none) = measure_ions(&rt, &y, ask(), p, None).unwrap();
    assert!(none.is_none());
    assert_eq!(floor_json_ions(&r, None), floor_json(&r));
    let (r2, i) = measure_ions(
        &rt,
        &y,
        ask(),
        p,
        Some(IonsAsk {
            kind: Some(ScanKind::Ems),
            facts: facts_of(ScanKind::Ems),
        }),
    )
    .unwrap();
    let with = floor_json_ions(&r2, i.as_ref());
    let base = floor_json(&r);
    assert!(with.starts_with(&base[..base.len() - 1]), "{with}");
    let v = parse(with.as_bytes()).unwrap();
    let o = v.get("ions").unwrap();
    assert_eq!(
        o.get("ion_scale").unwrap().get("scan_type").unwrap().str(),
        Some("ems")
    );
    assert_eq!(
        o.get("ion_scale_choice")
            .unwrap()
            .get("scan_type_from")
            .unwrap()
            .str(),
        Some("scan_type given")
    );
    assert_eq!(o.get("one_ion_cps").unwrap().f64(), Some(2000.0));
    assert_eq!(
        o.get("peak").unwrap().get("height_ions").unwrap().f64(),
        Some(2.0)
    );
    assert_eq!(
        o.get("check").unwrap().get("id").unwrap().str(),
        Some("noise_floor")
    );
    assert_eq!(o.get("noise_ions").unwrap().f64(), Some(1.0));
}

#[test]
fn the_wasm_numbers_are_the_same_ask() {
    assert_eq!(IonsAsk::from_slice(&[0.0, 0.0, 0.0]), None);
    assert_eq!(IonsAsk::from_slice(&[0.0, 1.0, 1.0, 0.0]), None);
    let nan = f64::NAN;
    let a = IonsAsk::from_slice(&[0.0, 1.0, 1.0, 1.0, 4.0, nan, 0.5, nan, nan, nan, nan]).unwrap();
    assert_eq!(a.kind, Some(ScanKind::Sim));
    assert_eq!(
        a.facts,
        Facts {
            dwell_ms: Some(0.5),
            ..Facts::default()
        }
    );
    let b = IonsAsk::from_slice(&[0.0, 1.0, 1.0, 1.0]).unwrap();
    assert_eq!((b.kind, b.facts), (None, Facts::default()));
}
