//! The phases of a conversion measured one at a time (open, header, decoding, `extra_scansione`, XML, base64, zlib, SHA-1, writing), and the whole
//! conversion at 1…N threads and zlib levels 0/1/6. The phases come from one instrumented single-thread loop whose clock runs around each call;
//! each variant of the loop leaves one stage out or turns one on, so that the difference is the stage.
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use mzlab_core::error::Result;
use mzlab_core::lettori::{self, FileSet, Lettore, Limiti, Membro};
use mzlab_core::lettori_mzml::{Conversione, Picchi};
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni};

use crate::conta::Alloc;
use crate::denso_lettore::Denso;
use crate::misura::{arrotonda, mediana, riassunto, ripeti};

pub enum Origine {
    Raw(PathBuf),
    Denso(PathBuf),
}

/// What is converted: a real `.raw` or the dense synthetic case, and whether its mzML carries the profile points.
pub struct Sorgente {
    pub etichetta: String,
    pub origine: Origine,
    pub byte: u64,
    pub profilo: bool,
}

impl Sorgente {
    pub fn apri(&self) -> Box<dyn Lettore> {
        match &self.origine {
            Origine::Denso(p) => Box::new(Denso::apri(p).expect("dense case")),
            Origine::Raw(p) => {
                let nome = p
                    .file_name()
                    .map_or(String::new(), |s| s.to_string_lossy().into_owned());
                let fs = FileSet {
                    membri: vec![Membro {
                        nome,
                        dim: self.byte,
                        dati: Box::new(File::open(p).expect("open input")),
                        sha1: None,
                    }],
                };
                lettori::apri(fs, &Limiti::desktop()).expect("the reader opens the file")
            }
        }
    }

    pub fn picchi(&self) -> Picchi {
        if self.profilo {
            Picchi::Profilo
        } else {
            Picchi::File
        }
    }
}

/// Counts the bytes written and, when asked, hashes them (the determinism check: the same file at any number of threads).
pub struct Sink {
    pub n: u64,
    sha: Option<Sha256>,
}

impl Sink {
    pub fn nuovo(con_sha: bool) -> Self {
        Sink {
            n: 0,
            sha: con_sha.then(Sha256::new),
        }
    }
    pub fn sha256(self) -> Option<String> {
        self.sha
            .map(|s| s.finalize().iter().map(|b| format!("{b:02x}")).collect())
    }
}

impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.n += b.len() as u64;
        if let Some(s) = &mut self.sha {
            s.update(b);
        }
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn apertura(src: &Sorgente, giri: usize) -> Value {
    let (g, l) = ripeti(giri, || src.apri());
    let mut l = l;
    let n = l.indice(0).map_or(0, |v| v.len());
    json!({"fase": "apertura", "scansioni": n, "misura": riassunto(&g)})
}

/// The header call (`testata_mzml`: SHA-1 of the whole input and the method text), on a fresh reader each run.
pub fn testata(src: &Sorgente, giri: usize) -> (Value, Header) {
    let mut h = None;
    let mut resto = Vec::new();
    for _ in 0..giri.max(1) {
        let mut l = src.apri();
        let (g, r) = ripeti(1, || l.testata_mzml(0));
        resto.push(g.into_iter().next().expect("one run"));
        h = r.ok().flatten();
    }
    let h = h.unwrap_or_else(|| Header {
        run: src.etichetta.clone(),
        software: ("mzLab".into(), "bench".into()),
        ..Header::default()
    });
    (json!({"fase": "testata", "misura": riassunto(&resto)}), h)
}

/// One variant of the instrumented loop.
#[derive(Clone, Copy)]
pub struct Variante {
    pub nome: &'static str,
    pub livello: u32,
    /// The arrays are written empty: the XML of the spectrum alone.
    pub vuoto: bool,
    pub indicizzato: bool,
    pub su_file: bool,
}

#[derive(Default, Clone)]
struct Parti {
    dec: f64,
    extra: f64,
    scrittura: f64,
    totale: f64,
    scansioni: usize,
    byte: u64,
    a_dec: Alloc,
    a_extra: Alloc,
    a_scr: Alloc,
}

fn ciclo(
    l: &mut dyn Lettore,
    h: &Header,
    profilo: bool,
    v: Variante,
    tmp: &std::path::Path,
) -> Result<Parti> {
    let n = l.indice(0)?.len();
    let opz = Opzioni {
        livello: v.livello,
        numpress: false,
        thread: 1,
        dad: false,
    };
    let uscita: Box<dyn Write> = if v.su_file {
        Box::new(BufWriter::with_capacity(
            1 << 20,
            File::create(tmp).map_err(|_| mzlab_core::Error::new("err.file.mzmlWrite"))?,
        ))
    } else {
        Box::new(Sink::nuovo(false))
    };
    let mut w = if v.indicizzato {
        MzmlWriter::indicizzato(uscita, h, opz)?
    } else {
        MzmlWriter::con_opzioni(uscita, h, opz)?
    };
    let mut p = Parti {
        scansioni: n,
        ..Parti::default()
    };
    let t_tot = Instant::now();
    w.open_spectra(n)?;
    for k in 0..n {
        let (a0, t0) = (Alloc::ora(), Instant::now());
        let mut s = if profilo {
            l.scansione_profilo(0, k)?
                .ok_or_else(|| mzlab_core::Error::new("err.conv.noProfile"))?
        } else {
            l.scansione(0, k)?
        };
        let (a1, t1) = (Alloc::ora(), Instant::now());
        let extra = l.extra_scansione(0, k, &s);
        let (a2, t2) = (Alloc::ora(), Instant::now());
        if v.vuoto {
            s.mz.clear();
            s.intensity.clear();
        }
        w.spectrum_with(&s, &extra)?;
        let (a3, t3) = (Alloc::ora(), Instant::now());
        p.dec += (t1 - t0).as_secs_f64();
        p.extra += (t2 - t1).as_secs_f64();
        p.scrittura += (t3 - t2).as_secs_f64();
        p.a_dec += a1.da(a0);
        p.a_extra += a2.da(a1);
        p.a_scr += a3.da(a2);
    }
    w.close_spectra()?;
    let mut out = w.finish()?;
    out.flush()
        .map_err(|_| mzlab_core::Error::new("err.file.mzmlWrite"))?;
    drop(out);
    p.totale = t_tot.elapsed().as_secs_f64();
    p.byte = if v.su_file {
        std::fs::metadata(tmp).map_or(0, |m| m.len())
    } else {
        0
    };
    Ok(p)
}

/// The byte count of a variant written to a sink (the file variant knows it from the file).
fn byte_di(l: &mut dyn Lettore, h: &Header, profilo: bool, v: Variante) -> u64 {
    let mut v = v;
    v.su_file = false;
    let n = l.indice(0).map_or(0, |x| x.len());
    let opz = Opzioni {
        livello: v.livello,
        numpress: false,
        thread: 1,
        dad: false,
    };
    let mut w = MzmlWriter::con_opzioni(Sink::nuovo(false), h, opz).expect("writer");
    w.open_spectra(n).expect("open");
    for k in 0..n {
        let mut s = if profilo {
            l.scansione_profilo(0, k).ok().flatten().expect("profile")
        } else {
            l.scansione(0, k).expect("scan")
        };
        let extra = l.extra_scansione(0, k, &s);
        if v.vuoto {
            s.mz.clear();
            s.intensity.clear();
        }
        w.spectrum_with(&s, &extra).expect("spectrum");
    }
    w.close_spectra().expect("close");
    w.finish().expect("finish").n
}

fn per_scan(a: &Alloc, n: usize) -> Value {
    json!({"chiamate": arrotonda(a.chiamate as f64 / n.max(1) as f64), "libere": arrotonda(a.libere as f64 / n.max(1) as f64), "byte": arrotonda(a.byte as f64 / n.max(1) as f64)})
}

/// The phases of the single-thread loop in the variants that isolate each stage; the stage costs are in `stadi`.
pub fn fasi(src: &Sorgente, giri: usize, h: &Header, tmp: &std::path::Path) -> Value {
    let v = |nome, livello, vuoto, indicizzato, su_file| Variante {
        nome,
        livello,
        vuoto,
        indicizzato,
        su_file,
    };
    let varianti = [
        v("xml-solo", 0, true, false, false),
        v("livello0", 0, false, false, false),
        v("livello1", 1, false, false, false),
        v("livello6", 6, false, false, false),
        v("livello1-indicizzato", 1, false, true, false),
        v("livello1-su-file", 1, false, false, true),
        v("livello1-indicizzato-su-file", 1, false, true, true),
    ];
    let mut righe = Vec::new();
    let mut l = src.apri();
    let mut mediane: Vec<Mediana> = Vec::new();
    for var in varianti {
        let (riga, m) = misura_variante(src, &mut *l, giri, h, tmp, var);
        righe.push(riga);
        mediane.push(m);
    }
    let _ = std::fs::remove_file(tmp);
    // the cost of each stage: a variant minus the one that lacks it
    let get = |nome: &str| {
        mediane
            .iter()
            .find(|m| m.0 == nome)
            .map_or(f64::NAN, |m| m.3)
    };
    let (xml, l0, l1, l6, idx) = (
        get("xml-solo"),
        get("livello0"),
        get("livello1"),
        get("livello6"),
        get("livello1-indicizzato"),
    );
    let dec = mediane
        .iter()
        .find(|m| m.0 == "livello1")
        .map_or(f64::NAN, |m| m.1);
    let extra = mediane
        .iter()
        .find(|m| m.0 == "livello1")
        .map_or(f64::NAN, |m| m.2);
    let file = get("livello1-su-file") - l1;
    json!({
        "varianti": righe,
        "stadi_s": {
            "decodifica": arrotonda(dec),
            "extra_scansione": arrotonda(extra),
            "xml_con_array_vuoti": arrotonda(xml),
            "array_in_base64_senza_zlib": arrotonda(l0 - xml),
            "zlib_livello1_oltre_livello0": arrotonda(l1 - l0),
            "zlib_livello6_oltre_livello0": arrotonda(l6 - l0),
            "sha1_uscita_e_offset": arrotonda(idx - l1),
            "scrittura_su_file_oltre_sink": arrotonda(file),
        },
    })
}

/// The whole conversion through `Conversione::scrivi` at each number of threads and zlib level, into a sink: time, CPU, memory, allocations and the
/// SHA-256 of the output (the same at every thread count: F9).
pub fn scala(
    src: &Sorgente,
    giri: usize,
    h_nome: &str,
    thread: &[usize],
    livelli: &[u32],
    indicizzato: bool,
    con_impronta: bool,
) -> Value {
    let mut prove = Vec::new();
    for &lv in livelli {
        for &t in thread {
            let mut l = src.apri();
            let opz = Opzioni {
                livello: lv,
                numpress: false,
                thread: t,
                dad: false,
            };
            let conv = Conversione {
                peaks: src.picchi(),
                opz,
                steps: indicizzato.then_some(&[][..]),
            };
            let mut byte = 0;
            // the timed runs do not hash: SHA-256 of the output is serial and would hide the scaling
            let (g, _) = ripeti(giri, || {
                let s = conv
                    .scrivi(&mut *l, 0, Sink::nuovo(false), &mut |_, _| {})
                    .expect("conversion");
                byte = s.n;
            });
            // one more run, outside the clock, for the determinism fingerprint
            let impronta = con_impronta.then(|| {
                let s = conv
                    .scrivi(&mut *l, 0, Sink::nuovo(true), &mut |_, _| {})
                    .expect("conversion");
                s.sha256()
            });
            prove.push(json!({
                "thread": t, "livello": lv, "indicizzato": indicizzato, "peaks": format!("{:?}", src.picchi()),
                "byte_uscita": byte, "sha256_uscita": impronta.flatten(), "misura": riassunto(&g),
            }));
        }
    }
    json!({"nome": h_nome, "prove": prove})
}

/// Medians of one variant: name, decoding, extra, writing, total, scans.
type Mediana = (&'static str, f64, f64, f64, f64, usize);

/// One variant of the loop measured `giri` times: its JSON row and its medians.
fn misura_variante(
    src: &Sorgente,
    l: &mut dyn Lettore,
    giri: usize,
    h: &Header,
    tmp: &std::path::Path,
    var: Variante,
) -> (Value, Mediana) {
    let mut parti: Vec<Parti> = Vec::new();
    let (g, _) = ripeti(giri, || {
        let p = ciclo(&mut *l, h, src.profilo, var, tmp).expect("loop");
        parti.push(p);
    });
    let med = |f: fn(&Parti) -> f64| mediana(&parti.iter().map(f).collect::<Vec<_>>());
    let (dec, extra, scr, tot) = (
        med(|p| p.dec),
        med(|p| p.extra),
        med(|p| p.scrittura),
        med(|p| p.totale),
    );
    let p0 = parti[0].clone();
    let byte = if var.su_file {
        p0.byte
    } else {
        byte_di(&mut *l, h, src.profilo, var)
    };
    let riga = json!({
        "variante": var.nome, "livello": var.livello, "indicizzato": var.indicizzato, "su_file": var.su_file, "xml_solo": var.vuoto,
        "decodifica_s": arrotonda(dec), "extra_s": arrotonda(extra), "scrittura_s": arrotonda(scr), "totale_s": arrotonda(tot),
        "byte_uscita": byte, "scansioni": p0.scansioni,
        "allocazioni_per_scansione": {"decodifica": per_scan(&p0.a_dec, p0.scansioni), "extra": per_scan(&p0.a_extra, p0.scansioni), "scrittura": per_scan(&p0.a_scr, p0.scansioni)},
        "misura": riassunto(&g),
    });
    (riga, (var.nome, dec, extra, scr, tot, p0.scansioni))
}
