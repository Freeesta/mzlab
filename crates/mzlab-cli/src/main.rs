//! `mzlab-cli`: the logic is in `lib.rs` (the tests call it without starting a process).
//! A panic (the release profile aborts) still ends as an answer with exit 7 and, under `--json`, one `error/1`, never a bare signal 134.

use std::process::ExitCode;

fn answer(o: &mzlab_cli::Outcome) -> u8 {
    eprint!("{}", o.stderr);
    mzlab_cli::error_text::write_stdout(o, &mut std::io::stdout().lock())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let for_panic = args.clone();
    std::panic::set_hook(Box::new(move |info| {
        let said = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        let at = info
            .location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let code = answer(&mzlab_cli::error_text::panic_outcome(
            &for_panic,
            &format!("{said}{at}"),
        ));
        std::process::exit(i32::from(code));
    }));
    // debug builds only: lets the process test of the hook panic on purpose
    if cfg!(debug_assertions) && std::env::var_os("MZLAB_TEST_PANIC").is_some() {
        panic!("panic on purpose");
    }
    let o = mzlab_cli::execute(&args);
    ExitCode::from(answer(&o))
}
