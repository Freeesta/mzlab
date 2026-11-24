//! The `peak` commands: every exit code they give is one their catalog row lists, and the codes a caller can provoke without data
//! (help 0, a bad option 2, a missing input 4) are the listed ones.

use crate::contract_tests::{run, MISSING};

fn listed(name: &str) -> Vec<u8> {
    let e = crate::cmd::entries().find(|e| e.name == name).unwrap();
    crate::catalog::exits_of(e, &crate::catalog::usage_of(e))
}

#[test]
fn the_exits_of_every_peak_command_are_listed() {
    let names = [
        "peak auto",
        "peak auto-lr",
        "peak noise",
        "peak edges",
        "peak area",
        "peak window",
        "peak split",
        "peak table",
    ];
    for n in names {
        let words: Vec<&str> = n.split(' ').collect();
        let exits = listed(n);
        let mut seen = Vec::new();
        let mut help = words.clone();
        help.push("--help");
        seen.push((run(&help).code, "help"));
        let mut bad = words.clone();
        bad.push("--no-such-option");
        seen.push((run(&bad).code, "a bad option"));
        let missing = MISSING.iter().find(|a| a[..2] == words[..]).unwrap();
        seen.push((run(missing).code, "a missing input"));
        assert_eq!(
            seen.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
            [0, 2, 4],
            "{n}"
        );
        for (c, what) in seen {
            assert!(
                exits.contains(&c),
                "{n}: {what} gives {c}, not in {exits:?}"
            );
        }
        // nothing outside done / data / usage / input / still-being-written, and 6 (output) exactly when the command writes with --out
        let out = crate::cmd::entries()
            .find(|e| e.name == n)
            .unwrap()
            .help
            .contains("--out");
        assert_eq!(exits.contains(&6), out, "{n}: {exits:?}");
        assert!(
            exits.iter().all(|c| [0, 1, 2, 4, 6, 8].contains(c)),
            "{n}: {exits:?}"
        );
    }
}
