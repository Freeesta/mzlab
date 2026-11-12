//! The acquisition time of a Waters run as ISO 8601 and the parameters of its method document.
use super::*;

#[test]
fn the_acquisition_date_and_clock_become_iso() {
    assert_eq!(
        iso("30-Apr-2022", "00:26:58").as_deref(),
        Some("2022-04-30T00:26:58")
    );
    assert_eq!(
        iso(" 1-dec-1999 ", " 23:59:59 ").as_deref(),
        Some("1999-12-01T23:59:59")
    );
    assert_eq!(iso("32-Apr-2022", "00:26:58"), None);
    assert_eq!(iso("30-Foo-2022", "00:26:58"), None);
    assert_eq!(iso("30-Apr-2022", "0:26"), None);
}

#[test]
fn a_parameter_keeps_its_value_only_when_it_is_a_number_besides_zero() {
    let json = |v: f64| {
        param("CE", "Collision energy", v, "eV")
            .map(|p| p.json())
            .unwrap_or_default()
    };
    assert!(json(20.0).contains("\"valore\":20"), "{}", json(20.0));
    assert!(json(20.0).contains("\"unita\":\"eV\""));
    assert!(!json(0.0).contains("valore"));
    assert!(!json(f64::NAN).contains("valore"));
}
