//! What is checked on one element at a time: attributes, the vocabulary of a cvParam and the terms an element must carry.

use std::collections::HashSet;

use quick_xml::events::BytesStart;

use super::cv::{self, Class};
use super::{Problem, Severity};

/// The attributes of an element, unescaped.
pub(super) struct Attrs(Vec<(String, String)>);

impl Attrs {
    pub fn of(e: &BytesStart) -> Attrs {
        let list = e
            .attributes()
            .with_checks(false)
            .filter_map(|a| a.ok())
            .map(|a| {
                let v = a
                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    .unwrap_or_default()
                    .into_owned();
                (a.key.into_inner().to_string(), v)
            })
            .collect();
        Attrs(list)
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn num<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.get(name).and_then(|v| v.trim().parse().ok())
    }
}

pub(super) fn push(
    out: &mut Vec<Problem>,
    severity: Severity,
    path: &str,
    code: &'static str,
    message: String,
) {
    out.push(Problem {
        severity,
        path: path.to_string(),
        code,
        message,
    });
}

/// The attributes the schema requires of an element.
pub(super) fn required_attributes(name: &str) -> &'static [&'static str] {
    match name {
        "cv" => &["id", "URI"],
        "cvParam" => &["cvRef", "accession", "name"],
        "userParam" => &["name"],
        "spectrum" | "chromatogram" => &["index", "id", "defaultArrayLength"],
        "run" => &["id"],
        "offset" => &["idRef"],
        "software" | "sourceFile" | "instrumentConfiguration" | "dataProcessing" => &["id"],
        _ => &[],
    }
}

/// The unit of a cvParam: its term, its name and whether the term allows it.
fn unit(a: &Attrs, acc: &str, cvs: &HashSet<String>, path: &str, out: &mut Vec<Problem>) {
    let cv = cv::get();
    let Some(ua) = a.get("unitAccession") else {
        let has_value = a.get("value").is_some_and(|v| !v.trim().is_empty());
        if has_value && !cv.units(acc).is_empty() {
            push(
                out,
                Severity::Warning,
                path,
                "unit.missing",
                format!("{acc} has a value and no unit"),
            );
        }
        return;
    };
    if !a.get("unitCvRef").is_some_and(|r| cvs.contains(r)) {
        push(
            out,
            Severity::Error,
            path,
            "cvRef",
            format!(
                "unitCvRef \"{}\" is not in the cvList",
                a.get("unitCvRef").unwrap_or("")
            ),
        );
    }
    match (cv.unit_name(ua), a.get("unitName")) {
        (None, _) if ua.starts_with("MS:") => push(
            out,
            Severity::Error,
            path,
            "unit.accession",
            format!("unit {ua} is not a term of the vocabulary"),
        ),
        (None, _) => push(
            out,
            Severity::Warning,
            path,
            "unit.accession",
            format!("unit {ua} is not known to the table"),
        ),
        (Some(n), Some(un)) if n != un => push(
            out,
            Severity::Error,
            path,
            "unit.name",
            format!("unit {ua} is \"{n}\", the file says \"{un}\""),
        ),
        _ => {}
    }
    let allowed = cv.units(acc);
    if !allowed.is_empty() && !allowed.iter().any(|u| u == ua) {
        push(
            out,
            Severity::Error,
            path,
            "unit.notAllowed",
            format!("{acc} does not take the unit {ua}"),
        );
    }
}

/// One cvParam against the vocabulary: its attributes, the CV it cites, the accession, the name, the unit.
pub(super) fn cv_param(a: &Attrs, cvs: &HashSet<String>, path: &str, out: &mut Vec<Problem>) {
    let Some(acc) = a.get("accession") else {
        return;
    };
    let path = format!("{path}/cvParam[{acc}]");
    if !a.get("cvRef").is_some_and(|r| cvs.contains(r)) {
        push(
            out,
            Severity::Error,
            &path,
            "cvRef",
            format!(
                "cvRef \"{}\" is not in the cvList",
                a.get("cvRef").unwrap_or("")
            ),
        );
    }
    if acc.starts_with("MS:") {
        match cv::get().term(acc) {
            None => push(
                out,
                Severity::Error,
                &path,
                "accession",
                format!("{acc} is not a term of the vocabulary"),
            ),
            Some((n, obsolete)) => {
                if a.get("name").is_some_and(|name| name != n) {
                    push(
                        out,
                        Severity::Error,
                        &path,
                        "name",
                        format!(
                            "{acc} is \"{n}\", the file says \"{}\"",
                            a.get("name").unwrap_or("")
                        ),
                    );
                }
                if obsolete {
                    push(
                        out,
                        Severity::Warning,
                        &path,
                        "obsolete",
                        format!("{acc} \"{n}\" is obsolete"),
                    );
                }
            }
        }
    }
    unit(a, acc, cvs, &path, out);
}

/// How many of `terms` are in `class`, against what the element needs: `exactly` one, or at least one.
fn need(
    terms: &[String],
    class: Class,
    what: &str,
    exactly: bool,
    path: &str,
    out: &mut Vec<Problem>,
) {
    let n = terms.iter().filter(|t| cv::get().is(t, class)).count();
    if n == 0 || (exactly && n > 1) {
        let want = if exactly {
            "exactly one"
        } else {
            "at least one"
        };
        push(
            out,
            Severity::Error,
            path,
            "required.term",
            format!("needs {want} {what} term and has {n}"),
        );
    }
}

/// The terms a spectrum, a chromatogram or a binary array must carry (`terms`: the accessions of its own cvParams).
pub(super) fn required_terms(element: &str, terms: &[String], path: &str, out: &mut Vec<Problem>) {
    match element {
        "spectrum" => {
            let levels = terms.iter().filter(|t| *t == "MS:1000511").count();
            if levels != 1 {
                push(
                    out,
                    Severity::Error,
                    path,
                    "required.term",
                    format!("needs exactly one ms level (MS:1000511) and has {levels}"),
                );
            }
            need(
                terms,
                Class::SpectrumType,
                "spectrum type",
                false,
                path,
                out,
            );
            need(
                terms,
                Class::SpectrumRepr,
                "profile or centroid spectrum",
                true,
                path,
                out,
            );
        }
        "chromatogram" => need(
            terms,
            Class::ChromType,
            "chromatogram type",
            true,
            path,
            out,
        ),
        "binaryDataArray" => {
            need(terms, Class::ArrayType, "array type", true, path, out);
            need(
                terms,
                Class::BinaryType,
                "number type (precision)",
                true,
                path,
                out,
            );
            need(terms, Class::Compression, "compression", true, path, out);
        }
        _ => {}
    }
}
