//! The evidence of `annot.level@1` beyond the library hits: a standards file of the user (Level 1), the suspects of a `screen/1` (3a from listed fragments found in the MS2),
//! the RT of a query from the MGF, and the Flow parameters that replace the profile thresholds of the same gates. Used by `flow_level` only.

use super::{gate, key, threshold, Assessment, Cand, Evidence};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

/// The Flow parameters of `annot.level@1` that replace thresholds of the profile (`std_*` and the fragments of a suspect).
#[derive(Clone, Debug)]
pub struct Params {
    pub std_rt_max: f64,
    pub std_score_min: f64,
    pub std_ppm_max: f64,
    pub suspect_fragments_min: f64,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            std_rt_max: 0.2,
            std_score_min: 0.7,
            std_ppm_max: 5.0,
            suspect_fragments_min: 2.0,
        }
    }
}

impl Params {
    /// From the resolved parameters of the step, or the object the page sends (the parameters merged with the defaults).
    pub fn from_params(p: &[(String, Value)]) -> Params {
        let d = Params::default();
        let num = |n: &str, v: f64| {
            p.iter()
                .find(|(k, _)| k == n)
                .and_then(|(_, v)| v.f64())
                .filter(|v| v.is_finite())
                .unwrap_or(v)
        };
        Params {
            std_rt_max: num("std_rt_max", d.std_rt_max),
            std_score_min: num("std_score_min", d.std_score_min),
            std_ppm_max: num("std_ppm_max", d.std_ppm_max),
            suspect_fragments_min: num("suspect_fragments_min", d.suspect_fragments_min),
        }
    }

    /// The thresholds of the profile (`soglie` keys) that these parameters replace.
    pub fn overrides(&self) -> [(&'static str, f64); 4] {
        [
            ("std_rt_max", self.std_rt_max),
            ("std_entropia_min", self.std_score_min),
            ("std_ppm_max", self.std_ppm_max),
            ("sospetto_frammenti_min", self.suspect_fragments_min),
        ]
    }
}

/// The best hit of a query on a standard of the user.
#[derive(Clone, Debug, Default)]
pub struct Std {
    pub name: String,
    pub ik: String,
    /// |query RT - standard RT| in minutes; None when the query RT is unknown (the rule did not run).
    pub drt: Option<f64>,
    pub ent: Option<f64>,
    pub cos: Option<f64>,
    pub dprec: Option<f64>,
    /// The listed RT, precursor m/z and adduct of the standard, echoed in the row.
    pub rt: f64,
    pub mz: Option<f64>,
    pub adduct: String,
}

/// A suspect of `screen/1` with the listed fragments found in the MS2 of the feature.
#[derive(Clone, Debug, Default)]
pub struct Suspect {
    pub name: String,
    pub ik: String,
    pub matched: usize,
    pub fragments: Vec<f64>,
}

/// A row of the standards file.
pub struct Standard {
    pub name: String,
    /// First block of the InChIKey, upper case.
    pub key: String,
    pub rt: f64,
    pub mz: Option<f64>,
    pub adduct: String,
}

/// The standards CSV: a header with `name`, `inchikey`, `rt` (min) and optionally `mz` (precursor) and `adduct`; `,`, `;` or tab separated; rows without a key or an RT are skipped.
pub fn read_standards(bytes: &[u8]) -> Result<Vec<Standard>> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text
        .trim_start_matches('\u{feff}')
        .lines()
        .filter(|l| !l.trim().is_empty());
    let head = lines.next().unwrap_or("");
    let sep = [';', '\t', ',']
        .into_iter()
        .find(|c| head.contains(*c))
        .unwrap_or(',');
    let cols: Vec<String> = head
        .split(sep)
        .map(|h| h.trim().trim_matches('"').to_ascii_lowercase())
        .collect();
    let at = |names: &[&str]| cols.iter().position(|c| names.contains(&c.as_str()));
    let need = |names: &[&str], what: &str| {
        at(names).ok_or_else(|| Error::new("err.flow.level.standards").with("column", what))
    };
    let (n, k, r) = (
        need(&["name"], "name")?,
        need(&["inchikey", "inchi_key"], "inchikey")?,
        need(&["rt", "rt_min", "rt (min)"], "rt")?,
    );
    let (m, a) = (
        at(&["mz", "m/z", "precursor_mz", "precursor"]),
        at(&["adduct"]),
    );
    let mut out = Vec::new();
    for line in lines {
        let f: Vec<&str> = line
            .split(sep)
            .map(|c| c.trim().trim_matches('"'))
            .collect();
        let cell = |i: usize| f.get(i).copied().unwrap_or("");
        let key: String = cell(k).chars().take(14).collect::<String>().to_uppercase();
        let Some(rt) = cell(r).parse::<f64>().ok().filter(|v| v.is_finite()) else {
            continue;
        };
        if key.len() != 14 {
            continue;
        }
        out.push(Standard {
            name: cell(n).to_string(),
            key,
            rt,
            mz: m.and_then(|i| cell(i).parse().ok()),
            adduct: a.map(|i| cell(i).to_string()).unwrap_or_default(),
        });
    }
    Ok(out)
}

/// The RT (minutes) of each query id from an MGF: the mean of the scans named by `SPECTRUMID`, `ACCESSION`, `FEATURE_ID` or `TITLE`.
pub fn read_rts(mgf: &[u8]) -> Vec<(String, f64)> {
    let text = String::from_utf8_lossy(mgf);
    let mut acc: Vec<(String, f64, f64)> = Vec::new();
    let (mut ids, mut rt): (Vec<String>, Option<f64>) = (Vec::new(), None);
    for line in text.lines().map(str::trim) {
        if line == "BEGIN IONS" {
            (ids, rt) = (Vec::new(), None);
        } else if line == "END IONS" {
            if let Some(r) = rt {
                for id in ids.drain(..) {
                    match acc.iter_mut().find(|(i, _, _)| *i == id) {
                        Some(a) => (a.1, a.2) = (a.1 + r, a.2 + 1.0),
                        None => acc.push((id, r, 1.0)),
                    }
                }
            }
        } else if let Some((k, v)) = line.split_once('=') {
            let v = v.trim();
            match k.trim().to_ascii_uppercase().as_str() {
                "SPECTRUMID" | "ACCESSION" | "FEATURE_ID" | "TITLE" => ids.push(v.to_string()),
                "RTINSECONDS" => {
                    rt = v
                        .split_whitespace()
                        .next()
                        .and_then(|x| x.parse::<f64>().ok())
                        .map(|s| s / 60.0)
                }
                _ => {}
            }
        }
    }
    acc.into_iter().map(|(i, s, n)| (i, s / n)).collect()
}

/// The standard evidence of a query: its best hit (entropy, else cosine) whose structure is a standard of the user, with the RT distance when the RT is known.
pub fn std_of(cands: &[Cand], standards: &[Standard], rt: Option<f64>) -> Option<Std> {
    let best = |c: &&Cand| c.ent.or(c.cos).unwrap_or(f64::NEG_INFINITY);
    cands
        .iter()
        .filter_map(|c| standards.iter().find(|s| s.key == key(c)).map(|s| (c, s)))
        .max_by(|a, b| best(&a.0).total_cmp(&best(&b.0)))
        .map(|(c, s)| Std {
            name: s.name.clone(),
            ik: c.ik.clone(),
            drt: rt.map(|r| (r - s.rt).abs()),
            ent: c.ent,
            cos: c.cos,
            dprec: c.dprec,
            rt: s.rt,
            mz: s.mz,
            adduct: s.adduct.clone(),
        })
}

/// The suspects of the `screen/1` documents by feature id: the candidate with the most matched fragments of each feature (the first in the file on a tie).
pub fn suspects_of(docs: &[Value]) -> Vec<(String, Suspect)> {
    let mut out: Vec<(String, Suspect)> = Vec::new();
    for doc in docs {
        for h in doc.get("hits").map(Value::arr).unwrap_or(&[]) {
            let Some(id) = h
                .get("feature")
                .and_then(Value::f64)
                .map(|f| format!("{}", f as i64))
            else {
                continue;
            };
            for c in h.get("candidates").map(Value::arr).unwrap_or(&[]) {
                let n = c
                    .get("fragments_matched")
                    .and_then(Value::f64)
                    .unwrap_or(0.0) as usize;
                let s = |k: &str| c.get(k).and_then(Value::str).unwrap_or("").to_string();
                let cand = Suspect {
                    name: s("name"),
                    ik: s("inchikey"),
                    matched: n,
                    fragments: c
                        .get("fragment_mz")
                        .map(Value::arr)
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(Value::f64)
                        .collect(),
                };
                match out.iter_mut().find(|(i, _)| *i == id) {
                    Some((_, o)) if o.matched >= n => {}
                    Some((_, o)) => *o = cand,
                    None => out.push((id.clone(), cand)),
                }
            }
        }
    }
    out
}

/// Level 1: the gates of the profile `standard` on the matched standard; true when all pass. A standard without an RT did not run: its gates show the missing value.
pub(super) fn by_standard(p: &Value, s: &Std, res: &mut Assessment) -> bool {
    let gates = p.get("standard").map(Value::arr).unwrap_or(&[]);
    res.std = Some(s.clone());
    let vals = |f: &str| match f {
        "drt" => s.drt,
        "ent" => s.ent.or(s.cos),
        "ppm" => s.dprec,
        _ => None,
    };
    res.std_gates = gates.iter().map(|g| gate(p, g, &vals)).collect();
    if s.ent.is_none() {
        // the gate names the score it read
        for g in res.std_gates.iter_mut().filter(|g| g.id == "stdscore") {
            g.id = "stdscore_cos".into();
        }
    }
    let pass = !res.std_gates.is_empty() && res.std_gates.iter().all(|g| g.ok);
    if pass {
        res.level = "1".into();
        res.why = "standard";
    }
    pass
}

/// 3a from a suspect with enough listed fragments found in the MS2; never above 3 and only reached when no library hit gave 2a or 3a.
pub(super) fn by_suspect(p: &Value, e: &Evidence, res: &mut Assessment) -> bool {
    let Some(s) = &e.suspect else {
        return false;
    };
    let min = threshold(p, "sospetto_frammenti_min");
    let min = if min.is_nan() { 2.0 } else { min };
    let yes = s.matched > 0 && s.matched as f64 >= min;
    if yes {
        res.level = "3a".into();
        res.why = "suspect_fragments";
        res.suspect = Some(s.clone());
    }
    yes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standards_file_reads_a_bom_and_semicolons_and_refuses_a_missing_column() {
        let list = read_standards("\u{feff}Name;InChIKey;RT;Adduct\r\nA;RYYVLZVUVIJVGH-UHFFFAOYSA-N;1,5;x\r\nB;short;2\r\n".replace("1,5", "1.5").as_bytes()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(
            (list[0].key.as_str(), list[0].rt, list[0].adduct.as_str()),
            ("RYYVLZVUVIJVGH", 1.5, "x")
        );
        assert!(read_standards(b"name,inchikey\nA,RYYVLZVUVIJVGH-UHFFFAOYSA-N\n").is_err());
    }
}
