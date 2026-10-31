"""The one table of recognised instrument formats (mzlab/web/formati-noti.js, run by node): vendor and format of each, readable or not, and its agreement with
ordine.rs (EXT), tools/converti.py and the file-like words of the bug report."""
import json
import re
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
pytestmark = pytest.mark.skipif(not shutil.which("node"), reason="node missing")


def nodo(js: str):
    uri = (ROOT / "mzlab/web/formati-noti.js").as_uri()
    src = f"globalThis.window={{}};const F=await import({uri!r});process.stdout.write(JSON.stringify({js}))"
    r = subprocess.run(["node", "--input-type=module", "-e", src], capture_output=True, text=True, encoding="utf-8", check=True)
    return json.loads(r.stdout)


@pytest.mark.parametrize("name,label,st", [
    ("a.raw", "Thermo / Waters .raw", "read"),
    ("a.WIFF", "SCIEX .wiff", "read"),
    ("a.wiff.scan", "SCIEX .wiff.scan", "read"),
    ("a.wiff2", "SCIEX .wiff2", "unsupported"),
    ("a.d", "Agilent / Bruker .d", "read"),
    ("analysis.baf", "Bruker .baf", "unsupported"),
    ("analysis.tdf", "Bruker .tdf / .tsf", "read"),
    ("x.lcd", "Shimadzu .lcd / .qgd", "read"),
    ("x.lcdx", "Shimadzu .lcdx", "unsupported"),
])
def test_classify_known(name, label, st):
    got = nodo(f"F.classify({name!r})")
    assert got["label"] == label and got["st"] == st


@pytest.mark.parametrize("name", ["x.xyz", "notes.txt", "wiff2", "a.wiff3", "photo.png"])
def test_classify_unknown(name):
    assert nodo(f"F.classify({name!r})") is None


def test_table_agrees_with_other_lists():
    exts = nodo("F.FORMATI.map(f=>f.ext)")
    ordine = (ROOT / "crates/mzlab-core/src/campioni/ordine.rs").read_text(encoding="utf-8")
    rust = set(re.findall(r'"(\w+)"', re.search(r"const EXT: \[&str; \d+\] = \[(.*?)\];", ordine, re.S).group(1)))
    assert {"raw", "wiff", "wiff2", "d", "cdf"} <= rust and {"raw", "wiff", "wiff2", "d", "cdf"} <= {e.lstrip(".") for e in exts}
    conv = (ROOT / "tools/converti.py").read_text(encoding="utf-8")
    assert all(e in exts for e in re.findall(r'"(\.wiff2?)"', conv))
    # every extension of the table is a file-like word of the bug report (no file name in a report)
    out = nodo("[" + ",".join(f"{('x' + e)!r}" for e in exts) + "].map(n=>F.classify(n)&&n)")
    assert all(out)
    br = subprocess.run(["node", "--input-type=module", "-e", f"import * as B from {(ROOT / 'mzlab/web/bug-report-testo.js').as_uri()!r};process.stdout.write(JSON.stringify(" + "[" + ",".join(f"{('failed s' + e)!r}" for e in exts) + "].map(n=>B.anonimizza(n))))"], capture_output=True, text=True, check=True)
    assert all("<file>" in x for x in json.loads(br.stdout)), br.stdout


def test_wiff2_is_never_a_reader():
    assert nodo("F.FORMATI.filter(f=>f.ext==='.wiff2').map(f=>f.st)") == ["unsupported"]
