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

/// The chain of reactions of an MS3+ scan as `opentfraw.reaction.N.*` values, N counting from 1: precursor, activation (and the
/// supplemental one), energy with its unit, the steps of a stepped NCE, the isolation width of the final step and the flags not yet understood.
/// Nothing for a scan with fewer than two reactions (MS2, stepped NCE included), whose header stays as it was.
pub(crate) fn reaction_fields(p: &opentfraw::PrecursorInfo) -> Vec<Campo> {
    if p.reactions.len() < 2 {
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
            // The unit is declared only when the trailer says NCE; anything else is not known to be eV.
            if r.energy_is_nce {
                put("energy unit", "NCE".into());
            }
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
#[path = "rawlettore_reazioni_prove.rs"]
mod reaction_tests;
