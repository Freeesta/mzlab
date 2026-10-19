"""The diagnosis of a loading error (mzlab/web/diagnosi.js, run by node): the cause per phase, no personal parts, the header without data, the text to copy; and the synthetic msconvert-style mzML
(tools/mzml_msconvert_sintetico.py) with its variants."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import mzml_msconvert_sintetico as gen  # noqa: E402

pytestmark = pytest.mark.skipif(not shutil.which("node"), reason="node missing")


def nodo(js: str):
    r = subprocess.run(["node", "-e", f"const D=require({str(ROOT / 'mzlab/web/diagnosi.js')!r});process.stdout.write(JSON.stringify({js}))"], capture_output=True, text=True, encoding="utf-8", check=True)
    return json.loads(r.stdout)


@pytest.mark.parametrize("fase,raw,causa", [
    ("copy", "QuotaExceededError: The quota has been exceeded", "quota"),
    ("copy", "NotReadableError: The requested file could not be read", "read"),
    ("index", '{"error_key":"error.memoryBudget"}', "memory"),
    ("index", "RuntimeError: unreachable", "engine"),
    ("index", "xml: unexpected end of file at offset 1234", "format"),
    ("decode", "zlib: invalid stored block lengths in scan 55", "decode"),
    ("open", "something odd", "unknown"),
])
def test_causa_per_fase(fase, raw, causa):
    d = nodo(f"D.da({{key:'err.engine.index',params:{{fase:{fase!r},raw:{json.dumps(raw)}}}}},{{name:'a.mzML',size:5}})")
    assert d["causa"] == causa and d["fase"] == fase and d["codice"] == "err.engine.index"
    assert set(d) == {"codice", "fase", "causa", "dettagli"}


def test_nessun_dato_personale():
    t = nodo(r"D.pulisci('read /Users/mario.rossi/Desktop/a.raw and C:\\Users\\Mario\\x.mzML and /home/luca/x and file:///Users/a/b')")
    assert "mario" not in t.lower() and "luca" not in t and "Users" not in t and t.count("<path>") == 4


def test_testa_senza_dati():
    xml = '<?xml version="1.0"?>\n<mzML id="Run1">\n<softwareList>\n</softwareList>\n<spectrumList count="9"><spectrum><binary>QUJD</binary>\n<spectrum>SECRET'
    t = nodo(f"D.testa({json.dumps(xml)})")
    assert "Run1" in t and "spectrumList" in t and "QUJD" not in t and "SECRET" not in t
    assert nodo("D.testa('\\u0000\\u0001binary')") == ""


def test_testo_da_copiare():
    t = nodo("D.testo(D.da({key:'k',params:{fase:'index',raw:'boom /home/x/y'}},{name:'privato.mzML',size:7},'<mzML>'),'1.2')")
    assert "phase: index" in t and "privato" not in t and "/home/x" not in t and "<mzML>" in t


@pytest.mark.parametrize("variante", sorted(gen.VARIANTI))
def test_sintetico_varianti(tmp_path, variante):
    p = tmp_path / f"{variante}.mzML"
    n = gen.scrivi(p, scans=24, peaks1=50, peaks2=10, variante=variante)
    s = p.read_bytes()
    assert n == len(s) and s.count(b"<spectrum ") == 24
    assert s.startswith(b"<?xml") and (b"<indexedmzML" in s) == (variante != "senza_wrapper")


def test_long_tasks_are_told_apart_by_phase():
    """The long tasks of the page carry the phase active when they started (Tabui marks its phases); the text to copy lists the longest ones and names no file."""
    r = nodo("""(() => {
      D.LONG.marks.push({t: 100, fase: 'start'}, {t: 200, fase: 'xic'}, {t: 900, fase: 'render'});
      D.LONG.registra(150, 60); D.LONG.registra(300, 420); D.LONG.registra(1000, 90);
      return {fasi: D.LONG.list.map(x => x.fase), righe: D.LONG.righe(0), dopo: D.LONG.righe(500).length, testo: D.testoLento('1.2.3', 0)};
    })()""")
    assert r["fasi"] == ["start", "xic", "render"]
    assert r["righe"][0] == "long tasks: 3, longest 420 ms, total 570 ms"
    assert r["righe"][1] == "  +300 ms: 420 ms in xic"
    assert r["dopo"] == 2          # the summary line and the one task after 500 ms
    assert r["testo"].startswith("mzLab 1.2.3\ncores: ") and "420 ms in xic" in r["testo"]
