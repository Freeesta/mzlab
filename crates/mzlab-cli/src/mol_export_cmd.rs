//! `mzlab-cli mol-export MOLFILE|- [--set format=sdf|mol] [--set ion=ADDUCT] [--set name=TEXT] [--out PATH]`: the drawing as a plain
//! molfile or SDF (`sdf/1`), byte for byte the «Export» MOL/SDF of the Draw page and of the app.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-export: a drawing as a plain .mol or .sdf file (sdf/1).

usage: mzlab-cli mol-export MOLFILE|- [--set NAME=VALUE...] [--out PATH]

  MOLFILE        a V2000 or V3000 molfile (- reads the standard input)
  --set format   sdf (default: one record per connected structure, with data items) or mol (the whole drawing in one record)
  --set ion      none (default) or an adduct: [M+H]+ [M+NH4]+ [M+Na]+ [M+K]+ [M-H]- [M+Cl]- [M+HCOO]-
  --set name     the title line of every record (default empty)
  --out PATH     write there instead of the standard output

Every SDF record has the data items MZLAB_FORMULA, MZLAB_EXACT_MASS, MZLAB_ION and MZLAB_MZ (the adduct of a neutral structure, or
\"drawn\" for a charged one; none without an ion) and MZLAB_HASH (the mol1: id of the structure). V2000, or V3000 above 999 atoms or
bonds or with enhanced-stereo groups. The m/z is a candidate to look for, not an identification.";

const NAMES: [&str; 3] = ["format", "ion", "name"];

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out) = (Vec::new(), None);
    let mut set = [String::from("sdf"), String::from("none"), String::new()];
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            "--set" => match it.next().as_deref().and_then(|kv| kv.split_once('=')) {
                Some((k, v)) => match NAMES.iter().position(|n| *n == k) {
                    Some(i) => set[i] = v.to_string(),
                    None => return Outcome::fail(&format!("unknown --set name {k} (format, ion, name)")),
                },
                None => return Outcome::fail("--set needs NAME=VALUE"),
            },
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let [src] = pos.as_slice() else {
        return Outcome::fail("mol-export needs one molfile (or - for the standard input)");
    };
    let text = if src == "-" {
        let mut t = String::new();
        if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut t) {
            return Outcome::fail(&e.to_string());
        }
        t
    } else {
        match std::fs::read_to_string(src) {
            Ok(t) => t,
            Err(e) => return Outcome::fail(&format!("{src}: {e}")),
        }
    };
    match mzlab_feat::chem::sdf::export(&text, &set[0], &set[1], &set[2]) {
        Ok(t) => crate::emit(t, &out),
        Err(e) => Outcome::fail(&e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::execute;

    fn run(a: &[&str]) -> crate::Outcome {
        execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    const MOL: &str = "\n  t\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.5000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\nM  END\n";

    #[test]
    fn the_command_prints_the_function_text() {
        let p = std::env::temp_dir().join(format!("mol_export_{}.mol", std::process::id()));
        std::fs::write(&p, MOL).unwrap();
        let path = p.to_str().unwrap();
        let o = run(&["mol-export", path, "--set", "ion=[M+H]+", "--set", "name=methanol"]);
        let m = run(&["mol-export", path, "--set", "format=mol"]);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0);
        let want = mzlab_feat::chem::sdf::export(MOL, "sdf", "[M+H]+", "methanol").unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), want);
        assert!(want.contains("> <MZLAB_FORMULA>\nCH4O\n") && want.ends_with("$$$$\n"));
        assert_eq!(String::from_utf8(m.stdout).unwrap(), mzlab_feat::chem::sdf::export(MOL, "mol", "none", "").unwrap());
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol-export"],
            vec!["mol-export", "a", "b"],
            vec!["mol-export", "--nope"],
            vec!["mol-export", "a", "--set", "colour=red"],
            vec!["mol-export", "/no/such/file.mol"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["mol-export", "--help"]).code, 0);
    }
}
