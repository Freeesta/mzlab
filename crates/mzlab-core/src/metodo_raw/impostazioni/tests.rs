use super::*;
use crate::metodo_raw::script::script_pairs;

const TEXT: &str = "Chrom. peak width (FWHM): 15 [s]\nApex trigger: 2-4 [s]\nLoop count: 5\nIntensity threshold: 1.0e4\nMinimum AGC target: 8.00e3\nDynamic exclusion: 8.0 [s]\nExclude isotopes: on\nCharge states: 2, 3, 4\n(N)CE: 20, 35, 60\nIsolation window: 1.5 [m/z]\nUnrelated: 3\n";

#[test]
fn the_known_labels_give_numbers_or_text_as_written() {
    let s = parse(&[TEXT]);
    let by = |id: &str| s.iter().find(|x| x.id == id).cloned();
    assert_eq!(
        by("chrom_peak_width").map(|x| (x.numbers, x.unit)),
        Some((vec![15.0], "s".into()))
    );
    assert_eq!(by("apex_trigger").map(|x| x.text), Some("2-4".into())); // a range is no number
    assert_eq!(by("loop_count").map(|x| x.numbers), Some(vec![5.0]));
    assert_eq!(by("exclude_isotopes").map(|x| x.text), Some("on".into()));
    assert_eq!(
        by("charge_states").map(|x| x.numbers),
        Some(vec![2.0, 3.0, 4.0])
    );
    assert_eq!(by("nce").map(|x| x.numbers), Some(vec![20.0, 35.0, 60.0]));
    assert_eq!(by("isolation_window").map(|x| x.numbers), Some(vec![1.5]));
    assert_eq!(s.len(), 10); // "Unrelated" is not a setting of the list
    assert_eq!(parse(&[TEXT, TEXT]).len(), 10); // an exact repeat is not listed twice
}

fn exp(pairs: &[(&str, f64)]) -> V {
    let p: Vec<V> = pairs
        .iter()
        .filter_map(|(i, v)| obj(vec![("id", text(i)), ("valore", Some(V::Num(*v)))]))
        .collect();
    obj(vec![
        ("indice", Some(V::Num(2.0))),
        ("parametri", Some(V::Arr(p))),
    ])
    .unwrap()
}

#[test]
fn a_stepped_energy_is_the_list_of_its_steps_and_the_window_is_a_width() {
    let s = parse(&[TEXT]);
    let out = refine(
        vec![exp(&[
            ("NCE", 38.33),
            ("Isolation_low", 20000.0),
            ("Isolation_high", 1.5),
            ("Resolution", 17500.0),
        ])],
        &s,
    );
    let e = &out[0];
    assert_eq!(
        e.get("nce"),
        Some(&V::Arr(vec![V::Num(20.0), V::Num(35.0), V::Num(60.0)]))
    );
    assert_eq!(e.get("isolamento_mz"), Some(&V::Num(1.5)));
    let ids: Vec<String> = match e.get("parametri") {
        Some(V::Arr(a)) => a
            .iter()
            .filter_map(|p| match p.get("id") {
                Some(V::Str(i)) => Some(i.clone()),
                _ => None,
            })
            .collect(),
        _ => vec![],
    };
    assert_eq!(ids, vec!["Resolution".to_string()]);
}

#[test]
fn a_mean_that_is_not_the_mean_of_the_steps_is_left_alone() {
    let s = parse(&[TEXT]);
    let out = refine(vec![exp(&[("NCE", 30.0), ("Resolution", 17500.0)])], &s);
    assert!(out[0].get("nce").is_none() && out[0].get("isolamento_mz").is_none());
    assert_eq!(out[0], exp(&[("NCE", 30.0), ("Resolution", 17500.0)]));
}

#[test]
fn solvents_come_from_the_equate_lines_before_the_first_timed_row() {
    let sc = "---- Script ----\ninitial     Instrument Setup\n            PumpModule.Pump.%A1_Equate: \"Water + 0.1% FA\"\n            PumpModule.Pump.%B1_Equate: \"Acetonitrile\"\n0.000 [min] Run\n";
    let v = solvents(&script_pairs(sc)).expect("solvents");
    let V::Arr(a) = v else { panic!() };
    assert_eq!(a.len(), 2);
    assert_eq!(a[0].get("canale"), Some(&V::s("A")));
    assert_eq!(a[0].get("nome"), Some(&V::s("Water + 0.1% FA")));
}
