"""Parity of the readers: the mzML that a readers module (.wasm, `to_mzml`, what the site writes in OPFS) gives for a dataset is byte for byte the one `mzlab-cli convert` writes
(what the desktop app writes). One synthetic dataset per family (no laboratory data): Python generators for aperti, agilent, waters, shimadzu, sciex; the Rust example `synthetic`
(crates/mzlab-lettori/examples, the writers of the reader tests) for thermo, bruker, andi. The modules run in Node (wasm-bindgen --target nodejs). Needs the wasm32 target and wasm-bindgen-cli (the version of Cargo.lock); without them the test is skipped,
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


_packages = {}


def package(tools, fam):
    """The module of the family, built as the site builds it (no wasm-opt: it does not change what the code computes) and packed for Node."""
    if fam not in _packages:
        b = run(["cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", fam])
        assert b.returncode == 0, b.stderr[-2000:]
        out = tools["tmp"] / f"pkg_{fam}"
        w = run(["wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), str(ROOT / "target/wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm")])
        assert w.returncode == 0, w.stderr[-2000:]
        _packages[fam] = out
    return _packages[fam]


def members(p: Path):
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
    piano.write_text(json.dumps({"members": [{"name": n, "path": str(f)} for n, f in members(ds)], "out": str(out)}))
    r = run(["node", str(ROOT / "tests/lettori_parita_node.js"), str(package(tools, fam)), str(piano)])
    assert r.returncode == 0, r.stderr[-2000:]
    return out.read_bytes()


def da_cli(tools, fam, ds: Path, *extra: str, code: int = 0) -> bytes:
    """The mzML the CLI writes; `code` 3 for a cut dataset (written, with the scans missing named on stderr)."""
    out = tools["tmp"] / f"cli_{fam}"
    r = run([str(tools["cli"]), "convert", str(ds), "--out", str(out), "--fresh", "--workers", "1", *extra])
    assert r.returncode == code, r.stdout[-1500:] + r.stderr[-1500:]
    if code == 3:
        assert "partial read, " in r.stderr, r.stderr[-1500:]
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


def create(fam: str, d: Path) -> Path:
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
    if fam in SYNTHETIC_RUST:
        synthetic(d.parent)
        return d.parent / "synthetic" / SYNTHETIC_RUST[fam]
    raise AssertionError(fam)


SYNTHETIC_RUST = {"thermo": "thermo/synt.raw", "bruker": "bruker/synt.d", "bruker_tagliato": "bruker_tagliato/tagliato.d", "andi": "andi/synt.cdf"}


def synthetic(base: Path):
    """Runs the example that writes the Thermo, Bruker and ANDI datasets (once per test run)."""
    if not (base / "synthetic").exists():
        r = run(["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-lettori", "--features", "tutte", "--example", "synthetic", "--", str(base / "synthetic")])
        assert r.returncode == 0, r.stderr[-2000:]


@pytest.mark.parametrize("fam", ["aperti", "agilent", "waters", "shimadzu", "sciex", "thermo", "bruker", "andi"])
def test_module_mzml_equals_the_cli_one(tools, fam):
    ds = create(fam, tools["tmp"] / f"in_{fam}")
    cli = da_cli(tools, fam, ds)
    wasm = da_wasm(tools, fam, ds)
    assert sha(wasm) == sha(cli), (fam, len(wasm), len(cli), first_diff(wasm, cli))


def test_waters_index_marks_equal_the_cli(tools):
    """A folder with the index marks set both ways (0x18 scans get the polynomial of `_HEADER.TXT`, 0x4C scans keep the stored m/z; a flag bit above the 22-bit count):
    the module and `mzlab-cli convert` write the same mzML, byte for byte."""
    import waters_sintetico
    ds = waters_sintetico.write_flags(tools["tmp"] / "in_waters_flags")
    cli = da_cli(tools, "waters_flags", ds)
    wasm = da_wasm(tools, "waters", ds)
    assert cli.count(b"<spectrum ") == 40
    assert sha(wasm) == sha(cli), (len(wasm), len(cli), first_diff(wasm, cli))


def test_waters_function_without_its_dat_equals_the_cli(tools):
    """The folder of the index marks without `_FUNC002.DAT`: both faces write the 20 scans of function 1, byte for byte, and `conv/1` counts 20 of 40 with the
    key of the missing member (`Reader::losses` of the Waters reader)."""
    import waters_sintetico
    ds = waters_sintetico.write_flags(tools["tmp"] / "in_waters_senza_dat")
    (ds / "_FUNC002.DAT").unlink()
    cli = da_cli(tools, "waters_senza_dat", ds, code=3)
    wasm = da_wasm(tools, "waters", ds)
    assert cli.count(b"<spectrum ") == 20
    assert b"warn.file.memberMissing" in cli
    assert sha(wasm) == sha(cli), (len(wasm), len(cli), first_diff(wasm, cli))


def test_bruker_con_tdf_bin_tagliato_uguale_al_cli(tools):
    """A `.d` whose `analysis.tdf_bin` ends inside the fourth of five frames: both faces write the three whole frames, byte for byte, and the conv/1 of both
    names the cut `_bin` as the cause of the partial read."""
    ds = create("bruker_tagliato", tools["tmp"] / "in_bruker_tagliato")
    cli = da_cli(tools, "bruker_tagliato", ds, code=3)
    wasm = da_wasm(tools, "bruker", ds)
    assert cli.count(b"<spectrum ") == 3
    assert b"warn.file.brukerCutBin" in cli
    assert sha(wasm) == sha(cli), (len(wasm), len(cli), first_diff(wasm, cli))


@pytest.mark.parametrize("membro", ["MSScan.bin", "MSPeak.bin"])
def test_cropped_agilent_equals_the_cli(tools, membro):
    """A `.D` whose index or points file ends inside the last of 60 scans: both faces write the 59 whole scans, byte for byte."""
    import agilent_sintetico
    ds = agilent_sintetico.write(tools["tmp"] / f"in_agilent_{membro}", "synt.D", 60)
    f = ds / "AcqData" / membro
    f.write_bytes(f.read_bytes()[:-7])
    cli = da_cli(tools, "agilent", ds, code=3)
    wasm = da_wasm(tools, "agilent", ds)
    assert cli.count(b"<spectrum ") == 59
    assert sha(wasm) == sha(cli), (len(wasm), len(cli), first_diff(wasm, cli))


def test_wiff_with_dad_and_method_equals_cli_with_dad(tools):
    """A 3200 QTRAP variant (version-1 blocks) with an optical detector and an LC method: the module writes the matrix (`MS:1000804` spectra) and the method
    (`mzLab metodo/1`), byte for byte what `mzlab-cli convert --set conv.dad=true` writes."""
    d = tools["tmp"] / "in_dadmetodo"
    r = run([sys.executable, str(ROOT / "tools/wiff_riferimento.py"), str(d)])
    assert r.returncode == 0, r.stderr[-1500:]
    ds = d / "dadmetodo.wiff"
    cli = da_cli(tools, "sciex_dad", ds, "--set", "conv.dad=true")
    wasm = da_wasm(tools, "sciex", ds)
    assert b"MS:1000804" in cli and b"mzLab metodo/1" in cli
    assert sha(wasm) == sha(cli), (len(wasm), len(cli), first_diff(wasm, cli))



def test_wiff_with_device_traces_equals_cli(tools):
    """A 3200 QTRAP variant with device traces (two pressures, a flow, a temperature: tools/wiff_dispositivi.py): the module writes them as chromatograms,
    the pressures in pascal as a `pressure array`, byte for byte what `mzlab-cli convert` writes."""
    d = tools["tmp"] / "in_dispositivi"
    r = run([sys.executable, str(ROOT / "tools/wiff_riferimento.py"), str(d)])
    assert r.returncode == 0, r.stderr[-1500:]
    ds = d / "dispositivi.wiff"
    cli = da_cli(tools, "sciex_dev", ds)
    wasm = da_wasm(tools, "sciex", ds)
    assert cli.count(b'name="pressure array"') == 2 and b"Oven Temperature" in cli
    assert sha(wasm) == sha(cli), (len(wasm), len(cli), first_diff(wasm, cli))

def test_wiff_with_sparse_product_scans_equals_cli(tools):
    """A 3200 QTRAP product-ion sample of sparse scans (tools/wiff_prodotto_rado.py: 0 to 5 points, padded streams, an index value at +26 that is not
    the point count): the module writes every scan, byte for byte what `mzlab-cli convert` writes."""
    d = tools["tmp"] / "in_rado"
    r = run([sys.executable, str(ROOT / "tools/wiff_prodotto_rado.py"), str(d)])
    assert r.returncode == 0, r.stderr[-1500:]
    ds = d / "rado.wiff"
    cli = da_cli(tools, "sciex_rado", ds)
    wasm = da_wasm(tools, "sciex", ds)
    assert cli.count(b"<spectrum ") == len(json.loads((d / "rado.json").read_text())["spectra"])
    assert sha(wasm) == sha(cli), (len(wasm), len(cli), first_diff(wasm, cli))


def first_diff(a: bytes, b: bytes):
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return i, a[max(0, i - 60):i + 60], b[max(0, i - 60):i + 60]
    return None
