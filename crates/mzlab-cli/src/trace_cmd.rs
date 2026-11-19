//! `mzlab-cli trace chrom|xic`: any trace of a file as `trace/1` JSON or a `rt_min,intensity` CSV (what `peak-*` read). `chrom` serves every channel `file channels`
//! lists (TIC, BPC, scan types, SRM/SIM transitions, UV, analog) with the answers of the page's `api/chrom` and `api/mrm`; `xic` is `api/xic`.
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

pub const HELP_XIC: &str = "mzlab-cli trace xic: the extracted ion chromatogram of an m/z as JSON (trace/1) or CSV (rt_min,intensity), the answer of the page's api/xic.

usage: mzlab-cli trace xic FILE --mz M (--ppm P | --da D) [--rt A,B] [--ms-level N] [--polarity +|-] [--sample ID] [--format json|csv] [--out PATH]

  --mz M      the ion's m/z
  --ppm P     half-window in ppm of M; --da D half-window in Da (one of the two, never both)
  --rt A,B    keep the points with A <= rt <= B (minutes)
  --ms-level N the level of the scans summed (default 1)
  --polarity  the + or - part of a mixed-polarity file (a file with no such part gives an empty trace, counted in the evidence)
  --sample ID the sample of a file with several (.wiff); default the first
  --format F  json (default) or csv with the header rt_min,intensity
  --out PATH  write there instead of the standard output";

/// A trace the way a route gives it, plus what produced it.
struct Answer {
    /// `{"rt":[..],"y":[..]}` (or the band of `uv:auto`).
    text: String,
    y_unit: String,
    /// The members of `evidence`, no braces.
    evidence: String,
    schema: &'static str,
}

fn tail(file: &str, s: &Sample) -> String {
    let sample = s.sample.map_or("null".to_string(), |i| i.to_string());
    format!("\"file\":{},\"sample\":{sample}", extract::jstr(file))
}

/// The document or CSV of `a` inside `win`.
fn render(a: Answer, s: &Sample, win: Option<(f64, f64)>, csv: bool, id: &str) -> Result<String> {
    if a.schema == "uvband/1" {
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
    return Ok(Answer {
        text,
        y_unit,
        evidence: family_ev(
            "mzml:uv-analog",
            id,
            &format!(",\"max_points_per_extreme\":{n}"),
        ),
        schema: "trace/1",
    });
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
                ("canale".into(), id.into()),
                ("kind".into(), kind.into()),
            ]);
            let text = s.routes.handle("/api/chrom", &q)?.ok_or_else(unknown)?;
            let ev = format!(
                "{},\"route\":\"/api/chrom\",\"query\":{}",
                family_ev("route", id, &meta),
                extract::jstr(&format!("canale={id}&kind={kind}"))
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

fn finish(r: Result<(String, String)>, out: &Option<PathBuf>) -> Outcome {
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

fn one_file(p: &Parsed, what: &str) -> std::result::Result<PathBuf, String> {
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
fn xic_answer(s: &mut Sample, mz: f64, tol: f64, level: u8, pol: Option<i8>) -> Result<Answer> {
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

pub fn xic(args: Vec<String>) -> Outcome {
    let go = || -> std::result::Result<_, String> {
        let Some(p) = extract::parse(
            args,
            &[
                "--mz",
                "--ppm",
                "--da",
                "--rt",
                "--ms-level",
                "--polarity",
                "--sample",
                "--format",
                "--out",
            ],
            &[],
        )?
        else {
            return Ok(Err(HELP_XIC));
        };
        let file = one_file(&p, "trace xic")?;
        let mz = p
            .num("--mz")?
            .filter(|m| *m > 0.0)
            .ok_or("trace xic needs --mz")?;
        let tol = match (p.num("--ppm")?, p.num("--da")?) {
            (Some(_), Some(_)) => return Err("give --ppm or --da, not both".into()),
            (Some(ppm), None) if ppm > 0.0 => (mz * ppm * 1e-6, Some(ppm)),
            (None, Some(da)) if da > 0.0 => (da, None),
            (None, None) => return Err("trace xic needs --ppm or --da".into()),
            _ => return Err("--ppm and --da need a positive number".into()),
        };
        let pol = match p.get("--polarity") {
            None => None,
            Some("+") => Some(1),
            Some("-") => Some(-1),
            Some(_) => return Err("--polarity takes + or -".into()),
        };
        Ok(Ok((
            p.sample(&file)?,
            p.window()?,
            p.csv()?,
            p.out(),
            file,
            mz,
            tol,
            p.level()?,
            pol,
        )))
    };
    let (sample, win, csv, out, file, mz, (tol, ppm), level, pol) = match go() {
        Ok(Ok(a)) => a,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    finish(
        extract::with_tmp("xic", |tmp| {
            let mut s = extract::open_sample(&file, sample, tmp)?;
            let mut a = xic_answer(&mut s, mz, tol, level, pol)?;
            if let Some(p) = ppm {
                a.evidence += &format!(",\"tolerance_ppm\":{}", extract::jnum(p));
            }
            Ok((
                render(a, &s, win, csv, &format!("xic:{}", extract::jnum(mz)))?,
                s.note.clone(),
            ))
        }),
        &out,
    )
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
