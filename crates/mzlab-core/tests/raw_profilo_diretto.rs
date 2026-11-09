//! The profile of a Thermo packet decoded straight into m/z and intensity (`opentfraw::pacchetto::decodifica_profilo`, what `extract_arrays` reads) against the
//! old way (the whole packet, then `Profile::to_mz_intensity_into`): the same points to the bit, on random packets with every count of coefficients, chunks with
//! and without fudge, bins that land on frequency 0 or give m/z <= 0, odd intensities, and every truncation.
#![cfg(feature = "thermo")]
use opentfraw::pacchetto::{decodifica, decodifica_profilo};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn u32s(v: &mut Vec<u8>, x: &[u32]) {
    x.iter().for_each(|x| v.extend_from_slice(&x.to_le_bytes()));
}

const STRANI: [f32; 9] = [
    f32::NAN,
    // a signalling NaN with a payload: kept to the bit, whatever the build profile
    f32::from_bits(0x7f80_0001),
    -0.0,
    0.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::MIN_POSITIVE / 4.0,
    -3.5,
    f32::MAX,
];

/// A packet with a profile of random chunks (`first_value`, `step` as given), five centroids and their labels.
fn packet(rng: &mut Rng, fudge: bool, first_value: f64, step: f64) -> Vec<u8> {
    let mut profilo = Vec::new();
    profilo.extend_from_slice(&first_value.to_le_bytes());
    profilo.extend_from_slice(&step.to_le_bytes());
    let chunks = 1 + rng.below(40) as u32;
    u32s(&mut profilo, &[chunks, 999_999]);
    let mut bin = rng.below(5) as u32;
    for _ in 0..chunks {
        let n = rng.below(70) as u32;
        u32s(&mut profilo, &[bin, n]);
        if fudge {
            let f = (rng.below(2001) as f32 - 1000.0) * 1e-4;
            profilo.extend_from_slice(&f.to_le_bytes());
        }
        for _ in 0..n {
            let v = if rng.below(10) == 0 {
                STRANI[rng.below(9) as usize]
            } else {
                rng.below(1 << 24) as f32 * 0.37
            };
            profilo.extend_from_slice(&v.to_le_bytes());
        }
        bin += n + rng.below(30) as u32;
    }
    let n = 5u32;
    let mut list = Vec::new();
    u32s(&mut list, &[n]);
    for i in 0..n {
        list.extend_from_slice(&(100.0f32 + i as f32).to_le_bytes());
        list.extend_from_slice(&(10.0f32 * i as f32).to_le_bytes());
    }
    let layout = if fudge { 1u32 } else { 0 };
    let mut v = Vec::new();
    u32s(
        &mut v,
        &[
            0,
            (profilo.len() / 4) as u32,
            (list.len() / 4) as u32,
            layout,
            n,
            n + 1,
            6,
            0,
        ],
    );
    v.extend_from_slice(&100.0f32.to_le_bytes());
    v.extend_from_slice(&900.0f32.to_le_bytes());
    v.extend(&profilo);
    v.extend(&list);
    u32s(&mut v, &(0..n).collect::<Vec<_>>());
    u32s(&mut v, &[n]);
    for i in 0..n {
        v.extend_from_slice(&(1000.0f32 + i as f32).to_le_bytes());
    }
    for x in [100.0f32, 5.0, 1.0, 900.0, 6.0, 2.0] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    v
}

/// The points the old way gives, `None` if the packet is refused.
fn vecchia(bytes: &[u8], coeff: &[f64]) -> Option<(Vec<f64>, Vec<f32>)> {
    let p = decodifica(bytes).ok()?;
    let (mut mz, mut int) = (Vec::new(), Vec::new());
    p.profile?.to_mz_intensity_into(coeff, &mut mz, &mut int);
    Some((mz, int))
}

fn nuova(bytes: &[u8], coeff: &[f64]) -> Option<(Vec<f64>, Vec<f32>)> {
    let (mut mz, mut int) = (Vec::new(), Vec::new());
    let c = decodifica_profilo(bytes, coeff, &mut mz, &mut int).ok()?;
    c.then_some((mz, int))
}

fn bit(p: &Option<(Vec<f64>, Vec<f32>)>) -> Option<(Vec<u64>, Vec<u32>)> {
    p.as_ref().map(|(m, i)| {
        (
            m.iter().map(|x| x.to_bits()).collect(),
            i.iter().map(|x| x.to_bits()).collect(),
        )
    })
}

#[test]
fn the_direct_profile_gives_the_points_of_the_chunks_to_the_bit() {
    // none (ion trap), LTQ-FT/ICR, Orbitrap v66, Orbitrap, counts freq_to_mz does not convert, one with A < 0 (m/z <= 0 for the high frequencies)
    let coeffs: [&[f64]; 9] = [
        &[],
        &[0.0, 1.5, 2.0e8, -3.0e12],
        &[0.0, 0.0, -0.25, 1.0e13, 3.0e6],
        &[0.0, 7.0, 0.01, 2.0e13, -4.0e6, 0.0, 0.0],
        &[1.0, 2.0, 3.0],
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        &[0.0, 0.0, -400.0, 1.0e13, 3.0e6],
        // terms of the same size: a change of the order of the sums changes the last bits
        &[0.0, 0.0, 0.3, 1.0e13, 7.7e20],
        &[0.0, 0.1, 2.0e5, 3.3e10],
    ];
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut points = 0;
    for lap in 0..400 {
        let fudge = lap % 2 == 0;
        // a step of -0.25 from 25.0: bin 100 lands exactly on frequency 0, the bins after it below zero
        let (fv, st) = match lap % 3 {
            0 => (258_000.0, -0.25),
            1 => (25.0, -0.25),
            _ => (150.0 + rng.below(1000) as f64, 0.0137),
        };
        let bytes = packet(&mut rng, fudge, fv, st);
        for c in coeffs {
            let (a, b) = (vecchia(&bytes, c), nuova(&bytes, c));
            assert!(a.is_some(), "giro {lap}");
            points += a.as_ref().map_or(0, |p| p.0.len());
            assert_eq!(bit(&a), bit(&b), "giro {lap}, {} coefficients", c.len());
        }
    }
    assert!(points > 100_000, "{points}");
}

#[test]
fn a_cut_packet_is_refused_by_both_ways_at_the_same_lengths() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let bytes = packet(&mut rng, true, 258_000.0, -0.25);
    let c: &[f64] = &[0.0, 0.0, -0.25, 1.0e13, 3.0e6];
    for n in 0..bytes.len() {
        let (a, b) = (vecchia(&bytes[..n], c), nuova(&bytes[..n], c));
        assert_eq!(bit(&a), bit(&b), "{n} of {}", bytes.len());
    }
}

#[test]
fn a_packet_without_profile_gives_false_and_no_point() {
    let mut v = Vec::new();
    u32s(&mut v, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let (mut mz, mut int) = (vec![1.0], vec![2.0]);
    let c = decodifica_profilo(&v, &[], &mut mz, &mut int);
    assert!(!c.unwrap());
    assert_eq!((mz, int), (vec![1.0], vec![2.0]));
}
