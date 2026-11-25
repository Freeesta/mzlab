"""`raw xic` in the site: the lazy .wasm `rawxic` (mzlab-rawxic-wasm, through Node: `raw_xic(size, read, pol, query)`, the call rust-raw.js makes for the route
`/api/rawxic` over the .mzidx of OPFS) gives the same members and the same rows, byte for byte, as `mzlab-cli raw xic` on the synthetic files of
`tools/dati_sintetici.py` (full scan, profile, MS2, alternating polarity); a question the route refuses has the same `error_key` in both. The desktop answers
the same route (`crates/mzlab-desktop/tests/parity_routes.rs`). Skipped on a machine without cargo, wasm-bindgen, node or numpy; in CI (GITHUB_ACTIONS)
those are required and a missing one fails. Run by the `rest` shards of test.yml."""
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
const { raw_xic } = require(dir + '/mzlab_rawxic_wasm.js'), fs = require('fs');
const b = fs.readFileSync(idxPath), read = (off, len) => new Uint8Array(b.subarray(off, off + len));
const out = queries.map(q => { try { return { s: raw_xic(b.length, read, Number(pol), q) }; } catch (x) { return { e: String(x) }; } });
process.stdout.write(JSON.stringify(out));"""
# the CLI flags and the route query of the same question
ASKS = [
    (["--mz", "229.1", "--rt", "6,8", "--da", "0.3"], "mz=229.1&rt=6,8&da=0.3"),
    (["--mz", "305.3,500", "--rt", "13.5,16"], "mz=305.3,500&rt=13.5,16"),
    (["--mz", "229.1", "--rt", "6,8", "--da", "0.3", "--set", "min_sn=50,min_points=5,alpha=0.01"], "mz=229.1&rt=6,8&da=0.3&min_sn=50&min_points=5&alpha=0.01"),
    (["--mz", "229.0", "--rt", "6,8", "--da", "0.1", "--mz-offset", "0.1"], "mz=229.0&rt=6,8&da=0.1&mz_offset=0.1"),
]
# questions both refuse: the same error key
BAD = [(["--mz", "229.1", "--rt", "8,6"], "mz=229.1&rt=8,6"), (["--mz", "229.1", "--rt", "6,8", "--da", "0"], "mz=229.1&rt=6,8&da=0")]


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
    w = tmp_path_factory.mktemp("raw_xic")
    r = np.random.default_rng(5)
    d.full_scan(w / "FS.mzML", 3.0, r)
    d.full_scan_profile(w / "FS_profile.mzML", 3.0, r)
    d.ms2(w / "MS2.mzML", r, 15)
    d.mixed_polarity(w / "POL.mzML", r)
    run(sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(w / "wiff"))
    shutil.copy(w / "wiff" / "two.wiff", w / "two.wiff")
    shutil.copy(w / "wiff" / "two.wiff.scan", w / "two.wiff.scan")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(w / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-rawxic-wasm")))
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
    """The wasm's members and rows are inside the CLI document, byte for byte."""
    assert "s" in got, f"{name} {query}: {got}"
    route = got["s"]
    # the sample holds the route's members, the rows hold the route's rows after `file` and `sample`: the bytes of the CLI, not a re-serialisation
    members = route[1:route.index(',"rows":[')]
    assert members in doc, f"{name} {flags} {query}:\nwasm {route[:400]}\ncli  {doc[:800]}"
    rows = re.split(r',(?=\{"mz":)', route[route.index('"rows":[') + 8:-2])
    assert len(rows) == len(json.loads(route)["rows"]) and rows
    for row in rows:
        assert row[1:] in doc, f"{name} {flags} {query}:\nwasm {row[:400]}"
    assert json.loads(route)["rows"][0]["verdict"] in ("peak", "trace", "nothing", "no_data")


@pytest.mark.parametrize("name", ["FS.mzML", "FS_profile.mzML", "MS2.mzML", "POL.mzML"])
def test_the_wasm_route_answers_the_members_and_rows_of_the_cli(work, name):
    for (flags, query), got in zip(ASKS, answers(work, name, [q for _, q in ASKS])):
        same_bytes(name, flags, query, got, cli("raw", "xic", str(work / name), *flags).stdout)


@pytest.mark.parametrize("sign,pol", [("+", 1), ("-", -1)])
def test_a_polarity_is_the_same_section_in_the_wasm_and_the_cli(work, sign, pol):
    """`--polarity +|-` is the engine's `set_polarity` 1 | -1: the alternating-polarity file answers each section with the bytes of the CLI."""
    asks = ASKS[:2]
    for (flags, query), got in zip(asks, answers(work, "POL.mzML", [q for _, q in asks], pol=pol)):
        doc = cli("raw", "xic", str(work / "POL.mzML"), *flags, "--polarity", sign).stdout
        same_bytes("POL.mzML", [*flags, "--polarity", sign], query, got, doc)
    # the two sections are different questions: the answers differ
    plus, minus = (answers(work, "POL.mzML", [asks[0][1]], pol=p)[0]["s"] for p in (1, -1))
    assert plus != minus


@pytest.mark.parametrize("sample", ["1", "2"])
def test_a_sample_of_a_multi_sample_file_is_the_same_in_the_wasm_and_the_cli(work, sample):
    """`--sample ID` of a two-sample .wiff (`tools/wiff_riferimento.py`: a scan sample, an MRM sample): the index of that sample answers as the CLI does."""
    for (flags, query), got in zip(ASKS[:2], answers(work, "two.wiff", [q for _, q in ASKS[:2]], sample=sample)):
        doc = cli("raw", "xic", str(work / "two.wiff"), *flags, "--sample", sample).stdout
        same_bytes("two.wiff", [*flags, "--sample", sample], query, got, doc)


def test_a_refused_question_has_the_same_error_key(work):
    for (flags, query), got in zip(BAD, answers(work, "FS.mzML", [q for _, q in BAD])):
        assert json.loads(got["e"])["error_key"] == "err.raw.xic.param", got
        r = cli("raw", "xic", str(work / "FS.mzML"), *flags, runner=lambda *a: subprocess.run(a, capture_output=True, encoding="utf-8"))
        assert r.returncode == 2 and json.loads(r.stdout)["schema"] == "error/1"
