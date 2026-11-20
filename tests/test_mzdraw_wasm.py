"""The mzdraw .wasm (crates/mzdraw-wasm) replays a command log to the same bytes as the native crate (its example `replay`) and as the recorded
outputs of tests/dati/mzdraw_log.json (held by crates/mzdraw/tests/replay.rs too): the document `mzdraw/1`, the molfile, the SVG and the log;
a refused request is the same keyed JSON error on both sides. The crate `mzdraw` stays publishable on its own: it depends on nothing of mzLab
but `mzlab-mol`. Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
import tomllib
from pathlib import Path

import pytest
from cargo_util import build, build_example, example_argv

ROOT = Path(__file__).resolve().parents[1]
FX = json.loads((ROOT / "tests" / "dati" / "mzdraw_log.json").read_text(encoding="utf-8"))
needs_tools = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    out = tmp_path_factory.mktemp("mzdraw")
    build("--target", "wasm32-unknown-unknown", "-p", "mzdraw-wasm", runner=run)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzdraw_wasm.wasm")
    build_example("mzdraw", "replay")
    return out


def native(request):
    return subprocess.run(example_argv("mzdraw", "replay"), cwd=ROOT, input=request, capture_output=True, encoding="utf-8")


def site(pkg, request):
    return subprocess.run(["node", "tests/mzdraw_wasm_node.js", str(pkg), request], cwd=ROOT, capture_output=True, encoding="utf-8")


@needs_tools
@pytest.mark.parametrize("case", FX["cases"], ids=lambda c: c["name"])
@pytest.mark.parametrize("out", ["mzdraw", "molfile", "svg", "log"])
def test_a_command_log_replays_to_the_same_bytes_in_the_site_and_natively(pkg, case, out):
    request = json.dumps({**case["request"], "out": out})
    a, b = site(pkg, request), native(request)
    assert a.returncode == 0 and b.returncode == 0, (a.stderr, b.stderr)
    assert a.stdout == b.stdout == case["out"][out]


@needs_tools
@pytest.mark.parametrize("case", FX["refused"], ids=lambda c: c["error_key"])
def test_a_refused_request_is_the_same_keyed_error_on_both_sides(pkg, case):
    request = json.dumps(case["request"])
    a, b = site(pkg, request), native(request)
    assert a.returncode == 1 and b.returncode == 1
    assert a.stderr == b.stderr and json.loads(a.stderr)["error_key"] == case["error_key"]


def test_the_editor_crate_depends_only_on_the_molecule_record():
    deps = tomllib.loads((ROOT / "crates" / "mzdraw" / "Cargo.toml").read_text(encoding="utf-8"))["dependencies"]
    assert sorted(deps) == ["libm", "mzlab-mol", "serde", "serde_json"]
    # the bridge needs mzLab only for the structure check, behind a feature that a standalone build leaves out
    wasm = tomllib.loads((ROOT / "crates" / "mzdraw-wasm" / "Cargo.toml").read_text(encoding="utf-8"))
    assert wasm["features"]["mzlab"] == ["dep:mzlab-feat"] and wasm["dependencies"]["mzlab-feat"]["optional"] is True


def test_the_page_is_hidden_and_never_in_the_teaching_site():
    web = ROOT / "mzlab" / "web"
    assert 'content="noindex,nofollow"' in (web / "draw2" / "index.html").read_text(encoding="utf-8")
    linked = [f.name for f in web.glob("*.js") if "draw2/" in f.read_text(encoding="utf-8")] + [f.name for f in web.glob("*.html") if "draw2/" in f.read_text(encoding="utf-8")]
    assert linked == [], linked          # no page or script of the site leads to it
    assert 'static / "draw2"' in (ROOT / "tools" / "build_site.py").read_text(encoding="utf-8")
