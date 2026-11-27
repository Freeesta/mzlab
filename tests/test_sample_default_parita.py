"""The page chooser's first ticks are the same from the engine .wasm (`sample_default`, through Node) and from `mzlab-cli sample select --default`. `--sample LIST` itself is CLI only
(the dialog has no text box); its outputs and exit codes are tested in `crates/mzlab-cli/src/sample_select_tests.rs`.
Synthetic data only (tools/wiff_grande.py). python3 is part of the test (it fails without it); skipped only without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli_argv, run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("sample_default")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    build("-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(d / "big"), "--samples", "3", "--scans", "40", "--points", "50"], check=True, capture_output=True)
    return d


DEFAULT = """const w = require(process.argv[2]);
console.log(w.sample_default(Uint32Array.from(JSON.parse(process.argv[3])), Float64Array.from(JSON.parse(process.argv[4])), Uint32Array.from(JSON.parse(process.argv[5])), Number(process.argv[6])));"""


def test_the_default_ticks_are_the_same_from_the_site_and_the_cli(work):
    """`sample_default` (site) and `sample select --default` (CLI) tick all of a small set and none of a set that does not fit the budget."""
    (work / "d.js").write_text(DEFAULT, encoding="utf-8")
    sized = subprocess.run(cli_argv("sample", "list", str(work / "big" / "grande.wiff"), "--json"), cwd=ROOT, capture_output=True, encoding="utf-8", check=True)
    smp = json.loads(sized.stdout)["files"][0]["samples"]
    ids, sizes = [x["id"] for x in smp], [x.get("size", 0) for x in smp]
    for mb in (1024, 0.0001, 0):                   # 0 is the default budget: the same answer from both
        site_out = subprocess.run(["node", str(work / "d.js"), str(work / "pkg" / "mzlab_wasm.js"), json.dumps(ids), json.dumps(sizes), "[]", str(mb)], cwd=ROOT, capture_output=True, encoding="utf-8", check=True).stdout.strip()
        cli_out = subprocess.run(cli_argv("sample", "select", str(work / "big" / "grande.wiff"), "--default", "--budget-mb", str(mb), "--json"), cwd=ROOT, capture_output=True, encoding="utf-8", check=True).stdout.strip()
        assert site_out == cli_out, (mb, site_out, cli_out)
        assert json.loads(cli_out)["samples"] == ([] if mb == 0.0001 else ids), (mb, cli_out)       # a tiny budget ticks nothing, the real and the default ones tick the small set


def test_a_sample_that_acquired_nothing_is_not_ticked_on_the_site_and_in_the_cli(work):
    """`sample_default(.., empty, ..)` (site) and `sample select --default` (CLI) leave the empty sample of pair `tipi` unticked and tick the MS and the SIM sample."""
    (work / "d.js").write_text(DEFAULT, encoding="utf-8")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_tipi.py"), str(work / "ref")], check=True, capture_output=True)
    wiff = work / "ref" / "tipi.wiff"
    sized = subprocess.run(cli_argv("sample", "list", str(wiff), "--json"), cwd=ROOT, capture_output=True, encoding="utf-8", check=True)
    smp = json.loads(sized.stdout)["files"][0]["samples"]
    ids, sizes = [x["id"] for x in smp], [x.get("size", 0) for x in smp]
    empty = [x["id"] for x in smp if x.get("why") == "empty"]
    assert empty == [3]
    site_out = subprocess.run(["node", str(work / "d.js"), str(work / "pkg" / "mzlab_wasm.js"), json.dumps(ids), json.dumps(sizes), json.dumps(empty), "0"], cwd=ROOT, capture_output=True, encoding="utf-8", check=True).stdout.strip()
    cli_out = subprocess.run(cli_argv("sample", "select", str(wiff), "--default", "--json"), cwd=ROOT, capture_output=True, encoding="utf-8", check=True).stdout.strip()
    assert site_out == cli_out == '{"schema":"sample-select/1","samples":[1,2]}', (site_out, cli_out)
