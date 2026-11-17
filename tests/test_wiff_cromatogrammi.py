"""The chromatograms of a .wiff sample on the site equal those of `mzlab-cli convert`, byte for byte (synthetic pairs of tools/wiff_cromatogrammi.py: Full Scan
and MRM experiments in one sample, two MRM experiments, Full Scan alone): one TIC per experiment with data (`TIC sample=N period=1 experiment=E` when
there are several), after an MRM TIC its BPC and SRM chromatograms, the BPC of each Full Scan experiment last. The engine (mzlab-wasm, `native_to_mzml`)
against the CLI, with the same compression; their data arrays (time, intensity) byte for byte. The readers module (`sciex`) against the CLI, whole mzML:
the precursor-ion and neutral-loss experiments with a TIC and a BPC, MS3 with a TIC only (`variante` of tools/wiff_riferimento.py, tools/wiff_modes.py).
Skipped without cargo, wasm-bindgen or node."""
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
WASM = ROOT / "target/wasm32-unknown-unknown/release"
PAIRS = ("fsmrm", "duemrm", "solofs")
CHROM = re.compile(rb"<chromatogram [^>]*>.*?</chromatogram>", re.S)


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("cromatogrammi")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    build("-p", "mzlab-cli")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "eng"), str(WASM / "mzlab_wasm.wasm"))
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_cromatogrammi.py"), str(d / "in")], check=True, capture_output=True)
    for n in PAIRS:
        run(str(ROOT / "target/release/mzlab-cli"), "convert", str(d / "in" / f"{n}.wiff"), "--out", str(d / "cli"))
        run("node", str(ROOT / "tests" / "mzml_nativo_node.js"), str(d / "eng"), str(d / f"{n}.mzML"), "0", str(d / "in" / f"{n}.wiff"), str(d / "in" / f"{n}.wiff.scan"))
    return d


def chroms(p: Path):
    return CHROM.findall(p.read_bytes())


def arrays(c):
    """The id, length and data arrays of a chromatogram (the engine leaves out the SRM dwell and energy parameters that `estesi` writes, rust-nativo.js `writerOf`)."""
    return re.search(rb' id="[^"]*" defaultArrayLength="\d+"', c).group(0), re.search(rb"<binaryDataArrayList.*</binaryDataArrayList>", c, re.S).group(0)


def ids(cs):
    return [re.search(rb' id="([^"]*)"', c).group(1).decode() for c in cs]


@pytest.mark.parametrize("name", PAIRS)
def test_the_engine_lists_the_chromatograms_of_convert(work, name):
    want = json.loads((work / "in" / "attesi.json").read_text())[name]
    got = ids(chroms(work / f"{name}.mzML"))
    assert ["SRM" if g.startswith("SRM ") else g for g in got] == want, got


@pytest.mark.parametrize("name", PAIRS)
def test_the_chromatograms_are_those_of_convert_byte_for_byte(work, name):
    eng, cli = chroms(work / f"{name}.mzML"), chroms(work / "cli" / f"{name}.mzML")
    assert ids(eng) == ids(cli)
    for e, c in zip(eng, cli):
        assert arrays(e) == arrays(c), (e[:600], c[:600])


MODULE = {"variante": ["TIC", "TIC", "TIC", "BPC", "BPC", "BPC"], "MS3": ["TIC"], "NL": ["TIC", "BPC"], "MRM": ["TIC", "BPC", "SRM", "SRM", "SRM"]}


@pytest.fixture(scope="module")
def module(tmp_path_factory):
    d = tmp_path_factory.mktemp("cromatogrammi_modulo")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", "sciex")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "mod"), str(WASM / "mzlab_lettori_wasm.wasm"))
    build("-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(d / "in")], check=True, capture_output=True)
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_modes.py"), str(d)], check=True, capture_output=True)
    for p in (d / "modi").glob("*.wiff*"):
        shutil.copy(p, d / "in" / p.name)
    return d


@pytest.mark.parametrize("name", MODULE)
def test_the_module_writes_the_mzml_of_convert(module, name):
    pair = [module / "in" / f"{name}.wiff", module / "in" / f"{name}.wiff.scan"]
    plan = module / f"plan-{name}.json"
    plan.write_text(json.dumps({"members": [{"name": p.name, "path": str(p)} for p in pair], "out": str(module / f"mod-{name}.mzML")}))
    run("node", str(ROOT / "tests" / "lettori_parita_node.js"), str(module / "mod"), str(plan))
    run(str(ROOT / "target/release/mzlab-cli"), "convert", str(pair[0]), "--out", str(module / f"cli-{name}"), "--fresh", "--workers", "1")
    mod, cli = (module / f"mod-{name}.mzML").read_bytes(), (module / f"cli-{name}" / f"{name}.mzML").read_bytes()
    assert mod == cli
    got = [g.split(" ")[0] for g in ids(chroms(module / f"mod-{name}.mzML"))]
    assert got == MODULE[name], got
