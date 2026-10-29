"""Parity of the readers: the mzML that a readers module (.wasm, `to_mzml`, what the site writes in OPFS) gives for a dataset is byte for byte the one `mzlab-cli convert` writes
(what the desktop app writes). One synthetic dataset per family that has a generator in the public repository (no laboratory data); the modules run in Node (wasm-bindgen --target nodejs).
Families without an example here: thermo, bruker (see docs/studi/parita-browser.md). Needs the wasm32 target and wasm-bindgen-cli (the version of Cargo.lock); without them the test is skipped,
with MZLAB_GOLDEN_STRICT=1 it fails."""
import base64
import hashlib
import json
import math
import os
import shutil
import struct
import subprocess
import sys
import zlib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
STRICT = os.environ.get("MZLAB_GOLDEN_STRICT") == "1"


def need(ok, why):
    if not ok:
        if STRICT:
            pytest.fail(why)
        pytest.skip(why)


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


@pytest.fixture(scope="module")
def tools(tmp_path_factory):
    need(shutil.which("node") and shutil.which("wasm-bindgen"), "node or wasm-bindgen-cli missing")
    need("wasm32-unknown-unknown" in run(["rustup", "target", "list", "--installed"]).stdout, "wasm32-unknown-unknown missing")
    b = run(["cargo", "build", "--release", "--locked", "-p", "mzlab-cli"])
    assert b.returncode == 0, b.stderr[-2000:]
    return {"cli": ROOT / "target/release/mzlab-cli", "tmp": tmp_path_factory.mktemp("parita")}


_pacchetti = {}


def pacchetto(tools, fam):
    """The module of the family, built as the site builds it (no wasm-opt: it does not change what the code computes) and packed for Node."""
    if fam not in _pacchetti:
        b = run(["cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", fam])
        assert b.returncode == 0, b.stderr[-2000:]
        out = tools["tmp"] / f"pkg_{fam}"
        w = run(["wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), str(ROOT / "target/wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm")])
        assert w.returncode == 0, w.stderr[-2000:]
        _pacchetti[fam] = out
    return _pacchetti[fam]


def membri(p: Path):
    """(logical name, file) of a dataset as `crates/mzlab-cli/src/flow_set.rs` `members` gives them: a folder leads its names, a .wiff brings its .wiff.scan."""
    if p.is_dir():
        def walk(d, prefix):
            for e in sorted(d.iterdir()):
                n = f"{prefix}/{e.name}"
                if e.is_dir():
                    yield from walk(e, n)
                else:
                    yield n, e
        return list(walk(p, p.name))
    out = [(p.name, p)]
    if p.name.lower().endswith(".wiff") and (p.parent / (p.name + ".scan")).is_file():
        out.append((p.name + ".scan", p.parent / (p.name + ".scan")))
    return out


def sha(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def da_wasm(tools, fam, ds: Path) -> bytes:
    piano = tools["tmp"] / f"piano_{fam}.json"
    out = tools["tmp"] / f"wasm_{fam}.mzML"
    piano.write_text(json.dumps({"members": [{"name": n, "path": str(f)} for n, f in membri(ds)], "out": str(out)}))
    r = run(["node", str(ROOT / "tests/lettori_parita_node.js"), str(pacchetto(tools, fam)), str(piano)])
    assert r.returncode == 0, r.stderr[-2000:]
    return out.read_bytes()


def da_cli(tools, fam, ds: Path) -> bytes:
    out = tools["tmp"] / f"cli_{fam}"
    r = run([str(tools["cli"]), "convert", str(ds), "--out", str(out), "--fresh", "--workers", "1"])
    assert r.returncode == 0, r.stdout[-1500:] + r.stderr[-1500:]
    files = sorted(out.glob("*.mzML"))
    assert len(files) == 1, files
    return files[0].read_bytes()


# ---- the synthetic datasets (one sample each) ----

def b64(b):
    return base64.b64encode(b).decode()


def mzxml(path: Path, n=40):
    """n MS1 scans, 64-bit big-endian peaks in zlib, with the <index> (as tests_e2e/e2e_lettori_modulo.py writes it)."""
    text = ('<?xml version="1.0" encoding="ISO-8859-1"?>\n<mzXML xmlns="http://sashimi.sourceforge.net/schema_revision/mzXML_3.2"><msRun scanCount="%d" startTime="PT0S" endTime="PT%dS">\n' % (n, 3 * n)
            + '<parentFile fileName="campione.raw" fileType="RAWData" fileSha1="0123456789abcdef0123456789abcdef01234567"/><msInstrument msInstrumentID="1"><msManufacturer category="msManufacturer" value="Test"/>'
            + '<msModel category="msModel" value="Modello X"/></msInstrument>\n')
    pos = []
    for k in range(n):
        mz = [100 + 0.37 * i + 0.001 * (i % 7) for i in range(30)]
        it = [1000 * (1 + math.sin(i + k)) ** 2 + 50 for i in range(30)]
        z = zlib.compress(b"".join(struct.pack(">dd", a, b) for a, b in zip(mz, it)))
        pos.append(len(text.encode("latin-1")))
        text += ('<scan num="%d" msLevel="1" peaksCount="30" polarity="+" scanType="Full" centroided="0" retentionTime="PT%.1fS" totIonCurrent="%r">' % (k + 1, 3.0 * k, sum(it))
                 + '<peaks precision="64" byteOrder="network" contentType="m/z-int" compressionType="zlib" compressedLen="%d">%s</peaks></scan>\n' % (len(z), b64(z)))
    text += "</msRun>\n"
    ix = len(text.encode("latin-1"))
    text += '<index name="scan">' + "".join('<offset id="%d">%d</offset>' % (i + 1, o) for i, o in enumerate(pos)) + '</index>\n<indexOffset>%d</indexOffset>\n</mzXML>\n' % ix
    path.write_bytes(text.encode("latin-1"))
    return path


def crea(fam: str, d: Path) -> Path:
    d.mkdir(parents=True, exist_ok=True)
    if fam == "aperti":
        return mzxml(d / "campione.mzXML")
    if fam == "agilent":
        import agilent_sintetico
        return agilent_sintetico.write(d, "synt.D", 60)
    if fam == "waters":
        import waters_sintetico
        return waters_sintetico.write(d, "synt.raw", 40)
    if fam == "shimadzu":
        import shimadzu_sintetico
        p = d / "synt.qgd"
        p.write_bytes(shimadzu_sintetico.qgd(12)[0])
        return p
    if fam == "sciex":
        r = run([sys.executable, str(ROOT / "tools/wiff_grande.py"), str(d), "--name", "synt", "--samples", "1", "--scans", "60", "--points", "200"])
        assert r.returncode == 0, r.stderr[-1500:]
        return d / "synt.wiff"
    raise AssertionError(fam)


@pytest.mark.parametrize("fam", ["aperti", "agilent", "waters", "shimadzu", "sciex"])
def test_mzml_del_modulo_uguale_a_quello_del_cli(tools, fam):
    ds = crea(fam, tools["tmp"] / f"in_{fam}")
    cli = da_cli(tools, fam, ds)
    wasm = da_wasm(tools, fam, ds)
    assert sha(wasm) == sha(cli), (fam, len(wasm), len(cli), first_diff(wasm, cli))


def first_diff(a: bytes, b: bytes):
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return i, a[max(0, i - 60):i + 60], b[max(0, i - 60):i + 60]
    return None
