//! `LogPValues::from_smiles` (our own Crippen tables, feature `record`) feeding the RT calibration, and the polarity direction from a change of logP.
//! Synthetic molecules only.

use mzlab_feat::attributio::review::rtlogp::{calibrate, check, CalPoint, LogPValues, MIN_N};
use mzlab_feat::hypo::Dir;

#[test]
fn values_come_from_the_largest_component() {
    let v = LogPValues::from_smiles("[Na+].[Cl-].CCO").unwrap();
    // ethanol: Crippen logP -0.0014 in RDKit, TPSA 20.23, one donor, one acceptor
    assert!((v.crippen.unwrap() + 0.0).abs() < 0.01, "{v:?}");
    assert_eq!(v.tpsa, Some(20.23));
    assert_eq!((v.hbd, v.hba, v.nrot), (Some(1.0), Some(1.0), Some(0.0)));
    assert_eq!(v.mw, Some(46.07));
    assert!(v.xlogp3.is_none() && v.class.is_none());
    assert!(LogPValues::from_smiles("not a smiles(").is_none());
}

/// Retention grows with logP in a made-up series of alkyl alcohols; the calibration from our own values finds the line and flags a far-off RT.
#[test]
fn own_crippen_values_calibrate_and_check() {
    let pts: Vec<CalPoint> = (1..=20)
        .map(|n| {
            let smiles = format!("{}O", "C".repeat(n));
            let logp = LogPValues::from_smiles(&smiles).unwrap();
            let y = 1.0 + 0.5 * n as f64 + if n % 2 == 0 { 0.02 } else { -0.02 };
            CalPoint { logp, y: Some(y) }
        })
        .collect();
    assert!(pts.len() >= MIN_N);
    let m = calibrate(&pts, 3.0, 1.0, MIN_N);
    assert_eq!(m.source, "crippen");
    assert!(m.fit.is_some(), "{}", m.note);
    let q = LogPValues::from_smiles("CCCCCCCCCCCCCCCCCCCCCO").unwrap();
    assert_eq!(check(&m, &q, Some(11.6), 3.0, 1.0).ok, Some(true));
    assert_eq!(check(&m, &q, Some(3.0), 3.0, 1.0).ok, Some(false));
}

#[test]
fn polarity_direction_from_delta_logp() {
    assert_eq!(Dir::from_delta_logp(-1.2, 0.3), Dir::MorePolar);
    assert_eq!(Dir::from_delta_logp(0.8, 0.3), Dir::LessPolar);
    assert_eq!(Dir::from_delta_logp(0.1, 0.3), Dir::Free);
    assert_eq!(Dir::MorePolar.agrees(-1.2, 0.3), Some(true));
    assert_eq!(Dir::MorePolar.agrees(0.8, 0.3), Some(false));
    assert_eq!(Dir::MorePolar.agrees(0.1, 0.3), None);
    assert_eq!(Dir::Free.agrees(2.0, 0.3), None);
}
