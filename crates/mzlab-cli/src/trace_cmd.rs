//! `mzlab-cli trace chrom`: any trace of a file as `trace/1` JSON or a `rt_min,intensity` CSV (what `peak-*` read). `chrom` serves every channel `file channels`
//! lists (TIC, BPC, scan types, SRM/SIM transitions, UV, analog) with the answers of the page's `api/chrom` and `api/mrm`; `trace xic` (trace_xic.rs) is `api/xic`.
//! The number tokens are the route's own, so the points are the bytes the site draws.

use std::path::PathBuf;

use mzlab_core::channels::{Famiglia, Source};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;
use mzlab_core::routes::Query;

use crate::extract::{self, Parsed, Sample};
use crate::Outcome;

pub const HELP_CHROM: &str = "mzlab-cli trace chrom: the trace of a channel of a file as JSON (trace/1) or CSV (rt_min,intensity).

usage: mzlab-cli trace chrom FILE --id ID [--sample ID] [--rt A,B] [--n N] [--format json|csv] [--out PATH]

  FILE        an mzML, or any single file the app reads (.raw, mzXML, .wiff, ...)
  --id ID     a channel `file channels` lists: tic, bpc, ms:<key>, dd:<level>, srm:<q1>><q3>[@ce], uv:twc, uv:<nm>±<bw>, an:<i>; uv:auto answers the XWC band
              proposed from the DAD matrix (uvband/1) instead of a trace; an id the file does not have: exit 5
  --sample ID the sample of a file with several (.wiff); default the first, the others are named on the standard error
  --rt A,B    keep the points with A <= rt <= B (minutes); for uv:auto the window the band is proposed in
  --n N       points at most per extreme of an analog trace (default 2000)
  --format F  json (default) or csv with the header rt_min,intensity, the input of every `peak` command
  --out PATH  write there instead of the standard output";

/// A trace the way a route gives it, plus what produced it.
pub(crate) struct Answer {
    /// `{"rt":[..],"y":[..]}` (or the band of `uv:auto`).
    pub(crate) text: String,
    pub(crate) y_unit: String,
    /// The members of `evidence`, no braces.
    pub(crate) evidence: String,
    pub(crate) schema: &'static str,
}

pub(crate) fn tail(file: &str, s: &Sample) -> String {
    let sample = s.sample.map_or("null".to_string(), |i| i.to_string());
    format!("\"file\":{},\"sample\":{sample}", extract::jstr(file))
}

/// The document or CSV of `a` inside `win`.
pub(crate) fn render(
    a: Answer,
    s: &Sample,
    win: Option<(f64, f64)>,
    csv: bool,
    id: &str,
) -> Result<String> {
    if a.schema == "uvband/1" {
        if csv {
            // the band is a document of its own (uvband/1): no rt,intensity table to print
            return Err(Error::new("err.cli.format.csvNotUvAuto"));
        }
        return Ok(format!(
            "{{\"schema\":\"uvband/1\",\"id\":{},{},{},{}}}",
            extract::jstr(id),
            tail(&s.name, s),
            format_args!("\"evidence\":{{{}}}", a.evidence),
            &a.text[1..a.text.len() - 1]
        ));
    }
    let bad = || Error::new("error.badIndex");
    let x = extract::tokens(&a.text, "rt")
        .or_else(|| extract::tokens(&a.text, "t"))
        .ok_or_else(bad)?;
    let y = extract::tokens(&a.text, "y").ok_or_else(bad)?;
    let pts = extract::pairs(&x, &y, win);
    if csv {
        return Ok(extract::points_csv("rt_min", "intensity", &pts));
    }
    let head = format!(
        "\"id\":{},{},\"x_unit\":\"min\",\"y_unit\":{}",
        extract::jstr(id),
        tail(&s.name, s),
        extract::jstr(&a.y_unit)
    );
    let window = win.map_or("null".to_string(), |(l, h)| {
        format!("[{},{}]", extract::jnum(l), extract::jnum(h))
    });
    let ev = format!(
        "\"evidence\":{{{},\"points_in_source\":{},\"rt_window_min\":{window}}}",
        a.evidence,
        x.len()
    );
    Ok(extract::points_doc(a.schema, &head, &pts, &ev))
}

fn rounded_trace(t: &[f64], y: &[f64]) -> String {
    let mut o = Out::default();
    o.raw("{\"rt\":")
        .rounded(t, 4)
        .raw(",\"y\":")
        .rounded(y, 1)
        .raw("}");
    o.0
}

fn family_ev(source: &str, id: &str, extra: &str) -> String {
    format!(
        "\"source\":{},\"channel\":{}{extra}",
        extract::jstr(source),
        extract::jstr(id)
    )
}

/// A UV or analog channel (`uv:*`, `an:*`): its trace from the mzML, or the band `uv:auto` proposes.
fn uv_answer(s: &mut Sample, id: &str, n: i64, win: Option<(f64, f64)>) -> Result<Answer> {
    if id == "uv:auto" {
        let text = match win {
            Some((a, b)) => mzlab_core::channels_uv::spettro(&mut s.mzml, id, a, b)?,
            None => mzlab_core::channels_uv::trace(&mut s.mzml, &s.chroms, id, n)?,
        };
        return Ok(Answer {
            text,
            y_unit: String::new(),
            evidence: family_ev("mzml:uv", id, ""),
            schema: "uvband/1",
        });
    }
    let text = mzlab_core::channels_uv::trace(&mut s.mzml, &s.chroms, id, n)?;
    // the analog route names its unit (bar, psi, ...); a UV trace is in mAU
    let y_unit = mzlab_core::json::parse(text.as_bytes())
        .ok()
        .and_then(|v| v.get("unit").and_then(|u| u.str().map(String::from)))
        .unwrap_or_else(|| {
            if id.starts_with("an:") {
                "counts".into()
            } else {
                "mAU".into()
            }
        });
    Ok(Answer {
        text,
        y_unit,
        evidence: family_ev(
            "mzml:uv-analog",
            id,
            &format!(",\"max_points_per_extreme\":{n}"),
        ),
        schema: "trace/1",
    })
}

/// The sum of the SRM traces by time (what `file channels` calls `tic` for a file with no TIC of its own).
fn srm_sum(s: &mut Sample) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for ch in s.chroms.clone().iter().filter(|x| x.kind == "srm") {
        let (t, y) = s.mzml.chromatogram(ch)?;
        pts.extend(t.into_iter().zip(y));
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut sum: Vec<(f64, f64)> = Vec::new();
    for (t, y) in pts {
        match sum.last_mut() {
            Some(l) if l.0 == t => l.1 += y,
            _ => sum.push((t, y)),
        }
    }
    Ok(sum.into_iter().unzip())
}

/// The trace of channel `id`: the route of the page for scan channels, the chromatogram of the mzML for transitions, UV and analog channels.
fn chrom_answer(s: &mut Sample, id: &str, n: i64, win: Option<(f64, f64)>) -> Result<Answer> {
    let unknown = || Error::new("err.canali.unknown").with("id", id);
    if mzlab_core::channels_uv::read_id(id).is_some() {
        return uv_answer(s, id, n, win);
    }
    let channels = s.routes.canali();
    let c = channels.v[channels.find(id).ok_or_else(unknown)?].clone();
    let meta = format!(
        ",\"family\":{},\"label\":{},\"ms_level\":{},\"polarity\":{}",
        extract::jstr(c.famiglia.name()),
        extract::jstr(&c.label),
        c.livello,
        c.pol
    );
    let counts = "counts".to_string();
    match (c.famiglia, &c.source) {
        (_, Source::Cromatogramma(i)) => {
            let ch = s
                .chroms
                .iter()
                .find(|x| x.index as i64 == *i)
                .ok_or_else(unknown)?
                .clone();
            let (t, y) = s.mzml.chromatogram(&ch)?;
            Ok(Answer {
                text: rounded_trace(&t, &y),
                y_unit: counts,
                evidence: family_ev("mzml:chromatogram", id, &meta),
                schema: "trace/1",
            })
        }
        (_, Source::Somma) => {
            let (t, y) = srm_sum(s)?;
            Ok(Answer {
                text: rounded_trace(&t, &y),
                y_unit: counts,
                evidence: family_ev("sum:srm", id, &meta),
                schema: "trace/1",
            })
        }
        _ => {
            let kind = if c.famiglia == Famiglia::Bpc {
                "bpc"
            } else {
                "tic"
            };
            let q = Query(vec![
                ("channel".into(), id.into()),
                ("kind".into(), kind.into()),
            ]);
            let text = s.routes.handle("/api/chrom", &q)?.ok_or_else(unknown)?;
            let ev = format!(
                "{},\"route\":\"/api/chrom\",\"query\":{}",
                family_ev("route", id, &meta),
                extract::jstr(&format!("channel={id}&kind={kind}"))
            );
            Ok(Answer {
                text,
                y_unit: counts,
                evidence: ev,
                schema: "trace/1",
            })
        }
    }
}

pub(crate) fn finish(r: Result<(String, String)>, out: &Option<PathBuf>) -> Outcome {
    match r {
        Err(e) => crate::sample_select::fail(&e),
        Ok((t, note)) => {
            let mut o = crate::emit(t, out);
            if o.code == 0 {
                o.stderr = note;
            }
            o
        }
    }
}

pub(crate) fn one_file(p: &Parsed, what: &str) -> std::result::Result<PathBuf, String> {
    match p.pos.as_slice() {
        [f] => Ok(PathBuf::from(f)),
        [] => Err(format!("{what} needs a file")),
        _ => Err(format!("{what} takes one file")),
    }
}

pub fn chrom(args: Vec<String>) -> Outcome {
    let go = || -> std::result::Result<_, String> {
        let Some(p) = extract::parse(
            args,
            &["--id", "--sample", "--rt", "--n", "--format", "--out"],
            &[],
        )?
        else {
            return Ok(Err(HELP_CHROM));
        };
        let file = one_file(&p, "trace chrom")?;
        let id = p.get("--id").ok_or("trace chrom needs --id")?.to_string();
        let n = p.num("--n")?.map_or(2000, |v| v as i64);
        Ok(Ok((
            p.sample(&file)?,
            p.window()?,
            p.csv()?,
            p.out(),
            file,
            id,
            n,
        )))
    };
    let (sample, win, csv, out, file, id, n) = match go() {
        Ok(Ok(a)) => a,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    finish(
        extract::with_tmp("trace", |tmp| {
            let mut s = extract::open_sample(&file, sample, tmp)?;
            let a = chrom_answer(&mut s, &id, n, win)?;
            Ok((render(a, &s, win, csv, &id)?, s.note.clone()))
        }),
        &out,
    )
}

/// `api/xic` of the page: the sum of the peaks within `tol` Da of `mz` in the scans of one level, by scan.
pub(crate) fn xic_answer(
    s: &mut Sample,
    mz: f64,
    tol: f64,
    level: u8,
    pol: Option<i8>,
) -> Result<Answer> {
    s.routes.pol = pol;
    let text = s.routes.xic(mz, tol, level)?;
    let used = s
        .routes
        .idx
        .section_of(level, pol)
        .map_or(0, |si| s.routes.idx.sections[si].n_scans);
    let ev = format!(
        "\"route\":\"/api/xic\",\"mz\":{},\"tolerance_da\":{},\"ms_level\":{level},\"polarity\":{},\"scans_in_section\":{used}",
        extract::jnum(mz),
        extract::jnum(tol),
        pol.map_or("null".to_string(), |p| format!("\"{}\"", if p > 0 { "+" } else { "-" })),
    );
    Ok(Answer {
        text,
        y_unit: "counts".into(),
        evidence: ev,
        schema: "trace/1",
    })
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
