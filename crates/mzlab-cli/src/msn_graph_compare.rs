//! `mzlab-cli msn-graph compare A.json B.json`: the validation of an MSn tree (`docs/agenti/msnlib.md` §4). Two `msngrafo/1` graphs of the same standard (two days, same options)
//! are checked on five criteria: the isolated nodes (same paths of precursors), the arcs with `rel` >= 10 % in both, the formulas proposed, the MS2 entropy per energy and the
//! m/z shift of the matched nodes. The answer is a JSON with the numbers; `convalidata` is true when every criterion holds. Used by `lib.rs` (`msn_graph_cmd`).

use std::path::PathBuf;

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, round, Out, Value};
use mzlab_lib::scores::{entropy_from_pairs, match_peaks};

/// Matching window of nodes, arcs and paths, ppm.
const PPM: f64 = 5.0;
/// Least relative intensity of an arc to count.
const REL_ARCO: f64 = 10.0;
/// Least MS2 entropy similarity of two spectra of the same energy.
const ENTROPIA_MIN: f64 = 0.8;
/// Largest m/z shift of a matched node, ppm.
const SCARTO_MAX: f64 = 3.0;

struct Nodo {
    mz: f64,
    isolato: bool,
    path: Vec<String>,
    formula: Option<String>,
}

struct Arco {
    da: f64,
    a: f64,
    rel: f64,
}

struct Spettro {
    att: String,
    ce: f64,
    path: Vec<String>,
    picchi: Vec<(f64, f64)>,
}

struct Grafo {
    nodi: Vec<Nodo>,
    archi: Vec<Arco>,
    spettri: Vec<Spettro>,
}

fn bad() -> Error {
    Error::new("err.msnlib.bad")
}

/// The m/z labels of the steps of a tree path (`305@cid35>244@cid35` → ["305", "244"]): the labels are rounded, the precise m/z is the one of the node.
fn passi(albero: &str) -> Vec<String> {
    albero
        .split('>')
        .filter_map(|s| Some(s.split('@').next()?.to_string()))
        .collect()
}

fn num(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(|x| x.f64()).unwrap_or(0.0)
}

fn carica(testo: &str) -> Result<Grafo> {
    let v = parse(testo.as_bytes())?;
    if v.get("formato").and_then(|f| f.str()) != Some("msngrafo/1") {
        return Err(bad());
    }
    let lista = |k: &str| v.get(k).map_or(&[][..], |x| x.arr());
    let nodi = lista("nodi")
        .iter()
        .map(|n| Nodo {
            mz: num(n, "mz"),
            isolato: n.get("isolato").and_then(|b| b.bool()).unwrap_or(false),
            path: n
                .get("albero")
                .and_then(|a| a.arr().first())
                .and_then(|p| p.str())
                .map(passi)
                .unwrap_or_default(),
            formula: n
                .get("formula")
                .and_then(|f| f.get("scelta")?.str())
                .map(str::to_string),
        })
        .collect::<Vec<_>>();
    let mz_di = |id: Option<&str>| {
        let k = id?.strip_prefix('n')?.parse::<usize>().ok()?;
        nodi.get(k).map(|n| n.mz)
    };
    let archi = lista("archi")
        .iter()
        .filter_map(|a| {
            let rel = a
                .get("misure")
                .map_or(&[][..], |m| m.arr())
                .iter()
                .map(|m| num(m, "rel"))
                .fold(0.0, f64::max);
            Some(Arco {
                da: mz_di(a.get("da")?.str())?,
                a: mz_di(a.get("a")?.str())?,
                rel,
            })
        })
        .collect();
    let spettri = lista("spettri")
        .iter()
        .filter(|s| num(s, "livello") == 2.0)
        .map(|s| {
            let mut picchi: Vec<(f64, f64)> = s
                .get("picchi")
                .map_or(&[][..], |p| p.arr())
                .iter()
                .map(|p| (num(p, "mz"), num(p, "rel")))
                .collect();
            picchi.sort_by(|x, y| x.0.total_cmp(&y.0));
            Spettro {
                att: s.get("att").and_then(|x| x.str()).unwrap_or("").into(),
                ce: num(s, "ce"),
                path: passi(s.get("albero").and_then(|x| x.str()).unwrap_or("")),
                picchi,
            }
        })
        .collect();
    Ok(Grafo {
        nodi,
        archi,
        spettri,
    })
}

fn ppm(a: f64, b: f64) -> f64 {
    (a - b) / b * 1e6
}

/// One-to-one pairs `(i, j)` whose cost is known, the cheapest first.
fn accoppia(
    na: usize,
    nb: usize,
    costo: impl Fn(usize, usize) -> Option<f64>,
) -> Vec<(usize, usize, f64)> {
    let mut c: Vec<(usize, usize, f64)> = (0..na)
        .flat_map(|i| (0..nb).map(move |j| (i, j)))
        .filter_map(|(i, j)| Some((i, j, costo(i, j)?)))
        .collect();
    c.sort_by(|x, y| x.2.total_cmp(&y.2));
    let (mut ua, mut ub) = (vec![false; na], vec![false; nb]);
    c.retain(|&(i, j, _)| {
        !(ua[i] || ub[j]) && {
            ua[i] = true;
            ub[j] = true;
            true
        }
    });
    c
}

fn criterio(o: &mut Out, nome: &str, ok: bool, campi: &[(&str, f64)]) {
    o.raw(",\"").raw(nome).raw("\":{");
    for (k, v) in campi {
        o.raw("\"").raw(k).raw("\":").num(round(*v, 4)).raw(",");
    }
    o.raw("\"ok\":")
        .raw(if ok { "true" } else { "false" })
        .raw("}");
}

fn forti(g: &Grafo) -> Vec<&Arco> {
    g.archi.iter().filter(|a| a.rel >= REL_ARCO).collect()
}

/// The comparison text for two graph texts.
pub fn confronta(a: &str, b: &str) -> Result<String> {
    let (ga, gb) = (carica(a)?, carica(b)?);
    let mut o = Out::default();
    let mut tutti = true;
    let mut ok_di = |ok: bool| {
        tutti &= ok;
        ok
    };
    let mut corpo = Out::default();

    let (ia, ib): (Vec<&Nodo>, Vec<&Nodo>) = (
        ga.nodi.iter().filter(|n| n.isolato).collect(),
        gb.nodi.iter().filter(|n| n.isolato).collect(),
    );
    let comuni = accoppia(ia.len(), ib.len(), |i, j| {
        let m = ppm(ia[i].mz, ib[j].mz).abs();
        (!ia[i].path.is_empty() && ia[i].path == ib[j].path && m <= PPM).then_some(m)
    })
    .len();
    let ok = ok_di(comuni == ia.len() && comuni == ib.len());
    criterio(
        &mut corpo,
        "nodi_isolati",
        ok,
        &[
            ("a", ia.len() as f64),
            ("b", ib.len() as f64),
            ("comuni", comuni as f64),
        ],
    );

    let (fa, fb) = (forti(&ga), forti(&gb));
    let comuni = accoppia(fa.len(), fb.len(), |i, j| {
        let m = ppm(fa[i].da, fb[j].da)
            .abs()
            .max(ppm(fa[i].a, fb[j].a).abs());
        (m <= PPM).then_some(m)
    })
    .len();
    let ok = ok_di(comuni == fa.len() && comuni == fb.len());
    criterio(
        &mut corpo,
        "archi",
        ok,
        &[
            ("forti_a", fa.len() as f64),
            ("forti_b", fb.len() as f64),
            ("comuni", comuni as f64),
        ],
    );

    let coppie = accoppia(ga.nodi.len(), gb.nodi.len(), |i, j| {
        let m = ppm(ga.nodi[i].mz, gb.nodi[j].mz).abs();
        (m <= PPM).then_some(m)
    });
    let (mut visti, mut diversi) = (0, 0);
    for &(i, j, _) in &coppie {
        let (x, y) = (&ga.nodi[i].formula, &gb.nodi[j].formula);
        if x.is_some() || y.is_some() {
            visti += 1;
            diversi += usize::from(x != y);
        }
    }
    let ok = ok_di(diversi == 0);
    criterio(
        &mut corpo,
        "formule",
        ok,
        &[("confrontati", visti as f64), ("diversi", diversi as f64)],
    );

    let sp = accoppia(ga.spettri.len(), gb.spettri.len(), |i, j| {
        let (x, y) = (&ga.spettri[i], &gb.spettri[j]);
        (x.path == y.path && x.att == y.att && (x.ce - y.ce).abs() < 0.01).then_some(0.0)
    });
    let ent: Vec<f64> = sp
        .iter()
        .map(|&(i, j, _)| entropia(&ga.spettri[i], &gb.spettri[j]))
        .collect();
    let minimo = ent.iter().copied().fold(1.0, f64::min);
    let senza = ga.spettri.len() + gb.spettri.len() - 2 * sp.len();
    let ok = ok_di(senza == 0 && minimo >= ENTROPIA_MIN);
    criterio(
        &mut corpo,
        "entropia_ms2",
        ok,
        &[
            ("minimo", minimo),
            ("soglia", ENTROPIA_MIN),
            ("coppie", sp.len() as f64),
            ("senza_controparte", senza as f64),
        ],
    );

    let massimo = coppie.iter().map(|c| c.2).fold(0.0, f64::max);
    let ok = ok_di(massimo <= SCARTO_MAX);
    criterio(
        &mut corpo,
        "scarto_ppm",
        ok,
        &[
            ("massimo", massimo),
            ("soglia", SCARTO_MAX),
            ("coppie", coppie.len() as f64),
        ],
    );

    o.raw("{\"formato\":\"msnconfronto/1\",\"ppm\":").num(PPM);
    o.raw(",\"convalidata\":")
        .raw(if tutti { "true" } else { "false" });
    o.raw(&corpo.0).raw("}\n");
    Ok(o.0)
}

/// Entropy similarity of two MS2 spectra, peaks matched within `PPM` of the largest m/z.
fn entropia(a: &Spettro, b: &Spettro) -> f64 {
    let (am, ai): (Vec<f64>, Vec<f64>) = a.picchi.iter().copied().unzip();
    let (bm, bi): (Vec<f64>, Vec<f64>) = b.picchi.iter().copied().unzip();
    let top = am.iter().chain(&bm).copied().fold(0.0, f64::max);
    let pairs = match_peaks(&am, &ai, &bm, &bi, top * PPM * 1e-6);
    entropy_from_pairs(&ai, &bi, &pairs)
}

/// The arguments after `compare`: the two graph files and `--out`.
pub fn esegui(args: impl Iterator<Item = String>) -> Result<(String, Option<PathBuf>)> {
    let usage = |m: &str| Error::new("err.flow.param.type").with("detail", m);
    let (mut files, mut out) = (Vec::new(), None);
    let mut it = args;
    while let Some(x) = it.next() {
        match x.as_str() {
            "--out" => out = it.next().map(PathBuf::from),
            f if !f.starts_with("--") => files.push(PathBuf::from(f)),
            f => return Err(usage(&format!("unknown argument {f}"))),
        }
    }
    let [a, b] = files.as_slice() else {
        return Err(usage(
            "usage: mzlab-cli msn-graph compare A.json B.json [--out PATH]",
        ));
    };
    let leggi = |p: &PathBuf| {
        std::fs::read_to_string(p)
            .map_err(|e| Error::new("err.flow.param.type").with("detail", e.to_string()))
    };
    Ok((confronta(&leggi(a)?, &leggi(b)?)?, out))
}
