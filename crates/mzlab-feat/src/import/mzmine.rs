//! mzMine: the full table (`spectral_db_matches:*`, `datafile:<F>:area`, ion identities), its annotations file, and the FBMN quant table with the IIN edges.

use super::chem::{check_structure, clean_name, norm_adduct, ppm_of};
use super::delim::{cell, each_row, num, txt, Cols};
use super::model::{Edge, Iin, Rec, Table, V};
use super::ImportError;

fn ends_ci(s: &str, suf: &str) -> bool {
    s.len() >= suf.len()
        && s.is_char_boundary(s.len() - suf.len())
        && s[s.len() - suf.len()..].eq_ignore_ascii_case(suf)
}

fn line_end(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `^datafile:(.+):(area|feature_state)$` (case-insensitive): the sample and whether the column is the area.
fn datafile(c: &str) -> Option<(String, bool)> {
    let (suf, area) = if ends_ci(c, ":area") {
        (5, true)
    } else if ends_ci(c, ":feature_state") {
        (14, false)
    } else {
        return None;
    };
    let head = &c[..c.len() - suf];
    if !head.is_char_boundary(9)
        || !head[..9].eq_ignore_ascii_case("datafile:")
        || head.len() == 9
        || head[9..].contains(line_end)
    {
        return None;
    }
    Some((strip_mzml(&head[9..]), area))
}

/// A sample name without a trailing «.mzML» (any case).
pub fn strip_mzml(s: &str) -> String {
    if ends_ci(s, ".mzml") {
        s[..s.len() - 5].into()
    } else {
        s.into()
    }
}

struct Full {
    id: Option<usize>,
    mz: Option<usize>,
    rt: Option<usize>,
    scans: Option<usize>,
    grp: Option<usize>,
    iin: Option<usize>,
    ion: Option<usize>,
    nm: Option<usize>,
    m: [Option<usize>; 12],
    samples: Vec<(String, usize)>,
    states: Vec<(String, usize)>,
}

const MATCH: [&str; 12] = [
    "compound_name",
    "similarity_score",
    "n_matching_signals",
    "explained_intensity_percent",
    "ion_adduct",
    "mol_formula",
    "smiles",
    "inchi",
    "precursor_mz",
    "mz_diff_ppm",
    "mz_diff",
    "rt_absolute_error",
];

fn full_columns(h: &[String]) -> Result<Full, ImportError> {
    let c = Cols::new(h);
    let mut m = [None; 12];
    for (i, k) in MATCH.iter().enumerate() {
        m[i] = c.pick(&[&format!("spectral_db_matches:{k}")]);
    }
    let mut x = Full {
        id: c.pick(&["id", "row id"]),
        mz: c.pick(&["mz", "row m/z"]),
        rt: c.pick(&["rt", "row retention time"]),
        scans: c.pick(&["fragment_scans"]),
        grp: c.pick(&["feature_group"]),
        iin: c.pick(&["ion_identities:iin_id"]),
        ion: c.pick(&["ion_identities:ion_identities"]),
        nm: c.pick(&["ion_identities:neutral_mass"]),
        m,
        samples: vec![],
        states: vec![],
    };
    for (i, col) in h.iter().enumerate() {
        let Some((sn, area)) = datafile(col) else {
            continue;
        };
        if area {
            x.samples.push((sn, i));
        } else if let Some(e) = x.states.iter_mut().find(|(s, _)| *s == sn) {
            e.1 = i;
        } else {
            x.states.push((sn, i));
        }
    }
    if x.id.is_none() || x.mz.is_none() || x.rt.is_none() {
        return Err(ImportError::new(
            "columns",
            "mzMine table: columns id, mz and rt not found",
        ));
    }
    Ok(x)
}

fn library_match(x: &Full, r: &[String], mz: f64) -> Option<Rec> {
    let nm = txt(cell(r, x.m[0]));
    if nm.is_empty() {
        return None;
    }
    let m = |i: usize| x.m[i].map(|_| num(cell(r, x.m[i]))).unwrap_or(None);
    let s = |i: usize| {
        if x.m[i].is_some() {
            txt(cell(r, x.m[i]))
        } else {
            ""
        }
    };
    let prec = m(8);
    let ppm = m(9).or_else(|| prec.and_then(|p| ppm_of(mz, p)));
    let mut a = Rec::default();
    for (k, v) in [
        ("source", V::s("mzMine library")),
        ("type", V::s("cosine")),
        ("score", V::opt(m(1))),
        ("shared", V::opt(m(2))),
        ("explained", V::opt(m(3))),
        ("ppm", V::opt(ppm)),
        ("name", V::s(nm)),
        ("cleanName", V::Str(clean_name(nm))),
        (
            "adduct",
            V::Str(if x.m[4].is_some() {
                norm_adduct(cell(r, x.m[4]))
            } else {
                String::new()
            }),
        ),
        ("formula", V::s(s(5))),
        ("smiles", V::s(s(6))),
        ("inchi", V::s(s(7))),
        ("ik", V::s("")),
        ("precursor", V::opt(prec)),
        ("analog", V::Bool(false)),
        ("propagated", V::Bool(false)),
        ("notes", V::List(vec![])),
    ] {
        a.set(k, v);
    }
    Some(a)
}

fn full_row(t: &mut Table, x: &Full, r: &[String]) {
    let (id, mz, rt) = (txt(cell(r, x.id)), num(cell(r, x.mz)), num(cell(r, x.rt)));
    let (Some(mz), Some(rt)) = (mz, rt) else {
        return;
    };
    if id.is_empty() {
        return;
    }
    let ann = library_match(x, r, mz);
    let f = t.add_feature(id, mz, rt);
    for (s, i) in &x.samples {
        if let Some(a) = num(cell(r, Some(*i))).filter(|a| *a != 0.0) {
            f.set_area(s, a);
        }
    }
    if !x.states.is_empty() {
        f.detected = Some(
            x.states
                .iter()
                .filter(|(_, i)| txt(cell(r, Some(*i))).eq_ignore_ascii_case("detected"))
                .map(|(s, _)| s.clone())
                .collect(),
        );
    }
    let sc = txt(cell(r, x.scans));
    if x.scans.is_some() && !sc.is_empty() {
        f.n_scans = Some(
            sc.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                .filter(|p| !p.is_empty())
                .count() as f64,
        );
    }
    if x.grp.is_some() && !txt(cell(r, x.grp)).is_empty() {
        f.group = Some(txt(cell(r, x.grp)).into());
    }
    if x.iin.is_some() && !txt(cell(r, x.iin)).is_empty() {
        let neutral = x.nm.and_then(|_| num(cell(r, x.nm)));
        f.iin = Some(Iin {
            id: txt(cell(r, x.iin)).into(),
            ions: if x.ion.is_some() {
                txt(cell(r, x.ion)).into()
            } else {
                String::new()
            },
            neutral,
        });
    }
    f.ann.extend(ann);
}

/// The full table of mzMine; `max_features` > 0 stops after that many features.
pub fn read_full(
    text: &str,
    name: &str,
    pol: i32,
    max_features: usize,
) -> Result<Table, ImportError> {
    let mut t = Table::new("mzmine", name, pol);
    let state = each_row(text, b',', None, full_columns, |x, r, _| {
        if max_features == 0 || t.features.len() < max_features {
            full_row(&mut t, x, r);
        }
        Ok(())
    })?;
    let Some(x) = state else {
        return Err(ImportError::new("empty", "empty table"));
    };
    t.samples = x.samples.iter().map(|s| s.0.clone()).collect();
    t.stat("features", t.features.len() as f64);
    t.stat(
        "withMatch",
        t.features.iter().filter(|f| !f.ann.is_empty()).count() as f64,
    );
    Ok(t)
}

/// The structure of a library entry counts only if its formula explains the precursor of the entry within 20 ppm (known bug of mzMine exports: reused keys).
pub fn check_structures(t: &mut Table) {
    let mut bad = 0.0;
    let tpol = t.pol;
    for f in t.features.iter_mut() {
        let (fmz, fpol) = (f.mz, f.pol);
        for a in f.ann.iter_mut() {
            if a.text("source") != "mzMine library" || a.flag("structureChecked") {
                continue;
            }
            a.set("structureChecked", V::Bool(true));
            let c = check_structure(
                a.text("formula"),
                a.num("precursor").or(Some(fmz)),
                a.text("adduct"),
                if fpol != 0 { fpol } else { tpol },
                20.0,
            );
            a.set(
                "structure",
                V::s(match c.ok {
                    Some(true) => "ok",
                    Some(false) => "unreliable",
                    None => "unverified",
                }),
            );
            if c.ok == Some(false) {
                for k in ["ik", "smiles", "inchi", "formula"] {
                    a.set(k, V::s(""));
                }
                a.push_note(
                    "unreliable library structure: the formula does not explain the precursor",
                );
                bad += 1.0;
            }
        }
    }
    let before = t.stat_of("unreliableStructures").unwrap_or(0.0);
    t.stat("unreliableStructures", before + bad);
}

/// The annotations file completes InChIKey / entry id of the SAME feature and name (else the same InChI); lipid rows are dropped.
pub fn merge_annotations(t: &mut Table, text: &str) -> Result<(), ImportError> {
    let (mut dropped, mut merged, mut rows) = (0.0, 0.0, 0.0);
    each_row(
        text,
        b',',
        None,
        |h| Ok(Cols::new(h)),
        |c, r, _| {
            let col = |k: &[&str]| c.pick(k);
            rows += 1.0;
            if txt(cell(r, col(&["method"])))
                .to_lowercase()
                .contains("matchedlipid")
            {
                dropped += 1.0;
                return Ok(());
            }
            let Some(fi) = t.feature(txt(cell(r, col(&["id"])))) else {
                return Ok(());
            };
            let nm = txt(cell(r, col(&["compound_name"])));
            let (cn, inchi) = (
                clean_name(nm).to_lowercase(),
                col(&["inchi"]).map_or("", |_| txt(cell(r, col(&["inchi"])))),
            );
            let Some(a) = t.features[fi].ann.iter_mut().find(|x| {
                x.text("source") == "mzMine library"
                    && ((!nm.is_empty()
                        && (x.text("name").to_lowercase() == nm.to_lowercase()
                            || x.text("cleanName").to_lowercase() == cn))
                        || (!inchi.is_empty() && x.text("inchi") == inchi))
            }) else {
                return Ok(());
            };
            let get = |k: &[&str]| col(k).map(|i| txt(cell(r, Some(i))));
            if let Some(v) = get(&["inchi_key", "inchikey"]).filter(|v| !v.is_empty()) {
                a.set("ik", V::s(v));
            }
            if let Some(v) = get(&["entry_id"]).filter(|v| !v.is_empty()) {
                a.set("entry", V::s(v));
            }
            for (k, names) in [
                ("smiles", &["smiles", "isomeric_smiles"][..]),
                ("formula", &["mol_formula"][..]),
            ] {
                if a.text(k).is_empty() {
                    if let Some(v) = get(names) {
                        a.set(k, V::s(v));
                    }
                }
            }
            if a.text("adduct").is_empty() {
                if let Some(i) = col(&["adduct"]) {
                    a.set("adduct", V::Str(norm_adduct(cell(r, Some(i)))));
                }
            }
            if a.num("precursor").is_none() {
                if let Some(i) = col(&["precursor_mz"]) {
                    a.set("precursor", V::opt(num(cell(r, Some(i)))));
                }
            }
            merged += 1.0;
            Ok(())
        },
    )?;
    t.stat("annotationRows", rows);
    t.stat("lipidRowsDropped", dropped);
    t.stat("annotationsMerged", merged);
    check_structures(t);
    Ok(())
}

/// A sample name from the header of a quant table: «<name> Peak area», without «.mzML».
fn peak_area(c: &str) -> Option<String> {
    let head = if ends_ci(c, "peak area") {
        c[..c.len() - 9].trim_end()
    } else {
        return None;
    };
    (!head.is_empty() && !head.contains(line_end)).then(|| strip_mzml(head))
}

type Quant = ([Option<usize>; 7], Vec<(String, usize)>);

/// The FBMN quant table of mzMine (areas per sample, correlation groups, annotation networks).
pub fn read_quant(text: &str, name: &str, pol: i32) -> Result<Table, ImportError> {
    let mut t = Table::new("mzmine", name, pol);
    let state: Option<Quant> = each_row(
        text,
        b',',
        None,
        |h| {
            let c = Cols::new(h);
            let ix = [
                c.pick(&["row id"]),
                c.pick(&["row m/z"]),
                c.pick(&["row retention time"]),
                c.pick(&["correlation group id"]),
                c.pick(&["annotation network number"]),
                c.pick(&["best ion"]),
                c.pick(&["neutral m mass"]),
            ];
            Ok((
                ix,
                h.iter()
                    .enumerate()
                    .filter_map(|(i, c)| peak_area(c).map(|n| (n, i)))
                    .collect::<Vec<_>>(),
            ))
        },
        |(x, s), r, _| {
            let (id, mz, rt) = (txt(cell(r, x[0])), num(cell(r, x[1])), num(cell(r, x[2])));
            let (Some(mz), Some(rt)) = (mz, rt) else {
                return Ok(());
            };
            if id.is_empty() {
                return Ok(());
            }
            let f = t.add_feature(id, mz, rt);
            for (n, i) in s.iter() {
                if let Some(a) = num(cell(r, Some(*i))).filter(|a| *a != 0.0) {
                    f.set_area(n, a);
                }
            }
            let ok =
                |i: Option<usize>| i.is_some_and(|_| num(cell(r, i)).is_some_and(|v| v >= 0.0));
            if ok(x[3]) {
                f.group = Some(txt(cell(r, x[3])).into());
            }
            if ok(x[4]) {
                f.iin = Some(Iin {
                    id: txt(cell(r, x[4])).into(),
                    ions: if x[5].is_some() {
                        txt(cell(r, x[5])).into()
                    } else {
                        String::new()
                    },
                    neutral: x[6].and_then(|_| num(cell(r, x[6]))),
                });
            }
            Ok(())
        },
    )?;
    t.samples = state.map_or(vec![], |(_, s)| s.into_iter().map(|x| x.0).collect());
    t.stat("features", t.features.len() as f64);
    Ok(t)
}

/// The IIN edges of FBMN (id1, id2, edge type, score, annotation).
pub fn read_edges(t: &mut Table, text: &str) -> Result<usize, ImportError> {
    let mut n = 0;
    each_row(
        text,
        b',',
        None,
        |h| {
            let k = Cols::new(h);
            Ok([
                k.pick(&["id1"]),
                k.pick(&["id2"]),
                k.pick(&["edgetype"]),
                k.pick(&["score"]),
                k.pick(&["annotation"]),
            ])
        },
        |x, r, _| {
            let (a, b) = (txt(cell(r, x[0])), txt(cell(r, x[1])));
            if a.is_empty() || b.is_empty() {
                return Ok(());
            }
            let score = x[3].and_then(|_| num(cell(r, x[3])));
            t.edges.push(Edge {
                a: a.into(),
                b: b.into(),
                kind: txt(cell(r, x[2])).into(),
                score,
                annotation: txt(cell(r, x[4])).into(),
            });
            n += 1;
            Ok(())
        },
    )?;
    t.stat("edges", n as f64);
    Ok(n)
}
