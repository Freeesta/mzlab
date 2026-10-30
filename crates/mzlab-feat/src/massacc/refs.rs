//! The reference ions of the check: the background list of the lab (`contaminanti.json`, Keller 2008, expanded into ions) and the CSV of the user (name, ion, RT, role).
//! The m/z of a listed ion with an adduct of the engine table comes from the engine (`elements::ion_mz`), the listed m/z only where the table has no such adduct.

use mzlab_core::elements::{ion_mz, ion_text_mz, mass, parse_formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

/// What a reference ion is checked for: its mass error only (`Reference`), its mass error and its retention (`Istd` in the files the samples mark with `istd`, `Standard` in all).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Reference,
    Istd,
    Standard,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Reference => "reference",
            Role::Istd => "istd",
            Role::Standard => "standard",
        }
    }
}

/// One reference ion.
#[derive(Clone, Debug, PartialEq)]
pub struct RefIon {
    pub name: String,
    /// Theoretical m/z.
    pub mz: f64,
    /// 1, -1, or 0 when the list does not say (a bare m/z): the polarity of the file.
    pub pol: i8,
    /// RT declared by the user (minutes).
    pub rt: Option<f64>,
    /// Where to look for it, minutes.
    pub window: Option<(f64, f64)>,
    pub role: Role,
    /// Of the lab list (always on), not of the user's file.
    pub background: bool,
}

fn bad(line: usize, message: &str) -> Error {
    Error::new("err.flow.massacc.references")
        .with("line", line)
        .with("message", message)
}

/// The ions of the lab list (`{"lists":[{"items":[{"name_en","adduct","formula","z","mz","pol"}]}]}`), in the order of the file.
pub fn background(json: &[u8]) -> Result<Vec<RefIon>> {
    let doc = parse(json)?;
    let mut out = Vec::new();
    for list in doc.get("lists").map_or(&[][..], Value::arr) {
        for it in list.get("items").map_or(&[][..], Value::arr) {
            let text = |k: &str| it.get(k).and_then(Value::str).unwrap_or("");
            let (adduct, listed) = (text("adduct"), it.get("mz").and_then(Value::f64));
            let pol = it.get("pol").and_then(Value::f64).unwrap_or(0.0) as i8;
            let single = it.get("z").and_then(Value::f64) == Some(1.0);
            let exact = single
                .then(|| parse_formula(text("formula")).ok())
                .flatten()
                .and_then(|f| ion_mz(mass(&f), adduct));
            let Some(mz) = exact.or(listed).filter(|m| *m > 0.0) else {
                continue;
            };
            let name = if text("name_en").is_empty() {
                text("name")
            } else {
                text("name_en")
            };
            out.push(RefIon {
                name: format!("{name} {adduct}"),
                mz,
                pol,
                rt: None,
                window: None,
                role: Role::Reference,
                background: true,
            });
        }
    }
    Ok(out)
}

/// The fields of one line (a delimiter outside quotes; `""` inside quotes is a quote).
fn fields(line: &str, d: char) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if quoted && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' if quoted || cur.is_empty() => quoted = !quoted,
            c if c == d && !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|f| f.trim().to_string()).collect()
}

/// The ions of the CSV of the user. Header (any order, any case): `name`, `ion` (an m/z, an ion formula with its charge sign, or a formula with its adduct), and optionally `rt`
/// (declared, minutes), `rt_min`, `rt_max` (the window), `role` (`reference`, `istd`, `standard`). A declared RT with no window looks half a minute either side.
pub fn from_csv(text: &str) -> Result<Vec<RefIon>> {
    let text = text.trim_start_matches('\u{feff}');
    let mut lines = text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty());
    let Some((_, head)) = lines.next() else {
        return Ok(Vec::new());
    };
    let d = [',', ';', '\t']
        .into_iter()
        .max_by_key(|&c| head.matches(c).count())
        .unwrap_or(',');
    let names: Vec<String> = fields(head, d).iter().map(|h| h.to_lowercase()).collect();
    let col = |keys: &[&str]| names.iter().position(|n| keys.contains(&n.as_str()));
    let (c_name, c_ion) = (col(&["name"]), col(&["ion", "mz", "m/z", "ion_formula"]));
    let (Some(c_name), Some(c_ion)) = (c_name, c_ion) else {
        return Err(bad(1, "the header needs the columns name and ion"));
    };
    let (c_rt, c_lo, c_hi, c_role) = (
        col(&["rt"]),
        col(&["rt_min"]),
        col(&["rt_max"]),
        col(&["role"]),
    );
    let mut out = Vec::new();
    for (i, line) in lines {
        let f = fields(line, d);
        let cell = |c: Option<usize>| c.and_then(|c| f.get(c)).map(String::as_str).unwrap_or("");
        let num = |c: Option<usize>| -> Result<Option<f64>> {
            let t = cell(c);
            if t.is_empty() {
                return Ok(None);
            }
            t.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .map(Some)
                .ok_or_else(|| bad(i + 1, "a number was expected"))
        };
        let name = cell(Some(c_name));
        if name.is_empty() {
            return Err(bad(i + 1, "the name is empty"));
        }
        let ion = cell(Some(c_ion));
        let (mz, pol) = match ion.parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 => (v, 0),
            Ok(_) => return Err(bad(i + 1, "the m/z must be positive")),
            Err(_) => ion_text_mz(ion)
                .map_err(|_| bad(i + 1, "the ion needs its charge sign or its adduct"))?,
        };
        let (rt, lo, hi) = (num(c_rt)?, num(c_lo)?, num(c_hi)?);
        let window = match (lo, hi, rt) {
            (Some(a), Some(b), _) if a <= b => Some((a, b)),
            (Some(_), Some(_), _) => return Err(bad(i + 1, "rt_min is above rt_max")),
            (None, None, Some(r)) => Some((r - 0.5, r + 0.5)),
            (None, None, None) => None,
            _ => return Err(bad(i + 1, "rt_min and rt_max go together")),
        };
        let role = match cell(c_role).to_lowercase().as_str() {
            "" | "reference" => Role::Reference,
            "istd" => Role::Istd,
            "standard" => Role::Standard,
            _ => return Err(bad(i + 1, "the role is reference, istd or standard")),
        };
        let rt = rt.or_else(|| window.map(|(a, b)| (a + b) / 2.0));
        out.push(RefIon {
            name: name.to_string(),
            mz,
            pol,
            rt,
            window,
            role,
            background: false,
        });
    }
    Ok(out)
}

/// The references to search, sorted by m/z: the same ion once (the user's before the list's), and a listed ion with another ion inside the search window left out (a trace could not be
/// told to belong to one of them).
pub fn prepare(mut refs: Vec<RefIon>, search_ppm: f64) -> Vec<RefIon> {
    // user entries first so that they win a tie; the sort is stable
    crate::sortc::any_by(&mut refs, |a, b| {
        a.mz.total_cmp(&b.mz).then(a.background.cmp(&b.background))
    });
    let within = |a: f64, b: f64, ppm: f64| (a - b).abs() <= a.max(b) * ppm * 1e-6;
    let same_sign = |a: &RefIon, b: &RefIon| a.pol == 0 || b.pol == 0 || a.pol == b.pol;
    let mut uniq: Vec<RefIon> = Vec::with_capacity(refs.len());
    for r in refs {
        let twin = uniq
            .iter()
            .rev()
            .take_while(|k| within(k.mz, r.mz, SAME_ION_PPM))
            .any(|k| same_sign(k, &r));
        if !twin {
            uniq.push(r);
        }
    }
    let crowded = |i: usize| {
        let r = &uniq[i];
        let near = |j: usize| within(uniq[j].mz, r.mz, search_ppm) && same_sign(&uniq[j], r);
        (0..i)
            .rev()
            .take_while(|&j| within(uniq[j].mz, r.mz, search_ppm))
            .any(near)
            || (i + 1..uniq.len())
                .take_while(|&j| within(uniq[j].mz, r.mz, search_ppm))
                .any(near)
    };
    let drop: Vec<bool> = (0..uniq.len())
        .map(|i| uniq[i].background && crowded(i))
        .collect();
    uniq.into_iter()
        .zip(drop)
        .filter(|(_, d)| !d)
        .map(|(r, _)| r)
        .collect()
}

/// Two entries this close (ppm) are the same ion.
const SAME_ION_PPM: f64 = 0.1;

/// The references of a run: the ions of the user's CSV (if any), then the lab list.
pub fn references(contaminants: &[u8], csv: Option<&[u8]>) -> Result<Vec<RefIon>> {
    let mut out = match csv {
        Some(b) => from_csv(&String::from_utf8_lossy(b))?,
        None => Vec::new(),
    };
    out.extend(background(contaminants)?);
    Ok(out)
}
