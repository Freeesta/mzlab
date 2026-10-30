//! `mzlab-cli ion-pattern FORMULA ADDUCT [--out PATH]`: the unit-resolution isotope pattern of an ion, the JSON of `api/ion_pattern`
//! (`mzlab_core::ionpattern`), byte for byte what the page and the app show.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli ion-pattern: the unit-resolution isotope pattern of a formula with an adduct, as JSON.

usage: mzlab-cli ion-pattern FORMULA ADDUCT [--out PATH]

  FORMULA     the neutral formula, e.g. C9H8O4
  ADDUCT      an adduct of the catalogue, e.g. \"[M+H]+\"
  --out PATH  write there instead of the standard output

The answer is {\"n\": element counts of the ion, \"z\": charge, \"peaks\": [{\"off\": nominal offset, \"mz\", \"rel\": % of the highest}]}.";

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out) = (Vec::new(), None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let [formula, adduct] = pos.as_slice() else {
        return Outcome::fail("ion-pattern needs a formula and an adduct");
    };
    match mzlab_core::ionpattern::route(formula, adduct) {
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
    fn the_command_prints_the_route_json() {
        let o = run(&["ion-pattern", "C9H8O4", "[M+H]+"]);
        assert_eq!(o.code, 0);
        let want = mzlab_core::ionpattern::route("C9H8O4", "[M+H]+").unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["ion-pattern"],
            vec!["ion-pattern", "C"],
            vec!["ion-pattern", "C", "[M+X]+"],
            vec!["ion-pattern", "--nope"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["ion-pattern", "--help"]).code, 0);
    }
}
