//! The cleavages of the structure of the user that explain the m/z of one node of an `msngrafo/1`: the fragmenter of mzLab (`fragmenter`) run on the
//! node's m/z and on the peaks of its own spectrum, the cuts whose piece gives the m/z of the node grouped by that m/z. Used by `mzlab-cli msn-graph
//! --strutture`; the types of the answer are in `mzlab_core::msngrafo`. Candidates, never identifications.

use super::fragmenter::{fragment_molecule, FragMatch, FragmentSide};
use super::grafo::Mol;
use mzlab_core::msngrafo::{
    CandidatoStruttura, IngressoStrutture, PiccoSpiegato, Strutture, Taglio,
};

/// Cleavable bonds taken at most (the fragmenter cuts the first ones in the order of the SMILES).
const MAX_LEGAMI: usize = 400;
/// Two theoretical m/z are the same composition within this distance (Da); the electron mass is the same on both sides.
const STESSA_MASSA: f64 = 2e-4;

fn taglio(
    ing: &IngressoStrutture,
    legame: (usize, (usize, usize), &str),
    lato: u8,
    (side, other): (&FragmentSide, &FragmentSide),
    m: &FragMatch,
) -> Taglio {
    let picchi = side
        .matches
        .iter()
        .filter(|x| x.peak != ing.mz)
        .map(|x| PiccoSpiegato {
            mz: x.peak,
            rel: ing
                .picchi
                .iter()
                .find(|p| p.0 == x.peak)
                .map_or(0.0, |p| p.1),
            ppm: x.ppm,
            formula: x.formula.clone(),
            perdita: x.loss.clone(),
        })
        .collect();
    Taglio {
        legame: legame.0,
        atomi: legame.1,
        etichetta: legame.2.to_string(),
        lato,
        pezzo: side.atoms.clone(),
        resto: other.atoms.clone(),
        formula: side.formula.clone(),
        variante: m.var.to_string(),
        perdita: m.loss.clone(),
        mz_teorico: m.mz_th,
        ppm: m.ppm,
        picchi,
    }
}

/// The cleavages of `smiles` that give the m/z of the node `ing` within `ppm`. `Err` = the SMILES cannot be read.
pub fn strutture(smiles: &str, ing: &IngressoStrutture, ppm: f64) -> Result<Strutture, String> {
    let mut peaks = vec![ing.mz];
    peaks.extend(ing.picchi.iter().map(|p| p.0).filter(|&mz| mz != ing.mz));
    let pol = if ing.polarita < 0 { "NEG" } else { "POS" };
    let frag = fragment_molecule(smiles, pol, &peaks, ppm, MAX_LEGAMI)?;
    let mixture = smiles.split('.').filter(|p| !p.trim().is_empty()).count() > 1;
    let mut out = Strutture {
        nodo: ing.nodo,
        smiles: smiles.to_string(),
        mz: ing.mz,
        ppm_tol: ppm,
        atomi: if mixture {
            0
        } else {
            Mol::parse(smiles).map_or(0, |m| m.atoms.len())
        },
        legami: frag.bonds.len(),
        candidati: vec![],
        avvisi: if mixture {
            vec!["warn.msngrafo.structMixture"]
        } else {
            vec![]
        },
    };
    for b in &frag.bonds {
        let (Some(f1), Some(f2)) = (&b.frag1, &b.frag2) else {
            continue;
        };
        let sides = [(1u8, (f1, f2)), (2u8, (f2, f1))];
        for (lato, pair) in sides {
            for m in pair.0.matches.iter().filter(|m| m.peak == ing.mz) {
                let t = taglio(ing, (b.bond_idx, b.atoms, &b.label), lato, pair, m);
                match out
                    .candidati
                    .iter_mut()
                    .find(|c| (c.mz_teorico - t.mz_teorico).abs() < STESSA_MASSA)
                {
                    Some(c) => c.tagli.push(t),
                    None => out.candidati.push(CandidatoStruttura {
                        formula: ing
                            .formule
                            .iter()
                            .find(|f| (f.1 - t.mz_teorico).abs() < STESSA_MASSA)
                            .map(|f| f.0.clone()),
                        mz_teorico: t.mz_teorico,
                        ppm: t.ppm,
                        tagli: vec![t],
                    }),
                }
            }
        }
    }
    for c in &mut out.candidati {
        c.tagli
            .sort_by_key(|t| (t.legame, t.lato, t.variante.clone()));
        c.ppm = c
            .tagli
            .iter()
            .map(|t| t.ppm)
            .min_by(|a, b| a.abs().total_cmp(&b.abs()))
            .unwrap_or(c.ppm);
    }
    out.candidati.sort_by(|a, b| {
        a.ppm
            .abs()
            .total_cmp(&b.ppm.abs())
            .then(a.mz_teorico.total_cmp(&b.mz_teorico))
    });
    Ok(out)
}
