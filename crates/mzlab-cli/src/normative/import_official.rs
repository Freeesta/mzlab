//! `mzlab-cli normative import-official READER CSV SIDECAR OUT [--file NAME] [--date YYYY-MM-DD]`: the user's own copy of an official list (READER `svhc` or
//! `ca_prop65`, a CSV) becomes the private `.mzsus` OUT, using the CAS sidecar (`mzlab-cas/1`, plain text) for the structures; the summary JSON goes to stdout.
//! The same function runs in the app (`normative/import-official`) and in the browser (the lazy `normative` .wasm); `tests/test_normative_import_parity.py` compares them.

use std::path::{Path, PathBuf};

use mzlab_core::json::Out;
use mzlab_feat::normative::official::import_official;

use super::NormError;

struct Args {
    reader: String,
    csv: PathBuf,
    sidecar: PathBuf,
    out: PathBuf,
    file: Option<String>,
    date: Option<String>,
}

fn parse(args: &[String]) -> Result<Args, NormError> {
    let (mut pos, mut file, mut date) = (Vec::new(), None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--file" => file = it.next().cloned(),
            "--date" => date = it.next().cloned(),
            _ => pos.push(a.clone()),
        }
    }
    let [reader, csv, sidecar, out] = <[String; 4]>::try_from(pos)
        .map_err(|_| NormError::usage("import-official needs READER CSV SIDECAR OUT"))?;
    Ok(Args {
        reader,
        csv: csv.into(),
        sidecar: sidecar.into(),
        out: out.into(),
        file,
        date,
    })
}

fn read(p: &Path) -> Result<Vec<u8>, NormError> {
    std::fs::read(p).map_err(|e| NormError::io(p, &e))
}

/// The summary JSON on stdout; the `.mzsus` is written to OUT.
pub fn run(args: &[String]) -> Result<String, NormError> {
    let a = parse(args)?;
    let csv = read(&a.csv)?;
    let side = String::from_utf8_lossy(&read(&a.sidecar)?).into_owned();
    let file = a.file.unwrap_or_else(|| {
        a.csv
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned())
    });
    let date = a.date.unwrap_or_else(super::fetch::today);
    let mut label = Out::default();
    label
        .raw("{\"file\":")
        .str(&file)
        .raw(",\"date\":")
        .str(&date)
        .raw("}");
    let b = import_official(&a.reader, &csv, &side, &label.0)
        .map_err(|e| NormError::parse(&e.list, &e.msg))?;
    let n = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
    std::fs::write(&a.out, &b[4 + n..]).map_err(|e| NormError::io(&a.out, &e))?;
    Ok(String::from_utf8_lossy(&b[4..4 + n]).into_owned() + "\n")
}
