"""Builds the site WITHOUT Pyodide and its Rust engine (.wasm) into site/, the page the e2e tests of tests_e2e/lib.py `Run` serve.

    python3 tools/prepara_sito.py

`tools/build_site.py --no-pyodide`, then `cargo build` of mzlab-wasm for wasm32 and `wasm-bindgen` into site/static/wasm (the same two commands the site
tests run when the engine is missing). Cargo skips what is up to date; the wasm-bindgen version must be the one of Cargo.lock.
"""
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WASM = ROOT / "target" / "wasm32-unknown-unknown" / "release" / "mzlab_wasm.wasm"


def run(cmd: list[str]) -> None:
    subprocess.run(cmd, cwd=ROOT, check=True)


def bindgen_version() -> str:
    m = re.search(r'name = "wasm-bindgen"\nversion = "([^"]+)"', (ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    return m.group(1) if m else ""


def main() -> int:
    if not shutil.which("cargo"):
        print("cargo not found: the e2e tests need the Rust toolchain (rustup show)")
        return 1
    run([sys.executable, "tools/build_site.py", "--no-pyodide"])
    run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm"])
    have = subprocess.run(["wasm-bindgen", "--version"], capture_output=True, text=True).stdout if shutil.which("wasm-bindgen") else ""
    want = bindgen_version()
    if want and want not in have:
        run(["cargo", "install", "wasm-bindgen-cli", "--version", want, "--locked", "--force"])
    run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", "site/static/wasm", str(WASM)])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
