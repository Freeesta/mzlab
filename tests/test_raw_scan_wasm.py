"""`raw scan` in the site: the lazy .wasm `rawscan` (mzlab-rawscan-wasm, through Node: `raw_scan(size, read, pol, query)`, the call rust-raw.js makes for the route
`/api/rawscan` over the .mzidx of OPFS) gives the same members, summary and rows, byte for byte, as `mzlab-cli raw scan` on the synthetic files of
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
const { raw_scan } = require(dir + '/mzlab_rawscan_wasm.js'), fs = require('fs');
const b = fs.readFileSync(idxPath), read = (off, len) => new Uint8Array(b.subarray(off, off + len));
const out = queries.map(q => { try { return { s: raw_scan(b.length, read, Number(pol), q) }; } catch (x) { return { e: String(x) }; } });
process.stdout.write(JSON.stringify(out));"""
# the CLI flags and the route query of the same question (the FS file has its signal between 6 and 8 min)
ASKS = [
    (["--rt", "6,8"], "rt=6,8"),
    (["--rt", "6,8", "--bin", "0.1", "--top", "5"], "rt=6,8&bin=0.1&top=5"),
    (["--rt", "13.5,16", "--side", "1", "--gap", "0.1", "--mz-range", "200,400"], "rt=13.5,16&side_min=1&gap_min=0.1&mz_range=200,400"),
    (["--rt", "6,8", "--set", "min_z=2,min_net=0.5,alpha=0.01"], "rt=6,8&min_z=2&min_net=0.5&alpha=0.01"),
    (["--rt", "6,8", "--bin", "0.1", "--mz-offset", "0.1"], "rt=6,8&bin=0.1&mz_offset=0.1"),
]
# questions both refuse: the same error key
BAD = [
    (["--rt", "8,6"], "rt=8,6", "err.raw.scan.param"),
    (["--rt", "6,8", "--bin", "0.2"], "rt=6,8&bin=0.2", "err.raw.scan.param"),
    (["--rt", "6,8", "--channel", "tic"], "rt=6,8&channel=tic", "err.raw.scan.channel"),
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
    w = tmp_path_factory.mktemp("raw_scan")
    r = np.random.default_rng(5)
    d.full_scan(w / "FS.mzML", 3.0, r)
    d.full_scan_profile(w / "FS_profile.mzML", 3.0, r)
    d.ms2(w / "MS2.mzML", r, 15)
    d.mixed_polarity(w / "POL.mzML", r)
    run(sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(w / "wiff"))
    shutil.copy(w / "wiff" / "two.wiff", w / "two.wiff")
    shutil.copy(w / "wiff" / "two.wiff.scan", w / "two.wiff.scan")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(w / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-rawscan-wasm")))
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
    """The wasm's members, summary and rows are inside the CLI document, byte for byte."""
    assert "s" in got, f"{name} {query}: {got}"
    route = got["s"]
    # the CLI tags the summary and each row with `file` and `sample`: the bytes after that tag are the route's, not a re-serialisation
    members = route[1:route.index(',"summary":{')]
    assert members in doc, f"{name} {flags} {query}:\nwasm {route[:400]}\ncli  {doc[:800]}"
    summary = route[route.index(',"summary":{') + 12:route.index(',"rows":[')]
    assert summary[1:] in doc, f"{name} {flags} {query}:\nwasm {summary[:400]}"
    body = route[route.index('"rows":[') + 8:-2]
    for row in (re.split(r',(?=\{"bin":)', body) if body else []):
        assert row[1:] in doc, f"{name} {flags} {query}:\nwasm {row[:400]}"
    rows = json.loads(route)["rows"]
    assert rows, route[:300]
    assert rows[0]["verdict"] in ("peak", "trace", "nothing", None)


@pytest.mark.parametrize("name", ["FS.mzML", "FS_profile.mzML", "MS2.mzML", "POL.mzML"])
def test_the_wasm_route_answers_the_members_and_rows_of_the_cli(work, name):
    for (flags, query), got in zip(ASKS, answers(work, name, [q for _, q in ASKS])):
        same_bytes(name, flags, query, got, cli("raw", "scan", str(work / name), *flags).stdout)


@pytest.mark.parametrize("sign,pol", [("+", 1), ("-", -1)])
def test_a_polarity_is_the_same_section_in_the_wasm_and_the_cli(work, sign, pol):
    """`--polarity +|-` is the engine's `set_polarity` 1 | -1: the alternating-polarity file answers each section with the bytes of the CLI."""
    asks = ASKS[:2]
    for (flags, query), got in zip(asks, answers(work, "POL.mzML", [q for _, q in asks], pol=pol)):
        doc = cli("raw", "scan", str(work / "POL.mzML"), *flags, "--polarity", sign).stdout
        same_bytes("POL.mzML", [*flags, "--polarity", sign], query, got, doc)
    # the two sections are different questions: the answers differ
    plus, minus = (answers(work, "POL.mzML", [asks[0][1]], pol=p)[0]["s"] for p in (1, -1))
    assert plus != minus


@pytest.mark.parametrize("sample", ["1", "2"])
def test_a_sample_of_a_multi_sample_file_is_the_same_in_the_wasm_and_the_cli(work, sample):
    """`--sample ID` of a two-sample .wiff (`tools/wiff_riferimento.py`: a scan sample, an MRM sample): the index of that sample answers as the CLI does."""
    for (flags, query), got in zip(ASKS[:2], answers(work, "two.wiff", [q for _, q in ASKS[:2]], sample=sample)):
        doc = cli("raw", "scan", str(work / "two.wiff"), *flags, "--sample", sample).stdout
        same_bytes("two.wiff", [*flags, "--sample", sample], query, got, doc)


def test_a_refused_question_has_the_same_error_key(work):
    for (flags, query, key), got in zip(BAD, answers(work, "FS.mzML", [q for _, q, _ in BAD])):
        assert json.loads(got["e"])["error_key"] == key, got
        r = cli("raw", "scan", str(work / "FS.mzML"), *flags, runner=lambda *a: subprocess.run(a, capture_output=True, encoding="utf-8"))
        doc = json.loads(r.stdout)
        assert r.returncode == 2 and doc["schema"] == "error/1" and doc["error_key"] == key, r.stdout[:300]
