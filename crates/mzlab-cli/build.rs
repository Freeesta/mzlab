//! Build identity of the CLI: the full commit this binary is built from, for `flow run` (`prov/1` agent) and `flow record`.
//! `MZLAB_COMMIT` wins, else `git rev-parse HEAD`, else none. A per-crate `rustc-env`: the engine and the `.wasm` never see it.

use std::path::PathBuf;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    (out.status.success() && !text.trim().is_empty()).then(|| text.trim().to_string())
}

fn is_commit(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn main() {
    println!("cargo:rerun-if-env-changed=MZLAB_COMMIT");
    println!("cargo:rerun-if-changed=build.rs");
    let forced = std::env::var("MZLAB_COMMIT").ok().map(|c| c.to_lowercase());
    let commit = forced
        .filter(|c| is_commit(c))
        .or_else(|| git(&["rev-parse", "HEAD"]).filter(|c| is_commit(c)));
    // a new commit or checkout moves HEAD or the ref it names
    let mut watch = vec![git(&["rev-parse", "--git-path", "HEAD"])];
    if let Some(r) = git(&["symbolic-ref", "-q", "HEAD"]) {
        watch.push(git(&["rev-parse", "--git-path", &r]));
    }
    watch.push(git(&["rev-parse", "--git-path", "packed-refs"]));
    for p in watch.into_iter().flatten() {
        let path = PathBuf::from(&p);
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir().map(|d| d.join(&path)).unwrap_or(path)
        };
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rustc-env=MZLAB_CLI_COMMIT={}", commit.unwrap_or_default());
}
