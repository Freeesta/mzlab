//! `spectrum get --minus FILE [--minus-sample ID] [--minus-rt A,B]`: the mean spectrum of a window minus the mean spectrum of a blank (another
//! file, or another sample of the same file). It is the engine's own `specdiff::spectrum_diff` (the page's `/api/spectrum` with `bgk`), so the lines
//! are the ones the site and the app draw; the choices made here (the blank's window, the factor, the clip) are written in the evidence.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};

use crate::extract::{self, Sample};
use crate::spectrum_cmd::{scan_index, At};

/// The blank: its opened sample and the window the user gave (`None`: the window of the main spectrum, or the blank's scan nearest to it).
pub(crate) struct Minus {
    pub sample: Sample,
    pub window: Option<(f64, f64)>,
}

/// The factor of the blank and the tolerance of the engine's request (`bgf`, `bgtol` defaults), named in the evidence.
const FACTOR: f64 = 1.0;
const TOL_PPM: f64 = 5.0;

/// Opens the blank under `tmp` (its own index, next to the main one).
pub(crate) fn open(
    file: &Path,
    sample: Option<u32>,
    window: Option<(f64, f64)>,
    tmp: &Path,
) -> Result<Minus> {
    let dir: PathBuf = tmp.join("minus");
    std::fs::create_dir_all(&dir).map_err(|e| Error::new("err.file.index").with("detail", e))?;
    Ok(Minus {
        sample: extract::open_sample(file, sample, &dir)?,
        window,
    })
}

/// The answer of `spectrum_diff` for the main window `win` (`single`: one scan, so the blank's scan nearest to it is the blank), and the `minus`
/// evidence member (no braces).
pub(crate) fn answer(
    s: &mut Sample,
    m: &mut Minus,
    base: &[(&str, String)],
    win: (f64, f64),
    single: bool,
    level: u8,
) -> Result<(String, String)> {
    let mut bg_scan = "null".to_string();
    let bg = match (m.window, single) {
        (Some(w), _) => w,
        (None, false) => win,
        (None, true) => {
            let i = scan_index(&mut m.sample, &At::Nearest(win.0), level, None)?
                .ok_or_else(|| Error::new("err.scan.none"))?;
            let sc = &m.sample.routes.idx.scans[i];
            bg_scan = format!(
                "{{\"scan_no\":{},\"rt_min\":{}}}",
                sc.number(i),
                extract::jnum(sc.rt)
            );
            (sc.rt, sc.rt)
        }
    };
    let mut q = base.to_vec();
    q.extend([
        ("bgk", "1".to_string()),
        ("bgrt0", bg.0.to_string()),
        ("bgrt1", bg.1.to_string()),
    ]);
    let text: String = q
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    let query = mzlab_core::routes::Query::parse(&text);
    let req = s
        .routes
        .diff_request(&query)?
        .ok_or_else(|| Error::new("err.spectrum.minus"))?;
    let out = mzlab_core::specdiff::spectrum_diff(&mut s.routes, Some(&mut m.sample.routes), &req)?
        .ok_or_else(|| Error::new("err.spectrum.minus"))?;
    let ev = format!(
        "\"minus\":{{\"file\":{},\"sample\":{},\"window_min\":[{},{}],\"scan\":{bg_scan},\"factor\":{},\"clip\":true,\"tol_ppm\":{}}}",
        extract::jstr(&m.sample.name),
        m.sample.sample.map_or("null".to_string(), |i| i.to_string()),
        extract::jnum(bg.0),
        extract::jnum(bg.1),
        extract::jnum(FACTOR),
        extract::jnum(TOL_PPM),
    );
    Ok((out, ev))
}
