//! `spectrum get --rt A,B --peak auto [--set NAME=VALUE] [--precursor MZ]`: the apex of the product-ion chromatogram searched between A and B, the window of
//! `factor` x FWHM around it, and the summed spectrum of that window minus the mean of the two sides (the engine's `/api/specpeak`, the route the site and
//! the app answer too). The document is `spectrum/1`; `evidence.peak` is the route's answer without its lines, so every choice is in it.

use mzlab_core::error::{Error, Result};

use crate::extract::{self, Parsed, Sample};
use crate::spectrum_cmd::{query, At};

/// The names `--set` takes here: the window factor, the side width and gap (min) and the thresholds of the apex.
const SET_KEYS: &[&str] = &["factor", "side_min", "gap_min", "min_sn", "min_points"];

/// `--peak auto` of a request: the `--set` pairs, the precursor and the fill time as given.
pub(crate) struct PeakOpt {
    pub set: Vec<(String, String)>,
    pub precursor: Option<String>,
    pub fill_ms: Option<String>,
}

/// `--peak`, `--set`, `--precursor` and `--fill-ms` of a request: `None` without `--peak`; a usage message for a request the apex search cannot answer.
pub(crate) fn options(
    p: &Parsed,
    at: &At,
    (has_blank, has_chan): (bool, bool),
) -> std::result::Result<Option<PeakOpt>, String> {
    let Some(mode) = p.get("--peak") else {
        for f in ["--set", "--precursor"] {
            if p.get(f).is_some() {
                return Err(format!("{f} goes with --peak auto"));
            }
        }
        return Ok(None);
    };
    if mode != "auto" {
        return Err("--peak takes auto".into());
    }
    if !matches!(at, At::Window(..)) {
        return Err("--peak auto searches the apex between two times: give --rt A,B".into());
    }
    if p.flag("--sum") {
        return Err("--peak auto sums the window by itself: drop --sum".into());
    }
    if has_blank {
        return Err("--peak auto takes its background from the sides: not with --minus".into());
    }
    if has_chan {
        return Err("--peak auto is not for --id (it reads the scans of the level)".into());
    }
    p.num("--fill-ms")?;
    p.num("--precursor")?;
    let mut set: Vec<(String, String)> = Vec::new();
    for v in p.all("--set") {
        for item in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (k, val) = item
                .split_once('=')
                .filter(|(k, _)| SET_KEYS.contains(k))
                .ok_or_else(|| {
                    format!(
                        "--set takes NAME=VALUE with a name of {}: {item}",
                        SET_KEYS.join(", ")
                    )
                })?;
            set.retain(|(n, _)| n != k);
            set.push((k.to_string(), val.to_string()));
        }
    }
    Ok(Some(PeakOpt {
        set,
        precursor: p.get("--precursor").map(String::from),
        fill_ms: p.get("--fill-ms").map(String::from),
    }))
}

/// The terminal line that says why no spectrum was made (`no_apex`, `no_data`, `window_only`); none for a spectrum.
fn note(evidence: &str) -> String {
    let v = mzlab_core::json::parse(evidence.as_bytes()).ok();
    let get = |k: &str| {
        v.as_ref()
            .and_then(|v| v.get(k))
            .and_then(|x| x.str().map(String::from))
    };
    match (get("verdict"), get("rule")) {
        (Some(w), Some(r)) if w != "peak" => format!("note: --peak auto: {w}: {r}\n"),
        _ => String::new(),
    }
}

/// The document (or CSV) of the apex search over `win`, and the terminal notes.
pub(crate) fn run(
    s: &mut Sample,
    level: u8,
    win: (f64, f64),
    o: &PeakOpt,
    (csv, level_auto): (bool, Option<&str>),
) -> Result<(String, String)> {
    let mut pairs: Vec<(&str, String)> = vec![
        ("rt0", win.0.to_string()),
        ("rt1", win.1.to_string()),
        ("level", level.to_string()),
    ];
    if let Some(p) = &o.precursor {
        pairs.push(("precursor", p.clone()));
    }
    if let Some(f) = &o.fill_ms {
        pairs.push(("fill_ms", f.clone()));
    }
    for (k, v) in &o.set {
        pairs.push((k.as_str(), v.clone()));
    }
    let q = query(&pairs);
    let text = mzlab_core::rawinfo::handle(&mut s.routes, "/api/specpeak", &q)?
        .ok_or_else(|| Error::new("err.scan.none"))?;
    let bad = || Error::new("error.badIndex");
    // the lines are the last two members of the answer: the evidence is what comes before them
    let cut = text.rfind(",\"mz\":[").ok_or_else(bad)?;
    let evidence = format!("{}}}", &text[..cut]);
    let x = extract::tokens(&text[cut..], "mz").ok_or_else(bad)?;
    let y = extract::tokens(&text[cut..], "y").ok_or_else(bad)?;
    let pts = extract::pairs(&x, &y, None);
    let notes = note(&evidence);
    if csv {
        return Ok((extract::points_csv("mz", "intensity", &pts), notes));
    }
    let head = format!(
        "\"file\":{},\"sample\":{},\"x_unit\":\"m/z\",\"y_unit\":\"counts\",\"ms_level\":{level}",
        extract::jstr(&s.name),
        s.sample.map_or("null".to_string(), |i| i.to_string())
    );
    let ev = format!(
        "\"evidence\":{{\"route\":\"/api/specpeak\",\"query\":{},\"peak\":{evidence}{}}}",
        extract::jstr(&pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")),
        // the level the file's only level gave (`spectrum get` without `--ms-level`)
        level_auto.map_or(String::new(), |a| format!(",\"level_auto\":{a}"))
    );
    Ok((extract::points_doc("spectrum/1", &head, &pts, &ev), notes))
}
