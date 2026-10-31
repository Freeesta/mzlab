//! `insilico pack PUBCHEMLITE_CSV OUT [--version TEXT] [--doi TEXT] [--piece-raw BYTES]`: the candidate package of `spectra.insilico` for the site and the app. The structures of a PubChemLite
//! CSV become CSV rows with the columns `read_candidates` reads, sorted by `MonoisotopicMass` then `Identifier`, cut into mass-range pieces that are each compressed alone
//! (zlib, `NNN.bin`, SHA-256) and described by `catalogo.json` (`insilico/1`, `docs/schemi/insilico-1.json`). No network, no clock: same input, same bytes.
//! `read_piece` is the reader of the app and the CLI: it checks and inflates one piece of a package folder. Layout and use: `docs/agenti/insilico.md`.

use std::io::{Read, Write};
use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out, Value};
use mzlab_feat::attributio::ms2_engine::insilico::{read_candidates_counted, Candidate};
use mzlab_feat::flow_insilico::{plain_name, FORMAT};

use crate::Outcome;

/// Raw bytes of a piece before it is cut (the zlib of a PubChemLite piece is a third of it, so about 4 MiB each).
pub const PIECE_RAW: usize = 12 << 20;
/// A piece inflated above this many bytes is refused.
const MAX_RAW: u64 = 64 << 20;
const COLUMNS: [&str; 5] = [
    "Identifier",
    "CompoundName",
    "SMILES",
    "MolecularFormula",
    "MonoisotopicMass",
];
const NOTICE: &str = "Candidate structures from PubChemLite for Exposomics (University of Luxembourg, LCSB), CC BY 4.0, built from PubChem. \
Names, structures and masses are the source's; a ranked candidate is a hypothesis, never an identification.";

const HELP: &str = "usage: mzlab-cli insilico pack PUBCHEMLITE_CSV OUT [--version TEXT] [--doi TEXT] [--piece-raw BYTES]\n  \
writes OUT/catalogo.json (insilico/1) and the mass-range pieces OUT/NNN.bin (zlib, cut every BYTES of rows, default 12 MiB); no network, same input, same bytes";

fn io(e: impl std::fmt::Display) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// One field of a CSV row: quoted when it holds a comma, a quote or a line break (a line break itself becomes a space).
fn field(s: &str, row: &mut String) {
    let s = s.trim();
    if s.contains([',', '"', '\n', '\r']) {
        row.push('"');
        for c in s.chars() {
            match c {
                '"' => row.push_str("\"\""),
                '\n' | '\r' => row.push(' '),
                c => row.push(c),
            }
        }
        row.push('"');
    } else {
        row.push_str(s);
    }
}

fn row_of(c: &Candidate) -> String {
    let mut r = String::new();
    for (i, f) in [
        c.id.as_str(),
        &c.name,
        &c.smiles,
        &c.formula,
        &format!("{}", c.mass),
    ]
    .iter()
    .enumerate()
    {
        if i > 0 {
            r.push(',');
        }
        field(f, &mut r);
    }
    r.push('\n');
    r
}

fn header() -> String {
    COLUMNS.join(",") + "\n"
}

fn deflate(raw: &[u8]) -> Result<Vec<u8>> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(9));
    e.write_all(raw).and_then(|_| e.finish()).map_err(io)
}

/// One cut piece: file, mass range, rows, raw size and SHA-256, compressed size and SHA-256.
struct Piece {
    file: String,
    mass: (f64, f64),
    rows: usize,
    raw: (usize, String),
    z: (usize, String),
}

fn catalog(c: &Cat, pieces: &[Piece]) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":")
        .str(FORMAT)
        .raw(",\"compression\":\"zlib\",\"piece_raw\":");
    o.int(c.piece_raw as i64).raw(",\"columns\":[");
    for (i, k) in COLUMNS.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k);
    }
    o.raw("],\"rows\":").int(c.rows as i64);
    o.raw(",\"skipped_rows\":").int(c.skipped as i64);
    o.raw(",\"size\":")
        .int(pieces.iter().map(|p| p.raw.0 as i64).sum());
    o.raw(",\"size_compressed\":")
        .int(pieces.iter().map(|p| p.z.0 as i64).sum());
    o.raw(",\"sha256\":").str(&c.sha256);
    o.raw(",\"mass_min\":")
        .num(pieces.first().map_or(0.0, |p| p.mass.0));
    o.raw(",\"mass_max\":")
        .num(pieces.last().map_or(0.0, |p| p.mass.1));
    o.raw(",\"source\":{\"name\":\"PubChemLite for Exposomics\",\"version\":")
        .str(&c.version);
    o.raw(",\"doi\":")
        .str(&c.doi)
        .raw(",\"sha256\":")
        .str(&c.source_sha256)
        .raw("}");
    o.raw(",\"licence\":\"CC-BY-4.0\",\"attribution\":")
        .str(NOTICE)
        .raw(",\"pieces\":[");
    for (i, p) in pieces.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(&p.file);
        o.raw(",\"mass_min\":")
            .num(p.mass.0)
            .raw(",\"mass_max\":")
            .num(p.mass.1);
        o.raw(",\"rows\":").int(p.rows as i64);
        o.raw(",\"size\":")
            .int(p.z.0 as i64)
            .raw(",\"sha256\":")
            .str(&p.z.1);
        o.raw(",\"size_raw\":")
            .int(p.raw.0 as i64)
            .raw(",\"sha256_raw\":")
            .str(&p.raw.1)
            .raw("}");
    }
    o.raw("]}\n");
    o.0
}

/// What the catalogue says about the package as a whole.
struct Cat {
    piece_raw: usize,
    rows: usize,
    skipped: usize,
    sha256: String,
    source_sha256: String,
    version: String,
    doi: String,
}

/// Packs the candidates of `csv` into `out`; returns a one-line summary.
pub fn pack(csv: &[u8], out: &Path, version: &str, doi: &str) -> Result<String> {
    pack_with(csv, out, version, doi, PIECE_RAW)
}

/// `pack` with the raw size at which a piece is cut.
pub fn pack_with(
    csv: &[u8],
    out: &Path,
    version: &str,
    doi: &str,
    piece_raw: usize,
) -> Result<String> {
    let (mut rows, total) = read_candidates_counted(&String::from_utf8_lossy(csv));
    if rows.is_empty() {
        return Err(Error::new("err.flow.insilico.suspects"));
    }
    rows.sort_by(|a, b| a.mass.total_cmp(&b.mass).then_with(|| a.id.cmp(&b.id)));
    std::fs::create_dir_all(out).map_err(io)?;
    let mut pieces: Vec<Piece> = Vec::new();
    let mut whole = Vec::new();
    let (mut raw, mut n, mut lo) = (header(), 0usize, 0.0f64);
    let mut cut = |raw: &mut String,
                   n: &mut usize,
                   lo: f64,
                   hi: f64,
                   pieces: &mut Vec<Piece>|
     -> Result<()> {
        let z = deflate(raw.as_bytes())?;
        let file = format!("{:03}.bin", pieces.len());
        std::fs::write(out.join(&file), &z).map_err(io)?;
        whole.extend_from_slice(raw.as_bytes());
        pieces.push(Piece {
            file,
            mass: (lo, hi),
            rows: *n,
            raw: (raw.len(), sha256_hex(raw.as_bytes())),
            z: (z.len(), sha256_hex(&z)),
        });
        *raw = header();
        *n = 0;
        Ok(())
    };
    for c in &rows {
        if n == 0 {
            lo = c.mass;
        }
        raw.push_str(&row_of(c));
        n += 1;
        if raw.len() >= piece_raw {
            cut(&mut raw, &mut n, lo, c.mass, &mut pieces)?;
        }
    }
    if n > 0 {
        let hi = rows[rows.len() - 1].mass;
        cut(&mut raw, &mut n, lo, hi, &mut pieces)?;
    }
    let cat = Cat {
        piece_raw,
        rows: rows.len(),
        skipped: total - rows.len(),
        sha256: sha256_hex(&whole),
        source_sha256: sha256_hex(csv),
        version: version.to_string(),
        doi: doi.to_string(),
    };
    let text = catalog(&cat, &pieces);
    std::fs::write(out.join("catalogo.json"), &text).map_err(io)?;
    Ok(format!(
        "{} candidates, {} row(s) without SMILES or mass not packed; {} B raw, {} B compressed in {} piece(s)",
        cat.rows,
        cat.skipped,
        whole.len(),
        pieces.iter().map(|p| p.z.0).sum::<usize>(),
        pieces.len()
    ))
}

/// The inflated CSV of piece `file` of the package whose catalogue is `catalogue`, read from `dir`: the size and SHA-256 of the compressed file and of the inflated
/// rows are checked against the catalogue.
pub fn read_piece(dir: &Path, catalogue: &[u8], file: &str) -> Result<Vec<u8>> {
    let bad = |d: &str| {
        Error::new("err.flow.insilico.piece")
            .with("file", file)
            .with("detail", d)
    };
    let v = json::parse(catalogue).map_err(|_| bad("catalogue"))?;
    let p = v
        .get("pieces")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .find(|p| p.get("file").and_then(Value::str) == Some(file))
        .ok_or_else(|| bad("not in the catalogue"))?;
    let (sz, hz) = (
        p.get("size").and_then(Value::f64),
        p.get("sha256").and_then(Value::str),
    );
    let (sr, hr) = (
        p.get("size_raw").and_then(Value::f64),
        p.get("sha256_raw").and_then(Value::str),
    );
    if !plain_name(file) || sz.is_none() || hz.is_none() || sr.is_none() || hr.is_none() {
        return Err(bad("entry"));
    }
    let z = std::fs::read(dir.join(file)).map_err(|e| bad(&e.to_string()))?;
    if Some(z.len() as f64) != sz || Some(sha256_hex(&z).as_str()) != hz {
        return Err(bad("compressed hash"));
    }
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(&z[..])
        .take(MAX_RAW + 1)
        .read_to_end(&mut raw)
        .map_err(|e| bad(&e.to_string()))?;
    if Some(raw.len() as f64) != sr || Some(sha256_hex(&raw).as_str()) != hr {
        return Err(bad("raw hash"));
    }
    Ok(raw)
}

fn fail(msg: &str) -> Outcome {
    Outcome {
        code: 2,
        stdout: Vec::new(),
        stderr: format!("error: {msg}\n{HELP}\n"),
    }
}

/// `mzlab-cli insilico ...`
pub fn run(args: Vec<String>) -> Outcome {
    if args.first().map(String::as_str) != Some("pack") {
        return if args.iter().any(|a| a == "--help") {
            Outcome {
                code: 0,
                stdout: format!("{HELP}\n").into_bytes(),
                stderr: String::new(),
            }
        } else {
            fail("unknown subcommand")
        };
    }
    let (mut paths, mut version, mut doi, mut cut) =
        (Vec::new(), String::new(), String::new(), PIECE_RAW);
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--version" => version = it.next().cloned().unwrap_or_default(),
            "--doi" => doi = it.next().cloned().unwrap_or_default(),
            "--piece-raw" => match it
                .next()
                .and_then(|v| v.parse::<usize>().ok())
                .filter(|&n| n > 0)
            {
                Some(n) => cut = n,
                None => return fail("--piece-raw needs a positive number of bytes"),
            },
            s if s.starts_with("--") => return fail(&format!("unknown option {s}")),
            s => paths.push(s.to_string()),
        }
    }
    let [csv, out] = paths.as_slice() else {
        return fail("wrong number of paths");
    };
    let bytes = match std::fs::read(csv) {
        Ok(b) => b,
        Err(e) => return fail(&format!("{csv}: {e}")),
    };
    match pack_with(&bytes, Path::new(out), &version, &doi, cut) {
        Ok(t) => Outcome {
            code: 0,
            stdout: (t + "\n").into_bytes(),
            stderr: String::new(),
        },
        Err(e) => Outcome {
            code: 1,
            stdout: Vec::new(),
            stderr: format!("error: {}\n", e.key),
        },
    }
}

#[cfg(test)]
#[path = "insilico_pack_tests.rs"]
mod tests;
