"""`raw isotopes` in the site: the lazy .wasm `rawiso` (mzlab-rawiso-wasm, through Node: `raw_isotopes(size, read, pol, query)`, the call rust-raw.js makes for the route
`/api/rawiso` over the .mzidx of OPFS) gives the same members and `result`, byte for byte, as `mzlab-cli raw isotopes` on the synthetic files of
`tools/dati_sintetici.py` (full scan, profile, MS2, alternating polarity, a two-sample .wiff); a question the route refuses has the same `error_key` in both. The
desktop answers the same route (`crates/mzlab-desktop/tests/parity_routes.rs`). Skipped on a machine without cargo, wasm-bindgen, node or numpy; in CI
(GITHUB_ACTIONS) those are required and a missing one fails. Run by the `rest` shards of test.yml."""
import json
import os
import re
import shutil
import subprocess
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

NODE = """const [dir, idxPath, pol, ...queries] = process.argv.slice(2);
const { raw_isotopes } = require(dir + '/mzlab_rawiso_wasm.js'), fs = require('fs');
const b = fs.readFileSync(idxPath), read = (off, len) => new Uint8Array(b.subarray(off, off + len));
const out = queries.map(q => { try { return { s: raw_isotopes(b.length, read, Number(pol), q) }; } catch (x) { return { e: String(x) }; } });
process.stdout.write(JSON.stringify(out));"""
# the CLI flags and the route query of the same question (the FS file has its 364.07 peak at 14.3 min)
ASKS = [
    (["--mz", "364", "--rt", "13.9,14.7"], "mz=364&rt=13.9,14.7"),
    (["--mz", "364", "--rt", "13.9,14.7", "--formula", "C14H12N4O2S2", "--adduct", "[M+H]+"], "mz=364&rt=13.9,14.7&formula=C14H12N4O2S2&adduct=[M%2BH]%2B"),
    (["--mz", "364", "--rt", "13.9,14.7", "--formula", "C14H12N4O2S2"], "mz=364&rt=13.9,14.7&formula=C14H12N4O2S2"),
    (["--mz", "364", "--rt", "13.9,14.7", "--set", "min_counts=50,z_ok=1.5,z_bad=4,carbon_k=3"], "mz=364&rt=13.9,14.7&min_counts=50&z_ok=1.5&z_bad=4&carbon_k=3"),
    (["--mz", "364", "--rt", "13.9,14.7", "--mz-offset", "0.1"], "mz=364&rt=13.9,14.7&mz_offset=0.1"),
    (["--mz", "364", "--rt", "13.9,14.7", "--ms-level", "2"], "mz=364&rt=13.9,14.7&ms_level=2"),
    (["--mz", "900", "--rt", "1,2"], "mz=900&rt=1,2"),
]
# questions both refuse: the same error key
BAD = [
    (["--mz", "364", "--rt", "14.7,13.9"], "mz=364&rt=14.7,13.9", "err.raw.isotopes.param"),
    (["--mz", "364", "--rt", "13.9,14.7", "--bin", "0.1"], "mz=364&rt=13.9,14.7&bin=0.1", "err.raw.isotopes.param"),
    (["--mz", "364", "--rt", "13.9,14.7", "--adduct", "[M+H]+"], "mz=364&rt=13.9,14.7&adduct=[M%2BH]%2B", "err.raw.isotopes.adductNeedsFormula"),
    (["--mz", "364", "--rt", "13.9,14.7", "--set", "z_ok=4,z_bad=2"], "mz=364&rt=13.9,14.7&z_ok=4&z_bad=2", "err.raw.isotopes.param"),
]


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    try:
        import numpy as np
    except ImportError:
        if CI:
            pytest.fail("numpy must be installed in CI: tools/dati_sintetici.py needs it")
        pytest.skip("numpy not available")
    import dati_sintetici as d
    w = tmp_path_factory.mktemp("raw_isotopes")
    r = np.random.default_rng(5)
    d.full_scan(w / "FS.mzML", 3.0, r)
    d.full_scan_profile(w / "FS_profile.mzML", 3.0, r)
    d.ms2(w / "MS2.mzML", r, 15)
    d.mixed_polarity(w / "POL.mzML", r)
    run(sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(w / "wiff"))
    shutil.copy(w / "wiff" / "two.wiff", w / "two.wiff")
    shutil.copy(w / "wiff" / "two.wiff.scan", w / "two.wiff.scan")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(w / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-rawiso-wasm")))
    build("-p", "mzlab-cli")
    (w / "run.js").write_text(NODE, encoding="utf-8")
    return w


def answers(work, name, queries, pol=0, sample=None):
    """The wasm's answers to `queries` over the index of `name` (of its sample `--sample`), with the polarity `pol` the engine's `set_polarity` takes."""
    f, cache = work / name, work / "cache"
    extra = ["--sample", sample] if sample else []
    key = json.loads(cli("raw", "cache", str(f), "--cache", str(cache), *extra).stdout)["files"][0]["key"]
    return json.loads(subprocess.run(["node", str(work / "run.js"), str(work / "pkg"), str(cache / key), str(pol), *queries], capture_output=True, encoding="utf-8", check=True).stdout)


def same_bytes(name, flags, query, got, doc):
    """The wasm's members and `result` are inside the CLI document, byte for byte."""
    assert "s" in got, f"{name} {query}: {got}"
    route = got["s"]
    # the CLI tags the result with `file` and `sample`: the bytes after that tag are the route's, not a re-serialisation
    members = route[1:route.index(',"result":{')]
    assert members in doc, f"{name} {flags} {query}:\nwasm {route[:400]}\ncli  {doc[:800]}"
    result = route[route.index(',"result":{') + 11:-1]
    assert result in doc, f"{name} {flags} {query}:\nwasm {result[:400]}"
    r = json.loads(route)["result"]
    assert r["m"]["verdict"] in ("peak", "trace", "nothing")


@pytest.mark.parametrize("name", ["FS.mzML", "FS_profile.mzML", "MS2.mzML", "POL.mzML"])
def test_the_wasm_route_answers_the_members_and_result_of_the_cli(work, name):
    for (flags, query), got in zip(ASKS, answers(work, name, [q for _, q in ASKS])):
        same_bytes(name, flags, query, got, cli("raw", "isotopes", str(work / name), *flags).stdout)


@pytest.mark.parametrize("sign,pol", [("+", 1), ("-", -1)])
def test_a_polarity_is_the_same_section_in_the_wasm_and_the_cli(work, sign, pol):
    """`--polarity +|-` is the engine's `set_polarity` 1 | -1: the alternating-polarity file answers each section with the bytes of the CLI."""
    asks = ASKS[:2]
    for (flags, query), got in zip(asks, answers(work, "POL.mzML", [q for _, q in asks], pol=pol)):
        doc = cli("raw", "isotopes", str(work / "POL.mzML"), *flags, "--polarity", sign).stdout
        same_bytes("POL.mzML", [*flags, "--polarity", sign], query, got, doc)


@pytest.mark.parametrize("sample", ["1", "2"])
def test_a_sample_of_a_multi_sample_file_is_the_same_in_the_wasm_and_the_cli(work, sample):
    """`--sample ID` of a two-sample .wiff (`tools/wiff_riferimento.py`: a scan sample, an MRM sample): the index of that sample answers as the CLI does."""
    for (flags, query), got in zip(ASKS[:2], answers(work, "two.wiff", [q for _, q in ASKS[:2]], sample=sample)):
        doc = cli("raw", "isotopes", str(work / "two.wiff"), *flags, "--sample", sample).stdout
        same_bytes("two.wiff", [*flags, "--sample", sample], query, got, doc)


def test_a_refused_question_has_the_same_error_key(work):
    for (flags, query, key), got in zip(BAD, answers(work, "FS.mzML", [q for _, q, _ in BAD])):
        assert json.loads(got["e"])["error_key"] == key, got
        r = cli("raw", "isotopes", str(work / "FS.mzML"), *flags, runner=lambda *a: subprocess.run(a, capture_output=True, encoding="utf-8"))
        doc = json.loads(r.stdout)
        assert r.returncode == 2 and doc["schema"] == "error/1" and doc["error_key"] == key, r.stdout[:300]


def test_the_peak_of_a_full_scan_has_ions_and_an_expected_pattern(work):
    """The check that the equality above is not between two empty answers: the window of the synthetic peak has the three ions and, with a formula, a pattern."""
    plain, formula = (json.loads(g["s"])["result"] for g in answers(work, "FS.mzML", [ASKS[0][1], ASKS[1][1]]))
    assert len(plain["ions"]) == 3 and plain["ions"][0]["net"] > 0 and plain["expected"] is None
    assert formula["expected"]["formula"] == "C14H12N4O2S2" and len(formula["expected"]["pattern"]) == 3
