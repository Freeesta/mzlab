//! `features.group@1` against a synthetic truth: invented compounds (no real names) seen as several ion species with their isotopologues in eight files,
//! co-eluting pairs of different compounds and lone rows. The grouping must reach ARI >= 0.8 against the compounds, find >= 0.8 of the true ion identity
//! edges, and keep every row of the per-ion table.

use mzlab_core::json::{parse, Value};
use mzlab_feat::ionnet::{group, group_text, params_of, EdgeKind};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

const FILES: usize = 8;
const PROTON: f64 = 1.007276;
/// (name, shift, molecules) of the species the generator draws.
const SPECIES: [(&str, f64, f64); 5] = [
    ("[M+H]+", PROTON, 1.0),
    ("[M+Na]+", 22.989218, 1.0),
    ("[M+NH4]+", 18.033823, 1.0),
    ("[M+H-H2O]+", PROTON - 18.010565, 1.0),
    ("[2M+H]+", PROTON, 2.0),
];

/// xorshift64*: the same numbers on every platform.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

/// One row of the truth: compound, species (None = isotopologue) and the row itself.
struct Truth {
    compound: Vec<usize>,
    species: Vec<Option<usize>>,
}

fn push_row(t: &mut Aligned, rng: &mut Rng, mz: f64, rt: f64, heights: &[f64]) {
    t.mz.push(mz * (1.0 + rng.range(-2e-6, 2e-6)));
    t.rt.push(rt);
    for &h in heights {
        let seen = h > 0.0;
        t.area.push(h * 6.0);
        t.height.push(h as f32);
        t.apex_rt
            .push((rt + if seen { rng.range(-0.005, 0.005) } else { 0.0 }) as f32);
        t.width.push(0.08);
        t.state.push(if seen { CELL_DETECTED } else { CELL_ABSENT });
    }
}

/// The synthetic series: `n` compounds, most with 2-4 species, every species with its M+1 (and M+2 for the strong ones).
fn series(seed: u64, n: usize) -> (Aligned, Truth) {
    let mut rng = Rng(seed);
    let mut t = Aligned::with_capacity(FILES, n * 8);
    let mut truth = Truth {
        compound: Vec::new(),
        species: Vec::new(),
    };
    for c in 0..n {
        let m = rng.range(150.0, 700.0);
        // pairs of compounds share an RT (co-elution within the tolerance) but not their levels over the files
        let rt = 1.0 + (c / 2) as f64 * 0.37 + if c % 2 == 1 { 0.03 } else { 0.0 };
        let level: Vec<f64> = (0..FILES)
            .map(|f| {
                if (c + f) % 7 == 3 {
                    0.0
                } else {
                    rng.range(0.2, 5.0) * 1e5
                }
            })
            .collect();
        let k = if c % 5 == 4 {
            1
        } else {
            2 + (rng.next() * 3.0) as usize
        };
        let mut kinds: Vec<usize> = vec![0];
        while kinds.len() < k {
            let s = 1 + (rng.next() * 4.0) as usize % 4;
            if !kinds.contains(&s) {
                kinds.push(s);
            }
        }
        for (i, &s) in kinds.iter().enumerate() {
            let (_, shift, mol) = SPECIES[s];
            let mz = mol * m + shift;
            let share = if i == 0 { 1.0 } else { rng.range(0.1, 0.6) };
            let h: Vec<f64> = level
                .iter()
                .map(|&l| l * share * rng.range(0.93, 1.07))
                .collect();
            push_row(&mut t, &mut rng, mz, rt, &h);
            truth.compound.push(c);
            truth.species.push(Some(s));
            let r1 = 0.000_475 * mz * mol;
            let h1: Vec<f64> = h
                .iter()
                .map(|&x| {
                    if x > 0.0 {
                        x * r1 * rng.range(0.95, 1.05)
                    } else {
                        0.0
                    }
                })
                .collect();
            push_row(&mut t, &mut rng, mz + 1.003_354_8, rt, &h1);
            truth.compound.push(c);
            truth.species.push(None);
        }
    }
    (t, truth)
}

fn choose2(n: f64) -> f64 {
    n * (n - 1.0) / 2.0
}

/// Adjusted Rand index of two labelings.
fn ari(a: &[usize], b: &[u32]) -> f64 {
    let n = a.len();
    let (ka, kb) = (
        a.iter().max().unwrap() + 1,
        *b.iter().max().unwrap() as usize + 1,
    );
    let mut cont = vec![0f64; ka * kb];
    for i in 0..n {
        cont[a[i] * kb + b[i] as usize] += 1.0;
    }
    let sum_ij: f64 = cont.iter().map(|&x| choose2(x)).sum();
    let sa: f64 = (0..ka)
        .map(|i| choose2((0..kb).map(|j| cont[i * kb + j]).sum()))
        .sum();
    let sb: f64 = (0..kb)
        .map(|j| choose2((0..ka).map(|i| cont[i * kb + j]).sum()))
        .sum();
    let exp = sa * sb / choose2(n as f64);
    (sum_ij - exp) / (0.5 * (sa + sb) - exp)
}

fn defaults() -> mzlab_feat::ionnet::Params {
    params_of(&[], 1).unwrap().0
}

#[test]
fn groups_match_the_compounds() {
    for seed in [7u64, 99, 2026] {
        let (t, truth) = series(seed, 40);
        let g = group(&t, &defaults());
        let score = ari(&truth.compound, &g.group);
        assert!(score >= 0.8, "seed {seed}: ARI {score}");
    }
}

#[test]
fn ion_identity_edges_are_found() {
    for seed in [7u64, 99, 2026] {
        let (t, truth) = series(seed, 40);
        let g = group(&t, &defaults());
        let found = |a: usize, b: usize| {
            g.edges
                .iter()
                .any(|e| e.kind == EdgeKind::Ion && e.a == a && e.b == b)
        };
        let (mut want, mut got) = (0, 0);
        for a in 0..t.len() {
            for b in a + 1..t.len() {
                if truth.compound[a] == truth.compound[b]
                    && truth.species[a].is_some()
                    && truth.species[b].is_some()
                {
                    want += 1;
                    got += usize::from(found(a, b));
                }
            }
        }
        let recall = got as f64 / want as f64;
        assert!(recall >= 0.8, "seed {seed}: recall {recall} ({got}/{want})");
        // the species named on a row is the true one
        let named = (0..t.len()).filter(|&r| g.ion[r].is_some()).count();
        let right = (0..t.len())
            .filter(|&r| matches!((g.ion[r], truth.species[r]), (Some(i), Some(s)) if defaults().ions[i].name == SPECIES[s].0))
            .count();
        assert!(
            right as f64 >= 0.9 * named as f64,
            "seed {seed}: {right}/{named} species right"
        );
    }
}

#[test]
fn every_row_stays() {
    let (t, _) = series(7, 40);
    let origin = Origin {
        inputs: (0..FILES)
            .map(|k| (format!("f{k}.mzML"), format!("{k:02x}").repeat(32)))
            .collect(),
        ..Origin::default()
    };
    let bytes = write_table(&t, &Recipe::default(), &origin);
    let text = group_text(&[], &bytes).unwrap();
    let v: Value = parse(text.as_bytes()).unwrap();
    assert_eq!(v.get("format").and_then(Value::str), Some("ionnet/1"));
    let ids: Vec<u32> = v
        .get("id")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap() as u32)
        .collect();
    assert_eq!(ids, (1..=t.len() as u32).collect::<Vec<_>>());
    for col in ["group", "corr_group", "charge", "iso", "ion", "neutral"] {
        assert_eq!(v.get(col).unwrap().arr().len(), t.len(), "{col}");
    }
    assert!(v
        .get("group")
        .unwrap()
        .arr()
        .iter()
        .all(|g| g.f64().unwrap() >= 1.0));
    // the same bytes twice
    assert_eq!(group_text(&[], &bytes).unwrap(), text);
}

#[test]
fn an_unknown_species_is_refused() {
    let p = vec![("ions_pos".to_string(), Value::Str("[M+H]+,[M+Xx]+".into()))];
    let e = params_of(&p, 1).unwrap_err();
    assert_eq!(e.key, "err.flow.group.ion");
    let p = vec![("polarity".to_string(), Value::Str("negative".into()))];
    let (q, positive) = params_of(&p, 1).unwrap();
    assert!(!positive && q.ions[0].name == "[M-H]-");
}
