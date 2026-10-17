"""The e2e that build their own copy of the site (tools/build_site.py --out) need the Rust engine in it: the page has no other engine. `add_wasm` copies the
.wasm built in site/static/wasm (the CI and the local runs build it there) or, without it, builds it. mzFinder has no other engine either: the modules of
site/static/wasm-feat (tools/build_feat_wasm.sh) are copied too when they exist."""
import shutil, subprocess
from pathlib import Path
from lib import ROOT


def add_wasm(site: Path) -> None:
    src = ROOT / "site" / "static" / "wasm"
    dst = Path(site) / "static" / "wasm"
    if not (src / "mzlab_wasm.js").exists():
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm"], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(src), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm"], cwd=ROOT, check=True)
    shutil.copytree(src, dst, dirs_exist_ok=True)
    feat = ROOT / "site" / "static" / "wasm-feat"
    if (feat / "manifest.json").exists():
        shutil.copytree(feat, Path(site) / "static" / "wasm-feat", dirs_exist_ok=True)
