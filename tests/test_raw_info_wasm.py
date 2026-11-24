"""`raw info` in the site: the lazy .wasm `raw` (mzlab-raw-wasm, through Node: `raw_info(size, read, pol, query)`, the call rust-raw.js makes for the
route `/api/rawinfo` over the .mzidx of OPFS) gives the same members, byte for byte, as each sample of `mzlab-cli raw info` on the synthetic files of
`tools/dati_sintetici.py` (full scan, profile, MS2, MRM, alternating polarity); the index is the entry `mzlab-cli raw cache` writes, and the desktop
answers the same route (`crates/mzlab-desktop/tests/parity_routes.rs`). Skipped without cargo, wasm-bindgen, node or numpy."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")

NODE = """const [dir, idxPath, ...queries] = process.argv.slice(2);
const { raw_info } = require(dir + '/mzlab_raw_wasm.js'), fs = require('fs');
const b = fs.readFileSync(idxPath), read = (off, len) => new Uint8Array(b.subarray(off, off + len));
const out = queries.map(q => { try { return { s: raw_info(b.length, read, 0, q) }; } catch (x) { return { e: String(x) }; } });
process.stdout.write(JSON.stringify(out));"""
# the CLI flags and the route query of the same question
ASKS = [([], "probe=64"), (["--probe", "8"], "probe=8"), (["--offset-ion", "364.4@14.3"], "probe=64&offset_mz=364.4&offset_rt=14.3")]


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    np = pytest.importorskip("numpy")
    import dati_sintetici as d
    w = tmp_path_factory.mktemp("raw_info")
    r = np.random.default_rng(5)
    d.full_scan(w / "FS.mzML", 3.0, r)
    d.full_scan_profile(w / "FS_profile.mzML", 3.0, r)
    d.ms2(w / "MS2.mzML", r, 15)
    d.mrm(w / "MRM.mzML", 1e5, r, 1)
    d.mixed_polarity(w / "POL.mzML", r)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(w / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-raw-wasm")))
    build("-p", "mzlab-cli")
    (w / "run.js").write_text(NODE, encoding="utf-8")
    return w


@pytest.mark.parametrize("name", ["FS.mzML", "FS_profile.mzML", "MS2.mzML", "MRM.mzML", "POL.mzML"])
def test_the_wasm_route_answers_the_members_of_the_cli(work, name):
    f, cache = work / name, work / "cache"
    built = json.loads(cli("raw", "cache", str(f), "--cache", str(cache)).stdout)
    key = built["files"][0]["key"]
    answers = json.loads(subprocess.run(["node", str(work / "run.js"), str(work / "pkg"), str(cache / key), *[q for _, q in ASKS]],
                                        capture_output=True, encoding="utf-8", check=True).stdout)
    for (flags, query), got in zip(ASKS, answers):
        assert "s" in got, f"{name} {query}: {got}"
        doc = cli("raw", "info", str(f), *flags).stdout
        sample = json.dumps(json.loads(doc)["files"][0]["samples"][0], separators=(",", ":"), ensure_ascii=False)
        # the sample is `{"id":..,"name":..,` + the route's members: the bytes of the CLI, not a re-serialisation
        assert got["s"][1:-1] in doc, f"{name} {query}:\nwasm {got['s'][:400]}\ncli  {sample[:400]}"
        v = json.loads(got["s"])
        assert v["n_scans"] + v["n_chromatograms"] > 0   # an MRM file holds chromatograms only
