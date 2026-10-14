//! The `metodo/1` document (`metodo.rs`) of an Agilent MassHunter method folder `*.m` that is not part of a run (the data-analysis method of a GC/MS:
//! `DaMethod/Quant/quantitative.xml`, with the acquisition method files next to it when the folder is a whole copy). It has no sample and no instrument: it says
//! WHAT the method looks for. Section `quantificazione`: the windows of the method and, for each compound, its expected RT, its extraction window, the qualifier ions
//! with their relative response and range, the calibration levels and the curve settings; every name/value of the files goes to `campi` with `mappato` where the section
//! carries it. What the file does not say is absent (a target ion of 0 is «not set», never an m/z).

use super::meta::{self, text_of, tree, Node};
use crate::metodo::{self, num, obj, text, Source, SrcFile, V};
use crate::metodo_raw::campi::Campo;
use sha1::{Digest as _, Sha1};

/// The compound-level elements the section carries (the rest stays in `campi` only).
const COMPOUND: [&str; 22] = [
    "CompoundID",
    "CompoundName",
    "CompoundType",
    "ConcentrationUnits",
    "CurveFit",
    "CurveFitOrigin",
    "CurveFitWeight",
    "ISTDFlag",
    "IonPolarity",
    "Integrator",
    "MZ",
    "MZExtractionWindowFilterLeft",
    "MZExtractionWindowFilterRight",
    "MZExtractionWindowUnits",
    "RetentionTime",
    "RetentionTimeDeltaUnits",
    "RetentionTimeWindow",
    "RetentionTimeWindowUnits",
    "ScanType",
    "LeftRetentionTimeDelta",
    "RightRetentionTimeDelta",
    "TimeSegment",
];
const QUALIFIER: [&str; 7] = [
    "CompoundID",
    "QualifierID",
    "MZ",
    "RelativeResponse",
    "QualifierRangeMinimum",
    "QualifierRangeMaximum",
    "Uncertainty",
];
const LEVEL: [&str; 5] = [
    "CompoundID",
    "LevelName",
    "LevelConcentration",
    "LevelEnable",
    "CalibrationType",
];
const ROOT: [&str; 5] = [
    "DataVersion",
    "ReferenceWindow",
    "ReferenceWindowPercentOrMinutes",
    "NonReferenceWindow",
    "CorrelationWindow",
];
const QUANT: &str = "DaMethod/Quant/quantitative.xml";

fn kid<'a>(n: &'a Node, k: &str) -> Option<&'a str> {
    n.kids
        .iter()
        .find(|c| c.name == k)
        .map(|c| c.text.trim())
        .filter(|t| !t.is_empty())
}
fn f(n: &Node, k: &str) -> Option<f64> {
    kid(n, k).and_then(|t| t.parse().ok())
}
fn attr<'a>(n: &'a Node, k: &str) -> Option<&'a str> {
    n.attrs
        .iter()
        .find(|(a, _)| a == k)
        .map(|(_, v)| v.as_str())
}
fn flag(n: &Node, k: &str) -> Option<V> {
    kid(n, k).map(|t| V::Bool(t.eq_ignore_ascii_case("true")))
}
fn same_compound(n: &Node, id: Option<f64>) -> bool {
    f(n, "CompoundID") == id
}

fn ion(q: &Node) -> Option<V> {
    obj(vec![
        ("indice", num(f(q, "QualifierID"))),
        ("mz", num(f(q, "MZ")).filter(|_| f(q, "MZ") > Some(0.0))),
        ("risposta_relativa_pct", num(f(q, "RelativeResponse"))),
        ("min_pct", num(f(q, "QualifierRangeMinimum"))),
        ("max_pct", num(f(q, "QualifierRangeMaximum"))),
        ("incertezza_pct", num(f(q, "Uncertainty"))),
    ])
}

fn level(l: &Node) -> Option<V> {
    obj(vec![
        ("nome", kid(l, "LevelName").and_then(text)),
        ("conc", num(f(l, "LevelConcentration"))),
        ("attivo", flag(l, "LevelEnable")),
    ])
}

fn compound(c: &Node, root: &Node) -> Option<V> {
    let id = f(c, "CompoundID");
    let mut ions: Vec<&Node> = root
        .kids
        .iter()
        .filter(|q| q.name == "TargetQualifier" && same_compound(q, id))
        .collect();
    crate::sortc::slice_by(&mut ions, |a, b| {
        f(a, "QualifierID")
            .partial_cmp(&f(b, "QualifierID"))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut levels: Vec<&Node> = root
        .kids
        .iter()
        .filter(|l| l.name == "Calibration" && same_compound(l, id))
        .collect();
    crate::sortc::slice_by(&mut levels, |a, b| {
        f(a, "LevelConcentration")
            .partial_cmp(&f(b, "LevelConcentration"))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    obj(vec![
        ("id", num(id)),
        ("nome", kid(c, "CompoundName").and_then(text)),
        ("tipo", kid(c, "CompoundType").and_then(text)),
        ("istd", flag(c, "ISTDFlag")),
        (
            "rt_min",
            num(f(c, "RetentionTime")).filter(|_| f(c, "RetentionTime") > Some(0.0)),
        ),
        (
            "finestra_rt",
            obj(vec![
                ("valore", num(f(c, "RetentionTimeWindow"))),
                ("unita", kid(c, "RetentionTimeWindowUnits").and_then(text)),
            ]),
        ),
        ("segmento_tempo", num(f(c, "TimeSegment"))),
        ("polarita", kid(c, "IonPolarity").and_then(text)),
        ("tipo_scansione", kid(c, "ScanType").and_then(text)),
        ("mz", num(f(c, "MZ")).filter(|_| f(c, "MZ") > Some(0.0))),
        (
            "finestra_mz",
            obj(vec![
                ("sinistra", num(f(c, "MZExtractionWindowFilterLeft"))),
                ("destra", num(f(c, "MZExtractionWindowFilterRight"))),
                ("unita", kid(c, "MZExtractionWindowUnits").and_then(text)),
            ]),
        ),
        ("integratore", kid(c, "Integrator").and_then(text)),
        (
            "curva",
            obj(vec![
                ("modello", kid(c, "CurveFit").and_then(text)),
                ("origine", kid(c, "CurveFitOrigin").and_then(text)),
                ("peso", kid(c, "CurveFitWeight").and_then(text)),
            ]),
        ),
        ("unita_conc", kid(c, "ConcentrationUnits").and_then(text)),
        (
            "ioni",
            Some(V::Arr(ions.into_iter().filter_map(ion).collect())),
        ),
        (
            "livelli",
            Some(V::Arr(levels.into_iter().filter_map(level).collect())),
        ),
    ])
}

/// The section from the parsed `quantitative.xml`.
fn section(root: &Node) -> Option<V> {
    let compounds: Vec<V> = root
        .kids
        .iter()
        .filter(|c| c.name == "TargetCompound")
        .filter_map(|c| compound(c, root))
        .collect();
    let a = |k| attr(root, k).and_then(text);
    let n = |k| attr(root, k).and_then(|t| t.parse().ok());
    obj(vec![
        ("versione_dati", num(n("DataVersion"))),
        (
            "finestre",
            obj(vec![
                ("riferimento", num(n("ReferenceWindow"))),
                ("non_riferimento", num(n("NonReferenceWindow"))),
                ("unita", a("ReferenceWindowPercentOrMinutes")),
                ("correlazione", num(n("CorrelationWindow"))),
            ]),
        ),
        ("composti", Some(V::Arr(compounds))),
    ])
}

fn mapped(origin: &str, name: &str) -> Option<&'static str> {
    if !origin.eq_ignore_ascii_case(QUANT) {
        return None;
    }
    let leaf = name.rsplit(['/', '@']).next().unwrap_or(name);
    let (prefix, list): (&str, &[&str]) = if name.contains("/TargetCompound/") {
        ("TargetCompound", &COMPOUND)
    } else if name.contains("/TargetQualifier/") {
        ("TargetQualifier", &QUALIFIER)
    } else if name.contains("/Calibration/") {
        ("Calibration", &LEVEL)
    } else {
        ("", &ROOT)
    };
    let in_root = prefix.is_empty() && name.contains('@') && name.matches('/').count() == 0;
    ((in_root || !prefix.is_empty()) && list.contains(&leaf)).then_some("quantificazione")
}

fn sha1_hex(b: &[u8]) -> String {
    Sha1::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// `metodo/1` of the method folder `name` (`x.m`) whose files are `files` (path inside the folder, bytes). Never an error: what is missing is absent.
pub fn metodo_m(name: &str, files: &[(&str, &[u8])]) -> String {
    let items = meta::collect(files);
    let quant = files
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(QUANT))
        .and_then(|(_, b)| tree(&text_of(b)))
        .and_then(|top| top.kids.into_iter().next()); // `tree` returns a holder whose only child is the document element
    let mut fields: Vec<Campo> = items
        .iter()
        .filter(|i| i.name != "text" && !i.value.is_empty())
        .map(|i| {
            let mut c = Campo::new(&i.origin, &i.name, &i.value);
            c.mappato = mapped(&i.origin, &i.name);
            c
        })
        .collect();
    crate::sortc::slice_by(&mut fields, |a, b| {
        (&a.origine, &a.chiave).cmp(&(&b.origine, &b.chiave))
    });
    let mut seen: Vec<&str> = Vec::new();
    for i in items.iter().filter(|i| i.name == "undecoded bytes") {
        if !seen.contains(&i.origin.as_str()) {
            seen.push(&i.origin);
        }
    }
    let src = Source {
        format: "agilent-m",
        files: files
            .iter()
            .map(|(n, b)| SrcFile {
                name: format!("{name}/{n}"),
                sha1: Some(sha1_hex(b)),
                bytes: b.len() as u64,
            })
            .collect(),
        sample: None,
    };
    let doc = obj(vec![
        ("schema", Some(V::s(metodo::SCHEMA))),
        ("origine", metodo::origin(&src, None)),
        ("quantificazione", quant.as_ref().and_then(section)),
        (
            "campi",
            Some(V::Arr(fields.iter().filter_map(Campo::json).collect())),
        ),
        (
            "avvisi",
            Some(V::Arr(
                seen.into_iter()
                    .filter_map(|o| {
                        obj(vec![
                            ("chiave", Some(V::s("meta.nonDecodificato"))),
                            ("dove", text(o)),
                        ])
                    })
                    .collect(),
            )),
        ),
    ]);
    doc.map_or_else(String::new, |d| d.json())
}
