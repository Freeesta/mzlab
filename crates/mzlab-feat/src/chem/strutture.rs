//! The cleavages of the structure of the user that explain the m/z of one node of an `msngrafo/1`: the fragmenter of mzLab (`fragmenter`) run on the
//! node's m/z and on the peaks of its own spectrum, the cuts whose piece gives the m/z of the node grouped by that m/z. Used by `mzlab-cli msn-graph
//! --strutture` and the `.wasm` `msngrafo` (`strutture_testo`, the same bytes); the types of the answer are in `mzlab_core::msngrafo`. Candidates, never identifications.

use super::fragmenter::{fragment_molecule, FragMatch, FragmentSide};
use super::grafo::Mol;
use crate::sortc::SortC;
use mzlab_core::error::{Error, Result as CoreResult};
use mzlab_core::msngrafo::{
    con_strutture, con_strutture_tutti, CandidatoStruttura, Grafo, IngressoStrutture,
    PiccoSpiegato, Strutture, Taglio,
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

/// The graph text `j` of `g` with the key `strutture` of node `nodo` (`nK` or `K`): the cleavages of `smiles` that give its m/z within `ppm`.
pub fn strutture_testo(
    g: &Grafo,
    j: &str,
    smiles: Option<&str>,
    (nodo, ppm): (&str, f64),
) -> CoreResult<String> {
    let smiles = smiles.filter(|s| !s.trim().is_empty());
    let smiles = smiles.ok_or_else(|| Error::new("err.msngrafo.noSmiles"))?;
    let bad = || Error::new("err.msngrafo.badNode").with("node", nodo);
    if nodo == "tutti" {
        let mut tutti = vec![];
        for (k, _) in g.nodi.iter().enumerate().filter(|(_, n)| n.livello >= 2) {
            let ing = g.ingresso_strutture(k).ok_or_else(bad)?;
            let s = strutture(smiles, &ing, ppm)
                .map_err(|e| Error::new("err.msngrafo.badSmiles").with("detail", e))?;
            if !s.candidati.is_empty() {
                tutti.push(s);
            }
        }
        return Ok(con_strutture_tutti(j, &tutti));
    }
    let k: usize = nodo.trim_start_matches('n').parse().map_err(|_| bad())?;
    let ing = g.ingresso_strutture(k).ok_or_else(bad)?;
    let s = strutture(smiles, &ing, ppm)
        .map_err(|e| Error::new("err.msngrafo.badSmiles").with("detail", e))?;
    Ok(con_strutture(j, &s))
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
            .sortc_by_key(|t| (t.legame, t.lato, t.variante.clone()));
        c.ppm = c
            .tagli
            .iter()
            .map(|t| t.ppm)
            .min_by(|a, b| a.abs().total_cmp(&b.abs()))
            .unwrap_or(c.ppm);
    }
    out.candidati.sortc_by(|a, b| {
        a.ppm
            .abs()
            .total_cmp(&b.ppm.abs())
            .then(a.mz_teorico.total_cmp(&b.mz_teorico))
    });
    Ok(out)
}
