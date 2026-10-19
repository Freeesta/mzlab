//! `reactivity_json(smiles, opts)`: the contract `reatt/1` (`docs/agenti/reattivita.md`). ETKDG geometry, GFN2-xTB of the neutral
//! molecule, the cation and the anion at the same geometry (ALPB water by default), condensed Fukui functions, f(−) on the lowest
//! conformers, C–H sites by symmetry class; extended Hückel as the fallback when an SCC does not converge.

use crate::embed::{embed, Conf, DEFAULT_CONFORMERS};
use crate::fukui::{ch_sites, Fukui, Site};
use crate::mol3d::{hbonds, symbol, Mol3d};
use crate::protomer::{self, Protomer};
use crate::xtb::{alpb, System};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out};
use mzlab_feat::chem::grafo::Mol;

/// Version of the engine written in `engine` and returned by the `.wasm` (`version()`).
pub const ENGINE: &str = "mzlab-chem/1";
/// GFN2-xTB is the published method; the extended Hückel fallback is used only when an SCC does not converge.
pub const PUBLISH_GFN2: bool = true;
/// Whether the pH 7 form is the default (`docs/agenti/reattivita.md`, section «Protomero a pH 7»).
pub const PROTOMER_DEFAULT: bool = false;
/// Conformers on which f(−) is also computed, to give its spread.
const SPREAD_CONFORMERS: usize = 3;

/// Options of `reactivity_json` (JSON object, every field optional).
pub struct Opts {
    /// `"water"` (default) or `"none"`.
    pub water: bool,
    pub conformers: usize,
    pub seed: u64,
    /// `"protomer":"ph7"` computes the most probable form at pH 7 (`protomer.rs`) instead of the SMILES as written.
    pub ph7: bool,
}

impl Opts {
    pub fn parse(text: &str) -> Result<Opts> {
        let mut o = Opts {
            water: true,
            conformers: DEFAULT_CONFORMERS,
            seed: 42,
            ph7: PROTOMER_DEFAULT,
        };
        if text.trim().is_empty() {
            return Ok(o);
        }
        let bad = |k: &str| Error::new("error.chem.opts").with("field", k);
        let v = json::parse(text.as_bytes()).map_err(|_| bad("json"))?;
        if let Some(s) = v.get("solvent").and_then(|s| s.str()) {
            o.water = match s {
                "water" => true,
                "none" => false,
                _ => return Err(bad("solvent")),
            };
        }
        if let Some(n) = v.get("conformers").and_then(|n| n.f64()) {
            o.conformers = (n as usize).clamp(1, 16);
        }
        if let Some(s) = v.get("protomer").and_then(|s| s.str()) {
            o.ph7 = match s {
                "ph7" => true,
                "none" => false,
                _ => return Err(bad("protomer")),
            };
        }
        if let Some(n) = v.get("seed").and_then(|n| n.f64()) {
            o.seed = n.max(0.0) as u64;
        }
        Ok(o)
    }
}

/// Everything `reatt/1` reports besides the molecule.
pub struct Result3 {
    pub method: &'static str,
    pub fukui: Fukui,
    pub homo_ev: f64,
    pub lumo_ev: f64,
    pub converged: [bool; 3],
    pub iterations: [usize; 3],
    /// Share of the HOMO and of the LUMO on each atom (empty for the EHT fallback).
    pub homo_share: Vec<f64>,
    pub lumo_share: Vec<f64>,
    pub spread: Option<f64>,
}

/// Fukui values, HOMO and LUMO (eV), converged, iterations of N, N+1 and N−1, and the HOMO and LUMO share of each atom.
pub type Gfn2 = (Fukui, f64, f64, [bool; 3], [usize; 3], [Vec<f64>; 2]);

/// GFN2-xTB of N, N+1 and N−1 for one geometry: (Fukui, HOMO, LUMO, converged, iterations, frontier shares).
pub fn gfn2(m: &Mol3d, xyz: &[[f64; 3]], water: bool) -> Result<Gfn2> {
    let sys = System::new(&m.el, xyz, water.then_some(&alpb::WATER))?;
    let charge: i32 = m.charge.iter().map(|&c| c as i32).sum();
    let n = sys.solve(charge, None)?;
    let a = sys.solve(charge - 1, Some(&n.guess))?;
    let c = sys.solve(charge + 1, Some(&n.guess))?;
    let nel = (sys.electrons() as i64 - charge as i64) as usize;
    let (h, l) = n.frontier(nel);
    let (hs, ls) = sys.frontier_shares(&n, nel);
    Ok((
        Fukui::from_charges(&n.q, &a.q, &c.q),
        h,
        l,
        [n.converged, a.converged, c.converged],
        [n.iterations, a.iterations, c.iterations],
        [hs, ls],
    ))
}

/// f(−) (cation and neutral only) on one more conformer.
fn f_minus(m: &Mol3d, xyz: &[[f64; 3]], water: bool) -> Option<Vec<f64>> {
    let sys = System::new(&m.el, xyz, water.then_some(&alpb::WATER)).ok()?;
    let charge: i32 = m.charge.iter().map(|&c| c as i32).sum();
    let n = sys.solve(charge, None).ok()?;
    let c = sys.solve(charge + 1, Some(&n.guess)).ok()?;
    (n.converged && c.converged).then(|| c.q.iter().zip(&n.q).map(|(a, b)| a - b).collect())
}

/// Largest per-atom difference of f(−) between the lowest conformers (the first one is `f0`).
fn spread(m: &Mol3d, confs: &[Conf], f0: &[f64], water: bool) -> Option<f64> {
    let more: Vec<Vec<f64>> = confs
        .iter()
        .skip(1)
        .take(SPREAD_CONFORMERS - 1)
        .filter_map(|c| f_minus(m, &c.xyz, water))
        .collect();
    if more.is_empty() {
        return None;
    }
    Some(
        (0..f0.len())
            .map(|i| {
                let v = more.iter().map(|f| f[i]).chain([f0[i]]);
                v.clone().fold(f64::MIN, f64::max) - v.fold(f64::MAX, f64::min)
            })
            .fold(0.0, f64::max),
    )
}

/// The calculation behind `reatt/1` on the lowest-energy conformer.
pub fn compute(m: &Mol3d, confs: &[Conf], water: bool) -> Result<Result3> {
    let xyz = &confs[0].xyz;
    if PUBLISH_GFN2 {
        let (fukui, homo_ev, lumo_ev, converged, iterations, [homo_share, lumo_share]) =
            gfn2(m, xyz, water)?;
        if converged.iter().all(|&c| c) {
            let spread = spread(m, confs, &fukui.f_minus, water);
            return Ok(Result3 {
                method: "GFN2-xTB",
                fukui,
                homo_ev,
                lumo_ev,
                converged,
                iterations,
                homo_share,
                lumo_share,
                spread,
            });
        }
    }
    let (fukui, homo_ev, lumo_ev) = crate::eht::fukui(&m.el, xyz)?;
    // One EHT diagonalisation: no SCC, no ions.
    Ok(Result3 {
        method: "EHT",
        fukui,
        homo_ev,
        lumo_ev,
        converged: [true, false, false],
        iterations: [1, 0, 0],
        homo_share: Vec::new(),
        lumo_share: Vec::new(),
        spread: None,
    })
}

/// `reatt/1` of a SMILES; `opts_json` as in `Opts` (empty for the defaults).
pub fn reactivity_json(smiles: &str, opts_json: &str) -> Result<String> {
    let o = Opts::parse(opts_json)?;
    let written =
        Mol::parse(smiles).map_err(|e| Error::new("error.chem.smiles").with("reason", e.key))?;
    let prot = o.ph7.then(|| protomer::assign(smiles, &written));
    let mol = prot.as_ref().map_or(written, |p| p.mol.clone());
    let mut m = Mol3d::from_mol(&mol)?;
    let confs = embed(&m, o.seed, o.conformers)?;
    let r = compute(&m, &confs, o.water)?;
    m.xyz = confs[0].xyz.clone();
    let sites = ch_sites(&mol, &m, &r.fukui.f_minus);
    Ok(write(
        smiles,
        &m,
        &r,
        &sites,
        confs.len(),
        o.water,
        prot.as_ref(),
    ))
}

fn write(
    smiles: &str,
    m: &Mol3d,
    r: &Result3,
    sites: &[Site],
    nconf: usize,
    water: bool,
    prot: Option<&Protomer>,
) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":\"reatt\",\"version\":1,\"engine\":")
        .str(ENGINE)
        .raw(",\"method\":")
        .str(r.method);
    o.raw(",\"solvent\":")
        .str(if water { "water" } else { "none" })
        .raw(",\"smiles\":")
        .str(smiles);
    if let Some(p) = prot {
        write_protomer(&mut o, p);
    }
    write_atoms(&mut o, m, r);
    o.raw(",\"hbonds\":[");
    for (k, (h, a)) in hbonds(m, &m.xyz).iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw(&format!("{{\"donor_h\":{h},\"acceptor\":{a}}}"));
    }
    o.raw("],\"global\":{\"homo_ev\":")
        .num(round(r.homo_ev, 4))
        .raw(",\"lumo_ev\":")
        .num(round(r.lumo_ev, 4));
    o.raw(",\"gap_ev\":").num(round(r.lumo_ev - r.homo_ev, 4));
    o.raw(",\"ip_ev\":null,\"ea_ev\":null,\"mu_ev\":null,\"eta_ev\":null,\"omega_ev\":null}");
    o.raw(&format!(
        ",\"conformers\":{{\"n\":{nconf},\"chosen\":0,\"f_minus_spread\":"
    ));
    o.opt(r.spread.map(|s| round(s, 4))).raw("}");
    let b = |v: bool| if v { "true" } else { "false" };
    let (c, it) = (r.converged, r.iterations);
    o.raw(&format!(
        ",\"scc\":{{\"converged\":[{},{},{}],\"iterations\":[{},{},{}]}}",
        b(c[0]),
        b(c[1]),
        b(c[2]),
        it[0],
        it[1],
        it[2]
    ));
    write_sites(&mut o, sites);
    o.raw("],\"notes\":[");
    for (k, n) in notes(r, water, prot).iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(n);
    }
    o.raw("],\"cite\":[");
    for (k, (t, d)) in cite(r.method).iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"text\":")
            .str(t)
            .raw(",\"doi\":")
            .str(d)
            .raw("}");
    }
    o.raw("]}");
    o.0
}

/// Field `protomer`: the form used (SMILES, total charge, groups changed with their typical pKa) and the one-form statement.
fn write_protomer(o: &mut Out, p: &Protomer) {
    o.raw(",\"protomer\":{\"ph\":7,\"smiles\":")
        .str(&p.smiles)
        .raw(&format!(",\"charge\":{},\"changes\":[", p.charge));
    for (k, c) in p.changes.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"group\":")
            .str(c.group)
            .raw(&format!(",\"heavy_index\":{},\"action\":", c.atom))
            .str(if c.deprotonated {
                "deprotonated"
            } else {
                "protonated"
            })
            .raw(",\"pka\":")
            .num(round(c.pka, 1))
            .raw("}");
    }
    o.raw("],\"note\":").str(protomer_note(p)).raw("}");
}

fn protomer_note(p: &Protomer) -> &'static str {
    if p.changes.len() > 1 {
        "Several ionisable groups: the single most probable charge form at pH 7 from typical pKa values (no enumeration of protomers, no quantum pKa)."
    } else {
        "Most probable charge form at pH 7 from typical pKa values (no quantum pKa)."
    }
}

fn write_sites(o: &mut Out, sites: &[Site]) {
    o.raw(",\"ch_sites\":[");
    for (k, s) in sites.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"class\":")
            .str(&s.class);
        o.raw(&format!(
            ",\"heavy_index\":{},\"n_h\":{},\"f_minus\":",
            s.heavy_index, s.n_h
        ))
        .num(round(s.f_minus, 4));
        o.raw(&format!(
            ",\"rank\":{},\"tie_first\":{}}}",
            s.rank,
            if s.tie_first { "true" } else { "false" }
        ));
    }
}

fn write_atoms(o: &mut Out, m: &Mol3d, r: &Result3) {
    let f = &r.fukui;
    o.raw(",\"atoms\":[");
    for i in 0..m.len() {
        o.raw(if i > 0 { "," } else { "" })
            .raw(&format!("{{\"i\":{i},\"el\":"))
            .str(symbol(m.el[i]))
            .raw(",\"heavy_index\":");
        match m.heavy_index[i] {
            Some(h) => o.int(h as i64),
            None => o.raw("null"),
        };
        let p = m.xyz[i];
        o.raw(",\"x\":")
            .num(round(p[0], 4))
            .raw(",\"y\":")
            .num(round(p[1], 4))
            .raw(",\"z\":")
            .num(round(p[2], 4));
        o.raw(",\"q\":")
            .opt(f.q[i].map(|v| round(v, 4)))
            .raw(",\"f_plus\":")
            .num(round(f.f_plus[i], 4));
        o.raw(",\"f_zero\":")
            .opt(f.f_zero[i].map(|v| round(v, 4)))
            .raw(",\"f_minus\":")
            .num(round(f.f_minus[i], 4));
        o.raw(",\"dual\":").num(round(f.dual(i), 4));
        if let (Some(h), Some(l)) = (r.homo_share.get(i), r.lumo_share.get(i)) {
            o.raw(",\"homo\":")
                .num(round(*h, 4))
                .raw(",\"lumo\":")
                .num(round(*l, 4));
        }
        o.raw("}");
    }
    o.raw("],\"bonds\":[");
    for (k, &(a, b, ord)) in m.bonds.iter().enumerate() {
        let order = if ord == 4 {
            "1.5".to_string()
        } else {
            ord.to_string()
        };
        o.raw(if k > 0 { "," } else { "" })
            .raw(&format!("{{\"a\":{a},\"b\":{b},\"order\":{order}}}"));
    }
    o.raw("]");
}

fn notes(r: &Result3, water: bool, prot: Option<&Protomer>) -> Vec<String> {
    let mut n = vec![
        "ETKDG geometry, no force-field refinement; the lowest-energy conformer is used."
            .to_string(),
    ];
    if let Some(p) = prot {
        n.push(protomer_note(p).to_string());
    }
    if r.method == "EHT" {
        n.push(crate::eht::NOTE.to_string());
    } else {
        n.push("GFN2-xTB Mulliken charges of N, N+1 and N−1 electrons at the same geometry (300 K Fermi smearing, no spin polarisation): f(+) = q(N) − q(N+1), f(−) = q(N−1) − q(N), f(0) = their mean, dual = f(+) − f(−).".to_string());
        n.push(if water {
            "Implicit water: ALPB for GFN2-xTB (Ehlert et al. 2021 water parameters), Born electrostatics and hydrogen-bond term coupled to the charges; the surface-tension term, which does not change the charges, is left out.".to_string()
        } else {
            "Gas phase (no solvent).".to_string()
        });
        if r.spread.is_some() {
            n.push(format!("f(−) also computed on the {SPREAD_CONFORMERS} lowest conformers: f_minus_spread is the largest difference of one atom."));
        }
    }
    n.push(format!("C–H sites: unsaturated carbons with hydrogens, f(−) averaged over each symmetry class; sites within {} of the first are a tie.", crate::fukui::TIE));
    n
}

fn cite(method: &str) -> Vec<(&'static str, &'static str)> {
    let mut c = vec![(
        "Riniker S., Landrum G. A. (2015) ETKDG. J. Chem. Inf. Model.",
        "10.1021/acs.jcim.5b00654",
    )];
    if method == "EHT" {
        c.push((
            "Hoffmann R. (1963) An extended Hückel theory. J. Chem. Phys.",
            "10.1063/1.1734456",
        ));
    } else {
        c.push((
            "Bannwarth C., Ehlert S., Grimme S. (2019) GFN2-xTB. J. Chem. Theory Comput.",
            "10.1021/acs.jctc.8b01176",
        ));
        c.push(("Ehlert S. et al. (2021) Robust and efficient implicit solvation model for fast semiempirical methods (ALPB). J. Chem. Theory Comput.", "10.1021/acs.jctc.1c00471"));
        c.push((
            "Caldeweyher E. et al. (2019) D4. J. Chem. Phys.",
            "10.1063/1.5090222",
        ));
    }
    c.push((
        "Yang W., Mortier W. J. (1986) Condensed Fukui functions. J. Am. Chem. Soc.",
        "10.1021/ja00279a008",
    ));
    c
}
