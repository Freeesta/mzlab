//! The inputs of the read estimate (`mzlab_core::mzml_estimate`) and the native peak memory of `spectra.ms2` on synthetic mzML shapes, for the fit of its coefficients:
//!
//!     cargo run --release -p mzlab-feat-wasm --example stima_lettura -- --write DIR     (the shapes of the sweep, one .mzML each)
//!     cargo run --release -p mzlab-feat-wasm --example stima_lettura -- FILE.mzML...
//!     cargo run --release -p mzlab-feat-wasm --example stima_lettura -- --fixture OUT.mzML   (the file of `tests/dati/stima_lettura.json`)
//!
//! One line per file: spectra, points, 64-bit points, file MB, the estimate (MB) and the growth of the resident memory of `mzml::read` and of `ms2_text` (MB, the file included).

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzml_estimate::{bytes, inputs, mgf_bytes};
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options};
use mzlab_feat::flow_ms2::{ms2_text, Ms2Params};

#[path = "../tests/lettura_comune/mod.rs"]
mod comune;

fn kb(field: &str) -> f64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find(|l| l.starts_with(field))
        .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        .unwrap_or(0.0)
}

/// `n` scans of `k` points, every other one an MS2 with a precursor; `wide` writes 64-bit intensities, `np` the numpress codecs.
fn shape(n: usize, k: usize, wide: bool, np: bool) -> Vec<u8> {
    let o = Options {
        numpress: np,
        ..Options::default()
    };
    let mut w = MzmlWriter::with_options(Vec::new(), &Header::default(), o).unwrap();
    w.open_spectra(n).unwrap();
    for sc in 0..n {
        let it: Vec<f32> = (0..k)
            .map(|i| 100.0 + ((sc * 31 + i * 17) % 977) as f32)
            .collect();
        let ms2 = sc % 2 == 1;
        w.spectrum(&Spectrum {
            index: sc,
            id: format!("scan={}", sc + 1),
            level: if ms2 { 2 } else { 1 },
            rt: sc as f64 * 0.005,
            polarity: 1,
            precursor: ms2.then_some(300.0 + (sc % 500) as f64),
            mz: (0..k)
                .map(|i| 100.0 + i as f64 * 900.0 / k as f64 + (sc % 7) as f64 * 1e-4)
                .collect(),
            wide: wide.then(|| it.iter().map(|&v| f64::from(v) + 0.125).collect()),
            intensity: it,
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--fixture") {
        std::fs::write(&args[1], comune::mzml(&comune::spec())).expect("write");
        return;
    }
    if args.first().map(String::as_str) == Some("--write") {
        let dir = std::path::Path::new(&args[1]);
        for (name, n, k, wide, np) in [
            ("dense", 2000, 5000, false, false),
            ("wide", 2000, 5000, true, false),
            ("small", 40000, 200, false, false),
            ("profile", 300, 40000, false, false),
        ] {
            std::fs::write(dir.join(format!("{name}.mzML")), shape(n, k, wide, np)).expect("write");
        }
        return;
    }
    let p = Ms2Params::from_params(&[]);
    for path in args {
        let _ = std::fs::write("/proc/self/clear_refs", "5");
        let base = kb("VmRSS:");
        let b = std::fs::read(&path).expect("read");
        let x = inputs(&b);
        let est = (bytes(&x, b.len()) + mgf_bytes(&x)) as f64 / 1048576.0;
        drop(mzlab_core::mzml::read(&b[..]).expect("read"));
        let read = kb("VmHWM:");
        let _ = std::fs::write("/proc/self/clear_refs", "5");
        let out = ms2_text(&p, &[("f.mzML", &b)], None).expect("ms2");
        let peak = kb("VmHWM:");
        println!(
            "{path}\tspectra {}\tpoints {}\twide {}\tfile {:.1}\test {est:.1}\tread {:.1}\tms2 {:.1}\tout {}",
            x.spectra,
            x.points,
            x.wide,
            b.len() as f64 / 1048576.0,
            (read - base) / 1024.0,
            (peak - base) / 1024.0,
            out.len()
        );
    }
}
