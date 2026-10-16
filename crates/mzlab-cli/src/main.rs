//! `mzlab-cli`: the logic is in `lib.rs` (the tests call it without starting a process).

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let o = mzlab_cli::execute(&args);
    let _ = std::io::stdout().write_all(&o.stdout);
    eprint!("{}", o.stderr);
    ExitCode::from(o.code)
}
