//! The encoding pool writes the same bytes as `spectrum_with` whatever the number of threads, and stops at the first error in the order of the spectra.
use sha2::{Digest as _, Sha256};

use super::*;

/// `n` spectra of very different sizes (from 3 to about 1500 points), MS1 and MS2, with and without 64-bit intensities.
fn spettri(n: usize) -> Vec<(Spectrum, SpectrumExtra)> {
    let mut seme = 99u64;
    let mut rnd = move || {
        seme = seme.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seme >> 33) as f64 / (1u64 << 31) as f64
    };
    (0..n)
        .map(|i| {
            let punti = 3 + (rnd() * rnd() * 1500.0) as usize;
            let (mut mz, mut inten) = (Vec::with_capacity(punti), Vec::with_capacity(punti));
            let mut x = 100.0;
            for _ in 0..punti {
                x += 0.01 + rnd() * 0.3;
                mz.push(x);
                inten.push((rnd() * rnd() * 1e5) as f32);
            }
            let wide = (i % 7 == 0).then(|| inten.iter().map(|&v| f64::from(v) + 0.25).collect());
            let sp = Spectrum { index: i, id: format!("scan={}", i + 1), level: if i % 5 == 0 { 1 } else { 2 }, rt: i as f64 * 0.01, polarity: 1, precursor: (i % 5 != 0).then_some(300.5), mz, intensity: inten, wide, ..Default::default() };
            (sp, SpectrumExtra::default())
        })
        .collect()
}

fn apri(opz: Opzioni, indicizzato: bool, n: usize) -> MzmlWriter<Vec<u8>> {
    let h = Header { run: "x".into(), ..Default::default() };
    let mut w = if indicizzato { MzmlWriter::indicizzato(Vec::new(), &h, opz) } else { MzmlWriter::con_opzioni(Vec::new(), &h, opz) }.unwrap();
    w.open_spectra(n).unwrap();
    w
}

fn chiudi(mut w: MzmlWriter<Vec<u8>>) -> Vec<u8> {
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The file without the pool: one `spectrum_with` after the other.
fn senza_gruppo(s: &[(Spectrum, SpectrumExtra)], opz: Opzioni, indicizzato: bool) -> Vec<u8> {
    let mut w = apri(opz, indicizzato, s.len());
    s.iter().for_each(|(sp, ex)| w.spectrum_with(sp, ex).unwrap());
    chiudi(w)
}

fn con_gruppo(s: &[(Spectrum, SpectrumExtra)], opz: Opzioni, indicizzato: bool, thread: usize) -> Vec<u8> {
    let mut w = apri(Opzioni { thread, ..opz }, indicizzato, s.len());
    w.spettri(s.iter().cloned().map(Ok)).unwrap();
    chiudi(w)
}

fn sha(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn the_bytes_are_the_same_with_1_2_7_threads_and_without_the_pool() {
    let s = spettri(2000);
    for (numpress, indicizzato) in [(false, false), (false, true), (true, false), (true, true)] {
        let opz = Opzioni { numpress, ..Opzioni::default() };
        let atteso = senza_gruppo(&s, opz, indicizzato);
        for thread in [1, 2, 7] {
            let b = con_gruppo(&s, opz, indicizzato, thread);
            assert_eq!(sha(&b), sha(&atteso), "numpress={numpress} indexed={indicizzato} threads={thread}");
            assert!(b == atteso);
        }
    }
}

#[test]
fn no_threads_and_an_empty_run_work() {
    let opz = Opzioni::default();
    let vuoto = con_gruppo(&[], opz, true, 3);
    assert!(vuoto == senza_gruppo(&[], opz, true));
    let s = spettri(5);
    assert!(con_gruppo(&s, opz, false, 0) == senza_gruppo(&s, opz, false));
}

/// What the writer has written when the pool stopped: the file up to the first `n` spectra, never closed.
fn fino_a(s: &[(Spectrum, SpectrumExtra)], n: usize, indicizzato: bool) -> Vec<u8> {
    let mut w = apri(Opzioni::default(), indicizzato, s.len());
    s[..n].iter().for_each(|(sp, ex)| w.spectrum_with(sp, ex).unwrap());
    w.out
}

#[test]
fn an_error_at_spectrum_1500_is_the_error_returned_and_the_file_is_not_closed() {
    let s = spettri(2000);
    for thread in [1, 4, 7] {
        let mut w = apri(Opzioni { thread, ..Opzioni::default() }, false, s.len());
        let guasto = |enc: &mut Codifica, k: usize, sp: &Spectrum, ex: &SpectrumExtra, t: &mut String| {
            if k == 1500 {
                return Err(Error::new("err.test.injected").with("k", k));
            }
            enc.spettro(k, sp, ex, t)
        };
        let e = w.spettri_con(s.iter().cloned().map(Ok), &guasto).unwrap_err();
        assert_eq!((e.key, e.params), ("err.test.injected", vec![("k", "1500".to_string())]));
        // nothing after the failed spectrum, nothing before it missing, and no closing tag
        assert!(w.out == fino_a(&s, 1500, false), "threads={thread}");
        assert!(!String::from_utf8_lossy(&w.out).contains("</mzML>"));
    }
}

#[test]
fn the_first_error_in_the_order_wins_over_a_later_one_and_over_the_reader() {
    let s = spettri(300);
    let mut w = apri(Opzioni { thread: 5, ..Opzioni::default() }, true, s.len());
    let guasto = |enc: &mut Codifica, k: usize, sp: &Spectrum, ex: &SpectrumExtra, t: &mut String| {
        if k == 250 || k == 120 {
            return Err(Error::new("err.test.injected").with("k", k));
        }
        enc.spettro(k, sp, ex, t)
    };
    let da_leggere = s.iter().cloned().enumerate().map(|(k, v)| if k == 200 { Err(Error::new("err.test.reader")) } else { Ok(v) });
    let e = w.spettri_con(da_leggere, &guasto).unwrap_err();
    assert_eq!((e.key, e.params), ("err.test.injected", vec![("k", "120".to_string())]));
    assert!(w.out == fino_a(&s, 120, true));
}

#[test]
fn a_reader_error_is_returned_after_the_spectra_before_it() {
    let s = spettri(300);
    let mut w = apri(Opzioni { thread: 3, ..Opzioni::default() }, false, s.len());
    let da_leggere = s.iter().cloned().enumerate().map(|(k, v)| if k == 200 { Err(Error::new("err.test.reader")) } else { Ok(v) });
    let e = w.spettri(da_leggere).unwrap_err();
    assert_eq!(e.key, "err.test.reader");
    assert!(w.out == fino_a(&s, 200, false));
}

#[test]
fn the_default_is_the_cores_minus_one_and_at_least_one() {
    let n = std::thread::available_parallelism().map_or(1, |n| n.get());
    assert_eq!(thread_predefinito(), n.saturating_sub(1).max(1));
    assert_eq!(Opzioni::default().thread, thread_predefinito());
}
