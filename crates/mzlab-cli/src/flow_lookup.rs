//! The native executor of `spectra.lookup@1`: the entries of a spectral library the user declares that carry a given name, formula, InChIKey, accession
//! or precursor m/z, written as an MGF that `spectra.match` or `spectra.chimeric` read as queries. Nothing is built in and nothing is looked up online.

use std::fmt::Write as _;

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};
use mzlab_lib::spec::Spectrum;

use crate::flow::{io, read_one, Exec, FileRunner};
use crate::flow_match::library_spectra;

/// The executor of the step, read by the runner after its own list.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("spectra.lookup", Exec::File("spectra", lookup_job))];

const BY: &[&str] = &["name", "formula", "inchikey", "accession", "precursor_mz"];

/// `spectra.lookup@1`: the library entries that match a name, formula, InChIKey, accession or precursor m/z.
pub fn lookup_step() -> Step {
    Step {
        ins: vec![Port::new(
            "library",
            DataType::Library,
            "Spectral library the user declares",
        )],
        outs: vec![Port::new("spectra", DataType::Spectra, "The library entries found, as MGF")],
        params: vec![
            Param::choice("by", "Look up by", BY)
                .default_str("name")
                .doc("The field of the library entries that value is compared with; precursor_mz takes a number and the window ppm"),
            Param::text("value", "Value")
                .doc("The text to find (for precursor_mz, the m/z); required"),
            Param::boolean("exact", "Whole name")
                .default_bool(false)
                .doc("By name: the name must be the whole value, not only contain it (any case); the other fields are always compared whole"),
            Param::number("ppm", "Precursor window")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm")
                .doc("By precursor_mz: the entries whose precursor is this close to the value"),
            Param::integer("top_n", "Entries kept")
                .min(1.0)
                .default_num(50.0)
                .doc("The first entries in the order of the library; the rest are not written"),
        ],
        runs_in: RunsIn {
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.flow.lookup.value", "err.flow.match.library"],
        ..Step::core(
            "spectra.lookup",
            "Find spectra in a library",
            "Writes the entries of the library you declare that have a given name, formula, InChIKey, accession or precursor m/z, with their peaks untouched, so that the next steps can use them as queries. The library is always the one you supply: nothing is built in and nothing is searched online.",
        )
    }
}

fn text<'a>(job: &'a Job, name: &str) -> &'a str {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.str()).unwrap_or("")
}

fn number(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

fn whole(job: &Job, name: &str) -> bool {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.bool()) == Some(true)
}

/// Does the entry carry the value in the field `by`?
fn keeps(job: &Job, by: &str, value: &str, s: &Spectrum) -> Result<bool> {
    let low = value.to_lowercase();
    Ok(match by {
        "name" if whole(job, "exact") => s.meta.name.to_lowercase() == low,
        "name" => s.meta.name.to_lowercase().contains(&low),
        "formula" => s.meta.formula.replace(' ', "") == value.replace(' ', ""),
        // a 14-letter value is the first block, the skeleton without the stereochemistry
        "inchikey" if low.len() == 14 => s.meta.inchikey.to_lowercase().starts_with(&low),
        "inchikey" => s.meta.inchikey.to_lowercase() == low,
        "accession" => s.meta.accession == value,
        "precursor_mz" => {
            let mz: f64 = value
                .parse()
                .map_err(|_| Error::new("err.flow.lookup.value"))?;
            let ppm = number(job, "ppm").unwrap_or(10.0);
            (s.prec - mz).abs() <= mz * ppm * 1e-6
        }
        _ => return Err(Error::new("err.flow.lookup.value")),
    })
}

fn block(out: &mut String, k: usize, s: &Spectrum) {
    let m = &s.meta;
    let title = if m.accession.is_empty() {
        format!("lib.{k}")
    } else {
        m.accession.clone()
    };
    let _ = writeln!(out, "BEGIN IONS\nTITLE={title}\nPEPMASS={:.5}", s.prec);
    let mode = match s.pol {
        1 => "Positive",
        -1 => "Negative",
        _ => "",
    };
    let fields = [
        ("NAME", &m.name),
        ("ADDUCT", &m.adduct),
        ("FORMULA", &m.formula),
        ("INCHIKEY", &m.inchikey),
        ("SMILES", &m.smiles),
        ("ACCESSION", &m.accession),
        ("IONMODE", &mode.to_string()),
    ];
    for (key, v) in fields.iter().filter(|(_, v)| !v.is_empty()) {
        let _ = writeln!(out, "{key}={v}");
    }
    for (mz, it) in s.mz.iter().zip(&s.it) {
        let _ = writeln!(out, "{mz:.5} {it}");
    }
    out.push_str("END IONS\n");
}

fn lookup_job(r: &FileRunner, job: &Job) -> Result<mzlab_core::flow::Artifact> {
    let value = text(job, "value").trim();
    if value.is_empty() {
        return Err(Error::new("err.flow.lookup.value"));
    }
    let by = match text(job, "by") {
        "" => "name",
        b => b,
    };
    let library = read_one(r, job, "library")?;
    let top = number(job, "top_n").unwrap_or(50.0).max(1.0) as usize;
    let mut out = String::new();
    let mut kept = 0;
    for (k, s) in library_spectra((&library.0, &library.1))?
        .iter()
        .enumerate()
    {
        if kept < top && keeps(job, by, value, s)? {
            block(&mut out, k, s);
            kept += 1;
        }
    }
    let path = r.work.join(format!("{}.mgf", job.instance.id));
    std::fs::write(&path, out).map_err(io)?;
    r.keep(path, DataType::Spectra)
}
