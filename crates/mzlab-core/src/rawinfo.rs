//! `raw info` (`/api/rawinfo`, `docs/studies/RAW_ACCESS_STUDY_2.md` §1): what one sample holds and whether its ions can be counted, read from its
//! `.mzidx` alone (no decode of the source): polarity, RT, scans per MS level, scan types (the scan channels), mass range, and on a few probed scans
//! the m/z grid, the ion quantum (`ionstat::quantum`) and the spectrum-type label against the data (the `spectrum_type` run detector).
//! The CLI (`raw info`, `raw cache`), the desktop app (`handle`) and the site (the lazy `.wasm` `mzlab-raw-wasm`: the engine has no room) all
//! answer through `Routes::raw_info`, so the bytes are one.

use crate::channels::{Famiglia, Source};
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::ionstat::Quantum;
use crate::json::{round, Out};
use crate::routes::{Query, Routes};
use crate::spectrum_type::Runs;

/// Scans probed by default (the `auto` probe of `spectrum_type`), equally spaced over the probed level.
pub const PROBE: usize = 64;
/// Most scans a probe may ask for.
pub const PROBE_MAX: usize = 4096;
/// A spacing is on a grid of step s when its ratio to s is within this of an integer.
pub const GRID_TOL: f64 = 0.02;
/// Share of the spacings that must sit on the grid for it to be accepted.
pub const GRID_SUPPORT: f64 = 0.9;
/// A grid within this of 1 Da is nominal.
pub const NOMINAL_TOL: f64 = 0.02;

/// What the probe of one level saw: the points of each probed scan (ascending m/z) and the intensities.
pub struct Probe {
    pub level: u8,
    /// Scans of the level in the section, and how many of them the file labels profile.
    pub n_level: usize,
    pub n_profile: usize,
    pub scans: Vec<Vec<f64>>,
    pub inten: Vec<f64>,
}

/// `Routes::handle` plus `/api/rawinfo` and `/api/rawxic`, which are outside the engine's dispatch so that the engine `.wasm` does not carry them:
/// the app's routes.
pub fn handle<R: ReadAt>(r: &mut Routes<R>, path: &str, q: &Query) -> Result<Option<String>> {
    if path == "/api/rawinfo" {
        // also for a file with chromatograms only (its scan members are empty)
        return r.raw_info_route(q);
    }
    if path == "/api/rawxic" {
        return r.raw_xic_route(q);
    }
    r.handle(path, q)
}

/// The polarity word of the study: `+`, `-`, `unknown` (a scan with no polarity).
fn pol_word(p: i8) -> &'static str {
    match p {
        1 => "+",
        -1 => "-",
        _ => "unknown",
    }
}

fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    crate::sortc::slice_by(v, f64::total_cmp);
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

impl<R: ReadAt> Routes<R> {
    /// `/api/rawinfo?probe=N[&offset_mz=M[&offset_rt=T]]`: the `raw-info/1` members of this sample as one JSON object.
    pub fn raw_info_route(&mut self, q: &Query) -> Result<Option<String>> {
        let probe = match q.num("probe")? {
            None => PROBE,
            Some(v) if v >= 1.0 && v <= PROBE_MAX as f64 && v.fract() == 0.0 => v as usize,
            Some(_) => return Err(Error::new("err.raw.probe").with("max", PROBE_MAX)),
        };
        let offset = match q.num("offset_mz")? {
            None => None,
            Some(m) if m > 0.0 => Some((m, q.num("offset_rt")?)),
            Some(_) => return Err(Error::new("err.mz.offset.spec").with("text", "offset_mz")),
        };
        Ok(Some(format!("{{{}}}", self.raw_info(probe, offset)?)))
    }

    /// The members of `raw-info/1` for this sample (no braces): `probe` scans, the offset measured on the ion `offset` (m/z, RT) when given.
    pub fn raw_info(&mut self, probe: usize, offset: Option<(f64, Option<f64>)>) -> Result<String> {
        let pol = self.pol;
        let scans: Vec<(u8, f64, i8)> = self
            .idx
            .scans
            .iter()
            .filter(|s| pol.is_none_or(|p| s.polarity == p || (p == 1 && s.polarity == 0)))
            .map(|s| (s.level, s.rt, s.polarity))
            .collect();
        let mut o = Out::default();
        // `+`, `-`, then `unknown` for any other value: a fixed order, no sort (the engine .wasm is near its cap)
        let mut sep = "";
        o.raw("\"polarity\":[");
        for p in [1, -1, 0] {
            if scans.iter().any(|s| pol_word(s.2) == pol_word(p)) {
                o.raw(sep).str(pol_word(p));
                sep = ",";
            }
        }
        let rt = scans.iter().fold(None, |a: Option<(f64, f64)>, s| {
            Some(a.map_or((s.1, s.1), |(lo, hi)| (lo.min(s.1), hi.max(s.1))))
        });
        o.raw("],\"rt\":");
        match rt {
            Some((a, b)) => o.rounded(&[a, b], 4),
            None => o.raw("null"),
        };
        o.raw(",\"n_scans\":").int(scans.len() as i64);
        let mut count = [0usize; 256];
        scans.iter().for_each(|s| count[usize::from(s.0)] += 1);
        let levels: Vec<u8> = (0..=255u8).filter(|l| count[usize::from(*l)] > 0).collect();
        o.raw(",\"ms_levels\":[");
        for (i, l) in levels.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            o.raw("{\"level\":").int(i64::from(*l));
            o.raw(",\"n_scans\":")
                .int(count[usize::from(*l)] as i64)
                .raw("}");
        }
        o.raw("],\"n_chromatograms\":").int(
            self.idx
                .run
                .get("n_chromatograms")
                .and_then(|v| v.f64())
                .unwrap_or(0.0) as i64,
        );
        o.raw(",\"scan_types\":").raw(&self.scan_types());
        o.raw(",\"mass_range\":");
        match self.mass_range(&levels)? {
            Some((a, b)) => o.rounded(&[a, b], 4),
            None => o.raw("null"),
        };
        let level = levels.iter().copied().find(|l| *l >= 1);
        let p = match level {
            Some(l) => Some(self.probe(l, probe)?),
            None => None,
        };
        o.raw(",\"probe\":").raw(&probe_json(p.as_ref(), probe));
        o.raw(",\"grid\":").raw(&grid_json(p.as_ref()));
        o.raw(",\"ion_quantum\":").raw(&quantum_json(p.as_ref()));
        o.raw(",\"mass_offset\":");
        match offset {
            None => o.raw("{\"status\":\"not_measured\",\"how\":\"--offset-ion MZ[@RT]\"}"),
            Some((mz, rt)) => {
                let off = crate::mzcal::estimate(&mut self.idx, pol, mz, rt)?;
                o.raw("{\"status\":\"measured\",\"da\":").num(off.da);
                o.raw(",\"evidence\":").raw(&off.evidence_json()).raw("}")
            }
        };
        o.raw(",\"spectrum_type\":")
            .raw(&spectrum_type_json(p.as_ref()));
        Ok(o.0)
    }

    /// The scan channels (`channels/1` ids with scans): id, code, level, polarity, scans and RT range.
    fn scan_types(&mut self) -> String {
        let c = self.canali();
        let mut o = Out::default();
        o.raw("[");
        let mut first = true;
        for ch in c.v.iter() {
            let keep = matches!(ch.source, Source::Scan { .. })
                && matches!(ch.famiglia, Famiglia::Ms | Famiglia::Dd | Famiglia::Sim);
            if !keep {
                continue;
            }
            o.raw(if first {
                "{\"channel\":"
            } else {
                ",{\"channel\":"
            })
            .str(&ch.id);
            first = false;
            o.raw(",\"code\":");
            match ch.sigla {
                Some(s) => o.str(s),
                None => o.raw("null"),
            };
            o.raw(",\"ms_level\":").int(i64::from(ch.livello));
            o.raw(",\"polarity\":").str(match ch.pol {
                1 => "+",
                -1 => "-",
                _ => "both",
            });
            o.raw(",\"n_scans\":").int(i64::from(ch.n));
            o.raw(",\"rt_min\":").num(round(ch.rt.0, 4));
            o.raw(",\"rt_max\":").num(round(ch.rt.1, 4)).raw("}");
        }
        o.raw("]");
        o.0
    }

    /// The lowest and highest m/z of the peaks of every level (first and last peak of each m/z-sorted section).
    fn mass_range(&mut self, levels: &[u8]) -> Result<Option<(f64, f64)>> {
        let mut r: Option<(f64, f64)> = None;
        for &l in levels {
            let Some(si) = self.idx.section_of(l, self.pol) else {
                continue;
            };
            let n = self.idx.sections[si].n_peaks;
            if n == 0 {
                continue;
            }
            // `mz_at` (the block cache of the XIC search): no new walker in the engine .wasm
            for mz in [self.idx.mz_at(si, 0)?, self.idx.mz_at(si, n - 1)?] {
                r = Some(r.map_or((mz, mz), |(a, b)| (a.min(mz), b.max(mz))));
            }
        }
        Ok(r)
    }

    /// At most `n` scans of `level`, equally spaced over the section (as the `auto` probe of `spectrum_type`): their points and intensities.
    pub fn probe(&mut self, level: u8, n: usize) -> Result<Probe> {
        let mut p = Probe {
            level,
            n_level: 0,
            n_profile: 0,
            scans: Vec::new(),
            inten: Vec::new(),
        };
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(p);
        };
        let n_scans = self.idx.sections[si].n_scans;
        let ids = self.idx.arrays(si)?.scan_ids.clone();
        p.n_level = n_scans;
        p.n_profile = ids
            .iter()
            .filter(|&&k| self.idx.scans.get(k as usize).is_some_and(|s| s.profile))
            .count();
        let count = n.min(n_scans);
        let mut lut = vec![u32::MAX; n_scans];
        for j in 0..count {
            let k = if count > 1 {
                j * (n_scans - 1) / (count - 1)
            } else {
                0
            };
            lut[k] = j as u32;
        }
        let mut scans = vec![Vec::new(); count];
        let inten = &mut p.inten;
        self.idx.each_peak_for(si, &lut, |j, mz, y| {
            scans[j].push(mz);
            inten.push(y);
        })?;
        // a scan with no points is not looked at (the probe of `spectrum_type` skips them too)
        p.scans = scans.into_iter().filter(|s| !s.is_empty()).collect();
        Ok(p)
    }
}

/// `probe`: the level probed, the scans asked and the scans read (those with points).
fn probe_json(p: Option<&Probe>, asked: usize) -> String {
    let mut o = Out::default();
    o.raw("{\"ms_level\":");
    match p {
        Some(p) => o.int(i64::from(p.level)),
        None => o.raw("null"),
    };
    o.raw(",\"asked\":").int(asked as i64);
    o.raw(",\"scans\":")
        .int(p.map_or(0, |p| p.scans.len()) as i64)
        .raw("}");
    o.0
}

/// Grid steps tried, largest first (Da): a spacing on 0.1 is also on 0.05, so the first that holds is the grid.
pub const GRID_STEPS: [f64; 8] = [1.0, 0.5, 0.25, 0.2, 0.1, 0.05, 0.02, 0.01];

/// The share of the spacings `d` that are whole multiples of `step` (±`GRID_TOL`); one copy, not one per step.
#[inline(never)]
fn share(d: &[f64], step: f64) -> f64 {
    let on = d
        .iter()
        .filter(|&&x| (x / step - (x / step).round()).abs() <= GRID_TOL)
        .count();
    on as f64 / d.len() as f64
}

/// `grid`: the step of the m/z values inside scans and their phase on it. The step is the first of `GRID_STEPS` of which `GRID_SUPPORT` of the
/// spacings are integer multiples (sparse counts many Da apart still sit on 1 Da); otherwise the grid is `continuous` with the median spacing.
pub fn grid_json(p: Option<&Probe>) -> String {
    let mut d: Vec<f64> = p
        .map(|p| {
            p.scans
                .iter()
                .flat_map(|s| s.windows(2).map(|w| w[1] - w[0]))
                .filter(|d| *d > 0.0)
                .collect()
        })
        .unwrap_or_default();
    let mut o = Out::default();
    if d.len() < 3 {
        o.raw("{\"step_da\":null,\"phase_da\":null,\"kind\":\"unknown\",\"support\":null,\"n_spacings\":");
        o.int(d.len() as i64).raw("}");
        return o.0;
    }
    let shares = GRID_STEPS.map(|s| share(&d, s));
    let found = (0..GRID_STEPS.len())
        .find(|&i| shares[i] >= GRID_SUPPORT)
        .map(|i| (GRID_STEPS[i], shares[i]));
    let support = found.map_or_else(|| shares.iter().copied().fold(0.0, f64::max), |f| f.1);
    if let Some((cand, _)) = found {
        let mut f: Vec<f64> = p
            .map(|p| {
                p.scans
                    .iter()
                    .flatten()
                    .map(|m| m - cand * (m / cand).round())
                    .collect()
            })
            .unwrap_or_default();
        // |v| <= cand / 2, so the Euclidean remainder is one add (no `fmod` in the .wasm)
        let phase = median(&mut f).map_or(f64::NAN, |v| if v < 0.0 { v + cand } else { v });
        let kind = if (cand - 1.0).abs() <= NOMINAL_TOL {
            "nominal"
        } else {
            "stepped"
        };
        o.raw("{\"step_da\":").num(round(cand, 4));
        o.raw(",\"phase_da\":").num(round(phase, 4));
        o.raw(",\"kind\":").str(kind);
    } else {
        let step = median(&mut d).unwrap_or(f64::NAN);
        o.raw("{\"step_da\":").num(round(step, 4));
        o.raw(",\"phase_da\":null,\"kind\":\"continuous\"");
    }
    o.raw(",\"support\":").num(round(support, 4));
    o.raw(",\"n_spacings\":").int(d.len() as i64).raw("}");
    o.0
}

/// The quantum of the probed intensities (`ionstat::quantum`), `None` without positive values.
pub fn quantum_of(p: Option<&Probe>) -> Option<Quantum> {
    p.and_then(|p| crate::ionstat::quantum(&p.inten))
}

/// `ion_quantum`: cps per ion when the quantum is accepted (else `null`, with the candidate and its evidence).
pub fn quantum_json(p: Option<&Probe>) -> String {
    let mut o = Out::default();
    let Some(q) = quantum_of(p) else {
        return "{\"cps\":null,\"candidate\":null,\"support\":null,\"n_positive\":0,\"median_ions\":null,\"method\":\"min_positive_or_step\",\"status\":\"none\"}".into();
    };
    o.raw("{\"cps\":");
    if q.accepted {
        o.num(q.q);
    } else {
        o.raw("null");
    }
    o.raw(",\"candidate\":").num(q.q);
    o.raw(",\"support\":").num(round(q.support, 4));
    o.raw(",\"n_positive\":").int(q.n_positive as i64);
    o.raw(",\"median_ions\":").num(round(q.level, 4));
    o.raw(",\"method\":\"min_positive_or_step\",\"status\":");
    o.str(if q.accepted { "ok" } else { "continuous" }).raw("}");
    o.0
}

/// The label of the probed level (`profile`, `centroid`, `mixed`) and what the run detector reads in the probed points.
pub fn spectrum_type(p: Option<&Probe>) -> (&'static str, &'static str, Runs) {
    let mut runs = Runs::default();
    let Some(p) = p else {
        return ("unknown", "unknown", runs);
    };
    p.scans.iter().for_each(|s| runs.add(s));
    let label = match p.n_profile {
        0 if p.n_level == 0 => "unknown",
        0 => "centroid",
        n if n == p.n_level => "profile",
        _ => "mixed",
    };
    let data = match (label, runs.runs_share()) {
        (_, None) | ("unknown", _) => "unknown",
        ("centroid", _) if runs.continuous() => "profile",
        ("centroid", _) => "centroid",
        _ if runs.sparse() => "centroid",
        _ => "profile",
    };
    (label, data, runs)
}

/// `spectrum_type`: the label, the data and whether they agree (`null` when either is unknown), with the shares behind the decision.
pub fn spectrum_type_json(p: Option<&Probe>) -> String {
    let (label, data, runs) = spectrum_type(p);
    let mut o = Out::default();
    o.raw("{\"label\":").str(label).raw(",\"data\":").str(data);
    o.raw(",\"agree\":").raw(match (label, data) {
        ("unknown", _) | (_, "unknown") => "null",
        (l, d) if l == d => "true",
        _ => "false",
    });
    o.raw(",\"runs_share\":")
        .opt(runs.runs_share().map(|v| round(v, 4)));
    o.raw(",\"fine_share\":")
        .opt(runs.fine_share().map(|v| round(v, 4)));
    o.raw(",\"profile_scans\":")
        .int(p.map_or(0, |p| p.n_profile) as i64);
    o.raw(",\"level_scans\":")
        .int(p.map_or(0, |p| p.n_level) as i64)
        .raw("}");
    o.0
}

#[cfg(test)]
#[path = "rawinfo_tests.rs"]
mod tests;
