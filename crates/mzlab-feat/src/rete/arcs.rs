//! The arcs of a network of mass differences, shared by the Dig (`legacy::network`) and the products network (`rete`): for every rule whose change of
//! formula takes the formula of one ion to the formula of another, one arc per pair of ions, directed by the kinetics and then from the parent.

use std::collections::BTreeMap;

use mzlab_core::elements::{mass, parse_formula};

use crate::hypo::{Counts, Rules};

/// Sampling step of the kinetics (min): a source may peak this much after its product and still be consistent.
pub const STEP: f64 = 5.0;

/// The best arc of a pair of ions: `from` and `to` are indexes into the formulas (0 = the parent), `rule` into `Rules::rules`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub from: usize,
    pub to: usize,
    pub rule: usize,
    /// `consistent`, `against` or `n/a`: whether the source peaks no later than the product.
    pub kinetic: &'static str,
}

pub fn counts(f: &str) -> Counts {
    let mut c = Counts::new();
    for (e, n) in parse_formula(f).unwrap_or_default() {
        *c.entry(e).or_insert(0) += n as i64;
    }
    c
}

/// Whether `b` is `a` plus `delta`, element by element.
pub fn same_change(a: &Counts, b: &Counts, delta: &Counts) -> bool {
    a.keys().chain(b.keys()).chain(delta.keys()).all(|e| {
        let g = |c: &Counts| c.get(e).copied().unwrap_or(0);
        g(b) - g(a) == g(delta)
    })
}

pub fn neutral_mass(c: &Counts) -> f64 {
    mass(
        &c.iter()
            .map(|(e, n)| (e.clone(), *n as u64))
            .collect::<Vec<_>>(),
    )
}

/// Best arc of every pair of ions (kinetics, then from the parent) over all the rules. Node 0 is the parent and never a target.
pub fn best_pairs(forms: &[Counts], tmax: &[Option<f64>], rules: &Rules) -> Vec<Pair> {
    let mut best: BTreeMap<(usize, usize), ((u8, u8), Pair)> = BTreeMap::new();
    for a in 0..forms.len() {
        for (ri, r) in rules.rules.iter().enumerate() {
            for b in
                (1..forms.len()).filter(|&b| b != a && same_change(&forms[a], &forms[b], &r.delta))
            {
                let kinetic = match (tmax[a], tmax[b]) {
                    (Some(ta), Some(tb)) if ta <= tb + STEP => "consistent",
                    (Some(_), Some(_)) => "against",
                    _ => "n/a",
                };
                let rank = (
                    match kinetic {
                        "consistent" => 2,
                        "n/a" => 1,
                        _ => 0,
                    },
                    u8::from(a == 0),
                );
                let key = (a.min(b), a.max(b));
                if best.get(&key).is_none_or(|e| rank > e.0) {
                    let pair = Pair {
                        from: a,
                        to: b,
                        rule: ri,
                        kinetic,
                    };
                    best.insert(key, (rank, pair));
                }
            }
        }
    }
    best.into_values().map(|e| e.1).collect()
}
