//! The micro-benchmarks that give the ceilings of the roofline: each stage of the conversion alone, on arrays and packets of the three shapes
//! (centroid list only, Fusion profile, dense profile), with nothing else running. `tmin` puts them in the formula of `RISPOSTE.md` §1.3.
use std::hint::black_box;
use std::time::Instant;

use base64::Engine;
use flate2::{Compress, Compression, FlushCompress};
use serde_json::{json, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni, SpectrumExtra};
use opentfraw::pacchetto::{decodifica, decodifica_picchi};

use crate::denso::{pacchetto, Parametri, COEFF};
use crate::fasi::Sink;
use crate::misura::arrotonda;

/// Seconds the repeated operation runs for, at least; the cost per operation is read from the last, longest pass.
const MIN_S: f64 = 0.4;

/// Nanoseconds per call of `f`: doubles the number of calls until the pass lasts `MIN_S`, then takes the median of three passes.
pub fn ns_per_op(mut f: impl FnMut()) -> f64 {
    let mut n = 1u64;
    loop {
        let t = Instant::now();
        for _ in 0..n {
            f();
        }
        let s = t.elapsed().as_secs_f64();
        if s >= MIN_S / 4.0 {
            let mut v = vec![s / n as f64];
            for _ in 0..2 {
                let t = Instant::now();
                for _ in 0..n {
                    f();
                }
                v.push(t.elapsed().as_secs_f64() / n as f64);
            }
            v.sort_by(f64::total_cmp);
            return v[1] * 1e9;
        }
        n *= 2;
    }
}

fn forma(nome: &str) -> Parametri {
    match nome {
        "lab-a" => Parametri::forma_lab_a(1),
        "lab-b" => Parametri::forma_lab_b(1),
        _ => Parametri::denso(1),
    }
}

const FORME: [&str; 3] = ["lab-a", "lab-b", "denso"];

fn decodifica_ft() -> Value {
    let mut out = Vec::new();
    for nome in FORME {
        let mut b = Vec::new();
        pacchetto(&mut b, &forma(nome), 0, None);
        let completo = ns_per_op(|| {
            black_box(decodifica(black_box(&b)).ok());
        });
        let solo_lista = ns_per_op(|| {
            black_box(decodifica_picchi(black_box(&b)).ok());
        });
        out.push(json!({
            "forma": nome, "byte_pacchetto": b.len(),
            "pacchetto_intero_ns": arrotonda(completo), "pacchetto_intero_mb_s": arrotonda(b.len() as f64 / completo * 1e3),
            "solo_lista_ns": arrotonda(solo_lista), "solo_lista_mb_s": arrotonda(b.len() as f64 / solo_lista * 1e3),
        }));
    }
    json!(out)
}

fn conversione_mz() -> Value {
    let mut b = Vec::new();
    pacchetto(&mut b, &Parametri::denso(1), 0, None);
    let pk = decodifica(&b).expect("packet");
    let p = pk.profile.expect("profile");
    let punti: usize = p.chunks.iter().map(|c| c.signal.len()).sum();
    let ns = ns_per_op(|| {
        black_box(p.to_mz_intensity(black_box(&COEFF)));
    });
    json!({"punti": punti, "ns_per_punto": arrotonda(ns / punti as f64), "mpunti_s": arrotonda(punti as f64 / ns * 1e3)})
}

fn spettro(n: usize) -> Spectrum {
    Spectrum {
        index: 0,
        id: "controllerType=0 controllerNumber=1 scan=1".into(),
        level: 1,
        rt: 1.5,
        polarity: 1,
        profile: false,
        filter: "FTMS + p ESI Full ms [150.0000-2000.0000]".into(),
        precursor: None,
        mz: (0..n).map(|i| 100.0 + i as f64 * 0.37).collect(),
        intensity: (0..n).map(|i| 1000.0 + (i * 31 % 977) as f32).collect(),
        wide: None,
    }
}

fn intestazione() -> Header {
    Header {
        run: "bench".into(),
        software: ("mzLab".into(), "bench".into()),
        ..Header::default()
    }
}

/// The XML of one spectrum: head with the trailer userParams, empty arrays, level 0; and the same with real arrays.
fn xml() -> Value {
    let extra = SpectrumExtra {
        params: (0..130)
            .map(|i| {
                (
                    format!("opentfraw.trailer.field {i}"),
                    format!("{}", i * 7 % 9973),
                )
            })
            .collect(),
        ..SpectrumExtra::default()
    };
    let opz = Opzioni {
        livello: 0,
        numpress: false,
        thread: 1,
    };
    let mut righe = Vec::new();
    for punti in [0usize, 64, 526] {
        let s = spettro(punti);
        let mut w =
            MzmlWriter::con_opzioni(Sink::nuovo(false), &intestazione(), opz).expect("writer");
        w.open_spectra(1 << 30).expect("open");
        let ns = ns_per_op(|| {
            w.spectrum_with(black_box(&s), black_box(&extra))
                .expect("spectrum");
        });
        let mut w1 =
            MzmlWriter::con_opzioni(Sink::nuovo(false), &intestazione(), opz).expect("writer");
        w1.spectrum_with(&s, &extra).expect("spectrum");
        let b = w1.finish().expect("finish").n;
        righe.push(
            json!({"punti": punti, "ns_per_spettro": arrotonda(ns), "byte_per_spettro_circa": b}),
        );
    }
    json!(righe)
}

fn base64() -> Value {
    let dati: Vec<u8> = (0..1 << 20)
        .map(|i: u32| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect();
    let mut s = String::with_capacity(dati.len() * 4 / 3 + 8);
    let ns = ns_per_op(|| {
        s.clear();
        base64::engine::general_purpose::STANDARD.encode_string(black_box(&dati), &mut s);
    });
    json!({"gb_s": arrotonda(dati.len() as f64 / ns)})
}

/// The arrays of a spectrum as `Codifica::array` makes them before zlib: the values as little-endian floats.
fn array_f64_f32(pk: &[u8], solo_lista: bool) -> (Vec<u8>, Vec<u8>) {
    let (mz, int): (Vec<f64>, Vec<f32>) = if solo_lista {
        decodifica_picchi(pk)
            .expect("list")
            .iter()
            .map(|p| (p.mz, p.abundance))
            .unzip()
    } else {
        let pr = decodifica(pk).expect("packet").profile.expect("profile");
        pr.to_mz_intensity(&COEFF)
            .iter()
            .filter(|(m, _)| *m > 0.0)
            .map(|(m, i)| (*m, *i as f32))
            .unzip()
    };
    (
        mz.iter().flat_map(|x| x.to_le_bytes()).collect(),
        int.iter().flat_map(|x| x.to_le_bytes()).collect(),
    )
}

/// zlib as `Codifica::deflate` uses it: the output through a chunk of 32 KB.
fn deflate(z: &mut Compress, dentro: &[u8], blocco: &mut [u8], fuori: &mut Vec<u8>) {
    fuori.clear();
    let mut fatto = 0;
    loop {
        let (i0, o0) = (z.total_in(), z.total_out());
        let st = z
            .compress(&dentro[fatto..], blocco, FlushCompress::Finish)
            .expect("deflate");
        fatto += (z.total_in() - i0) as usize;
        fuori.extend_from_slice(&blocco[..(z.total_out() - o0) as usize]);
        if st == flate2::Status::StreamEnd {
            return;
        }
    }
}

fn compressione() -> Value {
    let motore = if cfg!(feature = "banco-zlib-rs") {
        "zlib-rs"
    } else {
        "miniz_oxide"
    };
    let mut righe = Vec::new();
    let casi = [
        ("lab-a-centroidi", "lab-a", true),
        ("lab-b-centroidi", "lab-b", true),
        ("lab-b-profilo", "lab-b", false),
        ("denso-profilo", "denso", false),
    ];
    for (nome, f, lista) in casi {
        let mut pk = Vec::new();
        pacchetto(&mut pk, &forma(f), 0, None);
        let (mz, int) = array_f64_f32(&pk, lista);
        let totale = mz.len() + int.len();
        for livello in [0u32, 1, 6] {
            let mut blocco = vec![0u8; 32 * 1024];
            let mut fuori = Vec::new();
            let mut z = Compress::new(Compression::new(livello), true);
            let con_reset = ns_per_op(|| {
                for a in [&mz, &int] {
                    z.reset();
                    deflate(&mut z, black_box(a), &mut blocco, &mut fuori);
                }
            });
            let mut zn = Compress::new(Compression::new(livello), true);
            let nuovo = ns_per_op(|| {
                for a in [&mz, &int] {
                    zn = Compress::new(Compression::new(livello), true);
                    deflate(&mut zn, black_box(a), &mut blocco, &mut fuori);
                }
            });
            let mut compressi = 0;
            for a in [&mz, &int] {
                z.reset();
                deflate(&mut z, a, &mut blocco, &mut fuori);
                compressi += fuori.len();
            }
            righe.push(json!({
                "caso": nome, "livello": livello, "byte_array": totale, "rapporto": arrotonda(compressi as f64 / totale as f64),
                "con_reset_us_per_coppia": arrotonda(con_reset / 1e3), "con_reset_mb_s": arrotonda(totale as f64 / con_reset * 1e3),
                "nuovo_ogni_array_us_per_coppia": arrotonda(nuovo / 1e3),
            }));
        }
    }
    let mut z = Compress::new(Compression::new(1), true);
    let reset_ns = ns_per_op(|| z.reset());
    json!({"motore": motore, "prove": righe, "reset_livello1_ns": arrotonda(reset_ns)})
}

fn impronte() -> Value {
    let dati: Vec<u8> = (0..64u32 << 20)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect();
    let mb = dati.len() as f64 / 1e6;
    let t = Instant::now();
    black_box(Sha1::digest(black_box(&dati)));
    let sha1 = mb / t.elapsed().as_secs_f64();
    let t = Instant::now();
    black_box(Sha256::digest(black_box(&dati)));
    let sha256 = mb / t.elapsed().as_secs_f64();
    #[cfg(target_arch = "x86_64")]
    let sha_ni = std::arch::is_x86_feature_detected!("sha");
    #[cfg(not(target_arch = "x86_64"))]
    let sha_ni = false;
    json!({"sha1_mb_s": arrotonda(sha1), "sha256_mb_s": arrotonda(sha256), "cpu_sha_ni": sha_ni})
}

fn memoria() -> Value {
    let ns_malloc = ns_per_op(|| {
        black_box(Vec::<u8>::with_capacity(black_box(4096)));
    });
    let mut v = vec![0u8; 400 << 10];
    let ns_set = ns_per_op(|| {
        v.fill(black_box(0));
        black_box(&v);
    });
    json!({"malloc_free_ns": arrotonda(ns_malloc), "azzeramento_400kb_ns": arrotonda(ns_set), "azzeramento_gb_s": arrotonda(v.len() as f64 / ns_set)})
}

/// All the micro-benchmarks.
pub fn tutti() -> Value {
    json!({
        "decodifica_ft": decodifica_ft(), "conversione_mz": conversione_mz(), "xml_spettro": xml(), "base64": base64(),
        "compressione": compressione(), "impronte": impronte(), "memoria": memoria(),
    })
}
