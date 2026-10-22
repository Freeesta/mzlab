//! The outputs after the reader that `--downstream` adds to every scan row (docs/agenti/scandump.md): the mzML as written, the filter of `rawfilter`, the `.mzidx` row and the entry node of `msngrafo/1`.
use std::io::{Cursor, Write};

use mzlab_core::lettori::Lettore;
use mzlab_core::{lettori_mzml, msnalbero, mzidx_lettore, rawfilter::MzmlFilter};
use opentfraw::RawFileReader;
use serde_json::{json, Map, Value};

use super::{put, Scans};

/// `<mz>@<act><energy>` tokens of a Thermo filter: the chain of precursors, as `msnalbero` reads it.
fn chain_of(filter: &str) -> Vec<(f64, String, f64)> {
    filter
        .split_whitespace()
        .filter_map(|t| {
            let (mz, rest) = t.split_once('@')?;
            let act: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();
            let en = rest[act.len()..].parse().unwrap_or(0.0);
            Some((mz.parse().ok()?, act.to_lowercase(), en))
        })
        .collect()
}

fn attr<'a>(tag: &'a str, k: &str) -> Option<&'a str> {
    let key = format!("{k}=\"");
    let s = tag.find(&key)? + key.len();
    tag[s..].split('"').next()
}

fn scan_of(s: &str) -> Option<u32> {
    let r = &s[s.find("scan=")? + 5..];
    r[..r.find(|c: char| !c.is_ascii_digit()).unwrap_or(r.len())]
        .parse()
        .ok()
}

/// The cvParams of a piece of mzML as (accession, name, value).
fn cvs(part: &str) -> Vec<(&str, &str, &str)> {
    part.split("<cvParam ")
        .skip(1)
        .map(|t| {
            (
                attr(t, "accession").unwrap_or(""),
                attr(t, "name").unwrap_or(""),
                attr(t, "value").unwrap_or(""),
            )
        })
        .collect()
}

/// The precursors of every spectrum of an mzML text, in file order, as `scandump/1` writes them under `down.mzml`.
fn precursors_of(mzml: &str) -> Vec<Vec<Value>> {
    mzml.split("<spectrum ")
        .skip(1)
        .map(|sp| {
            let sp = sp.split("</spectrum>").next().unwrap_or("");
            sp.split("<precursor ")
                .skip(1)
                .map(|pr| {
                    let pr = pr.split("</precursor>").next().unwrap_or("");
                    let (head, act_part) = pr.split_once("<activation>").unwrap_or((pr, ""));
                    let mut o = Map::new();
                    put(
                        &mut o,
                        "spectrum_ref",
                        attr(head, "spectrumRef").and_then(scan_of),
                    );
                    for (acc, _, val) in cvs(head) {
                        let num = val.parse::<f64>().ok();
                        match acc {
                            "MS:1000744" => put(&mut o, "selected_mz", num),
                            "MS:1000827" => put(&mut o, "iso_target", num),
                            "MS:1000828" => put(&mut o, "iso_lower", num),
                            "MS:1000829" => put(&mut o, "iso_upper", num),
                            "MS:1000041" => put(&mut o, "charge", num),
                            _ => {}
                        }
                    }
                    let mut act = Vec::new();
                    for (acc, name, val) in cvs(act_part) {
                        if matches!(acc, "MS:1000045" | "MS:1000138" | "MS:1000509") {
                            o.insert(
                                "energy".into(),
                                json!({"name": name, "value": val.parse::<f64>().ok()}),
                            );
                        } else if val.is_empty() {
                            act.push(name.to_string());
                        }
                    }
                    if !act.is_empty() {
                        o.insert("activation".into(), json!(act));
                    }
                    Value::Object(o)
                })
                .collect()
        })
        .collect()
}

/// The `down` object of every scan row: what the outputs after the reader give for it (docs/agenti/scandump.md).
pub fn add_downstream(
    l: &mut dyn Lettore,
    raw: &RawFileReader,
    src: &mut Cursor<&[u8]>,
    name: &str,
    scans: &mut Scans,
) {
    let voci = l.indice(0).expect("index").to_vec();
    let mut mzml = Vec::new();
    lettori_mzml::scrivi(l, 0, &mut mzml, &mut |_, _| {}).expect("mzML");
    let written = precursors_of(&String::from_utf8_lossy(&mzml));
    let mut filtered = Vec::new();
    {
        let mut f = MzmlFilter::new(&mut filtered);
        opentfraw::write_mzml(raw, src, &mut f, name, false).expect("raw mzML");
        f.flush().expect("flush");
    }
    let refs = precursors_of(&String::from_utf8_lossy(&filtered));
    let table = mzidx_lettore::righe(l, 0).expect("rows");
    for (k, (row, meta)) in scans.iter_mut().enumerate() {
        let mut d = Map::new();
        d.insert("filter".into(), voci[k].filtro.clone().into());
        d.insert(
            "mzml".into(),
            json!({"precursors": written.get(k).cloned().unwrap_or_default()}),
        );
        let r = refs
            .get(k)
            .and_then(|p| p.first())
            .and_then(|p| p.get("spectrum_ref"))
            .cloned();
        if let Some(r) = r {
            d.insert("rawfilter".into(), json!({"spectrum_ref": r}));
        }
        if let Some(t) = table.rows.get(k) {
            let mut z = Map::new();
            z.insert("level".into(), t.level.into());
            put(&mut z, "precursor", t.precursor());
            z.insert("rt".into(), t.rt.into());
            d.insert("mzidx".into(), Value::Object(z));
        }
        let chain = chain_of(&voci[k].filtro);
        if meta.ms_level > 1 && !chain.is_empty() {
            let node = msnalbero::node_id(&chain);
            let c: Vec<Value> = chain
                .iter()
                .map(|(mz, a, e)| json!({"mz": mz, "activation": a, "energy": e}))
                .collect();
            d.insert(
                "msngraph".into(),
                json!({"node": node, "level": meta.ms_level, "chain": c}),
            );
        }
        if let Value::Object(m) = row {
            m.insert("down".into(), Value::Object(d));
        }
    }
}
