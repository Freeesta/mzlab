"""The Libro converter in WebAssembly (crates/mzlab-libro-wasm) gives, through Node, the same bytes as the committed HTML of every chapter of mzlab/web/teoria/src
(and so as tools/libro). Skipped without cargo, the wasm32 target, wasm-bindgen or node."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
TEORIA = ROOT / "mzlab" / "web" / "teoria"
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def converted(tmp_path_factory):
    pkg, out = tmp_path_factory.mktemp("pkg"), tmp_path_factory.mktemp("out")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-libro-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_libro_wasm.wasm")
    log = run("node", "tests/libro_wasm_node.js", str(pkg), str(TEORIA / "src"), str(out), str(ROOT / "tools/libro/importati.txt"))
    return out, log


def test_every_chapter_is_the_committed_html(converted):
    out, _ = converted
    chapters = sorted((TEORIA / "src").glob("*.md"))
    assert len(chapters) >= 23
    for md in chapters:
        name = md.stem + ".html"
        assert (out / name).read_text(encoding="utf-8") == (TEORIA / name).read_text(encoding="utf-8"), name


def test_version_and_error_message(converted):
    _, log = converted
    assert log.splitlines()[0] == "mzlab-libro/1"
    assert "error x.md:1: the section has no heading" in log
