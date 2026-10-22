//! Text export of an `msnlib/1` library (`mzlab-cli msn-lib export`, the Flow step `msn.library_export` and the page): one record per spectrum of every entry, as MSP or as MassBank `.txt` (MS_TYPE MS2, MS3, ...,
//! the path of precursors in the comment). With `annota` each peak a node explains carries its formula and the neutral loss from the precursor. Used by the CLI and
//! the `.wasm` of the page, so both write the same bytes.

use crate::error::{Error, Result};
use crate::json::{parse, Value};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Formato {
    Msp,
    MassBank,
}

impl Formato {
    pub fn parse(s: &str) -> Result<Formato> {
        match s {
            "msp" => Ok(Formato::Msp),
            "massbank" => Ok(Formato::MassBank),
            _ => {
                Err(Error::new("err.flow.param.type")
                    .with("detail", "--formato is msp or massbank"))
            }
        }
    }
}

/// A peak with what the annotation needs: the formula of its node and the loss that leads to it from the precursor of the spectrum.
struct Pk {
    mz: f64,
    rel: f64,
    formula: Option<(String, f64, f64)>,
    loss: Option<(String, f64)>,
}

impl Pk {
    /// `C12H15N3O; loss C2O4 (87.98002)`, empty when the peak has neither.
    fn note(&self) -> String {
        let mut parts = Vec::new();
        if let Some((f, ..)) = &self.formula {
            parts.push(f.clone());
        }
        if let Some((f, m)) = &self.loss {
            parts.push(format!("loss {f} ({m:.5})"));
        }
        parts.join("; ")
    }
}

/// One spectrum of one entry, with what both formats need.
struct Rec {
    id: String,
    ion: String,
    negative: bool,
    level: i64,
    tree: String,
    act: String,
    ce: f64,
    mz: f64,
    peaks: Vec<Pk>,
}

/// The peaks of a spectrum `s` of the graph `g`; with `annota` the formula of the node of each peak and the loss of the arc that comes from the precursor.
fn peaks(g: &Value, s: &Value, pulito: bool, annota: bool) -> Vec<Pk> {
    let all = s.get("picchi").map_or(&[][..], |p| p.arr());
    let by_id = |list: &str, id: &str, key: &str| {
        g.get(list)
            .map_or(&[][..], |x| x.arr())
            .iter()
            .find(|n| n.get(key).and_then(|v| v.str()) == Some(id))
    };
    let precursor = s.get("precursore").and_then(|p| p.str()).unwrap_or("");
    all.iter()
        .filter(|p| !pulito || p.get("nodo").is_some_and(|n| n.str().is_some()))
        .filter_map(|p| {
            let node = p.get("nodo").and_then(|n| n.str()).filter(|_| annota);
            let formula = node.and_then(|id| {
                let f = by_id("nodi", id, "id")?.get("formula")?;
                let name = f.get("scelta")?.str()?;
                let c = f
                    .get("candidati")?
                    .arr()
                    .iter()
                    .find(|c| c.get("formula").and_then(|v| v.str()) == Some(name))?;
                Some((
                    name.to_string(),
                    c.get("mz_teorico")?.f64()?,
                    c.get("ppm")?.f64()?,
                ))
            });
            let loss = node.and_then(|id| {
                let a = g.get("archi")?.arr().iter().find(|a| {
                    a.get("da").and_then(|v| v.str()) == Some(precursor)
                        && a.get("a").and_then(|v| v.str()) == Some(id)
                })?;
                let l = a.get("perdita")?;
                Some((
                    l.get("formula")?.str()?.to_string(),
                    l.get("da_massa")?.f64()?,
                ))
            });
            Some(Pk {
                mz: p.get("mz")?.f64()?,
                rel: p.get("rel")?.f64()?,
                formula,
                loss,
            })
        })
        .collect()
}

/// The precursor m/z of a spectrum: its node when the graph has one, else the last step of the path (`305@cid35>261@cid35` → 261).
fn precursor_mz(s: &Value, nodi: &[Value], tree: &str) -> f64 {
    let node = s
        .get("precursore")
        .and_then(|p| p.str())
        .and_then(|p| p.strip_prefix('n')?.parse::<usize>().ok())
        .and_then(|k| nodi.get(k))
        .and_then(|n| n.get("mz")?.f64());
    node.or_else(|| {
        let last = tree.rsplit('>').next()?;
        last.split('@').next()?.parse().ok()
    })
    .unwrap_or(0.0)
}

fn records(lib: &str, pulito: bool, tutti: bool, annota: bool) -> Result<Vec<Rec>> {
    let v = parse(lib.as_bytes())?;
    let mut out = Vec::new();
    for e in v.get("voci").map_or(&[][..], |x| x.arr()) {
        let g = e.get("grafo").ok_or_else(|| Error::new("err.msnlib.bad"))?;
        let nodi = g.get("nodi").map_or(&[][..], |x| x.arr());
        let text = |v: Option<&Value>| v.and_then(|x| x.str()).unwrap_or("").to_string();
        let ion = text(e.get("ione").and_then(|i| i.get("formula")));
        let negative = g
            .get("polarita")
            .and_then(|p| p.f64())
            .is_some_and(|p| p < 0.0);
        for s in g.get("spettri").map_or(&[][..], |x| x.arr()) {
            let level = s.get("livello").and_then(|x| x.f64()).unwrap_or(0.0) as i64;
            if level != 2 && !tutti {
                continue;
            }
            let tree = text(s.get("albero"));
            out.push(Rec {
                id: text(e.get("id")),
                ion: ion.clone(),
                negative,
                level,
                mz: precursor_mz(s, nodi, &tree),
                act: text(s.get("att")),
                ce: s.get("ce").and_then(|x| x.f64()).unwrap_or(0.0),
                peaks: peaks(g, s, pulito, annota),
                tree,
            });
        }
    }
    Ok(out)
}

fn msp(r: &Rec) -> String {
    let modo = if r.negative { "N" } else { "P" };
    let mut t = format!(
        "Name: {} {} MS{} {} CE{}\nPrecursorMZ: {:.5}\nFormula: {}\nIon_mode: {modo}\n",
        r.id, r.tree, r.level, r.act, r.ce, r.mz, r.ion
    );
    t += &format!("Collision_energy: {}\nNum Peaks: {}\n", r.ce, r.peaks.len());
    for p in &r.peaks {
        t += &format!("{:.5} {:.2}", p.mz, p.rel);
        match p.note() {
            n if n.is_empty() => {}
            n => t += &format!(" \"{n}\""),
        }
        t.push('\n');
    }
    t.push('\n');
    t
}

/// A MassBank record; `n` numbers the records of the export so every ACCESSION is unique.
fn massbank(r: &Rec, n: usize) -> String {
    let modo = if r.negative { "NEGATIVE" } else { "POSITIVE" };
    let path = r.tree.replace('>', " > ");
    let mut t = format!(
        "ACCESSION: MZLAB-{}-{n:04}\nRECORD_TITLE: {}; MS{}; {}; CE {}\n",
        r.id,
        r.id,
        r.level,
        r.act.to_uppercase(),
        r.ce
    );
    t += &format!(
        "COMMENT: precursor path {path}\nCOMMENT: ion formula {}\n",
        r.ion
    );
    t += &format!(
        "AC$MASS_SPECTROMETRY: MS_TYPE MS{}\nAC$MASS_SPECTROMETRY: ION_MODE {modo}\n",
        r.level
    );
    t += &format!(
        "AC$MASS_SPECTROMETRY: ACTIVATION_TYPE {}\nAC$MASS_SPECTROMETRY: COLLISION_ENERGY {}\n",
        r.act.to_uppercase(),
        r.ce
    );
    for p in r.peaks.iter().filter(|p| p.loss.is_some()) {
        let (f, m) = p.loss.as_ref().expect("filtered");
        t += &format!("COMMENT: peak {:.5} neutral loss {f} {m:.5}\n", p.mz);
    }
    t += &format!("MS$FOCUSED_ION: PRECURSOR_M/Z {:.5}\n", r.mz);
    if r.peaks.iter().any(|p| p.formula.is_some()) {
        t += "PK$ANNOTATION: m/z tentative_formula formula_count mass error(ppm)\n";
        for p in &r.peaks {
            if let Some((f, mass, ppm)) = &p.formula {
                t += &format!("  {:.5} {f} 1 {mass:.5} {ppm:.2}\n", p.mz);
            }
        }
    }
    t += &format!(
        "PK$NUM_PEAK: {}\nPK$PEAK: m/z int. rel.int.\n",
        r.peaks.len()
    );
    for p in &r.peaks {
        t += &format!(
            "  {:.5} {:.2} {}\n",
            p.mz,
            p.rel,
            (p.rel * 9.99).round().max(1.0) as i64
        );
    }
    t + "//\n\n"
}

/// The text of a library: one record per spectrum (MS2 only unless `tutti`) of every entry; `annota` writes the formula and the loss of each explained peak.
pub fn export_text(
    lib: &str,
    formato: Formato,
    (pulito, tutti, annota): (bool, bool, bool),
) -> Result<String> {
    let recs = records(lib, pulito, tutti, annota)?;
    Ok(match formato {
        Formato::Msp => recs.iter().map(msp).collect(),
        Formato::MassBank => recs
            .iter()
            .enumerate()
            .map(|(n, r)| massbank(r, n + 1))
            .collect(),
    })
}
