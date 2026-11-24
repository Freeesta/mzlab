"""A SCIEX .wiff dropped without its .scan: the `method/1` of a sample on the site (the `sciex` readers module, `wiff_metodo`) is the one `mzlab-cli file convert` writes into the mzML of
that sample, byte for byte, apart from `origine` (the site lists the .wiff only, the CLI the .wiff and its .scan). The file is synthetic (`metodocond` of tools/wiff_metodo_condiviso.py):
three samples that share `Method1` (a scheduled MRM experiment, its GLPTables) and a DAD on sample 2, so the extended metadata (`campi`, feature `estesi`) must be there.
The engine .wasm is built without `estesi` (its cap has no room): serving the call from it left `campi` out, which this test would catch. Skipped without cargo, wasm-bindgen or node."""
import html
import json
import re
import shutil
import subprocess
import sys

import pytest
from cargo_util import build, run, wasm_module
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("metodocond")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "mod"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", "sciex")))
    build("-p", "mzlab-cli")
    (d / "ref").mkdir()
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_metodo_condiviso.py"), str(d / "ref")], check=True, capture_output=True)
    run(str(ROOT / "target/release/mzlab-cli"), "file", "convert", str(d / "ref" / "metodocond.wiff"), "--out", str(d / "cli"), "--fresh", "--workers", "1")
    return d


def doc_of_cli(work, sample):
    head = (work / "cli" / f"metodocond-{sample}.mzML").read_bytes()[:400000].decode("utf-8")
    return html.unescape(re.search(r'name="mzLab method/1" value="([^"]*)"', head).group(1))


def doc_of_site(work, sample):
    return run("node", str(ROOT / "tests" / "wiff_metodo_node.js"), str(work / "mod"), "module", str(work / "ref" / "metodocond.wiff"), str(sample)).stdout


def without_origin(text):
    d = json.loads(text)
    d.pop("origin")
    return json.dumps(d, ensure_ascii=False, separators=(",", ":"))


@pytest.mark.parametrize("sample", [1, 2, 3])
def test_the_site_document_of_a_sample_is_the_one_of_the_cli(work, sample):
    assert without_origin(doc_of_site(work, sample)) == without_origin(doc_of_cli(work, sample))


def test_the_samples_that_share_a_method_carry_the_extended_metadata(work):
    site, cli = json.loads(doc_of_site(work, 2)), json.loads(doc_of_cli(work, 2))
    assert site["ms"] and len(site["fields"]) >= 7, list(site)
    assert site["fields"] == cli["fields"]
