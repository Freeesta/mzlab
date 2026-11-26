//! `mzlab-cli raw isotopes FILE... --mz M --rt A,B` (`raw-isotopes/1`, `docs/agenti/raw-access.md`): do M+1 and M+2 fit a formula, within counting error?
//! The members and the `result` come from `mzlab_core::rawiso` (the `/api/rawiso` route the app and the site answer), so their bytes are the route's; this
//! file reads the flags, opens the samples, turns the route's flags into warnings and adds the `provenance`.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out};
use mzlab_core::rawiso::Ask;
use mzlab_core::routes::Query;

use crate::extract::{self, jstr, Parsed};
use crate::raw_cmd::{choice, error_entry, missing, prov_file, provenance, warnings_of, whole_run};
use crate::raw_plan::plan_of;
use crate::raw_xic_ask::enc;
use crate::raw_xic_cmd::{choices_of, entry_json};
use crate::Outcome;

pub const ISO_HELP: &str = "mzlab-cli raw isotopes: do M+1 and M+2 fit this formula, within counting error? Net ion counts of the bins M, M+1 and M+2 over the peak
region of M (the sides subtracted), the observed ratios R1 = N1/N0 and R2 = N2/N0 with their delta-method sigma. JSON (raw-isotopes/1): per file a `result`.
With --formula the expected pattern of `ion pattern` and a z score and a verdict per ratio (consistent |z| < z_ok, inconsistent |z| >= z_bad, else
undetermined; too few counts are undetermined with their reason). Without a formula the carbon range from R1 and the M+2 hint (evidence only).
The correlation of M+1 with M over the region flags a co-eluting interference.

usage: mzlab-cli raw isotopes FILE... --mz M --rt A,B [--formula F [--adduct A]] [--bin nominal] [--ms-level N] [--polarity +|-]
                              [--mz-offset DA|auto:MZ[@RT]] [--set NAME=VALUE,...] [--sample ID] [--dry-run] [--out PATH]

  --mz M      the monoisotopic mass of the ion; the bins are whole masses (round(M), +1, +2)   --rt A,B  the window, min; the peak of M inside it is the region
  --formula F the neutral formula (C6H12O6); --adduct A a name of `ion pattern` (default [M+H]+, logged as a choice); the ion must be singly charged
  --set       min_counts=10 (least net counts of M and of the ratio's bin), z_ok=2, z_bad=3, c13_pct=1.1 (13C per carbon, %), carbon_k=2 (sigma of the range),
              m2_ratio=0.2 and m2_k=3 (the M+2 hint), min_r=0.8 (least correlation of M+1 with M), min_sn, min_points, alpha, side_min, gap_min (as raw xic)
  --dry-run   the resolved query and each file with its cache status (raw-plan/1); reads no peak
exit 0 answered (any verdict), 1 a file could not be read (its entry has `error`), 2 usage, 4 a file is missing, 5 unknown sample, 6 --out not writable";

/// The keys of `--set` and of the route.
const SET_KEYS: &[&str] = &[
    "min_counts",
    "z_ok",
    "z_bad",
    "c13_pct",
    "carbon_k",
    "m2_ratio",
    "m2_k",
    "min_r",
    "min_sn",
    "min_points",
    "alpha",
    "side_min",
    "gap_min",
];

/// The question: the files, the route question and the polarity.
pub(crate) struct IsoAsk {
    files: Vec<PathBuf>,
    ask: Ask,
    pol: Option<i8>,
}

/// `--set a=1,b=2` (repeatable) as pairs; a name outside `SET_KEYS` is refused.
fn set_of(p: &Parsed) -> std::result::Result<Vec<(String, String)>, Error> {
    let mut out: Vec<(String, String)> = Vec::new();
    for v in p.all("--set") {
        for item in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (k, val) = item
                .split_once('=')
                .filter(|(k, _)| SET_KEYS.contains(k))
                .ok_or_else(|| {
                    Error::new("err.cli.rawiso.set")
                        .with("item", item)
                        .with("names", SET_KEYS.join(", "))
                })?;
            out.retain(|(n, _)| n != k);
            out.push((k.to_string(), val.to_string()));
        }
    }
    Ok(out)
}

fn iso_ask_of(p: &Parsed) -> std::result::Result<IsoAsk, Error> {
    let files: Vec<PathBuf> = p.pos.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Err(Error::new("err.cli.rawiso.needsFile"));
    }
    let mz = p
        .get("--mz")
        .ok_or_else(|| Error::new("err.cli.rawiso.needsMz"))?;
    let rt = p
        .get("--rt")
        .ok_or_else(|| Error::new("err.cli.rawiso.needsRt"))?;
    let mut query = format!("mz={}&rt={}", enc(mz), enc(rt));
    for (flag, key) in [
        ("--formula", "formula"),
        ("--adduct", "adduct"),
        ("--bin", "bin"),
        ("--ms-level", "ms_level"),
        ("--mz-offset", "mz_offset"),
    ] {
        if let Some(v) = p.get(flag) {
            query += &format!("&{key}={}", enc(v));
        }
    }
    for (k, v) in set_of(p)? {
        query += &format!("&{k}={}", enc(&v));
    }
    let pol = match p.get("--polarity") {
        None => None,
        Some("+") => Some(1),
        Some("-") => Some(-1),
        Some(v) => return Err(Error::new("err.cli.rawiso.polarity").with("value", v)),
    };
    // the route reads the same text, so a value the site would refuse is refused here before any file is opened
    let ask = Ask::of(&Query::parse(&query))?;
    Ok(IsoAsk { files, ask, pol })
}

/// What one file answered: its entry, its result (the file and sample in front), the `provenance.files` member, warnings and choices.
struct Answer {
    entry: String,
    result: String,
    prov: String,
    warnings: Vec<String>,
    choices: Vec<String>,
}

fn warning(key: &str, file: &str, sample: Option<u32>, extra: &[(&str, String)]) -> String {
    let mut o = Out::default();
    o.raw("{\"key\":")
        .str(key)
        .raw(",\"params\":{\"file\":")
        .str(file);
    o.raw(",\"sample\":")
        .raw(&sample.map_or("null".into(), |s| s.to_string()));
    for (k, v) in extra {
        o.raw(",").str(k).raw(":").raw(v);
    }
    o.raw("}}");
    o.0
}

/// The route's flags and its interference test as warnings: evidence the user sees next to the numbers, never a changed value.
fn flag_warnings(a: &Ask, file: &str, sample: Option<u32>, result: &str) -> Vec<String> {
    let Ok(v) = parse(result.as_bytes()) else {
        return Vec::new();
    };
    let flags: Vec<&str> = v
        .get("flags")
        .map(|f| f.arr().iter().filter_map(|x| x.str()).collect())
        .unwrap_or_default();
    let mut out = Vec::new();
    for fl in flags {
        let w = match fl {
            "formula_mz_mismatch" => {
                let e = v
                    .get("expected")
                    .and_then(|e| e.get("mz"))
                    .and_then(|m| m.f64());
                warning(
                    "warn.raw.isotopes.formulaMismatch",
                    file,
                    sample,
                    &[
                        ("mz", a.mz.to_string()),
                        ("formula_mz", e.map_or("null".into(), |m| m.to_string())),
                    ],
                )
            }
            "no_expected_m" => warning(
                "warn.raw.isotopes.noExpected",
                file,
                sample,
                &[("formula", jstr(a.formula.as_deref().unwrap_or("")))],
            ),
            "no_peak_region" => warning("warn.raw.isotopes.noPeak", file, sample, &[]),
            _ => continue,
        };
        out.push(w);
    }
    let possible = v
        .get("interference")
        .and_then(|i| i.get("possible"))
        .and_then(|p| p.bool());
    if possible == Some(true) {
        let r = v
            .get("interference")
            .and_then(|i| i.get("r_m1"))
            .and_then(|r| r.f64());
        out.push(warning(
            "warn.raw.isotopes.interference",
            file,
            sample,
            &[
                ("r", r.map_or("null".into(), |r| r.to_string())),
                ("min_r", a.p.min_r.to_string()),
            ],
        ));
    }
    out
}

/// `{"file":..,"sample":..,` in front of the members of `obj` (a JSON object).
fn tagged(name: &str, sid: &str, obj: &str) -> String {
    format!("{{\"file\":{},\"sample\":{sid},{}", jstr(name), &obj[1..])
}

fn one(f: &Path, sample: Option<u32>, a: &IsoAsk) -> Result<(Answer, String)> {
    extract::with_tmp("raw", |tmp| {
        let mut s = extract::open_sample(f, sample, tmp)?;
        s.routes.pol = a.pol;
        let out = s.routes.raw_isotopes(&a.ask)?;
        let sha = s
            .routes
            .idx
            .source
            .get("sha256")
            .and_then(|v| v.str())
            .unwrap_or("")
            .to_string();
        let size = std::fs::metadata(f)
            .ok()
            .filter(|m| m.is_file())
            .map(|m| m.len());
        let sid = s.sample.map_or("null".into(), |i| i.to_string());
        let sname = s
            .listing
            .iter()
            .find(|l| Some(l.0) == s.sample)
            .map_or(s.name.clone(), |l| l.1.clone());
        let entry = entry_json(
            &s.name,
            &s.container,
            &sha,
            size,
            &sid,
            &sname,
            &out.members,
        );
        let mut warnings = Vec::new();
        warnings_of(&s.name, s.sample, &out.members, &mut warnings);
        warnings.extend(flag_warnings(&a.ask, &s.name, s.sample, &out.result));
        let choices = choices_of(
            &s.name,
            &sid,
            sample.is_none() && s.sample.is_some(),
            &out.members,
            a.ask.level,
            false,
        );
        let prov = prov_file(&s.name, &sha, size, s.sample);
        Ok((
            Answer {
                entry,
                result: tagged(&s.name, &sid, &out.result),
                prov,
                warnings,
                choices,
            },
            s.note,
        ))
    })
}

/// `provenance.params`: every value of the question with its default resolved.
fn params_json(p: &Parsed, a: &IsoAsk) -> String {
    let k = &a.ask;
    let t = &k.p;
    let mut o = Out::default();
    o.raw("{\"mz\":")
        .num(k.mz)
        .raw(",\"rt\":")
        .floats(&[k.rt.0, k.rt.1]);
    o.raw(",\"formula\":")
        .raw(&k.formula.as_deref().map_or("null".into(), jstr));
    o.raw(",\"adduct\":")
        .raw(&k.adduct.as_deref().map_or("null".into(), jstr));
    o.raw(",\"bin\":\"nominal\",\"ms_level\":")
        .int(i64::from(k.level));
    o.raw(",\"polarity\":").raw(
        &a.pol
            .map_or("null".into(), |p| jstr(if p > 0 { "+" } else { "-" })),
    );
    o.raw(",\"mz_offset\":")
        .raw(&k.offset.map_or("null".into(), |s| jstr(&s.to_string())));
    o.raw(",\"sample\":")
        .raw(&p.get("--sample").map_or("null".into(), jstr));
    o.raw(",\"set\":{");
    let vals = [
        t.min_counts,
        t.z_ok,
        t.z_bad,
        t.c13_pct,
        t.carbon_k,
        t.m2_ratio,
        t.m2_k,
        t.min_r,
        t.x.min_sn,
        t.x.min_points as f64,
        t.x.alpha,
        t.x.side_min,
        t.x.gap_min,
    ];
    for (i, (key, v)) in SET_KEYS.iter().zip(vals).enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(key).raw(":").num(v);
    }
    o.raw("}}");
    o.0
}

/// What the files answered, in file order.
#[derive(Default)]
struct Gathered {
    entries: Vec<String>,
    results: Vec<String>,
    provs: Vec<String>,
    warnings: Vec<String>,
    choices: Vec<String>,
    notes: String,
    bad: bool,
}

/// Each file through `one`; a file that cannot be answered keeps its place with an `error` entry, a whole-run failure ends the command.
fn gather(p: &Parsed, a: &IsoAsk) -> std::result::Result<Gathered, Outcome> {
    let mut g = Gathered::default();
    for f in &a.files {
        let sample = p.sample(f).map_err(|m| Outcome::fail(&m))?;
        match one(f, sample, a) {
            Ok((ans, note)) => {
                g.entries.push(ans.entry);
                g.results.push(ans.result);
                g.provs.push(ans.prov);
                g.warnings.extend(ans.warnings);
                g.choices.extend(ans.choices);
                g.notes += &note;
            }
            Err(e) if whole_run(&e) => return Err(crate::sample_select::fail(&e)),
            Err(e) => {
                g.bad = true;
                g.entries.push(error_entry(f, &e, "raw isotopes"));
            }
        }
    }
    Ok(g)
}

/// The `raw-isotopes/1` document of what was gathered.
fn render(p: &Parsed, a: &IsoAsk, command: &[String], mut g: Gathered) -> Outcome {
    if let (Some(ad), true) = (&a.ask.adduct, p.get("--adduct").is_none()) {
        g.choices.push(choice(
            "adduct",
            &jstr(ad),
            "no --adduct was given: the protonated ion, as `ion pattern` takes it",
        ));
    }
    g.choices.push(choice(
        "region",
        "null",
        "the peak the integrator of `quant targets` finds on the trace of M inside --rt (start to end of the peak); the window itself when there is none (flag no_peak_region). The background is the mean of the scans on both sides (as raw xic)",
    ));
    g.choices.push(choice(
        "bins",
        "null",
        "whole masses: round(--mz), +1 and +2, each bin M-0.5 to M+0.5 Da after the offset",
    ));
    let text = format!(
        "{{\"schema\":\"raw-isotopes/1\",\"query\":{},\"files\":[{}],\"results\":[{}],\"warnings\":[{}],{}}}\n",
        params_json(p, a),
        g.entries.join(","),
        g.results.join(","),
        g.warnings.join(","),
        provenance(command, &g.provs, &params_json(p, a), &g.choices)
    );
    let mut o = crate::emit(text, &p.out());
    if o.code == 0 {
        o.stderr = g.notes;
        if g.bad {
            o.code = crate::exit::DATA;
        }
    }
    o
}

pub fn iso_run(args: Vec<String>) -> Outcome {
    let command: Vec<String> = ["mzlab-cli", "raw", "isotopes"]
        .iter()
        .map(|s| s.to_string())
        .chain(args.iter().cloned())
        .collect();
    let valued = [
        "--mz",
        "--rt",
        "--formula",
        "--adduct",
        "--bin",
        "--ms-level",
        "--polarity",
        "--mz-offset",
        "--set",
        "--sample",
        "--out",
    ];
    let p = match extract::parse(args, &valued, &["--dry-run"]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{ISO_HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let a = match iso_ask_of(&p) {
        Ok(a) => a,
        Err(e) => return Outcome::fail_error(&e),
    };
    if let Some(e) = missing(&a.files) {
        return crate::sample_select::fail(&e);
    }
    if p.flag("--dry-run") {
        return plan_of(&p, &a.files, "raw isotopes", &params_json(&p, &a));
    }
    match gather(&p, &a) {
        Ok(g) => render(&p, &a, &command, g),
        Err(o) => o,
    }
}

#[cfg(test)]
#[path = "raw_iso_cmd_tests.rs"]
pub(crate) mod tests;
