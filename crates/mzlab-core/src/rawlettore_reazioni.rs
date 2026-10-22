//! The chain of reactions of an MS3+ scan as header fields of the raw reader (`rawlettore.rs`).

use crate::lettori::Campo;

fn field(nome: &str, valore: String) -> Campo {
    Campo {
        nome: nome.into(),
        valore,
        unita: None,
        cv: None,
    }
}

/// The chain of reactions of an MS3+ scan (or a stepped NCE) as `opentfraw.reaction.N.*` values, N counting from 1: precursor, activation (and the
/// supplemental one), energy with its unit, the steps of a stepped NCE, the isolation width of the final step and the flags not yet understood.
/// Nothing for the common MS2 scan, whose header stays as it was.
pub(crate) fn reaction_fields(p: &opentfraw::PrecursorInfo) -> Vec<Campo> {
    if p.reactions.len() < 2 && p.reactions.iter().all(|r| r.energy_steps.is_empty()) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (i, r) in p.reactions.iter().enumerate() {
        let mut put =
            |k: &str, v: String| out.push(field(&format!("opentfraw.reaction.{}.{k}", i + 1), v));
        put("precursor", r.precursor_mz.to_string());
        if let Some(a) = r.activation {
            put("activation", a.as_str().into());
        }
        if let Some(a) = r.supplemental {
            put("supplemental activation", a.as_str().into());
        }
        if let Some(e) = r.energy {
            put("energy", e.to_string());
            put(
                "energy unit",
                if r.energy_is_nce { "NCE" } else { "eV" }.into(),
            );
        }
        if !r.energy_steps.is_empty() {
            let steps: Vec<String> = r.energy_steps.iter().map(f64::to_string).collect();
            put("energy steps", steps.join(","));
        }
        if let Some(e) = r.supplemental_energy {
            put("supplemental energy", e.to_string());
        }
        if let Some(w) = r.isolation_width {
            put("isolation width", w.to_string());
        }
        put("raw flags", r.raw_flags.to_string());
    }
    out
}

#[cfg(test)]
mod reaction_tests {
    use super::*;
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
    fn a_stepped_nce_is_a_list_even_in_ms2() {
        let mut r = step(300.5, Activation::HCD, 35.0);
        r.energy_steps = vec![20.0, 35.0, 60.0];
        let p = PrecursorInfo {
            reactions: vec![r],
            ..Default::default()
        };
        let v = values(&p);
        assert!(v.contains(&(
            "opentfraw.reaction.1.energy steps".into(),
            "20,35,60".into()
        )));
    }
}
