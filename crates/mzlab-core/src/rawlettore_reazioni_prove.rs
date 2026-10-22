//! Tests of `rawlettore_reazioni.rs`.

use super::reaction_fields;
use opentfraw::{Activation, PrecursorInfo, ReactionInfo};

fn step(mz: f64, activation: Activation, energy: f64) -> ReactionInfo {
    ReactionInfo {
        precursor_mz: mz,
        activation: Some(activation),
        energy: Some(energy),
        energy_is_nce: true,
        raw_flags: 11,
        ..Default::default()
    }
}

fn values(p: &PrecursorInfo) -> Vec<(String, String)> {
    reaction_fields(p)
        .into_iter()
        .map(|c| (c.nome, c.valore))
        .collect()
}

#[test]
fn an_ms2_scan_adds_no_field() {
    let p = PrecursorInfo {
        reactions: vec![step(300.5, Activation::HCD, 30.0)],
        ..Default::default()
    };
    assert!(values(&p).is_empty());
}

#[test]
fn an_ms3_chain_is_listed_step_by_step() {
    let p = PrecursorInfo {
        reactions: vec![
            step(500.0, Activation::CID, 35.0),
            step(300.0, Activation::HCD, 40.0),
        ],
        ..Default::default()
    };
    let v = values(&p);
    let get = |k: &str| v.iter().find(|(n, _)| n == k).map(|(_, x)| x.as_str());
    assert_eq!(get("opentfraw.reaction.1.activation"), Some("cid"));
    assert_eq!(get("opentfraw.reaction.2.activation"), Some("hcd"));
    assert_eq!(get("opentfraw.reaction.2.energy"), Some("40"));
    assert_eq!(get("opentfraw.reaction.2.energy unit"), Some("NCE"));
}

#[test]
fn a_stepped_nce_is_a_list_only_in_a_chain() {
    let mut r = step(300.5, Activation::HCD, 35.0);
    r.energy_steps = vec![20.0, 35.0, 60.0];
    let one = PrecursorInfo {
        reactions: vec![r.clone()],
        ..Default::default()
    };
    assert!(values(&one).is_empty());
    let chain = PrecursorInfo {
        reactions: vec![step(500.0, Activation::CID, 35.0), r],
        ..Default::default()
    };
    assert!(values(&chain).contains(&(
        "opentfraw.reaction.2.energy steps".into(),
        "20,35,60".into()
    )));
}

#[test]
fn the_unit_is_declared_only_when_it_is_known() {
    let mut unknown = step(500.0, Activation::CID, 25.0);
    unknown.energy_is_nce = false;
    let p = PrecursorInfo {
        reactions: vec![unknown, step(300.0, Activation::CID, 25.0)],
        ..Default::default()
    };
    let v = values(&p);
    assert!(!v
        .iter()
        .any(|(n, _)| n == "opentfraw.reaction.1.energy unit"));
    assert!(v
        .iter()
        .any(|(n, _)| n == "opentfraw.reaction.2.energy unit"));
}
