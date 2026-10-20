//! The feature table as mzTab-M 2.0 (tab separated; sections MTD, SMH/SML, SFH/SMF, SEH/SME), written from the public specification of the format
//! (Hoffmann et al., Anal. Chem. 2019, 91, 3302; https://github.com/HUPO-PSI/mzTab). One small molecule and one feature per row of the view, one
//! evidence per row whose identity names a hypothesis. It adds no identity and raises no level: every row is reliability 4 (unknown compound) and a
//! named row only carries its neutral mass and adduct. Used by the `export.mztab` step.

use mzlab_core::elements;

use crate::export::{fix, sig8, Ctx};
use crate::rowident::adduct_names;

/// `null`: the value does not apply; `NaN`: it applies but was not measured.
const NULL: &str = "null";
const NAN: &str = "NaN";
/// Reliability of every small molecule in the default scheme of the format (1-4): unknown compound.
const RELIABILITY: &str = "4";

/// A field without the characters that split the lines and the fields.
fn clean(s: &str) -> String {
    s.chars()
        .map(|c| {
            if matches!(c, '\t' | '\r' | '\n') {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// A file name as the path of a `file:///` location.
fn location(name: &str) -> String {
    let mut out = String::from("file:///");
    for b in name.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn line(out: &mut String, cells: &[String]) {
    out.push_str(
        &cells
            .iter()
            .map(|c| clean(c))
            .collect::<Vec<_>>()
            .join("\t"),
    );
    out.push('\n');
}

/// The study variables: the groups of the samples in order of first appearance (`all` for the ones without group) and the assays of each.
fn study_variables(c: &Ctx) -> Vec<(String, Vec<usize>)> {
    let mut v: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, s) in c.samples.iter().enumerate() {
        let g = if s.group.is_empty() {
            "all"
        } else {
            s.group.as_str()
        };
        match v.iter_mut().find(|(n, _)| n == g) {
            Some((_, a)) => a.push(i),
            None => v.push((g.to_string(), vec![i])),
        }
    }
    v
}

fn metadata(out: &mut String, c: &Ctx, svs: &[(String, Vec<usize>)]) {
    let mut m = |k: &str, v: String| line(out, &["MTD".into(), k.into(), v]);
    m("mzTab-version", "2.0.0-M".into());
    m(
        "mzTab-ID",
        format!("mzlab-{}", &c.table_sha256[..c.table_sha256.len().min(16)]),
    );
    m(
        "software[1]",
        format!("[,, mzLab, {}]", env!("CARGO_PKG_VERSION")),
    );
    m(
        "quantification_method",
        "[MS, MS:1001834, LC-MS label-free quantitation analysis, ]".into(),
    );
    m(
        "small_molecule-quantification_unit",
        "[MS, MS:1001844, MS1 feature area, ]".into(),
    );
    m(
        "small_molecule_feature-quantification_unit",
        "[MS, MS:1001844, MS1 feature area, ]".into(),
    );
    m(
        "id_confidence_measure[1]",
        "[,, mass error in ppm, ]".into(),
    );
    m("database[1]", "[,, no database, null]".into());
    m("database[1]-prefix", NULL.into());
    m("database[1]-version", "Unknown".into());
    m("database[1]-uri", NULL.into());
    let polarity = if c.polarity < 0 {
        "[MS, MS:1000129, negative scan, ]"
    } else {
        "[MS, MS:1000130, positive scan, ]"
    };
    for (i, s) in c.samples.iter().enumerate() {
        let n = i + 1;
        m(&format!("ms_run[{n}]-location"), location(&s.file));
        m(&format!("ms_run[{n}]-scan_polarity[1]"), polarity.into());
        m(&format!("assay[{n}]"), format!("assay_{n}"));
        m(&format!("assay[{n}]-ms_run_ref"), format!("ms_run[{n}]"));
    }
    for (j, (name, assays)) in svs.iter().enumerate() {
        let n = j + 1;
        m(&format!("study_variable[{n}]"), name.clone());
        m(
            &format!("study_variable[{n}]-assay_refs"),
            assays
                .iter()
                .map(|a| format!("assay[{}]", a + 1))
                .collect::<Vec<_>>()
                .join("|"),
        );
        m(
            &format!("study_variable[{n}]-description"),
            format!("Samples of group {name}"),
        );
    }
    m("cv[1]-label", "MS".into());
    m("cv[1]-full_name", "PSI-MS controlled vocabulary".into());
    m("cv[1]-version", "4.2.6".into());
    m(
        "cv[1]-uri",
        "https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo".into(),
    );
}

/// Mean and coefficient of variation of the areas present in a study variable.
fn summary(areas: &[Option<f64>]) -> (String, String) {
    let v: Vec<f64> = areas.iter().flatten().copied().collect();
    if v.is_empty() {
        return (NAN.into(), NULL.into());
    }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let cv = if v.len() < 2 || mean <= 0.0 {
        NULL.into()
    } else {
        let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (v.len() - 1) as f64;
        sig8(var.sqrt() / mean)
    };
    (sig8(mean), cv)
}

/// What the three table sections read: the rows of the view and the identity each one names.
struct Body<'a> {
    c: &'a Ctx<'a>,
    rows: Vec<usize>,
    svs: Vec<(String, Vec<usize>)>,
    adducts: [&'static str; 7],
    /// Number of the evidence of each row (position in `rows`), for the rows that can carry one.
    evidence_of: Vec<Option<usize>>,
}

impl<'a> Body<'a> {
    fn new(c: &'a Ctx<'a>) -> Self {
        let mut b = Body {
            c,
            rows: c.rows(),
            svs: study_variables(c),
            adducts: adduct_names(),
            evidence_of: Vec::new(),
        };
        let mut n = 0;
        b.evidence_of = b
            .rows
            .iter()
            .map(|&r| {
                b.theoretical(r).map(|_| {
                    n += 1;
                    n
                })
            })
            .collect();
        b
    }

    /// Neutral mass and adduct of the hypothesis a row names (None without one).
    fn named(&self, r: usize) -> Option<(f64, Option<&'static str>)> {
        let id = &self.c.table.ident;
        (!id.is_empty() && id.src[r] >> 1 != 0).then(|| {
            (
                id.neutral[r],
                self.adducts
                    .get(usize::from(id.ion[r]).wrapping_sub(1))
                    .copied(),
            )
        })
    }

    /// The theoretical m/z of the ion of a named row; only a row with a known adduct can carry evidence.
    fn theoretical(&self, r: usize) -> Option<f64> {
        self.named(r)
            .and_then(|(m, a)| a.and_then(|a| elements::ion_mz(m, a)))
    }

    fn area(&self, r: usize, k: usize) -> Option<f64> {
        self.c.area(r, k).filter(|a| a.is_finite())
    }

    fn areas(&self, r: usize) -> Vec<String> {
        (0..self.c.samples.len())
            .map(|k| self.area(r, k).map_or(NAN.into(), sig8))
            .collect()
    }
}

fn small_molecules(body: &Body, out: &mut String) {
    let n_sv = body.svs.len();
    let mut head: Vec<String> = [
        "SMH",
        "SML_ID",
        "SMF_ID_REFS",
        "database_identifier",
        "chemical_formula",
        "smiles",
        "inchi",
        "chemical_name",
        "uri",
        "theoretical_neutral_mass",
        "adduct_ions",
        "reliability",
        "best_id_confidence_measure",
        "best_id_confidence_value",
    ]
    .map(String::from)
    .to_vec();
    head.extend((1..=body.c.samples.len()).map(|n| format!("abundance_assay[{n}]")));
    head.extend((1..=n_sv).map(|n| format!("abundance_study_variable[{n}]")));
    head.extend((1..=n_sv).map(|n| format!("abundance_variation_study_variable[{n}]")));
    line(out, &head);
    for (i, &r) in body.rows.iter().enumerate() {
        let id = (i + 1).to_string();
        let (mass, ion) = body
            .named(r)
            .map_or((NULL.to_string(), NULL.to_string()), |(m, a)| {
                (fix(m, 5), a.unwrap_or(NULL).to_string())
            });
        let mut cells: Vec<String> = vec![
            "SML".into(),
            id.clone(),
            id,
            NULL.into(),
            NULL.into(),
            NULL.into(),
            NULL.into(),
            format!("row {}", body.c.table.id(r)),
            NULL.into(),
            mass,
            ion,
            RELIABILITY.into(),
            NULL.into(),
            NULL.into(),
        ];
        cells.extend(body.areas(r));
        let sums: Vec<(String, String)> = body
            .svs
            .iter()
            .map(|(_, a)| summary(&a.iter().map(|&k| body.area(r, k)).collect::<Vec<_>>()))
            .collect();
        cells.extend(sums.iter().map(|s| s.0.clone()));
        cells.extend(sums.iter().map(|s| s.1.clone()));
        line(out, &cells);
    }
}

fn features(body: &Body, out: &mut String) {
    let (c, t) = (body.c, body.c.table);
    let mut head: Vec<String> = [
        "SFH",
        "SMF_ID",
        "SME_ID_REFS",
        "SME_ID_REF_ambiguity_code",
        "adduct_ion",
        "isotopomer",
        "exp_mass_to_charge",
        "charge",
        "retention_time_in_seconds",
        "retention_time_in_seconds_start",
        "retention_time_in_seconds_end",
    ]
    .map(String::from)
    .to_vec();
    head.extend((1..=c.samples.len()).map(|n| format!("abundance_assay[{n}]")));
    line(out, &head);
    let seconds = |m: f64| fix(m * 60.0, 2);
    for (i, &r) in body.rows.iter().enumerate() {
        let q = (!t.meta.is_empty()).then(|| t.meta.get(r));
        let edge = |v: Option<f32>| {
            v.filter(|x| x.is_finite())
                .map_or(NULL.to_string(), |x| seconds(f64::from(x)))
        };
        let charge = q
            .map(|q| i32::from(q.charge))
            .filter(|&z| z != 0)
            .unwrap_or(i32::from(c.polarity.signum()));
        let ion = body.named(r).and_then(|(_, a)| a).unwrap_or(NULL);
        let mut cells: Vec<String> = vec![
            "SMF".into(),
            (i + 1).to_string(),
            body.evidence_of[i].map_or(NULL.to_string(), |e| e.to_string()),
            NULL.into(),
            ion.into(),
            NULL.into(),
            fix(t.mz[r], 5),
            charge.to_string(),
            seconds(t.rt[r]),
            edge(q.map(|q| q.rt_lo)),
            edge(q.map(|q| q.rt_hi)),
        ];
        cells.extend(body.areas(r));
        line(out, &cells);
    }
}

fn evidence(body: &Body, out: &mut String) {
    let (c, t) = (body.c, body.c.table);
    let head = [
        "SEH",
        "SME_ID",
        "evidence_input_id",
        "database_identifier",
        "chemical_formula",
        "smiles",
        "inchi",
        "chemical_name",
        "uri",
        "derivatized_form",
        "adduct_ion",
        "exp_mass_to_charge",
        "charge",
        "theoretical_mass_to_charge",
        "spectra_ref",
        "identification_method",
        "ms_level",
        "id_confidence_measure[1]",
        "rank",
    ];
    line(out, &head.map(String::from));
    for (i, &r) in body.rows.iter().enumerate() {
        let (Some(e), Some(theo), Some((_, ion))) =
            (body.evidence_of[i], body.theoretical(r), body.named(r))
        else {
            continue;
        };
        // The run with the largest area of the row is the one the evidence maps to.
        let best = (0..c.samples.len()).max_by(|&a, &b| {
            body.area(r, a)
                .unwrap_or(-1.0)
                .total_cmp(&body.area(r, b).unwrap_or(-1.0))
        });
        let ppm = (t.mz[r] - theo) / theo * 1e6;
        let cells: Vec<String> = vec![
            "SME".into(),
            e.to_string(),
            format!("row_{}", t.id(r)),
            NULL.into(),
            NULL.into(),
            NULL.into(),
            NULL.into(),
            NULL.into(),
            NULL.into(),
            NULL.into(),
            ion.unwrap_or(NULL).into(),
            fix(t.mz[r], 5),
            i32::from(c.polarity.signum()).to_string(),
            fix(theo, 5),
            format!("ms_run[{}]", best.map_or(1, |k| k + 1)),
            "[,, mass match to a hypothesis, ]".into(),
            "[MS, MS:1000511, ms level, 1]".into(),
            fix(ppm, 2),
            "1".into(),
        ];
        line(out, &cells);
    }
}

/// The mzTab-M file of the rows of the view; the bytes depend only on the table, the samples and the view.
pub fn mztab(c: &Ctx) -> String {
    let body = Body::new(c);
    let mut out = String::new();
    metadata(&mut out, c, &body.svs);
    out.push('\n');
    small_molecules(&body, &mut out);
    out.push('\n');
    features(&body, &mut out);
    if body.evidence_of.iter().any(Option::is_some) {
        out.push('\n');
        evidence(&body, &mut out);
    }
    out
}
