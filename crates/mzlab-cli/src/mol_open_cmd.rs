//! `mzlab-cli mol open FILE|- [--check] [--out PATH]`: the text the Drawing tab hands to its editor for a structure file (`open/1`):
//! decoded as its byte-order mark or XML declaration says, the empty attributes of CDXML tags dropped, binary CDX as base64. Byte for byte what the page and the app read.
//! Its stdout is always the `open/1` document: `--json` is the default form, a failure is one `error/1`, and `--check` exit 1 is a verdict (`contract::VERDICT`).

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol open: the text the Drawing editor reads for a structure file (.cdxml, .cdx, .mol, ...), as JSON (open/1, always; --json only makes a failure an error/1).

usage: mzlab-cli mol open FILE|- [--check] [--out PATH]

  FILE        the structure file (any bytes, up to 8 MiB); - reads the standard input
  --check     exit 1 when the file needed a change (the answer is printed all the same, a verdict like file validate), 0 when it is read as it is
  --out PATH  write there instead of the standard output

The answer is {\"schema\": \"open/1\", \"text\": \"...\", \"changes\": [...]}; each change is one of
  {\"fix\": \"charset\", \"charset\": NAME}        the text was decoded as UTF-16 (byte-order mark, or NUL bytes with none) or an 8-bit charset
  {\"fix\": \"replaced\", \"count\": N}            N characters were not valid in that charset and became U+FFFD
  {\"fix\": \"empty\", \"names\": [\"a\", \"b\"]}    the attributes with an empty value dropped from the CDXML start tags (RDKit writes BondLength=\"\" and Indigo then draws nothing)
in this order. A UTF-16 declaration is rewritten as UTF-8. Binary CDX (VjCD) comes back as base64 with no change. Nothing else is parsed, and no
fidelity to any drawing program is claimed. A file over 8 MiB is refused (error.open.tooBig, exit 1): the page reads it on its own thread.
Exit codes: 0 ok, 1 --check and a change or a file over the limit, 2 usage, 4 file or standard input unreadable, 6 --out not writable.";

/// The bytes of `src` (`-`: the standard input), at most one byte over the limit so that the engine refuses a file that is too large without it being read whole.
fn read_input(src: &str) -> std::result::Result<Vec<u8>, Outcome> {
    use std::io::Read;
    let limit = mzlab_feat::chem::structure_file::MAX_BYTES as u64 + 1;
    let unreadable = |e: std::io::Error| {
        let path = if src == "-" { "standard input" } else { src };
        Outcome::fail_error(
            &mzlab_core::Error::new("err.cli.file").with("path", format!("{path}: {e}")),
        )
    };
    let mut b = Vec::new();
    if src == "-" {
        std::io::stdin().take(limit).read_to_end(&mut b)
    } else {
        std::fs::File::open(src).and_then(|f| f.take(limit).read_to_end(&mut b))
    }
    .map_err(unreadable)?;
    Ok(b)
}

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out, mut check) = (Vec::new(), None, false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--check" => check = true,
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let [src] = pos.as_slice() else {
        return Outcome::fail("mol open needs one file (or - for the standard input)");
    };
    let bytes = match read_input(src) {
        Ok(b) => b,
        Err(o) => return o,
    };
    let opened = match mzlab_feat::chem::structure_file::open_bytes(&bytes) {
        Ok(o) => o,
        Err(e) => return Outcome::fail_error(&e),
    };
    let mut o = crate::emit(format!("{}\n", opened.json()), &out);
    if check && !opened.changes.is_empty() && o.code == 0 {
        o.code = crate::exit::DATA;
    }
    o
}

#[cfg(test)]
#[path = "mol_open_tests.rs"]
mod tests;
