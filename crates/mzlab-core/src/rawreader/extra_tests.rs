//! Tests of the precursors of a `.raw` scan in mzML (`precursors`): simultaneous, MSn chain with SPS, single step, built from hand-made
//! `PrecursorInfo` values rather than from the reader.
use super::*;
use opentfraw::{PrecursorInfo, ReactionInfo};

fn rx(mz: f64, energy: Option<f64>, width: Option<f64>, multiple: bool) -> ReactionInfo {
    ReactionInfo {
        precursor_mz: mz,
        isolation_width: width,
        activation: Some(Activation::HCD),
        energy,
        multiple,
        ..Default::default()
    }
}

#[test]
fn precursors_isolated_together_are_one_each_with_their_own_window_and_energy() {
    let p = PrecursorInfo {
        target_mz: Some(700.0),
        selected_mz: Some(700.0),
        isolation_width: Some(2.0),
        collision_energy: Some(30.0),
        reactions: vec![
            rx(400.2, Some(25.0), Some(2.0), true),
            rx(550.3, Some(28.0), Some(2.0), true),
            rx(700.0, Some(30.0), Some(2.0), true),
        ],
        ..Default::default()
    };
    let v = precursors(&p, None, Some("scan=1".into()));
    assert_eq!(v.len(), 3);
    let mz: Vec<_> = v.iter().map(|q| q.selected_mz).collect();
    assert_eq!(mz, [Some(400.2), Some(550.3), Some(700.0)]);
    assert_eq!(v[0].iso_target, Some(400.2));
    // each window around its own target, not one window around the last
    assert!(v.iter().all(|q| q.iso_lower == Some(1.0)));
    assert_eq!(v[1].energy.as_ref().map(|e| e.1), Some(28.0));
    assert!(v
        .iter()
        .all(|q| q.spectrum_ref.as_deref() == Some("scan=1")));
}

#[test]
fn an_msn_chain_gives_one_precursor_per_step_with_the_trailer_ion_on_the_first() {
    let mut first = rx(800.0, Some(35.0), None, false);
    first.selected_mz = Some(799.9);
    first.charge = Some(2);
    let p = PrecursorInfo {
        target_mz: Some(500.0),
        selected_mz: Some(500.0),
        isolation_width: Some(1.0),
        reactions: vec![first, rx(500.0, Some(27.0), Some(1.0), false)],
        ..Default::default()
    };
    let v = precursors(&p, None, Some("scan=2".into()));
    assert_eq!(v.len(), 2);
    // the final step first, with its own energy and window and no charge of another step
    assert_eq!((v[0].selected_mz, v[0].charge), (Some(500.0), None));
    assert_eq!(v[0].energy.as_ref().map(|e| e.1), Some(27.0));
    assert_eq!(v[0].iso_lower, Some(0.5));
    assert_eq!(v[0].spectrum_ref.as_deref(), Some("scan=2"));
    assert_eq!((v[1].selected_mz, v[1].charge), (Some(799.9), Some(2)));
    assert_eq!(v[1].iso_target, Some(800.0));
    assert_eq!(v[1].energy.as_ref().map(|e| e.1), Some(35.0));
    assert!(v[1].spectrum_ref.is_none());
}

#[test]
fn the_sps_ions_are_the_final_step_of_an_ms3() {
    let p = PrecursorInfo {
        target_mz: Some(677.3),
        reactions: vec![
            rx(599.8, Some(27.0), None, false),
            rx(677.3, Some(33.0), Some(2.0), false),
        ],
        sps_mz: vec![410.2, 520.3, 633.4],
        ..Default::default()
    };
    let v = precursors(&p, None, None);
    let mz: Vec<_> = v.iter().map(|q| q.iso_target).collect();
    assert_eq!(mz, [Some(410.2), Some(520.3), Some(633.4), Some(599.8)]);
    // a notch has no window the file gives
    assert!(v[..3].iter().all(|q| q.iso_lower.is_none()));
    assert!(v[..3]
        .iter()
        .all(|q| q.energy.as_ref().map(|e| e.1) == Some(33.0)));
}

#[test]
fn a_single_step_is_one_precursor_and_an_unknown_activation_has_no_term() {
    let p = PrecursorInfo {
        target_mz: Some(500.0),
        isolation_width: Some(1.0),
        reactions: vec![rx(500.0, None, Some(1.0), false)],
        ..Default::default()
    };
    let v = precursors(&p, Some(499.0), None);
    assert_eq!(v.len(), 1);
    // no selected ion in the trailer: the target, not the spectrum's own guess
    assert_eq!(v[0].selected_mz, Some(500.0));
    assert!(v[0].activation.is_empty());
    assert!(v[0].energy.is_none());
}
