//! Native twin of the `.wasm` export `replay`, for the parity test: reads a request on stdin, writes the output on stdout; a refusal
//! goes to stderr as `{"error_key", "params": {"id"}}` with exit code 1.

use std::io::Read;

fn main() {
    let mut request = String::new();
    if std::io::stdin().read_to_string(&mut request).is_err() {
        std::process::exit(2);
    }
    match mzdraw::replay::replay(&request) {
        Ok(out) => print!("{out}"),
        Err(e) => {
            eprint!(
                "{{\"error_key\":\"{}\",\"params\":{{\"id\":{}}}}}",
                e.key, e.id
            );
            std::process::exit(1);
        }
    }
}
