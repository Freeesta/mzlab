//! Text export of an `msnlib/1` library for `mzlab-cli msn-lib export`: one record per spectrum of every entry, as MSP or as MassBank `.txt` (MS_TYPE MS2, MS3, ...,
//! the path of precursors in the comment). Used by `msn_lib`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

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
    peaks: Vec<(f64, f64)>,
}

fn peaks(s: &Value, pulito: bool) -> Vec<(f64, f64)> {
    let all = s.get("picchi").map_or(&[][..], |p| p.arr());
    let keep = |p: &&Value| !pulito || p.get("nodo").is_some_and(|n| n.str().is_some());
    all.iter()
        .filter(keep)
        .filter_map(|p| Some((p.get("mz")?.f64()?, p.get("rel")?.f64()?)))
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

fn records(lib: &str, pulito: bool, tutti: bool) -> Result<Vec<Rec>> {
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
                peaks: peaks(s, pulito),
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
    for (m, i) in &r.peaks {
        t += &format!("{m:.5} {i:.2}\n");
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
    t += &format!(
        "MS$FOCUSED_ION: PRECURSOR_M/Z {:.5}\nPK$NUM_PEAK: {}\nPK$PEAK: m/z int. rel.int.\n",
        r.mz,
        r.peaks.len()
    );
    for (m, i) in &r.peaks {
        t += &format!("  {m:.5} {i:.2} {}\n", (i * 9.99).round().max(1.0) as i64);
    }
    t + "//\n\n"
}

/// The text of a library: one record per spectrum (MS2 only unless `tutti`) of every entry.
pub fn export_text(lib: &str, formato: Formato, pulito: bool, tutti: bool) -> Result<String> {
    let recs = records(lib, pulito, tutti)?;
    Ok(match formato {
        Formato::Msp => recs.iter().map(msp).collect(),
        Formato::MassBank => recs
            .iter()
            .enumerate()
            .map(|(n, r)| massbank(r, n + 1))
            .collect(),
    })
}
