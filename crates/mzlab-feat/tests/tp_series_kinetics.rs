//! MEASURE (no default is changed): 50 consecutive-reaction chains A -> B -> C (Bateman, k1 = 0.05 /min, k2 from 0.15 to 0.40 /min, 5 % noise, fixed seed)
//! on the times 0, 5, 10, 20, 30, 45, 60 min. Share of the true arcs A->B and B->C that `tp::network::arcs` keeps (correlation 0.8 on at least 4 treated
//! times; parent->product needs the opposite sign), and how the kinetic coupling judges the intermediate B. Numbers: `docs/agenti/mzfeat-tp.md` (MISURA).

use mzlab_feat::evid::rowproofs::NONE;
use mzlab_feat::hypo::Rules;
use mzlab_feat::kinetic::{
    kinetic_report, parent_kinetics, ChainBank, FormBank, Setup, Sign, K_PARENT_MIN,
};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::*;

const TIMES: [f64; 7] = [0.0, 5.0, 10.0, 20.0, 30.0, 45.0, 60.0];
const O: f64 = 15.994915;
const K1: f64 = 0.05;
const CHAINS: usize = 50;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Areas of A, B and C at `TIMES` for the rate `k2`: response factors 1, 0.5 and 0.4, multiplicative noise of 5 % (uniform).
fn chain(k2: f64, rng: &mut Lcg) -> [Vec<f64>; 3] {
    let a0 = 1e8;
    let mut out = [vec![], vec![], vec![]];
    for t in TIMES {
        let a = (-K1 * t).exp();
        let b = K1 / (k2 - K1) * ((-K1 * t).exp() - (-k2 * t).exp());
        let c = 1.0 - a - b;
        for (o, (x, f)) in out.iter_mut().zip([(a, 1.0), (b, 0.5), (c, 0.4)]) {
            o.push(a0 * f * x * (1.0 + 0.1 * (rng.next() - 0.5)));
        }
    }
    out
}

fn rules() -> Rules {
    let j = r#"{"format":"regole_tp","version":1,"regole":[{"id":"oh","nome":"hydroxylation","delta_formula":"+O","passi_max":1,"livello_previsione":3,"fonte":"transformation_table","direzione_rt":"piu_polare"}]}"#;
    Rules::parse(j.as_bytes()).unwrap()
}

/// Files: blank, then the 7 times of `TIMES` (t0 included).
fn minutes() -> Vec<f64> {
    std::iter::once(f64::NAN).chain(TIMES).collect()
}

fn table(rows: &[(f64, f64, &Vec<f64>)]) -> Aligned {
    let mut t = Aligned::with_capacity(8, rows.len());
    for (mz, rt, a) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for v in std::iter::once(&0.0).chain(a.iter()) {
            t.area.push(*v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(*rt as f32);
            t.width.push(0.1);
            t.state
                .push(if *v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

/// Result of `measure`: chains whose arc A->B and B->C is kept, B judged against / for, C judged against / for.
#[derive(Debug, PartialEq)]
struct Tally {
    ab: usize,
    bc: usize,
    b_against: usize,
    b_for: usize,
    c_against: usize,
    c_for: usize,
}

/// Arcs of the three rows (0 = parent) and the kinetic sign of rows 1 and 2, for one chain `[a, b, c]`.
fn judge(
    a: &[Vec<f64>; 3],
    r: &Rules,
    forms: &FormBank,
    bank: &ChainBank,
) -> (bool, bool, [Sign; 2]) {
    let (mins, m0) = (minutes(), 300.0);
    let t = table(&[
        (m0, 5.0, &a[0]),
        (m0 + O, 4.5, &a[1]),
        (m0 + 2.0 * O, 4.0, &a[2]),
    ]);
    let signs = vec![[NONE; 10]; 3];
    let inp = TpInputs {
        aligned: &t,
        minutes: &mins,
        hyps: &[],
        signs: &signs,
        parent: (m0, 5.0),
        fragments: None,
        infusion: None,
        params: &TpParams::default(),
        rules: None,
    };
    let blk = build(&inp).unwrap().expect("the parent is in the table");
    let arcs = arcs(&inp, r, &blk, &Default::default());
    let s = Setup {
        times: &TIMES,
        forms,
        chain: bank,
        parent: parent_kinetics(forms, &TIMES, &a[0], None),
        pseudo: 1e6,
    };
    let sign = |y: &[f64]| kinetic_report(&s, y, &[0.0, 0.0]).evidence.sign;
    let has = |p: (u32, u32)| arcs.iter().any(|x| (x.from, x.to) == p);
    (has((0, 1)), has((1, 2)), [sign(&a[1]), sign(&a[2])])
}

fn measure() -> Tally {
    let mut rng = Lcg(7);
    let r = rules();
    let forms = FormBank::new(&TIMES);
    let bank = ChainBank::new(&TIMES, K_PARENT_MIN);
    let mut tl = Tally {
        ab: 0,
        bc: 0,
        b_against: 0,
        b_for: 0,
        c_against: 0,
        c_for: 0,
    };
    for i in 0..CHAINS {
        let k2 = 0.15 + 0.25 * i as f64 / (CHAINS - 1) as f64;
        let (ab, bc, [sb, sc]) = judge(&chain(k2, &mut rng), &r, &forms, &bank);
        tl.ab += ab as usize;
        tl.bc += bc as usize;
        tl.b_against += (sb == Sign::Against) as usize;
        tl.b_for += (sb == Sign::For) as usize;
        tl.c_against += (sc == Sign::Against) as usize;
        tl.c_for += (sc == Sign::For) as usize;
    }
    tl
}

/// Control of the harness: a product that only rises (k2 -> infinity is B = 1 - A, C absent) must keep its arc from the parent.
fn control_keeps_arc() -> bool {
    let a: Vec<f64> = TIMES.iter().map(|t| 1e8 * (-K1 * t).exp()).collect();
    let b: Vec<f64> = TIMES
        .iter()
        .map(|t| 1e8 * (1.0 - (-K1 * t).exp()) * 0.5)
        .collect();
    let c: Vec<f64> = TIMES.iter().map(|_| 0.0).collect();
    let forms = FormBank::new(&TIMES);
    let bank = ChainBank::new(&TIMES, K_PARENT_MIN);
    judge(&[a, b, c], &rules(), &forms, &bank).0
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let cov: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let (vx, vy): (f64, f64) = (
        x.iter().map(|a| (a - mx).powi(2)).sum(),
        y.iter().map(|b| (b - my).powi(2)).sum(),
    );
    cov / (vx * vy).sqrt()
}

/// Why the arcs fail: over the treated times the intermediate B falls late, so it co-varies with A (the network wants the opposite sign from the
/// parent) and against C (it wants the same sign between products). Noise-free ranges over k2.
#[test]
fn series_kinetics_measure() {
    assert!(
        control_keeps_arc(),
        "harness: a monotonic product keeps its arc"
    );
    let mut rng = Lcg(7);
    let (mut ab, mut bc) = ((f64::MAX, f64::MIN), (f64::MAX, f64::MIN));
    for i in 0..CHAINS {
        let k2 = 0.15 + 0.25 * i as f64 / (CHAINS - 1) as f64;
        let [a, b, c] = chain(k2, &mut rng);
        let (r1, r2) = (pearson(&a[1..], &b[1..]), pearson(&b[1..], &c[1..]));
        ab = (r1.min(ab.0), r1.max(ab.1));
        bc = (r2.min(bc.0), r2.max(bc.1));
    }
    // with 5 % noise: corr(A,B) in [0.8, 1], corr(B,C) in [-1, -0.8], so neither arc can hold
    assert!(
        ab.0 > 0.8 && bc.1 < -0.8,
        "corr(A,B) {ab:?}, corr(B,C) {bc:?}"
    );
    let want = Tally {
        ab: 0,
        bc: 0,
        b_against: 0,
        b_for: 49,
        c_against: 0,
        c_for: 5,
    };
    assert_eq!(measure(), want);
}
