"""The ion scale of the integrations table in the site: the lazy .wasm `manions` (mzlab-manions-wasm, through Node: `manual_table_ions(files, windows, sums, ions)` and
`manual_ion_choices(files, ions)`, the calls rust-manions.js makes) writes byte for byte the text `mzlab-cli peak table --ions` writes for the same traces and request, for
each scan type (ems, epi, q1, sim, other); the module `rotte` (`manual_table`) is untouched. Synthetic counts (20 cps per ion), no files of the lab.
Skipped on a machine without cargo, wasm-bindgen or node; in CI (GITHUB_ACTIONS) those are required and a missing one fails. Run by the `rest` shards of test.yml."""
import json
import os
import shutil
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")

NODE = """const [dir, filesPath, windowsPath, ionsPath, mode] = process.argv.slice(2);
const m = require(dir + '/mzlab_manions_wasm.js'), fs = require('fs');
const t = p => fs.readFileSync(p, 'utf8');
const out = mode === 'choices' ? m.manual_ion_choices(t(filesPath), t(ionsPath)) : m.manual_table_ions(t(filesPath), t(windowsPath), true, t(ionsPath));
process.stdout.write(out);"""

WINDOWS = '[{"file":0,"tr":0,"a":1.7,"b":2.3},{"file":1,"tr":0,"a":1.7,"b":2.3}]'

# the sample doc each CLI request implies, and the CLI flags that give it
CASES = {
    "ems": (["--scan-type", "ems", "--set", "fill_ms=100"], '"scan_type":"ems","scan_from":"scan type given","fill_ms":100'),
    "epi": (["--scan-type", "epi", "--set", "fill_ms=250"], '"scan_type":"epi","scan_from":"scan type given","fill_ms":250'),
    "q1": (["--scan-type", "q1", "--set", "scan_s=2,step_da=0.1,mass_lo=100,mass_hi=300"],
           '"scan_type":"q1","scan_from":"scan type given","scan_s":2,"step_da":0.1,"mass_lo":100,"mass_hi":300'),
    "sim": (["--scan-type", "sim", "--set", "dwell_ms=50"], '"scan_type":"sim","scan_from":"scan type given","dwell_ms":50'),
    "other": (["--scan-type", "other"], '"scan_type":"other","scan_from":"scan type given"'),
}


def trace(scale):
    t, y, s = [], [], 3
    for i in range(400):
        x = i * 0.01
        s = (s * 6364136223846793005 + 1442695040888963407) % (1 << 64)
        bg = (s >> 33) % 3
        peak = round(40.0 * scale * 2.718281828459045 ** (-(((x - 2.0) / 0.1) ** 2)))
        t.append(f"{x:.2f}")
        y.append(str((bg + peak) * 20))
    return '{"q1":100.1,"rt":[%s],"y":[%s]}' % (",".join(t), ",".join(y))


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    w = tmp_path_factory.mktemp("manions")
    files = '[{"name":"a.mzML","transitions":[%s]},{"name":"b.mzML","transitions":[%s]}]' % (trace(1.0), trace(2.0))
    (w / "files.json").write_text(files, encoding="utf-8")
    (w / "windows.json").write_text(WINDOWS, encoding="utf-8")
    build("-p", "mzlab-cli")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(w / "pkg"),
        str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-manions-wasm")))
    (w / "run.js").write_text(NODE, encoding="utf-8")
    return w


def wasm(work, ions, mode="table"):
    (work / "ions.json").write_text(ions, encoding="utf-8")
    p = run("node", str(work / "run.js"), str(work / "pkg"), str(work / "files.json"), str(work / "windows.json"), str(work / "ions.json"), mode)
    return p.stdout


@pytest.mark.parametrize("kind", list(CASES))
def test_the_wasm_table_is_the_cli_table_byte_for_byte(work, kind):
    flags, member = CASES[kind]
    ions = '{"samples":[{%s,"traces":[]},{%s,"traces":[]}]}' % (member, member)
    p = cli("peak", "table", "--traces", str(work / "files.json"), "--windows", str(work / "windows.json"), "--sums", "--ions", *flags)
    got = wasm(work, ions)
    assert got == p.stdout, kind
    head = got.splitlines()[0].split(",")
    assert head[-7:] == ["area_ions", "height_ions", "noise_ions", "sn_ions", "ions_per_cps", "ion_source", "ion_scan_type"]
    assert got.splitlines()[1].split(",")[-1] == kind


def test_the_choices_name_each_sample_and_the_cps_table_is_untouched(work):
    flags, member = CASES["sim"]
    ions = '{"samples":[{%s},{%s}]}' % (member, member)
    choices = json.loads(wasm(work, ions, "choices"))
    assert [c["sample"] for c in choices] == ["a", "b"]
    assert all(c["scan_type"] == "sim" and c["facts"] == {"dwell_ms": 50} for c in choices)
    plain = cli("peak", "table", "--traces", str(work / "files.json"), "--windows", str(work / "windows.json"), "--sums").stdout
    with_ions = wasm(work, ions)
    assert all(b.startswith(a) for a, b in zip(plain.splitlines(), with_ions.splitlines()))


def test_an_unknown_scan_type_is_the_engines_error(work):
    out = wasm(work, '{"samples":[{"scan_type":"qqq"}]}')
    assert out.startswith("E{") and "err.manint.ions" in out
