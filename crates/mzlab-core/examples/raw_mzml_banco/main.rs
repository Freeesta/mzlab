//! Bench of the Thermo `.raw` → mzML conversion: phases, scaling over threads, micro-benchmarks, why threads slow it down (D8), what `mzlab-cli convert`
//! adds to the writer (D11) and the floor `T_min` of the roofline. It changes no byte of the output. One JSON object on stdout (or `--json FILE`).
//!
//! `cargo run --profile cli -p mzlab-core --example raw_mzml_banco -- --file X.raw --etichetta LAB-B --giri 5`
//! `... -- --denso DIR --gb 2` (the dense synthetic case, generated in DIR when absent), `... -- --solo micro,d8` (machine only).
//! Launched and collected by `tools/bench/raw_mzml.py`; numbers and method in `docs/agenti/raw-mzml.md`.
mod conta;
mod d8;
mod denso;
mod denso_lettore;
#[cfg(test)]
mod denso_test;
mod fasi;
mod micro;
mod misura;

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{json, Map, Value};

use mzlab_core::budget::{Hashing, WriteAt};
use mzlab_core::lettori_mzml::Conversion;
use mzlab_core::mzmlwrite::Options;

use fasi::{Origin, Source};
use misura::round_to;

#[global_allocator]
static A: conta::Tally = conta::Tally;

#[derive(Default)]
struct Args {
    file: Option<PathBuf>,
    denso: Option<PathBuf>,
    gb: f64,
    shape: String,
    delete: bool,
    etichetta: String,
    giri: usize,
    only: Vec<String>,
    thread: Vec<usize>,
    profilo: Option<bool>,
    bw: (Option<f64>, Option<f64>),
    meta: Vec<(String, String)>,
    json: Option<PathBuf>,
    impronte: bool,
    tmp: PathBuf,
}

fn args() -> Args {
    let mut a = Args {
        gb: 2.0,
        giri: 3,
        impronte: true,
        tmp: std::env::temp_dir(),
        ..Args::default()
    };
    let mut it = std::env::args().skip(1);
    let v = |it: &mut dyn Iterator<Item = String>, n: &str| {
        it.next().unwrap_or_else(|| panic!("{n} needs a value"))
    };
    while let Some(x) = it.next() {
        match x.as_str() {
            "--file" => a.file = Some(v(&mut it, &x).into()),
            "--denso" => a.denso = Some(v(&mut it, &x).into()),
            "--gb" => a.gb = v(&mut it, &x).parse().expect("--gb"),
            "--forma" => a.shape = v(&mut it, &x),
            "--cancella" => a.delete = true,
            "--etichetta" => a.etichetta = v(&mut it, &x),
            "--giri" => a.giri = v(&mut it, &x).parse().expect("--giri"),
            "--solo" => a.only = v(&mut it, &x).split(',').map(str::to_string).collect(),
            "--thread" => {
                a.thread = v(&mut it, &x)
                    .split(',')
                    .map(|s| s.parse().expect("--thread"))
                    .collect()
            }
            "--peaks" => a.profilo = Some(v(&mut it, &x) == "profile"),
            "--bw-lettura" => a.bw.0 = v(&mut it, &x).parse().ok(),
            "--bw-scrittura" => a.bw.1 = v(&mut it, &x).parse().ok(),
            "--meta" => {
                let kv = v(&mut it, &x);
                let (k, w) = kv.split_once('=').expect("--meta KEY=VALUE");
                a.meta.push((k.into(), w.into()));
            }
            "--json" => a.json = Some(v(&mut it, &x).into()),
            "--senza-impronte" => a.impronte = false,
            "--senza-conta" => conta::set_to(false),
            "--tmp" => a.tmp = v(&mut it, &x).into(),
            _ => panic!("unknown option {x}"),
        }
    }
    a
}

fn macchina() -> Value {
    let cpu = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let model = cpu
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map_or("", |x| x.1.trim())
        .to_string();
    let ram = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let ram_mb = ram
        .lines()
        .find(|l| l.starts_with("MemTotal"))
        .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        .map_or(0.0, |k| k / 1024.0);
    json!({
        "cpu": model, "core": std::thread::available_parallelism().map_or(1, |n| n.get()), "ram_mb": ram_mb.round(),
        "sistema": std::env::consts::OS, "architettura": std::env::consts::ARCH,
        "backend_zlib": if cfg!(feature = "banco-zlib-rs") { "zlib-rs" } else { "miniz_oxide" },
    })
}

fn thread_list(a: &Args) -> Vec<usize> {
    if !a.thread.is_empty() {
        return a.thread.clone();
    }
    let n = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut v = vec![1, 2, 4, n];
    mzlab_core::sortc::slice_ord(&mut v);
    v.dedup();
    v
}

/// The source of the run: a real file, or the dense case generated in its folder when it is not there yet.
fn source(a: &Args) -> Option<(Source, Value)> {
    if let Some(p) = &a.file {
        let byte = std::fs::metadata(p).expect("input").len();
        let etichetta = if a.etichetta.is_empty() {
            p.file_name()
                .map_or(String::new(), |s| s.to_string_lossy().into_owned())
        } else {
            a.etichetta.clone()
        };
        let t = Instant::now();
        let sha = file_sha256(p);
        let info = json!({"tipo": "raw", "byte": byte, "sha256": sha, "sha256_s": round_to(t.elapsed().as_secs_f64())});
        return Some((
            Source {
                label: etichetta,
                origin: Origin::Raw(p.clone()),
                byte,
                profilo: a.profilo.unwrap_or(false),
            },
            info,
        ));
    }
    let dir = a.denso.as_ref()?;
    std::fs::create_dir_all(dir).expect("folder");
    let mut b = Vec::new();
    let shape = |n| match a.shape.as_str() {
        "lab-a" => denso::Parameters::lab_shape_a(n),
        "lab-b" => denso::Parameters::lab_shape_b(n),
        _ => denso::Parameters::denso(n),
    };
    denso::packet(&mut b, &shape(1), 0, None);
    let scansioni = ((a.gb * 1e9) / b.len() as f64).ceil() as usize;
    let p = shape(scansioni);
    let path = dir.join(format!(
        "denso_{}_{}_{:x}.bin",
        p.scansioni, p.peaks, p.seme
    ));
    let info = if path.exists() {
        json!({"tipo": "denso-sintetico", "generato_ora": false})
    } else {
        let t = Instant::now();
        let (_, byte, sha) =
            denso::generate(&path, &p, a.gb <= 3.0).expect("generate the dense case");
        json!({"tipo": "denso-sintetico", "generato_ora": true, "genera_s": round_to(t.elapsed().as_secs_f64()), "byte": byte, "sha256": sha})
    };
    let byte = std::fs::metadata(&path).expect("dense file").len();
    let mut info = info;
    info["byte"] = json!(byte);
    info["scansioni"] = json!(p.scansioni);
    info["punti_profilo_per_scansione"] = json!(if p.with_profile { p.peaks * 10 } else { 0 });
    info["picchi_per_scansione"] = json!(p.peaks);
    let etichetta = if a.etichetta.is_empty() {
        format!(
            "denso-{}-{:.2}gb",
            if a.shape.is_empty() { "std" } else { &a.shape },
            byte as f64 / 1e9
        )
    } else {
        a.etichetta.clone()
    };
    Some((
        Source {
            label: etichetta,
            origin: Origin::Denso(path),
            byte,
            profilo: a.profilo.unwrap_or(p.with_profile),
        },
        info,
    ))
}

struct Discard;
impl WriteAt for Discard {
    fn write_at(&mut self, _: u64, _: &[u8]) -> mzlab_core::Result<()> {
        Ok(())
    }
}

/// SHA-256 of a file in pieces of 1 MiB, as `mzlab-cli` does for the cache (`flow_hash::file_sha256`).
fn file_sha256(p: &Path) -> String {
    use std::io::Read;
    let mut f = std::fs::File::open(p).expect("open");
    let mut h = Hashing::new(Discard);
    let mut buf = vec![0u8; 1 << 20];
    let mut at = 0u64;
    loop {
        let n = f.read(&mut buf).expect("read");
        if n == 0 {
            break;
        }
        h.write_at(at, &buf[..n]).expect("hash");
        at += n as u64;
    }
    h.finish().2
}

/// D11: the pieces of `mzlab-cli convert` that are not the writer, each timed alone with the code the CLI calls, to be added up against the whole.
fn cli(src: &Source, a: &Args, intestazione_s: f64) -> Value {
    let Origin::Raw(p) = &src.origin else {
        return json!({"saltato": "only for a real file"});
    };
    let sha_in = time_it(a.giri, || file_sha256(p));
    let out = a
        .tmp
        .join(format!("raw_mzml_banco_{}.mzML", std::process::id()));
    let (writing, _) = misura::repeat(a.giri, || {
        let mut l = src.apri();
        let f = std::fs::File::create(&out).expect("create");
        let sink = std::io::BufWriter::new(f);
        let conv = Conversion {
            peaks: src.picchi(),
            opz: Options::default(),
            steps: Some(&[]),
        };
        let mut w = conv
            .write(&mut *l, 0, sink, &mut |_, _| {})
            .expect("conversion");
        std::io::Write::flush(&mut w).expect("flush");
    });
    let byte_out = std::fs::metadata(&out).map_or(0, |m| m.len());
    let sha_out = time_it(a.giri, || file_sha256(&out));
    let work = a
        .tmp
        .join(format!("raw_mzml_banco_{}.link.mzML", std::process::id()));
    let place = time_it(a.giri, || {
        let _ = std::fs::remove_file(&work);
        std::fs::hard_link(&out, &work)
            .or_else(|_| std::fs::copy(&out, &work).map(|_| ()))
            .expect("place");
    });
    let copia = time_it(a.giri, || {
        let _ = std::fs::remove_file(&work);
        std::fs::copy(&out, &work).expect("copy");
    });
    let apri = time_it(a.giri, || {
        let _ = src.apri();
    });
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&work);
    let sc = misura::median(&writing.iter().map(|g| g.s).collect::<Vec<_>>());
    let sum = sha_in + sc + 2.0 * sha_out + place;
    json!({
        "sha256_ingresso_s": round_to(sha_in), "apertura_e_scrittura_indicizzata_s": round_to(sc), "sha256_uscita_s": round_to(sha_out),
        "hard_link_o_copia_s": round_to(place), "copia_dei_byte_s": round_to(copia), "apertura_sola_s": round_to(apri),
        "testata_sha1_ingresso_e_metodo_s": round_to(intestazione_s), "byte_uscita": byte_out,
        "somma_ipotesi_s": round_to(sum),
        "ipotesi": "SHA-256 of the input (fill_inputs) + open and indexed write (SHA-1 of the input and of the output inside) + SHA-256 of the output in keep() + link + SHA-256 of the output in item()",
        "scritture_giri_s": writing.iter().map(|g| round_to(g.s)).collect::<Vec<_>>(),
    })
}

fn time_it<R>(giri: usize, f: impl FnMut() -> R) -> f64 {
    let (g, _) = misura::repeat(giri, f);
    misura::median(&g.iter().map(|x| x.s).collect::<Vec<_>>())
}

/// `T_min = max(B_in / BW_read, B_out / BW_write, W_cpu / N, W_serial)` of `RISPOSTE.md` §1.3, from the stages measured alone.
fn tmin(phases: &Value, testata_s: f64, byte_in: u64, a: &Args, micro: Option<&Value>) -> Value {
    let st = &phases["stadi_s"];
    let g = |k: &str| st[k].as_f64().unwrap_or(f64::NAN);
    let l1 = phases["varianti"]
        .as_array()
        .and_then(|v| v.iter().find(|x| x["variante"] == "livello1"));
    let byte_out = l1
        .and_then(|x| x["byte_uscita"].as_f64())
        .unwrap_or(f64::NAN);
    let n = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
    // the stages that divide among cores: decoding, trailer, XML, base64 and zlib (level 1)
    let w_cpu = g("decodifica")
        + g("extra_scansione")
        + g("xml_con_array_vuoti")
        + g("array_in_base64_senza_zlib")
        + g("zlib_livello1_oltre_livello0");
    let sha1_out = micro
        .and_then(|m| m["impronte"]["sha1_mb_s"].as_f64())
        .map(|r| byte_out / 1e6 / r);
    // the SHA-1 of the output is one stream; so is the SHA-1 of the input of the header
    let seriale = [sha1_out.unwrap_or(0.0), testata_s]
        .into_iter()
        .fold(0.0, f64::max);
    let reading = a.bw.0.map(|b| byte_in as f64 / 1e6 / b);
    let writing = a.bw.1.map(|b| byte_out / 1e6 / b);
    let t = [reading, writing, Some(w_cpu / n), Some(seriale)]
        .into_iter()
        .flatten()
        .fold(0.0, f64::max);
    json!({
        "t_min_s": round_to(t), "formula": "max(B_in/BW_lettura, B_out/BW_scrittura, W_cpu/N_core, W_seriale)",
        "B_in": byte_in, "B_out": byte_out, "N_core": n, "W_cpu_s": round_to(w_cpu), "W_cpu_su_N_s": round_to(w_cpu / n),
        "W_seriale_s": round_to(seriale), "sha1_uscita_s": sha1_out.map(round_to), "testata_s": round_to(testata_s),
        "lettura_s": reading.map(round_to), "scrittura_s": writing.map(round_to),
        "bw_lettura_mb_s": a.bw.0, "bw_scrittura_mb_s": a.bw.1,
    })
}

/// The sections that need a source (open, header, phases, scaling, CLI) added to the document.
fn source_sections(
    doc: &mut Map<String, Value>,
    a: &Args,
    src: &Source,
    info: Value,
    voglio: &dyn Fn(&str) -> bool,
    thread: &[usize],
    micro_json: Option<&Value>,
) {
    doc.insert(
        "ingresso".into(),
        json!({"etichetta": src.label, "peaks": format!("{:?}", src.picchi()), "origine": info}),
    );
    let mut before = Map::new();
    before.insert("apertura".into(), fasi::opening(src, a.giri));
    let (th, h) = fasi::file_header(src, a.giri);
    let testata_s = th["misura"]["mediana_s"].as_f64().unwrap_or(f64::NAN);
    before.insert("testata".into(), th);
    if voglio("fasi") {
        let tmp = a
            .tmp
            .join(format!("raw_mzml_banco_{}.fasi.mzML", std::process::id()));
        let f = fasi::phases(src, a.giri, &h, &tmp);
        before.insert("tmin".into(), tmin(&f, testata_s, src.byte, a, micro_json));
        before.insert("fasi".into(), f);
    }
    if voglio("scala") {
        let mut s = vec![fasi::scale(
            src,
            a.giri,
            "scrivi_con",
            thread,
            &[0, 1, 6],
            false,
            a.impronte,
        )];
        s.push(fasi::scale(
            src,
            a.giri,
            "indicizzato",
            &[1, *thread.last().unwrap_or(&1)],
            &[1],
            true,
            a.impronte,
        ));
        before.insert("scala".into(), json!(s));
    }
    if voglio("cli") {
        before.insert("cli".into(), cli(src, a, testata_s));
    }
    for (k, v) in before {
        doc.insert(k, v);
    }
}

fn main() {
    let a = args();
    let voglio = |s: &str| {
        a.only.is_empty() && !["micro", "d8"].contains(&s)
            || a.only.iter().any(|x| x == s || x == "tutto")
    };
    let thread = thread_list(&a);
    let mut doc = Map::new();
    doc.insert("banco".into(), json!("raw_mzml_banco/1"));
    doc.insert("macchina".into(), macchina());
    doc.insert(
        "meta".into(),
        json!(a
            .meta
            .iter()
            .cloned()
            .collect::<std::collections::BTreeMap<_, _>>()),
    );
    doc.insert("giri".into(), json!(a.giri));
    doc.insert(
        "rss_picco_azzerabile".into(),
        json!(conta::reset_rss_peak()),
    );
    let mut micro_json = None;
    if voglio("micro") {
        let m = micro::tutti();
        doc.insert("micro".into(), m.clone());
        micro_json = Some(m);
    }
    if voglio("d8") {
        doc.insert(
            "d8".into(),
            d8::all(
                &thread
                    .iter()
                    .copied()
                    .filter(|t| *t <= 8)
                    .collect::<Vec<_>>(),
            ),
        );
    }
    let mut denso_path = None;
    if let Some((src, info)) = source(&a) {
        if let Origin::Denso(p) = &src.origin {
            denso_path = Some(p.clone());
        }
        source_sections(
            &mut doc,
            &a,
            &src,
            info,
            &voglio,
            &thread,
            micro_json.as_ref(),
        );
    }
    if a.delete
        && let Some(p) = denso_path
    {
        let _ = std::fs::remove_file(p);
    }
    let text = serde_json::to_string_pretty(&Value::Object(doc)).expect("json");
    match &a.json {
        Some(p) => std::fs::write(p, text).expect("write the JSON"),
        None => println!("{text}"),
    }
}
