"""A scheduled-MRM .wiff with no Log (`programmati` of tools/wiff_riferimento.py, synthetic, no laboratory data) on the site: the `probe` of the `sciex` module says
«variante», so shell.js hands the pair to the module, whose mzML is byte for byte the one `mzlab-cli convert` writes (one TIC per experiment, the scheduled
transitions without their skip markers, the generic instrument term: no Log names no model). The engine (mzlab-wasm, `build_mzidx_native`) refuses the sparse
blocks (`err.file.wiff`, `sparse MRM block`) instead of reading the markers as intensities. `mrmrado` (tools/wiff_mrm_rado.py) has sparse blocks but no `sMRM`
stream in its method: the probe still says «variante» from the blocks themselves. Skipped without cargo, wasm-bindgen or node."""
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


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("programmati")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", "sciex")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "mod"), str(WASM / "mzlab_lettori_wasm.wasm"))
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "eng"), str(WASM / "mzlab_wasm.wasm"))
    run("cargo", "build", "--release", "--locked", "-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(d / "ref")], check=True, capture_output=True)
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_mrm_rado.py"), str(d / "ref")], check=True, capture_output=True)
    return d


def pair(work, name="programmati"):
    return [work / "ref" / f"{name}.wiff", work / "ref" / f"{name}.wiff.scan"]


def probe(work, name):
    js = ("const fs=require('fs'),w=require(process.argv[1]+'/mzlab_lettori_wasm.js'),m=process.argv.slice(2);"
          "const read=(i,o,n)=>{const b=Buffer.alloc(n);const fd=fs.openSync(m[i],'r');fs.readSync(fd,b,0,n,o);fs.closeSync(fd);return new Uint8Array(b);};"
          "process.stdout.write(w.probe(m.map(x=>require('path').basename(x)),Float64Array.from(m.map(x=>fs.statSync(x).size)),read));")
    return json.loads(run("node", "-e", js, str(work / "mod"), *map(str, pair(work, name))).stdout)


def module_and_cli(work, name):
    plan = work / f"plan-{name}.json"
    plan.write_text(json.dumps({"members": [{"name": p.name, "path": str(p)} for p in pair(work, name)], "out": str(work / f"mod-{name}.mzML")}))
    run("node", str(ROOT / "tests" / "lettori_parita_node.js"), str(work / "mod"), str(plan))
    run(str(ROOT / "target/release/mzlab-cli"), "convert", str(pair(work, name)[0]), "--out", str(work / f"cli-{name}"), "--fresh", "--workers", "1")
    return (work / f"mod-{name}.mzML").read_bytes(), (work / f"cli-{name}" / f"{name}.mzML").read_bytes()


def test_the_probe_of_the_module_hands_the_pair_to_the_module(work):
    out = probe(work, "programmati")
    assert "variante" in out["motivi"], out


def test_the_mzml_of_the_module_is_the_one_of_the_cli(work):
    mod, cli = module_and_cli(work, "programmati")
    assert mod == cli
    assert b"TIC sample=1 period=1 experiment=2" in cli and b" rt=0.5 window=60 " in cli
    assert b'accession="MS:1000031" name="instrument model"' in cli and b"MS:1000651" not in cli


def test_the_engine_refuses_the_sparse_blocks(work):
    p = subprocess.run(["node", str(ROOT / "tests" / "mzidx_nativo_node.js"), str(work / "eng"), str(work / "eng.mzidx"), "0", "programmati.wiff", *map(str, pair(work))],
                       cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode != 0 and "sparse MRM block" in p.stderr, (p.returncode, p.stdout[-500:], p.stderr[-1500:])


def test_sparse_blocks_without_a_scheduled_method_go_to_the_module_too(work):
    out = probe(work, "mrmrado")
    assert "variante" in out["motivi"], out
    mod, cli = module_and_cli(work, "mrmrado")
    assert mod == cli and cli.count(b'id="SRM SIC') == 136
    p = subprocess.run(["node", str(ROOT / "tests" / "mzidx_nativo_node.js"), str(work / "eng"), str(work / "eng-rado.mzidx"), "0", "mrmrado.wiff", *map(str, pair(work, "mrmrado"))],
                       cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode != 0 and "sparse MRM block" in p.stderr, (p.returncode, p.stdout[-500:], p.stderr[-1500:])
