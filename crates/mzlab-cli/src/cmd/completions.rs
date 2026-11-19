//! `completions bash|zsh|fish`: the shell completion script, generated from the command table, so a command added to the table completes at once.

use std::collections::BTreeSet;

use super::{entries, Entry};
use crate::Outcome;

/// The words that can follow `prefix` (a path of command words) in the table, sorted; hidden commands are left out.
fn next_words(prefix: &[&str]) -> Vec<&'static str> {
    let mut set = BTreeSet::new();
    for e in entries().filter(|e| e.name != "help-md") {
        let w: Vec<&'static str> = e.name.split(' ').collect();
        if w.len() > prefix.len() && w[..prefix.len()] == *prefix {
            set.insert(w[prefix.len()]);
        }
    }
    set.into_iter().collect()
}

/// Every path of command words that has children (the root is the empty path).
fn paths() -> Vec<Vec<&'static str>> {
    let mut set: BTreeSet<Vec<&'static str>> = BTreeSet::new();
    set.insert(Vec::new());
    for e in entries().filter(|e| e.name != "help-md") {
        let w: Vec<&'static str> = e.name.split(' ').collect();
        for k in 1..w.len() {
            set.insert(w[..k].to_vec());
        }
    }
    set.into_iter().collect()
}

fn bash() -> String {
    let mut s = String::from(
        "# bash completion for mzlab-cli: source this file\n_mzlab_cli() {\n  local cur=\"${COMP_WORDS[COMP_CWORD]}\" path=\"\" i\n  for ((i = 1; i < COMP_CWORD; i++)); do path=\"$path ${COMP_WORDS[i]}\"; done\n  local words=\"\"\n  case \"$path\" in\n",
    );
    for p in paths() {
        let key = p.iter().map(|w| format!(" {w}")).collect::<String>();
        s += &format!("    \"{key}\") words=\"{}\" ;;\n", next_words(&p).join(" "));
    }
    s + "  esac\n  COMPREPLY=($(compgen -W \"$words\" -- \"$cur\"))\n}\ncomplete -o default -F _mzlab_cli mzlab-cli\n"
}

fn zsh() -> String {
    let mut s = String::from(
        "#compdef mzlab-cli\n# zsh completion for mzlab-cli: put this file in a folder of $fpath as _mzlab-cli\n_mzlab_cli() {\n  local path=\"${(j: :)words[2,CURRENT-1]}\"\n  local -a w\n  case \"$path\" in\n",
    );
    for p in paths() {
        s += &format!(
            "    \"{}\") w=({}) ;;\n",
            p.join(" "),
            next_words(&p).join(" ")
        );
    }
    s + "  esac\n  (( ${#w} )) && compadd -a w || _files\n}\ncompdef _mzlab_cli mzlab-cli\n"
}

fn fish() -> String {
    let mut s = String::from(
        "# fish completion for mzlab-cli: save as ~/.config/fish/completions/mzlab-cli.fish\nfunction __mzlab_at\n  set -l t (commandline -opc)\n  set -e t[1]\n  test \"$t\" = \"$argv\"\nend\n",
    );
    for p in paths() {
        s += &format!(
            "complete -c mzlab-cli -f -n '__mzlab_at {}' -a '{}'\n",
            p.join(" "),
            next_words(&p).join(" ")
        );
    }
    s
}

fn run_with(shell: &str, args: Vec<String>) -> Outcome {
    if let Some(a) = args.first() {
        return Outcome::fail(&format!("unknown argument {a}"));
    }
    let text = match shell {
        "bash" => bash(),
        "zsh" => zsh(),
        _ => fish(),
    };
    Outcome::ok(text.into_bytes())
}

pub(crate) static TABLE: &[Entry] = &[
    Entry::new(
        "completions bash",
        "mzlab-cli completions bash   the bash completion script, on stdout",
        |a| run_with("bash", a),
    ),
    Entry::new(
        "completions zsh",
        "mzlab-cli completions zsh   the zsh completion script, on stdout",
        |a| run_with("zsh", a),
    ),
    Entry::new(
        "completions fish",
        "mzlab-cli completions fish   the fish completion script, on stdout",
        |a| run_with("fish", a),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_script_names_every_group_and_hides_help_md() {
        for s in [bash(), zsh(), fish()] {
            for g in ["flow", "file", "peak", "msn", "ion", "completions"] {
                assert!(s.contains(g), "{g} missing");
            }
            assert!(!s.contains("help-md"));
        }
    }

    // The bash script, sourced in a real shell, completes `mzlab-cli fl` to `flow` and `mzlab-cli flow ru` to `run`.
    #[test]
    fn bash_script_completes_in_a_real_shell() {
        let complete = |line: &str, cword: usize| {
            let script = format!(
                "{}\nCOMP_WORDS=({line}); COMP_CWORD={cword}; _mzlab_cli; echo \"${{COMPREPLY[*]}}\"",
                bash()
            );
            let o = std::process::Command::new("bash")
                .args(["-c", &script])
                .output()
                .unwrap();
            String::from_utf8(o.stdout).unwrap().trim().to_string()
        };
        assert_eq!(complete("mzlab-cli fl", 1), "flow");
        assert_eq!(complete("mzlab-cli flow ru", 2), "run");
    }
}
