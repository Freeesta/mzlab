//! The command line that reruns a plan ("Copy command" of the Flow canvas, `mzlab-cli flow command`): a POSIX (bash/zsh) script that writes the canonical
//! plan with a quoted here-document (nothing is expanded) and runs `mzlab-cli flow run` on it. Built here so the page never assembles a command itself.

use super::{parse, plan_json, Plan};
use crate::error::Result;

/// The folder in the script until the page fills in the one the user gives.
pub const FOLDER_PLACEHOLDER: &str = "/path/to/the/folder/with/your/files";

/// A name of an input for a comment line: control characters (a newline would end the comment) become `?`.
fn comment_safe(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

/// The here-document delimiter: `MZFLOW`, longer if a line of the plan is exactly that.
fn delimiter(text: &str) -> String {
    let mut d = String::from("MZFLOW");
    while text.lines().any(|l| l == d) {
        d.push('_');
    }
    d
}

/// The script that reruns `plan` on files in the folder `$MZLAB_IN`. The folder is a placeholder between single quotes: the page puts the folder of the user in its place.
pub fn command(plan: &Plan) -> String {
    let text = plan_json(plan);
    let d = delimiter(&text);
    let names: Vec<String> = plan.inputs.iter().map(|f| comment_safe(&f.name)).collect();
    let mut s = String::from("# Rerun this mzLab Flow plan on the command line.\n");
    // there are no prebuilt binaries: the program is built once from a clone of the repository
    s += "# mzlab-cli is built once from a clone of the repository (needs Rust): run this from the clone.\n";
    s += "command -v mzlab-cli >/dev/null || cargo install --path crates/mzlab-cli\n";
    if !names.is_empty() {
        s += &format!("# Files expected in MZLAB_IN: {}\n", names.join(", "));
    }
    s += &format!("MZLAB_IN='{FOLDER_PLACEHOLDER}'\nRUN=\"mzlab-run-$(date +%Y%m%d-%H%M%S)\"\nmkdir -p \"$RUN\"\n");
    // `plan_json` ends in a newline already
    s += &format!("cat > \"$RUN/plan.json\" <<'{d}'\n{text}{d}\n");
    s += "mzlab-cli flow run \"$RUN/plan.json\" --in \"$MZLAB_IN\" --out \"$RUN/out\"\n";
    s
}

/// `command` of the `mzflow/2` text `body` (the route `api/flow/command` of the page and `mzlab-cli flow command`).
pub fn route(body: &str) -> Result<String> {
    Ok(command(&parse(body.as_bytes())?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAN: &str =
        r#"{"format":"mzflow/2","inputs":[{"name":"a b.mzML"},{"name":"x\ny.mzML"}],"steps":[]}"#;

    #[test]
    fn the_script_holds_the_plan_in_a_quoted_here_document() {
        let s = route(PLAN).unwrap();
        assert!(s.contains("<<'MZFLOW'\n{\"format\":\"mzflow/2\""));
        assert!(s.ends_with("--out \"$RUN/out\"\n"));
        // the check for the program comes first and the folder is quoted once, in single quotes
        assert!(s.contains(
            "command -v mzlab-cli >/dev/null || cargo install --path crates/mzlab-cli\n"
        ));
        assert!(s.contains(&format!("MZLAB_IN='{FOLDER_PLACEHOLDER}'\n")));
        assert!(s.contains("RUN=\"mzlab-run-$(date +%Y%m%d-%H%M%S)\"\n"));
        // a name with a newline cannot leave its comment line
        assert!(s.contains("# Files expected in MZLAB_IN: a b.mzML, x?y.mzML\n"));
    }

    #[test]
    fn the_plan_inside_is_the_canonical_text() {
        let plan = parse(PLAN.as_bytes()).unwrap();
        let s = command(&plan);
        let body = s.split("<<'MZFLOW'\n").nth(1).unwrap();
        let body = body.split("MZFLOW\n").next().unwrap();
        assert_eq!(body, plan_json(&plan));
    }

    #[test]
    fn the_delimiter_never_appears_alone_on_a_line_of_the_plan() {
        assert_eq!(delimiter("a\nMZFLOW\nb"), "MZFLOW_");
        assert_eq!(delimiter("a MZFLOW b"), "MZFLOW");
    }
}
