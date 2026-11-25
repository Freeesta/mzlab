"""`samples_list/1` of a SCIEX `.wiff` pair is the same bytes from the engine .wasm (`native_samples(.., true)`, through Node) and from `mzlab-cli sample list --json`,
on every synthetic pair of tools/wiff_riferimento.py and on a pair of tools/wiff_grande.py (the app's native route is pinned to the same text by mzlab-desktop natives_tests.rs).
Synthetic data only. python3 is part of the test (it fails without it). Skipped on a machine without cargo, wasm-bindgen or node; in CI (GITHUB_ACTIONS) those are
required and a missing one fails. The app's listing alone adds `consistency` (the site has no file times) when a pair disagrees with itself: the test strips it and compares the rest."""
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli_argv, run

ROOT = Path(__file__).resolve().parents[1]
TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there the wasm leg must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


LIST = """const w = require(process.argv[2]), fs = require("fs"), names = process.argv.slice(3), bs = names.map(n => fs.readFileSync(n));
console.log(w.native_samples(names.map(n => n.split("/").pop()), new Float64Array(bs.map(b => b.length)), (i, o, n) => bs[i].subarray(o, o + n), true));"""


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("samples_list")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    build("-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(d / "ref")], check=True, capture_output=True)
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(d / "big"), "--samples", "3", "--scans", "40", "--points", "50"], check=True, capture_output=True)
    (d / "l.js").write_text(LIST, encoding="utf-8")
    return d


def site(work, wiff: Path):
    """(exit code, stdout) of the engine .wasm listing of the pair, through Node."""
    p = subprocess.run(["node", str(work / "l.js"), str(work / "pkg" / "mzlab_wasm.js"), str(wiff), str(wiff) + ".scan"], cwd=ROOT, capture_output=True, encoding="utf-8")
    return p.returncode, p.stdout.strip()


def app(wiff: Path):
    """(exit code, stdout, stderr) of `mzlab-cli sample list --json`."""
    p = subprocess.run(cli_argv("sample", "list", str(wiff), "--json"), cwd=ROOT, capture_output=True, encoding="utf-8")
    return p.returncode, unwrap(p.stdout.strip()), p.stderr


def unwrap(text: str) -> str:
    """The one `sample-list/1` document holds each file's own document in `files`: the single file's bytes, as the site writes them."""
    head, tail = '{"schema":"sample-list/1","files":[', "]}"
    return text[len(head):-len(tail)] if text.startswith(head) and text.endswith(tail) else text


def same_but_refused(site_line: str, app_line: str):
    """True when both listings agree, except on the samples the engine refuses: a variant (MS3, TOF, ...) is read by the `sciex` module and the app, whose reader opens it,
    so only the id and the name of such a sample are compared (the engine's refusal is its own, and the page then hands the pair to the module)."""
    a, b = json.loads(site_line), json.loads(app_line)
    sa, sb = a.pop("samples"), b.pop("samples")
    if a != b or len(sa) != len(sb):
        return False
    return all((x["id"], x["name"]) == (y["id"], y["name"]) if x.get("why") == "refused" else x == y for x, y in zip(sa, sb))


def test_the_site_and_the_app_list_every_synthetic_pair_with_the_same_bytes(work):
    pairs = sorted(work.glob("*/*.wiff"))
    assert len(pairs) > 10, pairs
    listed = 0
    for wiff in pairs:
        code, out = site(work, wiff)
        rc, text, err = app(wiff)
        if code == 0:
            assert json.loads(out)["schema"] == "samples_list/1", wiff
            # the app alone checks the pair against its own index (`consistency`, only on a mismatch): the site has no file times, so its text stays the index's
            text, found = re.subn(r',"consistency":\{[^{}]*\}\}$', "}", text)
            assert found == wiff.name.startswith("unindexed"), (wiff, found)
            assert same_but_refused(out, text), wiff
            if '"why":"refused"' not in out:
                assert text == out, wiff  # the line of the app is the very text of the site
            assert rc in (0, 1, 8), (wiff, rc, err)  # 1 = some sample refused or a partial copy; 8 = a partial copy modified a moment ago
            listed += 1
        else:
            # a pair no reader takes: the app answers with an error line, never a listing
            assert rc in (1, 4) and '"schema":"error/1"' in text and "samples_list/1" not in text, (wiff, rc, text)
        assert "listing " in err, (wiff, err)  # progress goes to stderr
    assert listed > 10


def test_the_probe_adds_the_size_of_the_scan_file_and_a_cut_copy_exits_1(work):
    rc, text, _ = app(work / "big" / "grande.wiff")
    assert rc == 0 and "probe" not in json.loads(text)
    p = subprocess.run(cli_argv("sample", "list", str(work / "big" / "grande.wiff"), "--json", "--probe"), cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode == 0 and json.loads(unwrap(p.stdout.strip()))["probe"]["status"] == "ok", p.stdout
    cut = work / "cut"
    cut.mkdir()
    shutil.copy(work / "big" / "grande.wiff", cut / "grande.wiff")
    data = (work / "big" / "grande.wiff.scan").read_bytes()
    (cut / "grande.wiff.scan").write_bytes(data[:-500])
    fresh = subprocess.run(cli_argv("sample", "list", str(cut / "grande.wiff"), "--json", "--probe"), cwd=ROOT, capture_output=True, encoding="utf-8")
    assert fresh.returncode == 8 and '"still_writing":true' in fresh.stdout, (fresh.returncode, fresh.stdout)  # written a moment ago: it may still be growing
    old = os.stat(cut / "grande.wiff").st_mtime - 3600
    for n in ("grande.wiff", "grande.wiff.scan"):
        os.utime(cut / n, (old, old))  # an hour old: a finished, partial copy
    p = subprocess.run(cli_argv("sample", "list", str(cut / "grande.wiff"), "--json", "--probe"), cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode == 1 and json.loads(unwrap(p.stdout.strip()))["probe"]["status"] == "short", (p.returncode, p.stdout)
    missing = subprocess.run(cli_argv("sample", "list", str(work / "nothing.wiff"), "--json"), cwd=ROOT, capture_output=True, encoding="utf-8")
    assert missing.returncode == 4 and "error/1" in missing.stdout
    usage = subprocess.run(cli_argv("sample", "list", "--json"), cwd=ROOT, capture_output=True, encoding="utf-8")
    assert usage.returncode == 2
