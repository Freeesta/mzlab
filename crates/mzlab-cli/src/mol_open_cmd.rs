//! `mzlab-cli mol open FILE|- [--check] [--out PATH]`: the text the Drawing tab hands to its editor for a structure file (`open/1`):
//! decoded as its byte-order mark or XML declaration says, the empty attributes of CDXML tags dropped, binary CDX as base64. Byte for byte what the page and the app read.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol open: the text the Drawing editor reads for a structure file (.cdxml, .cdx, .mol, ...), as JSON (open/1).

usage: mzlab-cli mol open FILE|- [--check] [--out PATH]

  FILE        the structure file (any bytes); - reads the standard input
  --check     exit 1 when the file needed a change (the answer is printed all the same), 0 when it is read as it is
  --out PATH  write there instead of the standard output

The answer is {\"schema\": \"open/1\", \"text\": \"...\", \"changes\": [{\"fix\": \"charset\", \"charset\": NAME} | {\"fix\": \"empty\", \"names\": \"a, b\"}]}.
Only two changes are made, and each is listed: text in UTF-16 or another 8-bit charset is decoded (a UTF-16 declaration is rewritten as UTF-8),
and the attributes with an empty value are dropped from every CDXML start tag (RDKit writes BondLength=\"\" and Indigo then draws nothing).
Binary CDX (VjCD) comes back as base64 with no change. Nothing else is parsed, and no fidelity to any drawing program is claimed.";

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
    let bytes = if src == "-" {
        let mut b = Vec::new();
        if let Err(e) = std::io::Read::read_to_end(&mut std::io::stdin(), &mut b) {
            return Outcome::fail(&e.to_string());
        }
        b
    } else {
        match std::fs::read(src) {
            Ok(b) => b,
            Err(e) => return Outcome::fail(&format!("{src}: {e}")),
        }
    };
    let changed = !mzlab_feat::chem::structure_file::open_bytes(&bytes)
        .changes
        .is_empty();
    let mut o = crate::emit(
        format!(
            "{}\n",
            mzlab_feat::chem::structure_file::open_json_bytes(&bytes)
        ),
        &out,
    );
    if check && changed && o.code == 0 {
        o.code = crate::exit::DATA;
    }
    o
}

#[cfg(test)]
mod tests {
    use crate::execute;

    fn run(a: &[&str]) -> crate::Outcome {
        execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    fn file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("mol_open_{}_{name}", std::process::id()));
        std::fs::write(&d, bytes).unwrap();
        d
    }

    #[test]
    fn the_command_prints_the_function_json() {
        let bytes = b"<CDXML BondLength=\"\" Name=\"a\"><page/></CDXML>";
        let p = file("a.cdxml", bytes);
        let o = run(&["mol", "open", p.to_str().unwrap()]);
        assert_eq!(o.code, 0);
        assert_eq!(
            String::from_utf8(o.stdout).unwrap(),
            format!(
                "{}\n",
                mzlab_feat::chem::structure_file::open_json_bytes(bytes)
            )
        );
        let c = run(&["mol", "open", p.to_str().unwrap(), "--check"]);
        assert_eq!(c.code, 1, "a file that needed a change exits 1");
        assert!(!c.stdout.is_empty());
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn a_file_read_as_it_is_exits_0_under_check() {
        let p = file("b.cdxml", b"<CDXML Name=\"a\"><page/></CDXML>");
        assert_eq!(
            run(&["mol", "open", p.to_str().unwrap(), "--check"]).code,
            0
        );
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol", "open"],
            vec!["mol", "open", "a", "b"],
            vec!["mol", "open", "--nope"],
            vec!["mol", "open", "/no/such/file.cdxml"],
        ] {
            assert_eq!(run(&a).code, crate::exit::usage_or_missing(&a), "{a:?}");
        }
        assert_eq!(run(&["mol", "open", "--help"]).code, 0);
    }
}
