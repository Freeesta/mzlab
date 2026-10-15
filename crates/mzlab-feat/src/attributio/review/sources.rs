// Ported from Attributio src/attributio/review/sources.py and readers/gnps.py (clean_library_hit_name).
//
// The hits the pipeline's own tools gave a feature: SIRIUS spectral matches (exact and analog), formulas and CSI:FingerID structures, the GNPS2
// library hits and the edges of its molecular network. The input is the text of the files (the page reads them from the folder of the campaign);
// the hit tables are typed like the ones Attributio builds with pandas. `UsiFetcher` stays out (it needs the network).

use super::tsv::{Cell, Tsv};

/// The text of the tool outputs of a campaign, per polarity (`"POS"` then `"NEG"`).
#[derive(Debug, Clone, Default)]
pub struct SourceTexts {
    /// `(polarity, table name without .tsv, text)` of the SIRIUS summary tables.
    pub sirius: Vec<(String, String, String)>,
    /// `(polarity, text)` of `nf_output/library/merged_results_with_gnps.tsv`.
    pub gnps_library: Vec<(String, String)>,
    /// `(polarity, text)` of `nf_output/networking/filtered_pairs.tsv`.
    pub gnps_pairs: Vec<(String, String)>,
}

/// A SIRIUS summary table of both polarities, rows with a numeric feature id only (`Campaign.load_sirius`).
struct Sirius {
    tables: Vec<(String, Tsv)>,
    /// `(table, row, feature id)`.
    rows: Vec<(usize, usize, i64)>,
}

impl Sirius {
    fn load(texts: &SourceTexts, table: &str) -> Sirius {
        let mut s = Sirius {
            tables: Vec::new(),
            rows: Vec::new(),
        };
        for pol in ["POS", "NEG"] {
            for (_, _, text) in texts
                .sirius
                .iter()
                .filter(|(p, t, _)| p == pol && t == table)
            {
                let t = Tsv::parse(text);
                let idcol = if t.has("mappingFeatureId") {
                    "mappingFeatureId"
                } else {
                    "SIRIUS_mappingFeatureId"
                };
                s.tables.push((pol.to_string(), t));
                let ti = s.tables.len() - 1;
                for r in 0..s.tables[ti].1.rows.len() {
                    if let Some(id) = s.tables[ti].1.get(r, idcol).num() {
                        s.rows.push((ti, r, id as i64));
                    }
                }
            }
        }
        s
    }

    fn has(&self, col: &str) -> bool {
        self.tables.iter().any(|(_, t)| t.has(col))
    }

    fn cell(&self, i: usize, col: &str) -> &Cell {
        let (t, r, _) = self.rows[i];
        self.tables[t].1.get(r, col)
    }

    fn pol(&self, i: usize) -> &str {
        &self.tables[self.rows[i].0].0
    }

    fn id(&self, i: usize) -> i64 {
        self.rows[i].2
    }

    /// Row indices ordered by `(pol, id, rank)` with a missing rank last, stable.
    fn by_rank(&self, rank_col: &str) -> Vec<usize> {
        let mut o: Vec<usize> = (0..self.rows.len()).collect();
        o.sort_by(|a, b| {
            (self.pol(*a), self.id(*a))
                .cmp(&(self.pol(*b), self.id(*b)))
                .then_with(|| {
                    na_last(
                        self.cell(*a, rank_col).num(),
                        self.cell(*b, rank_col).num(),
                        false,
                    )
                })
        });
        o
    }
}

/// Order of two optional numbers with the missing one last, ascending or descending.
fn na_last(a: Option<f64>, b: Option<f64>, descending: bool) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) if descending => y.total_cmp(&x),
        (Some(x), Some(y)) => x.total_cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

/// `'public_spectra_2506:(CCMSLIB00003139303)'` -> `'CCMSLIB00003139303'` (the first non-empty parenthesis).
pub fn reference_id(link: &str) -> Option<String> {
    link.match_indices('(').find_map(|(i, _)| {
        let rest = &link[i + 1..];
        rest.find(')')
            .filter(|j| *j > 0)
            .map(|j| rest[..j].to_string())
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct SiriusHit {
    pub pol: String,
    pub id: i64,
    /// `"exact"` or `"analog"`.
    pub kind: &'static str,
    pub ref_id: String,
    pub name: Cell,
    pub similarity: Option<f64>,
    pub shared: Option<f64>,
    pub ref_prec: Option<f64>,
    pub adduct: Option<String>,
    pub ik1: Option<String>,
    pub smiles: Cell,
    pub instrument: Cell,
    pub ce: Cell,
    pub rank: Option<f64>,
}

fn sirius_kind(s: &Sirius, kind: &'static str) -> Vec<SiriusHit> {
    let mut out = Vec::new();
    for i in 0..s.rows.len() {
        let analog = s.has("analogHit") && s.cell(i, "analogHit").py_str().to_lowercase() == "true";
        if analog != (kind == "analog") {
            continue;
        }
        let Some(ref_id) = reference_id(&s.cell(i, "referenceLinks").py_str()) else {
            continue;
        };
        out.push(SiriusHit {
            pol: s.pol(i).to_string(),
            id: s.id(i),
            kind,
            ref_id,
            name: s.cell(i, "referenceName").clone(),
            similarity: s.cell(i, "similarity").num(),
            shared: s.cell(i, "sharedPeaks").num(),
            ref_prec: s.cell(i, "referencePrecursorMz").num(),
            adduct: s
                .cell(i, "referenceAdduct")
                .text()
                .map(|v| v.replace(' ', "")),
            ik1: s
                .cell(i, "InChIkey2D")
                .text()
                .filter(|v| v.chars().count() >= 14)
                .map(|v| v.chars().take(14).collect()),
            smiles: s.cell(i, "referenceSmiles").clone(),
            instrument: s.cell(i, "referenceInstrument").clone(),
            ce: s.cell(i, "referenceCE").clone(),
            rank: s.cell(i, "spectralMatchRank").num(),
        });
    }
    out
}

/// One row per SIRIUS hit (exact and analog), best similarity first within `(pol, id, kind)`, one row per reference.
pub fn sirius_hits(texts: &SourceTexts) -> Vec<SiriusHit> {
    let mut all = Vec::new();
    for (table, kind) in [
        ("spectral_matches_top-5", "exact"),
        ("spectral_matches_analog_top-5", "analog"),
    ] {
        let s = Sirius::load(texts, table);
        if s.rows.is_empty() || !s.has("similarity") {
            continue;
        }
        all.extend(sirius_kind(&s, kind));
    }
    all.sort_by(|a, b| {
        (a.pol.as_str(), a.id, a.kind)
            .cmp(&(b.pol.as_str(), b.id, b.kind))
            .then_with(|| na_last(a.similarity, b.similarity, true))
    });
    let mut out: Vec<SiriusHit> = Vec::new();
    for h in all {
        if !out
            .iter()
            .any(|o| o.pol == h.pol && o.id == h.id && o.kind == h.kind && o.ref_id == h.ref_id)
        {
            out.push(h);
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormulaCandidate {
    pub rank: Option<i64>,
    pub formula: String,
    pub adduct: String,
    pub ion: String,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructureCandidate {
    pub rank: Option<i64>,
    pub name: String,
    pub smiles: String,
    pub xlogp: Option<f64>,
    pub ik1: Option<String>,
    pub formula: String,
}

/// The first `n` rows of every `(pol, id)` in rank order, `(pol, id)` in the order of the sorted table.
fn top_n<T>(
    s: &Sirius,
    rank_col: &str,
    n: usize,
    make: impl Fn(&Sirius, usize) -> T,
) -> Vec<((String, i64), Vec<T>)> {
    let mut out: Vec<((String, i64), Vec<T>)> = Vec::new();
    for i in s.by_rank(rank_col) {
        let key = (s.pol(i).to_string(), s.id(i));
        if !out.last().is_some_and(|(k, _)| *k == key) {
            out.push((key, Vec::new()));
        }
        let g = &mut out.last_mut().expect("group").1;
        if g.len() < n {
            g.push(make(s, i));
        }
    }
    out
}

/// The candidate formulas of SIRIUS (`formula_identifications_top-5`), five per feature.
pub fn sirius_formulas(texts: &SourceTexts) -> Vec<((String, i64), Vec<FormulaCandidate>)> {
    let s = Sirius::load(texts, "formula_identifications_top-5");
    if s.rows.is_empty() || !s.has("molecularFormula") {
        return Vec::new();
    }
    top_n(&s, "formulaRank", 5, |s, i| FormulaCandidate {
        rank: s.cell(i, "formulaRank").num().map(|r| r as i64),
        formula: s.cell(i, "molecularFormula").py_str(),
        adduct: if s.has("adduct") {
            s.cell(i, "adduct").py_str().replace(' ', "")
        } else {
            String::new()
        },
        ion: if s.has("precursorFormula") {
            s.cell(i, "precursorFormula").py_str()
        } else {
            String::new()
        },
        score: s.cell(i, "SiriusScoreNormalized").num(),
    })
}

/// The CSI:FingerID candidates (`structure_identifications_top-5`), five per feature, for the RT versus logP ranking of a Level 3.
pub fn csi_top5(texts: &SourceTexts) -> Vec<((String, i64), Vec<StructureCandidate>)> {
    let s = Sirius::load(texts, "structure_identifications_top-5");
    if s.rows.is_empty() || !s.has("smiles") {
        return Vec::new();
    }
    let text = |s: &Sirius, i: usize, c: &str| {
        if s.has(c) {
            s.cell(i, c).py_str()
        } else {
            String::new()
        }
    };
    top_n(&s, "structurePerIdRank", 5, |s, i| {
        let ik: String = text(s, i, "InChIkey2D").chars().take(14).collect();
        StructureCandidate {
            rank: s.cell(i, "structurePerIdRank").num().map(|r| r as i64),
            name: text(s, i, "name"),
            smiles: s.cell(i, "smiles").py_str(),
            xlogp: s.cell(i, "xlogp").num(),
            ik1: Some(ik).filter(|v| !v.is_empty()),
            formula: text(s, i, "molecularFormula"),
        }
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub pol: String,
    pub a: i64,
    pub b: i64,
    pub cosine: Option<f64>,
    pub component: Option<f64>,
}

/// The edges of the GNPS2 molecular network as drawn (feature-based: the cluster id is the mzMine feature id).
pub fn network_edges(texts: &SourceTexts) -> Vec<Edge> {
    let mut out = Vec::new();
    for pol in ["POS", "NEG"] {
        for (_, text) in texts.gnps_pairs.iter().filter(|(p, _)| p == pol) {
            let t = Tsv::parse(text);
            if !(t.has("CLUSTERID1") && t.has("CLUSTERID2") && t.has("Cosine")) {
                continue;
            }
            for r in 0..t.rows.len() {
                let (Some(a), Some(b)) =
                    (t.get(r, "CLUSTERID1").num(), t.get(r, "CLUSTERID2").num())
                else {
                    continue;
                };
                out.push(Edge {
                    pol: pol.to_string(),
                    a: a as i64,
                    b: b as i64,
                    cosine: t.get(r, "Cosine").num(),
                    component: t.get(r, "ComponentIndex").num(),
                });
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct GnpsHit {
    pub pol: String,
    pub id: i64,
    pub ref_id: Cell,
    pub name: String,
    pub score: Option<f64>,
    pub shared: Option<f64>,
    pub mass_diff: Option<f64>,
    pub ref_prec: Option<f64>,
    pub adduct: Cell,
    pub smiles: Cell,
    pub inchikey: Cell,
    pub instrument: Cell,
    pub library: Cell,
}

/// One row per GNPS2 library hit (`merged_results_with_gnps.tsv`); `mass_diff` other than 0 with Analog Search ON marks an analog.
pub fn gnps_hits(texts: &SourceTexts) -> Vec<GnpsHit> {
    let mut out = Vec::new();
    for pol in ["POS", "NEG"] {
        for (_, text) in texts.gnps_library.iter().filter(|(p, _)| p == pol) {
            let t = Tsv::parse(text);
            if t.rows.is_empty() || !t.has("#Scan#") {
                continue;
            }
            let ik = ["InChIKey-Planar", "InChIKey"]
                .into_iter()
                .find(|c| t.has(c));
            for r in 0..t.rows.len() {
                let Some(id) = t.get(r, "#Scan#").num() else {
                    continue;
                };
                out.push(GnpsHit {
                    pol: pol.to_string(),
                    id: id as i64,
                    ref_id: t.get(r, "SpectrumID").clone(),
                    name: clean_library_hit_name(&t.get(r, "Compound_Name").py_str()),
                    score: t.get(r, "MQScore").num(),
                    shared: t.get(r, "SharedPeaks").num(),
                    mass_diff: t.get(r, "MassDiff").num(),
                    ref_prec: t.get(r, "LibMZ").num(),
                    adduct: t.get(r, "Adduct").clone(),
                    smiles: t.get(r, "Smiles").clone(),
                    inchikey: ik.map_or(Cell::Na, |c| t.get(r, c).clone()),
                    instrument: t.get(r, "Instrument").clone(),
                    library: t.get(r, "LibraryName").clone(),
                });
            }
        }
    }
    out
}

const DB_PREFIXES: [&str; 5] = [
    "Massbank:",
    "MassbankEU:",
    "MoNA:",
    "ReSpect:",
    "Spectral Match to ",
];
const INSTR_SUFFIXES: [&str; 4] = [" from NIST14", " - 30.0 eV", " - 40.0 eV", " - 20.0 eV"];

/// Length of a leading accession (`EA255002 `, `PT110640 `, `1083201 `) with its spaces: two or three capitals and five or more digits,
/// or five or more digits, then white space.
fn accession_len(n: &str) -> usize {
    let b: Vec<char> = n.chars().collect();
    let after = |from: usize| -> Option<usize> {
        let d = b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
        let sp = b[from + d..]
            .iter()
            .take_while(|c| c.is_whitespace())
            .count();
        (d >= 5 && sp >= 1).then_some(from + d + sp)
    };
    let caps = b.iter().take_while(|c| c.is_ascii_uppercase()).count();
    let end = [3usize, 2]
        .into_iter()
        .filter(|k| caps >= *k)
        .find_map(after)
        .or_else(|| after(0));
    end.map_or(0, |e| b[..e].iter().map(|c| c.len_utf8()).sum())
}

/// Strips database prefixes, instrument suffixes, a leading accession and a `_CE15` tag from a library compound name.
pub fn clean_library_hit_name(n: &str) -> String {
    let mut n = n.trim().trim_matches('"').trim().to_string();
    for p in DB_PREFIXES {
        n = n.replace(p, "");
    }
    n = n.split('|').next().unwrap_or("").to_string();
    for s in INSTR_SUFFIXES {
        n = n.replace(s, "");
    }
    let t = n.trim();
    let t = &t[accession_len(t)..];
    let mut t = t.trim().to_string();
    if let Some(p) = t.rfind("_CE") {
        let rest = &t[p + 3..];
        let (int, frac) = rest
            .split_once('.')
            .map_or((rest, None), |(a, b)| (a, Some(b)));
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
        if digits(int) && frac.is_none_or(digits) {
            t.truncate(p);
        }
    }
    t.trim().to_string()
}
