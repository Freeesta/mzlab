use super::*;
use crate::mzidx;
use crate::mzml::{self, Item, Spectrum};
use crate::mzmlwrite::{Header, MzmlWriter, Opzioni};
use std::collections::HashMap;
use std::io::Cursor;

/// A courier that decodes in this process: `refuse` every n-th send, `lose` every n-th answer (0 = never).
struct Locale {
    window: usize,
    dec: Decodificatore,
    answers: HashMap<u64, Option<Vec<u8>>>,
    sent: u64,
    refuse: u64,
    lose: u64,
}

impl Locale {
    fn new(window: usize, refuse: u64, lose: u64) -> Self {
        Locale { window, dec: Decodificatore::default(), answers: HashMap::new(), sent: 0, refuse, lose }
    }
}

impl Corriere for Locale {
    fn window(&self) -> usize {
        self.window
    }
    fn send(&mut self, seq: u64, job: &[u8], bound: usize) -> bool {
        self.sent += 1;
        if self.refuse > 0 && self.sent % self.refuse == 0 {
            return false;
        }
        // the numbers of the jobs out are consecutive, and never more than the window are out
        assert_eq!(seq, self.answers.keys().max().map_or(seq, |m| m + 1));
        assert!(self.answers.len() < self.window);
        let a = self.dec.decode(job);
        assert!(a.as_ref().is_none_or(|a| a.len() <= bound), "answer over its bound");
        let a = if self.lose > 0 && seq % self.lose == 1 { None } else { a };
        self.answers.insert(seq, a);
        true
    }
    fn take(&mut self, seq: u64) -> Option<Vec<u8>> {
        self.answers.remove(&seq).expect("taken twice or never sent")
    }
}

/// A small profile mzML like the one of the wasm bench: MS1 profile scans, three MS2 after every fourth.
fn file(f64_intensity: bool) -> Vec<u8> {
    let mut seed = 11u64;
    let mut rnd = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let feats: Vec<(f64, f64)> = (0..40).map(|_| (100.0 + 900.0 * rnd(), 1e4 + 1e6 * rnd())).collect();
    let mut sp = Vec::new();
    for i in 0..60usize {
        let mut mz = Vec::new();
        let mut it = Vec::new();
        for &(m, h) in &feats {
            for k in -3i32..=3 {
                mz.push(m + f64::from(k) * 0.001);
                it.push((h * rnd()) as f32);
            }
        }
        let idx = sp.len();
        let wide = f64_intensity.then(|| it.iter().map(|&x| f64::from(x) + 0.25).collect());
        sp.push(Spectrum { index: idx, id: format!("scan={}", idx + 1), level: 1, rt: i as f64 * 0.1, polarity: 1, profile: true, mz, intensity: it, wide, ..Default::default() });
        if i % 4 == 0 {
            for _ in 0..3 {
                let prec = 150.0 + 700.0 * rnd();
                let mz: Vec<f64> = (0..20).map(|j| 50.0 + f64::from(j) * 7.0 + rnd()).collect();
                let it = mz.iter().map(|_| (1e5 * rnd()) as f32).collect();
                let idx = sp.len();
                sp.push(Spectrum { index: idx, id: format!("scan={}", idx + 1), level: 2, rt: i as f64 * 0.1, polarity: 1, precursor: Some(prec), mz, intensity: it, ..Default::default() });
            }
        }
    }
    let mut w = MzmlWriter::con_opzioni(Vec::new(), &Header { run: "x".into(), ..Default::default() }, Opzioni { livello: 1, ..Opzioni::default() }).unwrap();
    w.open_spectra(sp.len()).unwrap();
    for s in &sp {
        w.spectrum(s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn spectra(b: &[u8], c: Option<&mut dyn Corriere>) -> Vec<Spectrum> {
    let mut out = Vec::new();
    let f = |it| {
        if let Item::Spectrum(s) = it {
            out.push(s);
        }
    };
    match c {
        Some(c) => mzml::parse_corriere(Cursor::new(b), f, c).unwrap(),
        None => mzml::parse(Cursor::new(b), f).unwrap(),
    }
    out
}

/// Through a courier (any window, refused jobs, lost answers) the spectra are those of `parse`.
#[test]
fn courier_gives_the_spectra_of_parse() {
    for wide in [false, true] {
        let b = file(wide);
        let want = spectra(&b, None);
        assert!(want.len() > 100 && want.iter().any(|s| s.wide.is_some()) == wide);
        for (window, refuse, lose) in [(0, 0, 0), (1, 0, 0), (3, 0, 0), (12, 5, 3), (2, 2, 2)] {
            let mut c = Locale::new(window, refuse, lose);
            assert_eq!(spectra(&b, Some(&mut c)), want, "window {window} refuse {refuse} lose {lose}");
            assert!(c.answers.is_empty());
        }
    }
}

/// The `.mzidx` built through a courier is the same, byte for byte, in memory, with a scratch file and in rounds that decode again.
#[test]
fn courier_gives_the_same_index() {
    let b = file(false);
    let mut v = b.clone();
    let p = crate::upload::prepare(&mut v, b.len() as u64, "x.mzML").unwrap();
    for budget in [64 << 20, 1 << 20] {
        let (want, st) = mzidx::build_mzml_to_vec(&p.table, &b, budget).unwrap();
        for spill in [false, true] {
            for window in [1, 4] {
                let mut c = Locale::new(window, 7, 5);
                let mut out = Cursor::new(Vec::new());
                let mut sp = Cursor::new(Vec::new());
                let got = mzidx::build_mzml_corriere(&p.table, Cursor::new(&b), &mut out, budget, spill.then_some(&mut sp as &mut dyn crate::mzidx_spill::Spill), &mut c).unwrap();
                assert!(out.into_inner() == want, "budget {budget} spill {spill} window {window}");
                if !spill {
                    assert_eq!(got.passes, st.passes);
                }
            }
        }
        if budget == 1 << 20 {
            assert!(st.passes > 1, "the small budget must decode again");
        }
    }
}

/// What is not a job is refused, not decoded into something else.
#[test]
fn bad_jobs_are_refused() {
    let mut d = Decodificatore::default();
    for job in [&b""[..], &[1, 2, 3], &[0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0]] {
        assert!(d.decode(job).is_none());
    }
    assert!(read_peaks(&[0; 15]).is_none() && read_peaks(&[0; 17]).is_none());
}
