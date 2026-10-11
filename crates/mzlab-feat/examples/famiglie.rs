//! Families of the peaks of one or more `.mzidx` files: count, size, roles and time; with the truth of a synthetic series, the check.
//!
//!     cargo run --release -p mzlab-feat --example famiglie -- <file.mzidx>... [--neg] [--truth verita.json]
//!
//! `--truth` reads `verita.json` of `tools/valida/sintetici_identita.py`: for every base ion (parent, products, isomers) the members it has in
//! the file are looked for and the family, the roles and the open readings are compared with the truth.

use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat::family::{analyse_families, FamParams, Families};
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::table::*;
use mzlab_feat::trace::Params;
use serde_json::Value;

/// Row within 5 ppm of `mz` and 0.1 min of `rt` (the nearest in m/z).
fn find(f: &FileFeatures, mz: f64, rt: f64) -> Option<usize> {
    (0..f.len())
        .filter(|&i| (f.mz[i] - mz).abs() <= mz * 5e-6 && (f.rt[i] as f64 - rt).abs() <= 0.1)
        .min_by(|&a, &b| (f.mz[a] - mz).abs().total_cmp(&(f.mz[b] - mz).abs()))
}

#[derive(Default)]
struct Score {
    bases: usize,
    right: usize,
    foreign: usize,
    /// (right, total) per truth role.
    by_role: Vec<(&'static str, usize, usize)>,
    nh4_open: (usize, usize),
    wrong: Vec<String>,
}

impl Score {
    fn count(&mut self, role: &'static str, ok: bool) {
        match self.by_role.iter_mut().find(|r| r.0 == role) {
            Some(r) => {
                r.1 += ok as usize;
                r.2 += 1;
            }
            None => self.by_role.push((role, ok as usize, 1)),
        }
    }
}

/// Expected role of a truth role.
fn expected(role: &str) -> Option<u8> {
    Some(match role {
        "isotopo" => ROLE_ISOTOPE,
        "addotto" => ROLE_ADDUCT,
        "isf" => ROLE_ISF,
        "isf_finto" => ROLE_UNDETERMINED,
        _ => return None,
    })
}

fn row_of(r: &Value, f: &FileFeatures) -> Option<usize> {
    find(
        f,
        r["mz"].as_f64()?,
        *r["rt_obs"].get(0)?.as_f64().as_ref()?,
    )
}

fn judge(f: &FileFeatures, truth: &Value) -> Score {
    let ions = truth["ioni"].as_array().expect("ioni");
    let mut s = Score::default();
    for b in ions.iter().filter(|r| {
        matches!(
            r["ruolo"].as_str(),
            Some("progenitore" | "tp1" | "tp2" | "isomero")
        )
    }) {
        let Some(bi) = row_of(b, f) else { continue };
        s.bases += 1;
        let fam = f.family_id[bi];
        let mut ok = f.role[bi] == ROLE_M;
        let mut members = vec![bi];
        for d in ions.iter().filter(|r| r["progenitore"] == b["id"]) {
            let (Some(role), Some(di)) = (d["ruolo"].as_str().and_then(expected), row_of(d, f))
            else {
                continue;
            };
            let name = d["ruolo"].as_str().unwrap_or("");
            let nh4 = name == "addotto" && d["id"].as_str().is_some_and(|x| x.ends_with("NH4"));
            // +17.0265 is [M+NH4]+ or the loss of NH3: both readings stay open until a proof separates them
            let open = nh4 && f.role[di] == ROLE_UNDETERMINED && f.alt[di] & ALT_NH3 != 0;
            let good = f.family_id[di] == fam && (f.role[di] == role || open);
            if name == "isf_finto" {
                s.count("isf_finto not declared ISF", f.role[di] != ROLE_ISF);
                s.count("isf_finto labelled undetermined", good);
            } else {
                s.count(
                    if name == "isotopo" {
                        "isotopo"
                    } else if name == "addotto" {
                        "addotto"
                    } else {
                        "isf"
                    },
                    good,
                );
            }
            if nh4 {
                s.nh4_open.1 += 1;
                s.nh4_open.0 += open as usize;
            }
            if !good {
                s.wrong.push(format!(
                    "{} ({name}): family {} role {}",
                    d["id"].as_str().unwrap_or(""),
                    f.family_id[di] == fam,
                    f.role[di]
                ));
            }
            ok &= good || (name == "isf_finto" && f.role[di] == ROLE_M);
            members.push(di);
        }
        let extra = (0..f.len())
            .filter(|&i| {
                f.family_id[i] == fam && f.role[i] != ROLE_ISOTOPE && !members.contains(&i)
            })
            .count();
        s.foreign += extra;
        if extra > 0 && s.wrong.len() < 40 {
            let others: Vec<String> = (0..f.len())
                .filter(|&i| {
                    f.family_id[i] == fam && f.role[i] != ROLE_ISOTOPE && !members.contains(&i)
                })
                .map(|i| format!("{:.4}@{:.2} role {}", f.mz[i], f.rt[i], f.role[i]))
                .collect();
            s.wrong.push(format!(
                "{} family has also {}",
                b["id"].as_str().unwrap_or(""),
                others.join(", ")
            ));
        }
        s.right += ok as usize;
    }
    s
}

fn summary(name: &str, f: &FileFeatures, fam: &Families, secs: f64) {
    let n = |r: u8| f.role.iter().filter(|&&x| x == r).count();
    let mut size = vec![0usize; f.len() + 1];
    for &id in f.family_id.iter().filter(|&&i| i > 0) {
        size[id as usize] += 1;
    }
    let multi = size.iter().filter(|&&s| s > 1).count();
    let biggest = size.iter().max().copied().unwrap_or(0);
    println!(
        "{name}: {} features, {} families ({} with more than one member, largest {biggest}); roles M {} isotope {} adduct {} isf {} dimer {} undetermined {}; {} arcs; {:.2} s",
        f.len(), fam.count(), multi, n(ROLE_M), n(ROLE_ISOTOPE), n(ROLE_ADDUCT), n(ROLE_ISF), n(ROLE_DIMER), n(ROLE_UNDETERMINED), fam.arcs.len(), secs
    );
}

fn report(s: &Score) {
    let pct = |a: usize, b: usize| {
        if b == 0 {
            100.0
        } else {
            100.0 * a as f64 / b as f64
        }
    };
    println!("  truth: {} base ions found, {} with their family right ({:.1} %), {} foreign rows in their families", s.bases, s.right, pct(s.right, s.bases), s.foreign);
    for (role, ok, n) in &s.by_role {
        println!("    {role}: {ok}/{n}");
    }
    println!(
        "    NH4 adducts left with both readings open: {}/{}",
        s.nh4_open.0, s.nh4_open.1
    );
    for w in s.wrong.iter().take(10) {
        println!("    wrong: {w}");
    }
}

fn main() {
    let (mut p, mut files, mut truth) = (Params::default(), Vec::new(), None::<Value>);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--neg" => p.polarity = Some(-1),
            "--truth" => {
                truth = args.next().map(|t| {
                    serde_json::from_slice(&std::fs::read(t).expect("truth")).expect("json")
                })
            }
            _ => files.push(a),
        }
    }
    for path in &files {
        let fh = std::fs::File::open(path).expect("open");
        let size = fh.metadata().expect("size").len();
        let mut ix = Index::open(fh, size).unwrap_or_else(|e| panic!("{path}: {e}"));
        let (mut f, links) =
            analyse(&mut ix, &p, &Steps::default()).unwrap_or_else(|e| panic!("{path}: {e}"));
        let t0 = Instant::now();
        let fam = analyse_families(&mut ix, &mut f, &links, &p, &FamParams::default())
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        summary(path, &f, &fam, t0.elapsed().as_secs_f64());
        if let Some(t) = &truth {
            report(&judge(&f, t));
        }
    }
}
