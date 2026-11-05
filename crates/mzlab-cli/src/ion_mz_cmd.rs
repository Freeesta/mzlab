//! `mzlab-cli ion-mz TEXT [--adduct A] [--out PATH]`: the JSON of `api/formula` (`mzlab_core::elements::formula_mz`), byte for byte what the dialog «Extract ions»
//! of the page and the app shows: a formula or an m/z, an adduct and a signed shift of the m/z (`288.2+16`, `C9H8O4-18`).

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli ion-mz: the m/z of an ion text, with an optional signed shift, as JSON.

usage: mzlab-cli ion-mz TEXT [--adduct ADDUCT] [--out PATH]

  TEXT           a neutral formula (C9H8O4) or an m/z with a shift (288.2+16); a signed number closing the text
                 is added to the m/z after the adduct, as a literal Da value (+16 is 16.000, not an oxygen)
  --adduct A     an adduct of the catalogue, e.g. \"[M+H]+\"; \"none\" = the text is already the ion
  --out PATH     write there instead of the standard output

The answer is {\"formula\", \"neutral\", \"adducts\", \"adduct\", \"mz\", \"mz5\", \"mz1\", \"nominal\", \"offset\" (only with a shift)}.
The m/z is a candidate to extract, never an identification.";

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out, mut adduct) = (Vec::new(), None, String::new());
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            "--adduct" => match it.next() {
                Some(v) => adduct = v,
                None => return Outcome::fail("--adduct needs a value"),
            },
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let [text] = pos.as_slice() else {
        return Outcome::fail("ion-mz needs one text");
    };
    match mzlab_core::elements::formula_mz(
        text,
        if adduct.is_empty() {
            None
        } else {
            Some(&adduct)
        },
    ) {
        Ok(j) => crate::emit(format!("{j}\n"), &out),
        Err(e) => Outcome::fail(&e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::execute;

    fn run(a: &[&str]) -> crate::Outcome {
        execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn the_command_prints_the_route_json_with_the_shift() {
        let o = run(&["ion-mz", "C9H8O4+16", "--adduct", "[M+H]+"]);
        assert_eq!(o.code, 0);
        let want = mzlab_core::elements::formula_mz("C9H8O4+16", Some("[M+H]+")).unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
        let o = run(&["ion-mz", "288.2-18"]);
        assert!(String::from_utf8(o.stdout)
            .unwrap()
            .contains("\"mz\":270.2"));
    }

    #[test]
    fn usage_and_malformed_shift_exit_2() {
        for a in [
            vec!["ion-mz"],
            vec!["ion-mz", "C9H8O4+1.2.3"],
            vec!["ion-mz", "--nope"],
            vec!["ion-mz", "x", "--adduct"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["ion-mz", "--help"]).code, 0);
    }
}
