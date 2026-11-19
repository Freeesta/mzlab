//! `mzlab-cli ion elements --json [--out PATH]`: the periodic table for MS (`elements/1`, `mzlab_core::periodic`), byte for byte what the dialog
//! «Periodic table» of the page and the app reads.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli ion elements: the periodic table for mass spectrometry as JSON (elements/1).

usage: mzlab-cli ion elements --json [--out PATH]

  --json         the only format (accepted so that the command reads like the others)
  --out PATH     write there instead of the standard output

The answer is {\"schema\", \"classes\": [{key, code, name}], \"elements\": [{z, symbol, name, class, mono, avg, mono_only, m2_rel,
isotopes: [{a, mass, abund}]}]}: mono = exact mass of the most abundant isotope, avg = abundance-weighted mean mass, abund in percent,
m2_rel = M+2 / main intensity when it is 0.04 or more. Elements without natural isotopes in the tables have null masses.";

pub fn run(args: Vec<String>) -> Outcome {
    let mut out = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--json" => {}
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            o => return Outcome::fail(&format!("unknown argument {o}")),
        }
    }
    crate::emit(format!("{}\n", mzlab_core::periodic::elements_json()), &out)
}

#[cfg(test)]
mod tests {
    use crate::execute;

    fn run(a: &[&str]) -> crate::Outcome {
        execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn the_command_prints_the_route_json() {
        let o = run(&["ion", "elements", "--json"]);
        assert_eq!(o.code, 0);
        let want = mzlab_core::periodic::elements_json();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
        assert_eq!(run(&["ion", "elements", "--nope"]).code, 2);
        assert_eq!(run(&["ion", "elements", "--help"]).code, 0);
    }
}
