"""Partial reads told alike on the site (engine .wasm through Node) and in the app. A truncated mzML whose run userParams push `<spectrumList count>`
far past the `<run ` tag (mzLab's own `metodo/1` does) declares its spectra in `prepare_upload` and in the CLI (`flow` step `index.mzidx`, warning on
stderr) (H6-3). A sample the engine refuses is listed by `native_samples` with its reason, the text the app lists (`mzlab-desktop` natives_tests.rs) (H6-15).
Synthetic files (built here, tools/wiff_riferimento.py). Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
NODE = """const w = require(process.argv[2]), fs = require("fs"), b = fs.readFileSync(process.argv[3]);
const p = w.prepare_upload(b.length, (o, n) => b.subarray(o, o + n), "cut.mzML", undefined, undefined, 0);
console.log(JSON.stringify({ rows: p[0].length, declared: p[9] }));"""
LIST = """const w = require(process.argv[2]), fs = require("fs"), names = process.argv.slice(3), bs = names.map(n => fs.readFileSync(n));
console.log(w.native_samples(names.map(n => n.split("/").pop()), new Float64Array(bs.map(b => b.length)), (i, o, n) => bs[i].subarray(o, o + n)));"""
# the listing of mzlab-core/tests/wiff_refused_listing.rs and mzlab-desktop natives_tests.rs, byte for byte
LISTING = ('{"format":"SciexWiff","samples":[{"id":1,"name":"rifiutato_s1","scans":1},{"id":2,"name":"rifiutato_s2","scans":0,"why":"empty"},'
           '{"id":3,"name":"rifiutato_s3","scans":0,"why":"refused","refused":{"error_key":"err.file.wiffScanType","params":{"code":"8","experiment":"1","type":"TOF MS"}}}]}')


def spectrum(i):
    arr = '<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="{}" name="x"/><binary></binary></binaryDataArray>'
    return (f'<spectrum index="{i}" id="scan={i + 1}" defaultArrayLength="0"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>'
            '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/>'
            f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{i * 0.1}" unitName="minute"/></scan></scanList>'
            f'<binaryDataArrayList count="2">{arr.format("MS:1000514")}{arr.format("MS:1000515")}</binaryDataArrayList></spectrum>\n')


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("finestra")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    run("cargo", "build", "--release", "--locked", "-p", "mzlab-cli")
    head = ('<?xml version="1.0" encoding="utf-8"?>\n<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="r">\n'
            f'<userParam name="mzLab metodo/1" value="{"m" * 6000}"/>\n<spectrumList count="6">\n')
    (d / "cut.mzML").write_text(head + "".join(spectrum(i) for i in range(3)), encoding="utf-8")
    return d


def test_site_and_cli_see_the_declared_spectra_of_a_cut_file(work):
    (work / "w.js").write_text(NODE, encoding="utf-8")
    site = json.loads(run("node", str(work / "w.js"), str(work / "pkg" / "mzlab_wasm.js"), str(work / "cut.mzML")).stdout)
    assert site == {"rows": 3, "declared": 6}, site
    plan = work / "plan.json"
    plan.write_text(json.dumps({"format": "mzflow/2", "inputs": [{"name": "cut.mzML", "type": "mzml"}],
                                "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:cut.mzML"]}}]}), encoding="utf-8")
    p = subprocess.run([str(ROOT / "target/release/mzlab-cli"), "flow", "run", str(plan), "--in", str(work / "cut.mzML"), "--out", str(work / "o")],
                       cwd=ROOT, capture_output=True, encoding="utf-8")
    assert "cut.mzML: 3 of 6 spectra read" in p.stderr, (p.returncode, p.stderr[-1500:])


def test_site_lists_a_refused_sample_with_its_reason_as_the_app_does(work):
    run(sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(work / "wf"))
    (work / "l.js").write_text(LIST, encoding="utf-8")
    pair = [str(work / "wf" / n) for n in ("rifiutato.wiff", "rifiutato.wiff.scan")]
    assert run("node", str(work / "l.js"), str(work / "pkg" / "mzlab_wasm.js"), *pair).stdout.strip() == LISTING
