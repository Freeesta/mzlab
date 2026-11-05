"""The `probe` of the `sciex` module (shell.js asks it for every .wiff pair before the engine opens it) reads the blocks of sample 0 only: the bytes it reads of the
.wiff.scan are the same for a pair of one Full Scan sample and for a pair of six, where it read the heads of every sample (2.2x slower loading of Full Scan
pairs). A variant in another sample is still said by its method (`mistopi`: sample 2 code 6). Synthetic pairs (Writer of tools/wiff_riferimento.py) only.
Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
WASM = ROOT / "target/wasm32-unknown-unknown/release"
sys.path.insert(0, str(ROOT / "tools"))
from wiff_riferimento import Writer, header, ion_spray  # noqa: E402

# probe(names, sizes, read) with a read that counts the bytes asked of each member; prints {"info": ..., "reads": [bytes per member]}
PROBE = ("const fs=require('fs'),path=require('path'),w=require(process.argv[1]+'/mzlab_lettori_wasm.js'),m=process.argv.slice(2),n=m.map(()=>0);"
         "const read=(i,o,k)=>{n[i]+=k;const b=Buffer.alloc(k);const fd=fs.openSync(m[i],'r');fs.readSync(fd,b,0,k,o);fs.closeSync(fd);return new Uint8Array(b);};"
         "const info=JSON.parse(w.probe(m.map(x=>path.basename(x)),Float64Array.from(m.map(x=>fs.statSync(x).size)),read));"
         "process.stdout.write(JSON.stringify({info,reads:n}));")


# 3000 points per scan: each injection fills more than the 256 KB the module reads ahead (LETTURA), or the read of sample 0 runs into the others anyway
POINTS = [(2 * k + 1, 100 + k) for k in range(3000)]


def full_scan(out, stem, samples):
    w = Writer()
    for s in range(1, samples + 1):
        w.sample(s, [("scan", POINTS, 1000.0 * (k + 1), 1.0e5) for k in range(40)],
                 {"Experiment0/ExperimentHeader": header(15, 30.0), "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)})
    w.save(out, stem)


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("sonda")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", "sciex")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "mod"), str(WASM / "mzlab_lettori_wasm.wasm"))
    full_scan(d, "uno", 1)
    full_scan(d, "sei", 6)
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_misto_pi.py"), str(d)], check=True, capture_output=True)
    return d


def probe(work, stem):
    return json.loads(run("node", "-e", PROBE, str(work / "mod"), str(work / f"{stem}.wiff"), str(work / f"{stem}.wiff.scan")).stdout)


def test_the_probe_reads_the_blocks_of_sample_0_only(work):
    one, six = probe(work, "uno"), probe(work, "sei")
    assert len(six["info"]["campioni"]) == 6 and six["info"]["motivi"] == [], six["info"]
    assert six["reads"][1] == one["reads"][1], (one["reads"], six["reads"])


def test_a_variant_in_another_sample_is_still_said(work):
    assert "variante" in probe(work, "mistopi")["info"]["motivi"]
