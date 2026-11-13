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
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options, SpectrumExtra};
use opentfraw::pacchetto::{decodifica, decodifica_picchi};

use crate::dense::{packet, Parameters, COEFF};
use crate::measure::round_to;
use crate::phases::Sink;

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
            mzlab_core::sortc::slice_by(&mut v, f64::total_cmp);
            return v[1] * 1e9;
        }
        n *= 2;
    }
}

fn shape(name: &str) -> Parameters {
    match name {
        "lab-a" => Parameters::lab_shape_a(1),
        "lab-b" => Parameters::lab_shape_b(1),
        _ => Parameters::denso(1),
    }
}

const SHAPES: [&str; 3] = ["lab-a", "lab-b", "denso"];

fn decode_ft() -> Value {
    let mut out = Vec::new();
    for name in SHAPES {
        let mut b = Vec::new();
        packet(&mut b, &shape(name), 0, None);
        let complete = ns_per_op(|| {
            black_box(decodifica(black_box(&b)).ok());
        });
        let list_only = ns_per_op(|| {
            black_box(decodifica_picchi(black_box(&b)).ok());
        });
        out.push(json!({
            "forma": name, "byte_pacchetto": b.len(),
            "pacchetto_intero_ns": round_to(complete), "pacchetto_intero_mb_s": round_to(b.len() as f64 / complete * 1e3),
            "solo_lista_ns": round_to(list_only), "solo_lista_mb_s": round_to(b.len() as f64 / list_only * 1e3),
        }));
    }
    json!(out)
}

fn mz_conversion() -> Value {
    let mut b = Vec::new();
    packet(&mut b, &Parameters::denso(1), 0, None);
    let pk = decodifica(&b).expect("packet");
    let p = pk.profile.expect("profile");
    let points: usize = p.chunks.iter().map(|c| c.signal.len()).sum();
    let ns = ns_per_op(|| {
        black_box(p.to_mz_intensity(black_box(&COEFF)));
    });
    json!({"punti": points, "ns_per_punto": round_to(ns / points as f64), "mpunti_s": round_to(points as f64 / ns * 1e3)})
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

fn scan_header() -> Header {
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
    let opz = Options {
        livello: 0,
        numpress: false,
        thread: 1,
        dad: false,
    };
    let mut rows = Vec::new();
    for points in [0usize, 64, 526] {
        let s = spettro(points);
        let mut w =
            MzmlWriter::with_options(Sink::nuovo(false), &scan_header(), opz).expect("writer");
        w.open_spectra(1 << 30).expect("open");
        let ns = ns_per_op(|| {
            w.spectrum_with(black_box(&s), black_box(&extra))
                .expect("spectrum");
        });
        let mut w1 =
            MzmlWriter::with_options(Sink::nuovo(false), &scan_header(), opz).expect("writer");
        w1.spectrum_with(&s, &extra).expect("spectrum");
        let b = w1.finish().expect("finish").n;
        rows.push(
            json!({"punti": points, "ns_per_spettro": round_to(ns), "byte_per_spettro_circa": b}),
        );
    }
    json!(rows)
}

fn base64() -> Value {
    let data: Vec<u8> = (0..1 << 20)
        .map(|i: u32| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect();
    let mut s = String::with_capacity(data.len() * 4 / 3 + 8);
    let ns = ns_per_op(|| {
        s.clear();
        base64::engine::general_purpose::STANDARD.encode_string(black_box(&data), &mut s);
    });
    json!({"gb_s": round_to(data.len() as f64 / ns)})
}

/// The arrays of a spectrum as `Encoding::array` makes them before zlib: the values as little-endian floats.
fn array_f64_f32(pk: &[u8], list_only: bool) -> (Vec<u8>, Vec<u8>) {
    let (mz, int): (Vec<f64>, Vec<f32>) = if list_only {
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

/// zlib as `Encoding::deflate` uses it: the output through a chunk of 32 KB.
fn deflate(z: &mut Compress, inside: &[u8], block: &mut [u8], outside: &mut Vec<u8>) {
    outside.clear();
    let mut done = 0;
    loop {
        let (i0, o0) = (z.total_in(), z.total_out());
        let st = z
            .compress(&inside[done..], block, FlushCompress::Finish)
            .expect("deflate");
        done += (z.total_in() - i0) as usize;
        outside.extend_from_slice(&block[..(z.total_out() - o0) as usize]);
        if st == flate2::Status::StreamEnd {
            return;
        }
    }
}

fn compression() -> Value {
    let engine = if cfg!(feature = "banco-zlib-rs") {
        "zlib-rs"
    } else {
        "miniz_oxide"
    };
    let mut rows = Vec::new();
    let cases = [
        ("lab-a-centroidi", "lab-a", true),
        ("lab-b-centroidi", "lab-b", true),
        ("lab-b-profilo", "lab-b", false),
        ("denso-profilo", "denso", false),
    ];
    for (name, f, list) in cases {
        let mut pk = Vec::new();
        packet(&mut pk, &shape(f), 0, None);
        let (mz, int) = array_f64_f32(&pk, list);
        let total = mz.len() + int.len();
        for level in [0u32, 1, 6] {
            let mut block = vec![0u8; 32 * 1024];
            let mut outside = Vec::new();
            let mut z = Compress::new(Compression::new(level), true);
            let with_reset = ns_per_op(|| {
                for a in [&mz, &int] {
                    z.reset();
                    deflate(&mut z, black_box(a), &mut block, &mut outside);
                }
            });
            let mut zn = Compress::new(Compression::new(level), true);
            let nuovo = ns_per_op(|| {
                for a in [&mz, &int] {
                    zn = Compress::new(Compression::new(level), true);
                    deflate(&mut zn, black_box(a), &mut block, &mut outside);
                }
            });
            let mut compressed = 0;
            for a in [&mz, &int] {
                z.reset();
                deflate(&mut z, a, &mut block, &mut outside);
                compressed += outside.len();
            }
            rows.push(json!({
                "caso": name, "livello": level, "byte_array": total, "rapporto": round_to(compressed as f64 / total as f64),
                "con_reset_us_per_coppia": round_to(with_reset / 1e3), "con_reset_mb_s": round_to(total as f64 / with_reset * 1e3),
                "nuovo_ogni_array_us_per_coppia": round_to(nuovo / 1e3),
            }));
        }
    }
    let mut z = Compress::new(Compression::new(1), true);
    let reset_ns = ns_per_op(|| z.reset());
    json!({"motore": engine, "prove": rows, "reset_livello1_ns": round_to(reset_ns)})
}

fn impronte() -> Value {
    let data: Vec<u8> = (0..64u32 << 20)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect();
    let mb = data.len() as f64 / 1e6;
    let t = Instant::now();
    black_box(Sha1::digest(black_box(&data)));
    let sha1 = mb / t.elapsed().as_secs_f64();
    let t = Instant::now();
    black_box(Sha256::digest(black_box(&data)));
    let sha256 = mb / t.elapsed().as_secs_f64();
    #[cfg(target_arch = "x86_64")]
    let sha_ni = std::arch::is_x86_feature_detected!("sha");
    #[cfg(not(target_arch = "x86_64"))]
    let sha_ni = false;
    json!({"sha1_mb_s": round_to(sha1), "sha256_mb_s": round_to(sha256), "cpu_sha_ni": sha_ni})
}

fn memory() -> Value {
    let ns_malloc = ns_per_op(|| {
        black_box(Vec::<u8>::with_capacity(black_box(4096)));
    });
    let mut v = vec![0u8; 400 << 10];
    let ns_set = ns_per_op(|| {
        v.fill(black_box(0));
        black_box(&v);
    });
    json!({"malloc_free_ns": round_to(ns_malloc), "azzeramento_400kb_ns": round_to(ns_set), "azzeramento_gb_s": round_to(v.len() as f64 / ns_set)})
}

/// All the micro-benchmarks.
pub fn tutti() -> Value {
    json!({
        "decodifica_ft": decode_ft(), "conversione_mz": mz_conversion(), "xml_spettro": xml(), "base64": base64(),
        "compressione": compression(), "impronte": impronte(), "memoria": memory(),
    })
}
