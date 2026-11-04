//! «+D», «-D», «+2D» read as deuterium in the deltas of mzFinder and of the hypothesis rules; the mass comes from the element table.
use mzlab_core::elements::mass_of;
use mzlab_feat::hypo::parse_delta as rules_delta;
use mzlab_feat::legacy::transf::parse_delta;

#[test]
fn deuterium_delta_spellings() {
    assert_eq!(parse_delta("+D").unwrap(), vec![("D".to_string(), 1)]);
    assert_eq!(parse_delta("-D").unwrap(), vec![("D".to_string(), -1)]);
    assert_eq!(parse_delta("+2D").unwrap(), vec![("D".to_string(), 2)]);
    assert_eq!(
        parse_delta("-H+2D").unwrap(),
        vec![("D".to_string(), 2), ("H".to_string(), -1)]
    );
    assert_eq!(rules_delta("+2D").unwrap().get("D"), Some(&2));
    assert!((mass_of("D").unwrap() - 2.01410177812).abs() < 1e-9);
}

#[test]
fn other_spellings_unchanged() {
    assert_eq!(parse_delta("+O").unwrap(), vec![("O".to_string(), 1)]);
    assert_eq!(parse_delta("-H2").unwrap(), vec![("H".to_string(), -2)]);
    assert!(parse_delta("+Xx").is_err());
}
