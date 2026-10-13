//! The MGF of FBMN: MSLEVEL=2 blocks only; of several blocks with the same FEATURE_ID the one with most peaks wins.

use super::delim::{js_number, jstrim, num};
use super::model::{pol_of, Ms2, Table};
use std::collections::HashMap;

pub struct Spectrum {
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    pub prec: Option<f64>,
    pub ionmode: String,
    pub merged: Option<f64>,
    pub ce: String,
}

/// The spectra by feature id (first appearance order), the number of blocks and of those that are not MS2.
pub struct Mgf {
    pub spectra: Vec<(String, Spectrum)>,
    pub blocks: usize,
    pub not_ms2: usize,
}

#[derive(Default)]
struct Block {
    h: Vec<(String, String)>,
    mz: Vec<f64>,
    it: Vec<f64>,
}

impl Block {
    fn get(&self, k: &str) -> Option<&str> {
        self.h.iter().find(|(x, _)| x == k).map(|(_, v)| v.as_str())
    }
}

/// `^[\d.]+[\s\t]`: a peak line that happens to hold an «=».
fn peak_like(line: &str) -> bool {
    let n = line
        .bytes()
        .take_while(|c| c.is_ascii_digit() || *c == b'.')
        .count();
    n > 0 && line[n..].chars().next().is_some_and(char::is_whitespace)
}

fn add_line(cur: &mut Block, line: &str) {
    match line.find('=') {
        Some(eq) if eq > 0 && !peak_like(line) => {
            let k = jstrim(&line[..eq]).to_uppercase();
            let v = jstrim(&line[eq + 1..]).to_string();
            match cur.h.iter_mut().find(|(x, _)| *x == k) {
                Some(e) => e.1 = v,
                None => cur.h.push((k, v)),
            }
        }
        _ => {
            let mut p = line.split(char::is_whitespace).filter(|s| !s.is_empty());
            let (a, b) = (p.next().and_then(js_number), p.next().and_then(js_number));
            if let (Some(a), Some(b)) = (a.filter(|v| v.is_finite()), b.filter(|v| v.is_finite())) {
                cur.mz.push(a);
                cur.it.push(b);
            }
        }
    }
}

pub fn read_mgf(text: &str) -> Mgf {
    let mut out = Mgf {
        spectra: vec![],
        blocks: 0,
        not_ms2: 0,
    };
    let mut at: HashMap<String, usize> = HashMap::new();
    let mut cur: Option<Block> = None;
    for raw in text.split('\n') {
        let line = jstrim(raw);
        if line.is_empty() {
            continue;
        }
        if line == "BEGIN IONS" {
            cur = Some(Block::default());
            continue;
        }
        if line == "END IONS" {
            if let Some(b) = cur.take() {
                out.blocks += 1;
                let id = b.get("FEATURE_ID").or_else(|| b.get("SCANS"));
                if b.get("MSLEVEL").is_some_and(|l| jstrim(l) != "2") {
                    out.not_ms2 += 1;
                } else if let (Some(id), false) = (id, b.mz.is_empty()) {
                    let first =
                        |s: Option<&str>| s.and_then(|s| s.split_whitespace().next()).and_then(num);
                    let s = Spectrum {
                        prec: first(b.get("PEPMASS")),
                        ionmode: b.get("IONMODE").unwrap_or("").into(),
                        merged: b.get("MERGED_ACROSS_N_SAMPLES").and_then(num),
                        ce: b.get("COLLISION_ENERGY").unwrap_or("").into(),
                        mz: b.mz.clone(),
                        it: b.it.clone(),
                    };
                    match at.get(id) {
                        Some(&i) if s.mz.len() > out.spectra[i].1.mz.len() => out.spectra[i].1 = s,
                        Some(_) => {}
                        None => {
                            at.insert(id.into(), out.spectra.len());
                            out.spectra.push((id.into(), s));
                        }
                    }
                }
            }
            continue;
        }
        if let Some(b) = cur.as_mut() {
            add_line(b, line);
        }
    }
    out
}

/// Puts the spectra on the features with the same id; the polarity comes from IONMODE when the table has none.
pub fn attach_ms2(t: &mut Table, mgf: &Mgf) -> usize {
    let mut n = 0;
    for (id, s) in &mgf.spectra {
        let Some(i) = t.feature(id) else { continue };
        let f = &mut t.features[i];
        f.ms2 = Some(Ms2 {
            mz: s.mz.clone(),
            it: s.it.clone(),
            extra: Some((s.prec, s.ce.clone())),
        });
        if let Some(m) = s.merged.filter(|m| *m != 0.0) {
            f.n_scans = Some(f.n_scans.unwrap_or(0.0).max(m));
        }
        if t.pol == 0 && pol_of(&s.ionmode) != 0 {
            t.pol = pol_of(&s.ionmode);
        }
        n += 1;
    }
    t.stat(
        "ms2",
        t.features.iter().filter(|f| f.ms2.is_some()).count() as f64,
    );
    t.stat("mgfBlocks", mgf.blocks as f64);
    t.stat("mgfNotMs2", mgf.not_ms2 as f64);
    n
}
