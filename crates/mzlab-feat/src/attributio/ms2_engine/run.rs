// Ported from Attributio ms2_engine/generate.py and plugins/{metfrag,msfinder,rt_prediction,chimeric}.py (the request layer of the page).
//! One JSON request in, one JSON answer out, over the files of a campaign folder (`Io`) plus extra files the page adds (libraries, candidate lists): the five
//! pieces of the MS2 engine behind a single door, so the `.wasm`, the desktop route and the native test share the code. Requests carry `op` = `engine` |
//! `insilico` | `rearrange` | `rt` | `chimeric`. Everything here is evidence for the chemist; nothing sets a level.

use super::chimeric::{default_window, intruders, FeaturePoint};
use super::compile_library;
use super::generate::generate_hits;
use super::hits::engine_hits_csv;
use super::insilico::{rank, read_candidates, InSilicoOpts};
use super::matcher::{Ms2Library, SearchOpts};
use super::rearrange::attribute;
use super::rtpredict::RtModel;
use crate::attributio::campaign::{Campaign, Env};
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::io::Io;
use crate::attributio::review::explain::{parse_formula, prep_ms2};
use crate::attributio::review::settings::ReviewSettings;
use crate::attributio::sources::{load_sources, Sources};
use mzlab_core::json::{self, Out, Value};

fn bad(msg: &str) -> CampaignError {
    CampaignError::value(msg.to_string())
}

fn num(v: &Value, key: &str, default: f64) -> f64 {
    v.get(key).and_then(Value::f64).unwrap_or(default)
}

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::str)
        .ok_or_else(|| bad(&format!("the request needs '{key}'")))
}

fn campaign_and_sources(io: &Io, root: &str) -> Result<(Campaign, Sources)> {
    let t = io
        .text("campaign.toml")
        .ok_or_else(|| CampaignError::not_found("campaign.toml is not in the folder"))?;
    let c = Campaign::from_toml(&t, "campaign.toml", root, Env::default())?;
    let s = load_sources(&c, io)?;
    Ok((c, s))
}

/// The cleaned spectrum and precursor of a feature of the campaign.
fn feature_peaks(src: &Sources, pol: &str, id: i64) -> Result<(f64, Vec<f64>, Vec<f64>)> {
    let p = src
        .pol(pol)
        .ok_or_else(|| bad("no such polarity in the campaign"))?;
    let key = p
        .feature_keys()
        .into_iter()
        .find(|k| k.id == id)
        .ok_or_else(|| bad("no such feature"))?;
    let mz = key.mz.ok_or_else(|| bad("the feature has no m/z"))?;
    let sp = p
        .feature_spectra
        .get(&id)
        .ok_or_else(|| bad("the feature has no MS/MS spectrum"))?;
    let (m, i, _) = prep_ms2(&sp.mz, &sp.intensity, mz, 0.01);
    Ok((mz, m, i))
}

fn pol_of(v: &Value) -> Result<&str> {
    match text(v, "pol")? {
        "POS" => Ok("POS"),
        "NEG" => Ok("NEG"),
        _ => Err(bad("pol must be POS or NEG")),
    }
}

/// Answers a request. `extra` reads the files the page added next to the campaign (libraries, lists), by the path the request names.
pub fn ms2_json(
    io: &Io,
    root: &str,
    extra: &dyn Fn(&str) -> Option<Vec<u8>>,
    request: &str,
) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| bad("the request is not JSON"))?;
    let mut o = Out::default();
    match text(&v, "op")? {
        "engine" => engine(io, root, extra, &v, &mut o)?,
        "insilico" => insilico(io, root, extra, &v, &mut o)?,
        "rearrange" => rearrange(io, root, &v, &mut o)?,
        "rt" => rt(&v, &mut o)?,
        "chimeric" => chimeric(io, root, &v, &mut o)?,
        other => return Err(bad(&format!("unknown op: {other}"))),
    }
    Ok(o.0)
}

fn engine(
    io: &Io,
    root: &str,
    extra: &dyn Fn(&str) -> Option<Vec<u8>>,
    v: &Value,
    o: &mut Out,
) -> Result<()> {
    let (_, src) = campaign_and_sources(io, root)?;
    let mut entries = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for p in v.get("libraries").map(Value::arr).unwrap_or(&[]) {
        let path = p.str().ok_or_else(|| bad("libraries are paths"))?;
        let data = extra(path).ok_or_else(|| bad(&format!("cannot read {path}")))?;
        let (e, st) = compile_library(path, &data).map_err(|m| bad(&m))?;
        if st.dropped > 0 {
            warnings.push(format!(
                "{path}: {} records were dropped by the reader",
                st.dropped
            ));
        }
        entries.extend(e);
    }
    if entries.is_empty() {
        return Err(bad("no library spectrum with a polarity and peaks"));
    }
    let d = SearchOpts::default();
    let opts = SearchOpts {
        ppm: num(v, "ppm", d.ppm),
        min_score: num(v, "min_score", d.min_score),
        min_shared: num(v, "min_shared", d.min_shared as f64) as usize,
        top: d.top,
    };
    let lib = Ms2Library::new(entries);
    o.raw("{\"library\":").int(lib.len() as i64);
    for (key, pol) in [("pos", "POS"), ("neg", "NEG")] {
        let rows = generate_hits(&src, pol, &lib, &opts);
        o.raw(",\"").raw(key).raw("_hits\":").int(rows.len() as i64);
        o.raw(",\"")
            .raw(key)
            .raw("\":")
            .str(&engine_hits_csv(&rows));
    }
    o.raw(",\"warnings\":[");
    for (i, w) in warnings.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(w);
    }
    o.raw("]}");
    Ok(())
}

fn insilico(
    io: &Io,
    root: &str,
    extra: &dyn Fn(&str) -> Option<Vec<u8>>,
    v: &Value,
    o: &mut Out,
) -> Result<()> {
    let (_, src) = campaign_and_sources(io, root)?;
    let (pol, id) = (pol_of(v)?, num(v, "id", -1.0) as i64);
    let (mz, m, i) = feature_peaks(&src, pol, id)?;
    let path = text(v, "candidates")?;
    let list =
        String::from_utf8_lossy(&extra(path).ok_or_else(|| bad(&format!("cannot read {path}")))?)
            .into_owned();
    let cands = read_candidates(&list);
    let d = InSilicoOpts::default();
    let opts = InSilicoOpts {
        db_ppm: num(v, "db_ppm", d.db_ppm),
        frag_ppm: num(v, "frag_ppm", d.frag_ppm),
        ..d
    };
    let peaks: Vec<(f64, f64)> = m.into_iter().zip(i).collect();
    o.raw("{\"candidates\":")
        .int(cands.len() as i64)
        .raw(",\"ranked\":[");
    for (k, r) in rank(mz, pol, &peaks, &cands, &opts).iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(&r.candidate.id);
        o.raw(",\"name\":")
            .str(&r.candidate.name)
            .raw(",\"smiles\":")
            .str(&r.candidate.smiles);
        o.raw(",\"formula\":")
            .str(&r.candidate.formula)
            .raw(",\"score\":")
            .num(r.score);
        o.raw(",\"mass_ppm\":")
            .num(r.mass_ppm)
            .raw(",\"explained\":")
            .floats(&r.explained)
            .raw("}");
    }
    o.raw("]}");
    Ok(())
}

fn rearrange(io: &Io, root: &str, v: &Value, o: &mut Out) -> Result<()> {
    let (_, src) = campaign_and_sources(io, root)?;
    let (pol, id) = (pol_of(v)?, num(v, "id", -1.0) as i64);
    let (mz, m, i) = feature_peaks(&src, pol, id)?;
    let fd = parse_formula(text(v, "formula")?);
    let a = attribute(&fd, mz, pol, (&m, &i), &ReviewSettings::default());
    let Some(a) = a else {
        o.raw("{\"attribution\":null}");
        return Ok(());
    };
    o.raw("{\"attribution\":{\"adduct\":")
        .str(&a.adduct)
        .raw(",\"ion_formula\":")
        .str(&a.ion_formula);
    o.raw(",\"n_explained\":")
        .int(a.n_explained as i64)
        .raw(",\"n_rearranged\":")
        .int(a.n_rearranged as i64);
    o.raw(",\"frac_explained\":")
        .num(a.frac_explained)
        .raw(",\"score\":")
        .num(a.score)
        .raw(",\"peaks\":[");
    for (k, (mzp, h)) in m.iter().zip(&a.hits).enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(*mzp);
        match h {
            Some(h) => {
                o.raw(",\"shift\":")
                    .int(h.shift as i64)
                    .raw(",\"formula\":")
                    .str(&h.formula);
                o.raw(",\"ppm\":").num(h.ppm).raw(",\"loss\":").str(&h.loss);
            }
            None => {
                o.raw(",\"shift\":null");
            }
        }
        o.raw("}");
    }
    o.raw("]}}");
    Ok(())
}

fn rt(v: &Value, o: &mut Out) -> Result<()> {
    let rows: Vec<(String, f64)> = v
        .get("train")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .filter_map(|r| Some((r.at(0)?.str()?.to_string(), r.at(1)?.f64()?)))
        .collect();
    let Some(model) = RtModel::train(&rows, 42) else {
        o.raw("{\"model\":false,\"predictions\":[]}");
        return Ok(());
    };
    o.raw("{\"model\":true,\"trained\":")
        .int(rows.len() as i64)
        .raw(",\"predictions\":[");
    for (k, s) in v
        .get("predict")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let smiles = s.str().unwrap_or("");
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"smiles\":")
            .str(smiles)
            .raw(",\"rt\":")
            .opt(model.predict(smiles))
            .raw("}");
    }
    o.raw("]}");
    Ok(())
}

fn chimeric(io: &Io, root: &str, v: &Value, o: &mut Out) -> Result<()> {
    let (c, src) = campaign_and_sources(io, root)?;
    let (pol, id) = (pol_of(v)?, num(v, "id", -1.0) as i64);
    let p = src
        .pol(pol)
        .ok_or_else(|| bad("no such polarity in the campaign"))?;
    let all: Vec<FeaturePoint> = p
        .feature_keys()
        .into_iter()
        .filter_map(|k| {
            Some(FeaturePoint {
                pol: k.pol,
                id: k.id,
                mz: k.mz?,
                rt: k.rt?,
            })
        })
        .collect();
    let target = all
        .iter()
        .find(|f| f.id == id)
        .ok_or_else(|| bad("no such feature"))?;
    let window = num(v, "window", default_window(&c.instrument));
    o.raw("{\"isolation_window\":")
        .num(window)
        .raw(",\"intruders\":[");
    for (k, x) in intruders(&all, target, window, num(v, "rt_tol", 0.1))
        .iter()
        .enumerate()
    {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"id\":")
            .int(x.id)
            .raw(",\"mz\":")
            .num(x.mz);
        o.raw(",\"rt\":")
            .num(x.rt)
            .raw(",\"delta_mz\":")
            .num(x.delta_mz)
            .raw(",\"delta_rt\":")
            .num(x.delta_rt)
            .raw("}");
    }
    o.raw("]}");
    Ok(())
}
