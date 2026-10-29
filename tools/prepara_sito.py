"""Builds the site WITHOUT Pyodide and its Rust engine (.wasm) into site/, the page the e2e tests of tests_e2e/lib.py `Run` serve.

    python3 tools/prepara_sito.py

`tools/build_site.py`, then `cargo build` of mzlab-wasm for wasm32 and `wasm-bindgen` into site/static/wasm (the same two commands the site
tests run when the engine is missing). Nothing is done when site/ is newer than its sources; cargo skips what is up to date; the wasm-bindgen version must be the one of Cargo.lock.
"""
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

import build_site
import ci_toolchain

ROOT = Path(__file__).resolve().parent.parent
WASM = ROOT / "target" / "wasm32-unknown-unknown" / "release" / "mzlab_wasm.wasm"


def build_env(target: str, commit: bool) -> dict[str, str]:
    """The identity of the build for `option_env!` (`flow::build_agent`): rustc, SHA-256 of Cargo.lock, target and, for a native build, the full commit.
    The .wasm is built WITHOUT the commit: it would change its bytes at every commit (the desktop updater and the cache would download it again);
    the commit of the site is in version.js and static/build.json."""
    env = {"MZLAB_TARGET": target}
    for k, v in (("MZLAB_RUSTC", build_site.rustc_version()), ("MZLAB_CARGO_LOCK_SHA256", build_site.lock_sha256()), ("MZLAB_COMMIT", build_site.git_commit() if commit else None)):
        if v and v != "dev":
            env[k] = v
    return env


def run(cmd: list[str], env: dict[str, str] | None = None) -> None:
    subprocess.run(cmd, cwd=ROOT, check=True, env={**os.environ, **env} if env else None)


def bindgen_version() -> str:
    m = re.search(r'name = "wasm-bindgen"\nversion = "([^"]+)"', (ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    return m.group(1) if m else ""


def up_to_date() -> bool:
    """site/ and its .wasm exist and nothing they are made from is newer (a site built with mzFinder's Pyodide, as the site tests do, is kept)."""
    out = ROOT / "site" / "static" / "wasm" / "mzlab_wasm_bg.wasm"
    if not out.exists() or not (ROOT / "site" / "index.html").exists():
        return False
    t = out.stat().st_mtime
    sources = [ROOT / "Cargo.lock", *(ROOT / "mzlab" / "web").rglob("*"), *(ROOT / "crates").rglob("*.rs")]
    return all(f.stat().st_mtime <= t for f in sources if f.is_file())


def main() -> int:
    # before the tests that call cargo side by side (tools/verifica.py gives them RUSTUP_AUTO_INSTALL=0): one install, checked
    why = ci_toolchain.ensure()
    if why:
        print(f"Rust toolchain of rust-toolchain.toml is not usable: {why}")
        return 1
    if up_to_date():
        print("site/ is up to date")
        return 0
    if not shutil.which("cargo"):
        print("cargo not found: the e2e tests need the Rust toolchain (rustup show)")
        return 1
    run([sys.executable, "tools/build_site.py"])
    run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm"], build_env("wasm32-unknown-unknown", commit=False))
    have = subprocess.run(["wasm-bindgen", "--version"], capture_output=True, text=True).stdout if shutil.which("wasm-bindgen") else ""
    want = bindgen_version()
    if want and want not in have:
        run(["cargo", "install", "wasm-bindgen-cli", "--version", want, "--locked", "--force"])
    run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", "site/static/wasm", str(WASM)])
    run([sys.executable, "tools/build_site.py", "--build-json"])      # the .wasm is in place now: build.json names its SHA-256
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
