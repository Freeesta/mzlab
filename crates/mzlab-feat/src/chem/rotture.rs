//! The bond breaks of the structure of the user that explain the ions of an `msngrafo/1`: every set of one or two single bonds is cut, each
//! piece takes a hydrogen per cut and may gain or lose a few more (hydrogen transfer), and the pieces whose m/z is a node's become its hypotheses;
//! an arc is explained when a hypothesis of the child lies inside one of the parent. Used by `mzlab-cli msn-graph --rotture` and the `.wasm`
//! `msngrafo` (`rotture_testo`, the same bytes); the types of the answer are in `mzlab_core::msngrafo`. Hypotheses, never identifications.

use crate::sortc::SortC;
use super::grafo::Mol;
use mzlab_core::elements::{self, Formula};
use mzlab_core::error::{Error, Result as CoreResult};
use mzlab_core::msngrafo::{
    con_rotture, Grafo, Ipotesi, RotturaArco, RotturaNodo, RotturaSpettro, Rotture,
};

const ION_H: f64 = 1.00727646677;
const MASSA_H: f64 = 1.00782503207;
/// Single bonds cut at most (as the one-bond cleavages of `strutture`).
const MAX_LEGAMI: usize = 400;
/// Bonds that take part in pairs at most (a pair is a quadratic number of cuts).
const MAX_COPPIE: usize = 120;
/// Hypotheses written per node (the count `n` is of all).
const MAX_IPOTESI: usize = 20;

/// The parameters of the calculation: window in ppm of the m/z of a node, hydrogens moved at most, bonds cut at most (1 or 2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parametri {
    pub ppm: f64,
    pub h_max: u8,
    pub max_legami: u8,
}

impl Default for Parametri {
    fn default() -> Self {
        Parametri {
            ppm: 10.0,
            h_max: 2,
            max_legami: 2,
        }
    }
}

/// Component label of every atom once the bonds in `tagli` are removed, and the number of components.
fn componenti(m: &Mol, tagli: &[usize]) -> (Vec<usize>, usize) {
    let mut lab = vec![usize::MAX; m.atoms.len()];
    let mut n = 0;
    for s in 0..m.atoms.len() {
        if lab[s] != usize::MAX {
            continue;
        }
        lab[s] = n;
        let mut st = vec![s];
        while let Some(u) = st.pop() {
            for (v, k) in &m.adj[u] {
                if !tagli.contains(k) && lab[*v] == usize::MAX {
                    lab[*v] = n;
                    st.push(*v);
                }
            }
        }
        n += 1;
    }
    (lab, n)
}

fn sottoinsieme(a: &[usize], b: &[usize]) -> bool {
    let mut j = 0;
    for x in a {
        while j < b.len() && b[j] < *x {
            j += 1;
        }
        if j == b.len() || b[j] != *x {
            return false;
        }
    }
    true
}

/// The hypotheses of every node, the explained arcs and the explained intensity of `g` for the structure `smiles`. `Err` = the SMILES cannot be read.
pub fn rotture(g: &Grafo, smiles: &str, p: Parametri) -> Result<Rotture, String> {
    let mixture = smiles.split('.').filter(|s| !s.trim().is_empty()).count() > 1;
    let mut out = Rotture {
        smiles: smiles.to_string(),
        ppm: p.ppm,
        h_max: p.h_max,
        max_legami: p.max_legami.clamp(1, 2),
        atomi: 0,
        legami: 0,
        avvisi: vec![],
        nodi: vec![],
        archi: vec![],
        spettri: vec![],
    };
    let mut ipotesi: Vec<Vec<Ipotesi>> = vec![vec![]; g.nodi.len()];
    if mixture {
        Mol::parse(smiles.split('.').next().unwrap_or("")).map_err(|_| "Invalid SMILES")?;
        out.avvisi.push("warn.msngrafo.structMixture");
    } else {
        let m = Mol::parse(smiles).map_err(|_| "Invalid SMILES")?;
        out.atomi = m.atoms.len();
        out.legami = m.bonds.len();
        enumera(g, &m, &mut out, &mut ipotesi, p);
    }
    for (k, v) in ipotesi.iter_mut().enumerate() {
        v.sortc_by(|a, b| {
            a.ppm
                .abs()
                .total_cmp(&b.ppm.abs())
                .then(a.legami.len().cmp(&b.legami.len()))
                .then(a.pezzo.cmp(&b.pezzo))
                .then(a.h.cmp(&b.h))
        });
        if v.is_empty() {
            continue;
        }
        out.nodi.push(RotturaNodo {
            nodo: k,
            n: v.len(),
            ipotesi: vec![],
        });
    }
    for a in &g.archi {
        let spiegato = ipotesi[a.a].iter().any(|c| {
            ipotesi[a.da].iter().any(|q| {
                sottoinsieme(&c.pezzo, &q.pezzo) && (c.pezzo.len() < q.pezzo.len() || c.h != q.h)
            })
        });
        out.archi.push(RotturaArco {
            da: a.da,
            a: a.a,
            spiegato,
        });
    }
    for s in &g.spettri {
        let Some(pr) = s.precursore else { continue };
        let (mut sp, mut tot) = (0.0, 0.0);
        for &(_, rel, nodo) in &s.picchi {
            tot += rel;
            if nodo.is_some_and(|n| {
                out.archi
                    .iter()
                    .any(|a| a.da == pr && a.a == n && a.spiegato)
            }) {
                sp += rel;
            }
        }
        out.spettri.push(RotturaSpettro {
            albero: s.albero.clone(),
            spiegata: sp,
            totale: tot,
        });
    }
    for n in &mut out.nodi {
        n.ipotesi = ipotesi[n.nodo].iter().take(MAX_IPOTESI).cloned().collect();
    }
    Ok(out)
}

/// What the cuts need: the molecule, the nodes by m/z, the mass of each atom with its hydrogens, the ion shift.
struct Ctx<'a> {
    g: &'a Grafo,
    m: &'a Mol,
    ordine: Vec<usize>,
    massa_atomo: Vec<f64>,
    delta: f64,
    p: Parametri,
}

impl Ctx<'_> {
    /// Cuts the bonds `tagli` and adds, per node, the pieces (and hydrogen shifts) that give its m/z.
    fn prova(&self, tagli: &[usize], ipotesi: &mut [Vec<Ipotesi>]) {
        let m = self.m;
        let (lab, nc) = componenti(m, tagli);
        if tagli
            .iter()
            .any(|&k| lab[m.bonds[k].a] == lab[m.bonds[k].b])
        {
            return;
        }
        for c in 0..nc {
            let pezzo: Vec<usize> = (0..lab.len()).filter(|&i| lab[i] == c).collect();
            let mut keep = vec![false; lab.len()];
            for &i in &pezzo {
                keep[i] = true;
            }
            let attacchi: usize = tagli
                .iter()
                .map(|&k| {
                    [m.bonds[k].a, m.bonds[k].b]
                        .iter()
                        .filter(|&&e| keep[e])
                        .count()
                })
                .sum();
            let base: f64 =
                pezzo.iter().map(|&i| self.massa_atomo[i]).sum::<f64>() + attacchi as f64 * MASSA_H;
            let h = self.p.h_max as i8;
            for k in -h..=h {
                let mz = base + k as f64 * MASSA_H + self.delta;
                if mz > 0.0 {
                    nodi_a(
                        self,
                        mz,
                        (tagli, &pezzo, &keep, attacchi as i64 + k as i64),
                        k,
                        ipotesi,
                    );
                }
            }
        }
    }
}

/// Adds the piece to every node within the window of `mz`.
fn nodi_a(
    cx: &Ctx,
    mz: f64,
    (tagli, pezzo, keep, nh): (&[usize], &[usize], &[bool], i64),
    k: i8,
    ipotesi: &mut [Vec<Ipotesi>],
) {
    let (g, tol) = (cx.g, mz * cx.p.ppm * 1e-6);
    let lo = cx.ordine.partition_point(|&n| g.nodi[n].mz < mz - tol);
    for &n in cx.ordine[lo..]
        .iter()
        .take_while(|&&n| g.nodi[n].mz <= mz + tol)
    {
        // the fewest cuts come first, so a repeated piece keeps the simplest way to get it
        if ipotesi[n].iter().any(|x| x.pezzo == pezzo && x.h == k) {
            continue;
        }
        let mut f: Formula = cx.m.formula_of(keep);
        if nh != 0 {
            match f.iter_mut().find(|(e, _)| e == "H") {
                Some(x) => x.1 = (x.1 as i64 + nh).max(0) as u64,
                None => f.push(("H".to_string(), nh.max(0) as u64)),
            }
        }
        ipotesi[n].push(Ipotesi {
            legami: tagli.to_vec(),
            pezzo: pezzo.to_vec(),
            formula: elements::fmt(&f),
            h: k,
            mz_teorico: mz,
            ppm: (g.nodi[n].mz - mz) / mz * 1e6,
        });
    }
}

/// Cuts the sets of bonds and collects, per node, the pieces that give its m/z.
fn enumera(g: &Grafo, m: &Mol, out: &mut Rotture, ipotesi: &mut [Vec<Ipotesi>], p: Parametri) {
    let on_ring = m.ring_bonds();
    let mut cand: Vec<usize> = (0..m.bonds.len())
        .filter(|&k| {
            let o = m.bonds[k].order;
            o == 1.0 || (o == 1.5 && !on_ring[k])
        })
        .collect();
    cand.truncate(MAX_LEGAMI);
    let mut ordine: Vec<usize> = (0..g.nodi.len()).collect();
    ordine.sortc_by(|&a, &b| g.nodi[a].mz.total_cmp(&g.nodi[b].mz));
    let ctx = Ctx {
        g,
        m,
        ordine,
        massa_atomo: m
            .atoms
            .iter()
            .map(|a| elements::mass(&vec![(a.el.clone(), 1u64)]) + a.h as f64 * MASSA_H)
            .collect(),
        delta: if g.polarita < 0 { -ION_H } else { ION_H },
        p,
    };
    ctx.prova(&[], ipotesi);
    for &a in &cand {
        ctx.prova(&[a], ipotesi);
    }
    if p.max_legami < 2 {
        return;
    }
    if cand.len() > MAX_COPPIE {
        cand.truncate(MAX_COPPIE);
        out.avvisi.push("warn.msngrafo.rottureCapped");
    }
    for (i, &a) in cand.iter().enumerate() {
        for &b in &cand[i + 1..] {
            ctx.prova(&[a, b], ipotesi);
        }
    }
}

/// The graph text `j` of `g` with the key `rotture` for `smiles`.
pub fn rotture_testo(g: &Grafo, j: &str, smiles: Option<&str>, p: Parametri) -> CoreResult<String> {
    let smiles = smiles.filter(|s| !s.trim().is_empty());
    let smiles = smiles.ok_or_else(|| Error::new("err.msngrafo.noSmiles"))?;
    let r = rotture(g, smiles, p)
        .map_err(|e| Error::new("err.msngrafo.badSmiles").with("detail", e))?;
    Ok(con_rotture(j, &r))
}
