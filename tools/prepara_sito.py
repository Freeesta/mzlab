"""Builds the site WITHOUT Pyodide and its Rust engine (.wasm) into site/, the page the e2e tests of tests_e2e/lib.py `Run` serve.

    python3 tools/prepara_sito.py

`tools/build_site.py`, then `cargo build` of mzlab-wasm for wasm32 and `wasm-bindgen` into site/static/wasm (the same two commands the site
tests run when the engine is missing), the library engine mzlab-lib-wasm into site/static/wasm-lib and the reader modules with their manifest into
site/static/wasm-lettori (tools/build_lettori_wasm.sh, the script of pages.yml) as the published site has them. Nothing is done when site/ is newer than its sources; cargo skips what is up to date; the wasm-bindgen version must be the one of Cargo.lock.
"""
import hashlib
import json
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
    st = ROOT / "site" / "static"
    if not out.exists() or not (ROOT / "site" / "index.html").exists() or not (st / "wasm-lib" / "mzlab_lib_wasm.js").exists() or not (st / "wasm-lettori" / "manifest.json").exists():
        return False
    t = out.stat().st_mtime
    sources = [ROOT / "Cargo.lock", *(ROOT / "mzlab" / "web").rglob("*"), *(ROOT / "crates").rglob("*.rs")]
    return all(f.stat().st_mtime <= t for f in sources if f.is_file())


def add_routes() -> None:
    """The lazy modules `rotte` (statistics, molecules, Data and Flow routes, crates/mzlab-rotte-wasm) and `ticfondo` (api/tic_background) into
    site/static/wasm-feat with their manifest entries; the rest of the manifest is the CI's."""
    mp = ROOT / "site" / "static" / "wasm-feat" / "manifest.json"
    m = json.loads(mp.read_text(encoding="utf-8")) if mp.exists() else {}
    for key in ("rotte", "ticfondo"):
        mod = mp.parent / key
        run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(mod), str(WASM.with_name(f"mzlab_{key}_wasm.wasm"))])
        m[key] = {"js": f"{key}/mzlab_{key}_wasm.js", "wasm": f"{key}/mzlab_{key}_wasm_bg.wasm", "sha256": hashlib.sha256((mod / f"mzlab_{key}_wasm_bg.wasm").read_bytes()).hexdigest()}
    mp.write_text(json.dumps(m, indent=1), encoding="utf-8")


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
    run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm", "-p", "mzlab-rotte-wasm", "-p", "mzlab-ticfondo-wasm"], build_env("wasm32-unknown-unknown", commit=False))
    have = subprocess.run(["wasm-bindgen", "--version"], capture_output=True, text=True).stdout if shutil.which("wasm-bindgen") else ""
    want = bindgen_version()
    if want and want not in have:
        run(["cargo", "install", "wasm-bindgen-cli", "--version", want, "--locked", "--force"])
    run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", "site/static/wasm", str(WASM)])
    add_routes()
    # its own cargo run: built with mzlab-wasm, unified features could change the engine's bytes
    run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lib-wasm"], build_env("wasm32-unknown-unknown", commit=False))
    run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", "site/static/wasm-lib", str(WASM.with_name("mzlab_lib_wasm.wasm"))])
    # the readers (one .wasm per family of formats) and their manifest: without them a .wiff, .raw or .d asks the page for a file the site does not have
    run(["bash", "tools/build_lettori_wasm.sh", "site/static"], build_env("wasm32-unknown-unknown", commit=False))
    run([sys.executable, "tools/build_site.py", "--build-json"])      # the .wasm is in place now: build.json names its SHA-256
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
