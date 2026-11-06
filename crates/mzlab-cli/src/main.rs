//! `mzlab-cli`: the logic is in `lib.rs` (the tests call it without starting a process).

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let o = mzlab_cli::execute(&args);
    eprint!("{}", o.stderr);
    let code = mzlab_cli::testo_errore::write_stdout(&o, &mut std::io::stdout().lock());
    ExitCode::from(code)
}
