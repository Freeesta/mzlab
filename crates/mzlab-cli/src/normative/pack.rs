//! `normative pack IN OUT`: the committed lists (`IN/lists/*.csv`, `IN/provenance.json`) become the data release: `normative.mzsus` (one row per skeleton, the
//! `listings` extension of `mzlab_feat::suspects`), its zlib pieces of 4 MiB with SHA-256 and `catalogo.json` (`normative/1`). No network, no clock: same
//! input, same bytes. `normative listings PATH` reads a package back (folder or `.mzsus`). Used by `pages.yml` and the app; layout in `docs/agenti/sospetti.md`.
//! The writer itself is `mzlab_feat::normative::pack`; this file reads the folder and writes the pieces.

use std::io::Write;
use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{self, Value};
use mzlab_feat::attributio::reglists::catalog;
use mzlab_feat::normative::pack::{build_rows, catalog_json, raw_of, sorted};
use mzlab_feat::suspects::Suspects;

pub use mzlab_feat::normative::pack::{FORMAT, PIECE, RAW_NAME};

use super::NormError;

fn io(p: &Path, e: &std::io::Error) -> NormError {
    NormError::io(p, e)
}

fn read(p: &Path) -> Result<Vec<u8>, NormError> {
    std::fs::read(p).map_err(|e| io(p, &e))
}

/// zlib (level 9) of `raw`.
fn deflate(raw: &[u8]) -> Result<Vec<u8>, NormError> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(9));
    e.write_all(raw)
        .and_then(|_| e.finish())
        .map_err(|e| NormError::parse("pack", &e.to_string()))
}

/// The compressed package cut into pieces `NNN.bin` of at most `PIECE` bytes: `(file, size, sha256)`.
fn write_pieces(out: &Path, z: &[u8]) -> Result<Vec<(String, usize, String)>, NormError> {
    let mut pieces = Vec::new();
    for (i, b) in z.chunks(PIECE).enumerate() {
        let name = format!("{i:03}.bin");
        std::fs::write(out.join(&name), b).map_err(|e| io(&out.join(&name), &e))?;
        pieces.push((name, b.len(), sha256_hex(b)));
    }
    Ok(pieces)
}

pub fn pack(input: &Path, out: &Path) -> Result<Vec<String>, NormError> {
    let prov_path = input.join("provenance.json");
    let prov = json::parse(&read(&prov_path)?)
        .map_err(|_| NormError::parse("provenance.json", "not JSON"))?;
    let all: Vec<&Value> = prov
        .get("lists")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .collect();
    let ids: Vec<&str> = all
        .iter()
        .filter_map(|l| l.get("id").and_then(Value::str))
        .collect();
    let order = catalog::ordered(&ids);
    let lists: Vec<&Value> = order
        .iter()
        .filter_map(|id| {
            all.iter()
                .copied()
                .find(|l| l.get("id").and_then(Value::str) == Some(id))
        })
        .collect();
    if lists.is_empty() {
        return Err(NormError::parse("provenance.json", "no lists"));
    }
    let mut texts: Vec<String> = Vec::new();
    for id in &order {
        let p = input.join("lists").join(format!("{id}.csv"));
        texts.push(String::from_utf8_lossy(&read(&p)?).into_owned());
    }
    let pairs: Vec<(&str, &str)> = order
        .iter()
        .map(String::as_str)
        .zip(texts.iter().map(String::as_str))
        .collect();
    let perr = |e: mzlab_feat::normative::Error| NormError::parse(&e.list, &e.msg);
    let (rows, skipped) = build_rows(&pairs).map_err(perr)?;
    let sorted = sorted(&rows);
    let raw = raw_of(&sorted, &lists).map_err(perr)?;

    std::fs::create_dir_all(out).map_err(|e| io(out, &e))?;
    let z = deflate(&raw)?;
    let pieces = write_pieces(out, &z)?;
    let cat = catalog_json(&raw, z.len(), sorted.len(), skipped, &lists, &pieces);
    let cp = out.join("catalogo.json");
    std::fs::write(&cp, &cat).map_err(|e| io(&cp, &e))?;
    std::fs::write(out.join(RAW_NAME), &raw).map_err(|e| io(&out.join(RAW_NAME), &e))?;
    Ok(vec![format!(
        "{} rows, {} lists, {} B raw, {} B compressed in {} piece(s); {skipped} screenable row(s) without mass or formula not packed",
        sorted.len(),
        lists.len(),
        raw.len(),
        z.len(),
        pieces.len()
    )])
}

/// `normative listings PATH`: the `listings` JSON of a package folder (with `normative.mzsus`) or of a `.mzsus` file.
pub fn listings(path: &Path) -> Result<String, NormError> {
    let p = if path.is_dir() {
        path.join(RAW_NAME)
    } else {
        path.to_path_buf()
    };
    let s =
        Suspects::open(&read(&p)?).map_err(|e| NormError::parse("listings", &format!("{e:?}")))?;
    s.listings_json()
        .ok_or_else(|| NormError::parse("listings", "the file has no listings extension"))
}
