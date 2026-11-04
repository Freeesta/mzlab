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
use mzlab_core::lettori_mzml::Conversione;
use mzlab_core::mzmlwrite::Opzioni;

use fasi::{Origine, Sorgente};
use misura::arrotonda;

#[global_allocator]
static A: conta::Conta = conta::Conta;

#[derive(Default)]
struct Args {
    file: Option<PathBuf>,
    denso: Option<PathBuf>,
    gb: f64,
    forma: String,
    cancella: bool,
    etichetta: String,
    giri: usize,
    solo: Vec<String>,
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
            "--forma" => a.forma = v(&mut it, &x),
            "--cancella" => a.cancella = true,
            "--etichetta" => a.etichetta = v(&mut it, &x),
            "--giri" => a.giri = v(&mut it, &x).parse().expect("--giri"),
            "--solo" => a.solo = v(&mut it, &x).split(',').map(str::to_string).collect(),
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
            "--senza-conta" => conta::imposta(false),
            "--tmp" => a.tmp = v(&mut it, &x).into(),
            _ => panic!("unknown option {x}"),
        }
    }
    a
}

fn macchina() -> Value {
    let cpu = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let modello = cpu
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
        "cpu": modello, "core": std::thread::available_parallelism().map_or(1, |n| n.get()), "ram_mb": ram_mb.round(),
        "sistema": std::env::consts::OS, "architettura": std::env::consts::ARCH,
        "backend_zlib": if cfg!(feature = "banco-zlib-rs") { "zlib-rs" } else { "miniz_oxide" },
    })
}

fn thread_elenco(a: &Args) -> Vec<usize> {
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
fn sorgente(a: &Args) -> Option<(Sorgente, Value)> {
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
        let info = json!({"tipo": "raw", "byte": byte, "sha256": sha, "sha256_s": arrotonda(t.elapsed().as_secs_f64())});
        return Some((
            Sorgente {
                etichetta,
                origine: Origine::Raw(p.clone()),
                byte,
                profilo: a.profilo.unwrap_or(false),
            },
            info,
        ));
    }
    let dir = a.denso.as_ref()?;
    std::fs::create_dir_all(dir).expect("folder");
    let mut b = Vec::new();
    let forma = |n| match a.forma.as_str() {
        "lab-a" => denso::Parametri::forma_lab_a(n),
        "lab-b" => denso::Parametri::forma_lab_b(n),
        _ => denso::Parametri::denso(n),
    };
    denso::pacchetto(&mut b, &forma(1), 0, None);
    let scansioni = ((a.gb * 1e9) / b.len() as f64).ceil() as usize;
    let p = forma(scansioni);
    let path = dir.join(format!(
        "denso_{}_{}_{:x}.bin",
        p.scansioni, p.picchi, p.seme
    ));
    let info = if path.exists() {
        json!({"tipo": "denso-sintetico", "generato_ora": false})
    } else {
        let t = Instant::now();
        let (_, byte, sha) =
            denso::genera(&path, &p, a.gb <= 3.0).expect("generate the dense case");
        json!({"tipo": "denso-sintetico", "generato_ora": true, "genera_s": arrotonda(t.elapsed().as_secs_f64()), "byte": byte, "sha256": sha})
    };
    let byte = std::fs::metadata(&path).expect("dense file").len();
    let mut info = info;
    info["byte"] = json!(byte);
    info["scansioni"] = json!(p.scansioni);
    info["punti_profilo_per_scansione"] = json!(if p.con_profilo { p.picchi * 10 } else { 0 });
    info["picchi_per_scansione"] = json!(p.picchi);
    let etichetta = if a.etichetta.is_empty() {
        format!(
            "denso-{}-{:.2}gb",
            if a.forma.is_empty() { "std" } else { &a.forma },
            byte as f64 / 1e9
        )
    } else {
        a.etichetta.clone()
    };
    Some((
        Sorgente {
            etichetta,
            origine: Origine::Denso(path),
            byte,
            profilo: a.profilo.unwrap_or(p.con_profilo),
        },
        info,
    ))
}

struct Scarta;
impl WriteAt for Scarta {
    fn write_at(&mut self, _: u64, _: &[u8]) -> mzlab_core::Result<()> {
        Ok(())
    }
}

/// SHA-256 of a file in pieces of 1 MiB, as `mzlab-cli` does for the cache (`flow_hash::file_sha256`).
fn file_sha256(p: &Path) -> String {
    use std::io::Read;
    let mut f = std::fs::File::open(p).expect("open");
    let mut h = Hashing::new(Scarta);
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
fn cli(src: &Sorgente, a: &Args, intestazione_s: f64) -> Value {
    let Origine::Raw(p) = &src.origine else {
        return json!({"saltato": "only for a real file"});
    };
    let sha_in = time_it(a.giri, || file_sha256(p));
    let out = a
        .tmp
        .join(format!("raw_mzml_banco_{}.mzML", std::process::id()));
    let (scrittura, _) = misura::ripeti(a.giri, || {
        let mut l = src.apri();
        let f = std::fs::File::create(&out).expect("create");
        let sink = std::io::BufWriter::new(f);
        let conv = Conversione {
            peaks: src.picchi(),
            opz: Opzioni::default(),
            steps: Some(&[]),
        };
        let mut w = conv
            .scrivi(&mut *l, 0, sink, &mut |_, _| {})
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
    let sc = misura::mediana(&scrittura.iter().map(|g| g.s).collect::<Vec<_>>());
    let somma = sha_in + sc + 2.0 * sha_out + place;
    json!({
        "sha256_ingresso_s": arrotonda(sha_in), "apertura_e_scrittura_indicizzata_s": arrotonda(sc), "sha256_uscita_s": arrotonda(sha_out),
        "hard_link_o_copia_s": arrotonda(place), "copia_dei_byte_s": arrotonda(copia), "apertura_sola_s": arrotonda(apri),
        "testata_sha1_ingresso_e_metodo_s": arrotonda(intestazione_s), "byte_uscita": byte_out,
        "somma_ipotesi_s": arrotonda(somma),
        "ipotesi": "SHA-256 of the input (fill_inputs) + open and indexed write (SHA-1 of the input and of the output inside) + SHA-256 of the output in keep() + link + SHA-256 of the output in item()",
        "scritture_giri_s": scrittura.iter().map(|g| arrotonda(g.s)).collect::<Vec<_>>(),
    })
}

fn time_it<R>(giri: usize, f: impl FnMut() -> R) -> f64 {
    let (g, _) = misura::ripeti(giri, f);
    misura::mediana(&g.iter().map(|x| x.s).collect::<Vec<_>>())
}

/// `T_min = max(B_in / BW_read, B_out / BW_write, W_cpu / N, W_serial)` of `RISPOSTE.md` §1.3, from the stages measured alone.
fn tmin(fasi: &Value, testata_s: f64, byte_in: u64, a: &Args, micro: Option<&Value>) -> Value {
    let st = &fasi["stadi_s"];
    let g = |k: &str| st[k].as_f64().unwrap_or(f64::NAN);
    let l1 = fasi["varianti"]
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
    let lettura = a.bw.0.map(|b| byte_in as f64 / 1e6 / b);
    let scrittura = a.bw.1.map(|b| byte_out / 1e6 / b);
    let t = [lettura, scrittura, Some(w_cpu / n), Some(seriale)]
        .into_iter()
        .flatten()
        .fold(0.0, f64::max);
    json!({
        "t_min_s": arrotonda(t), "formula": "max(B_in/BW_lettura, B_out/BW_scrittura, W_cpu/N_core, W_seriale)",
        "B_in": byte_in, "B_out": byte_out, "N_core": n, "W_cpu_s": arrotonda(w_cpu), "W_cpu_su_N_s": arrotonda(w_cpu / n),
        "W_seriale_s": arrotonda(seriale), "sha1_uscita_s": sha1_out.map(arrotonda), "testata_s": arrotonda(testata_s),
        "lettura_s": lettura.map(arrotonda), "scrittura_s": scrittura.map(arrotonda),
        "bw_lettura_mb_s": a.bw.0, "bw_scrittura_mb_s": a.bw.1,
    })
}

/// The sections that need a source (open, header, phases, scaling, CLI) added to the document.
fn sezioni_sorgente(
    doc: &mut Map<String, Value>,
    a: &Args,
    src: &Sorgente,
    info: Value,
    voglio: &dyn Fn(&str) -> bool,
    thread: &[usize],
    micro_json: Option<&Value>,
) {
    doc.insert("ingresso".into(), json!({"etichetta": src.etichetta, "peaks": format!("{:?}", src.picchi()), "origine": info}));
    let mut prima = Map::new();
    prima.insert("apertura".into(), fasi::apertura(src, a.giri));
    let (th, h) = fasi::testata(src, a.giri);
    let testata_s = th["misura"]["mediana_s"].as_f64().unwrap_or(f64::NAN);
    prima.insert("testata".into(), th);
    if voglio("fasi") {
        let tmp = a
            .tmp
            .join(format!("raw_mzml_banco_{}.fasi.mzML", std::process::id()));
        let f = fasi::fasi(src, a.giri, &h, &tmp);
        prima.insert("tmin".into(), tmin(&f, testata_s, src.byte, a, micro_json));
        prima.insert("fasi".into(), f);
    }
    if voglio("scala") {
        let mut s = vec![fasi::scala(
            src,
            a.giri,
            "scrivi_con",
            thread,
            &[0, 1, 6],
            false,
            a.impronte,
        )];
        s.push(fasi::scala(
            src,
            a.giri,
            "indicizzato",
            &[1, *thread.last().unwrap_or(&1)],
            &[1],
            true,
            a.impronte,
        ));
        prima.insert("scala".into(), json!(s));
    }
    if voglio("cli") {
        prima.insert("cli".into(), cli(src, a, testata_s));
    }
    for (k, v) in prima {
        doc.insert(k, v);
    }
}

fn main() {
    let a = args();
    let voglio = |s: &str| {
        a.solo.is_empty() && !["micro", "d8"].contains(&s)
            || a.solo.iter().any(|x| x == s || x == "tutto")
    };
    let thread = thread_elenco(&a);
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
        json!(conta::azzera_rss_picco()),
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
            d8::tutti(
                &thread
                    .iter()
                    .copied()
                    .filter(|t| *t <= 8)
                    .collect::<Vec<_>>(),
            ),
        );
    }
    let mut denso_path = None;
    if let Some((src, info)) = sorgente(&a) {
        if let Origine::Denso(p) = &src.origine {
            denso_path = Some(p.clone());
        }
        sezioni_sorgente(
            &mut doc,
            &a,
            &src,
            info,
            &voglio,
            &thread,
            micro_json.as_ref(),
        );
    }
    if a.cancella
        && let Some(p) = denso_path
    {
        let _ = std::fs::remove_file(p);
    }
    let testo = serde_json::to_string_pretty(&Value::Object(doc)).expect("json");
    match &a.json {
        Some(p) => std::fs::write(p, testo).expect("write the JSON"),
        None => println!("{testo}"),
    }
}
