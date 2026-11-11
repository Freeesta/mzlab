"""The MS-Numpress arrays `mzlab-cli convert --compression smallest` writes, decoded by a second, independent implementation: the reference C++ as built in
pyopenms. Every array of every reader family gives back exactly `defaultArrayLength` values (an encoder pad the reference does not expect reads back as one
value more) and the same numbers as the zlib conversion of the same input, within the codec's declared precision. The readers modules write the same bytes
(`test_lettori_parita.py`), so this covers the site too. pyopenms is a test-only dependency (`.[test]`); without it the test is skipped (MZLAB_GOLDEN_STRICT=1: fails)."""
import base64
import os
import shutil
import sys
import xml.etree.ElementTree as etree
import zlib
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from test_lettori_parita import create, need, run  # noqa: E402

if os.environ.get("MZLAB_GOLDEN_STRICT") == "1":
    import pyopenms as oms
else:
    oms = pytest.importorskip("pyopenms")
NS = "{http://psi.hupo.org/ms/mzml}"
FAMILIES = ["aperti", "agilent", "waters", "shimadzu", "sciex", "thermo", "bruker", "andi"]
# accession -> (codec, zlib after it)
CODECS = {
    "MS:1002312": ("LINEAR", False), "MS:1002313": ("PIC", False), "MS:1002314": ("SLOF", False),
    "MS:1002746": ("LINEAR", True), "MS:1002747": ("PIC", True), "MS:1002748": ("SLOF", True),
}


@pytest.fixture(scope="module")
def cli(tmp_path_factory):
    need(shutil.which("cargo"), "cargo missing")
    b = run(["cargo", "build", "--release", "--locked", "-p", "mzlab-cli"])
    assert b.returncode == 0, b.stderr[-2000:]
    return {"cli": ROOT / "target/release/mzlab-cli", "tmp": tmp_path_factory.mktemp("numpress")}


def convert(t, ds: Path, tag: str, compression: str) -> Path:
    out = t["tmp"] / f"out_{tag}_{compression}"
    r = run([str(t["cli"]), "convert", str(ds), "--out", str(out), "--fresh", "--workers", "1", "--compression", compression])
    assert r.returncode == 0, r.stdout[-1500:] + r.stderr[-1500:]
    files = sorted(out.glob("*.mzML"))
    assert len(files) == 1, files
    return files[0]


def arrays(p: Path):
    """[(owner id, declared length, array accession, accessions, base64 text)] for every binaryDataArray of spectra and chromatograms."""
    out = []
    root = etree.parse(str(p)).getroot()
    for owner in [*root.iter(f"{NS}spectrum"), *root.iter(f"{NS}chromatogram")]:
        n = int(owner.get("defaultArrayLength"))
        for a in owner.iter(f"{NS}binaryDataArray"):
            accs = {c.get("accession") for c in a.findall(f"{NS}cvParam")}
            kind = sorted(accs & {"MS:1000514", "MS:1000515", "MS:1000595", "MS:1000516"})
            out.append(((owner.tag.rsplit("}", 1)[-1], owner.get("id")), int(a.get("arrayLength", n)), tuple(kind), accs, a.findtext(f"{NS}binary") or ""))
    return out


def plain(accs, text):
    b = base64.b64decode(text)
    if "MS:1000574" in accs:
        b = zlib.decompress(b)
    return np.frombuffer(b, dtype="<f8" if "MS:1000523" in accs else "<f4").astype(float)


def by_reference(accs, text):
    (codec, z), = [CODECS[a] for a in accs if a in CODECS]
    cfg = oms.NumpressConfig()
    cfg.np_compression = getattr(oms.MSNumpressCoder.NumpressCompression, codec)
    out = []
    oms.MSNumpressCoder().decodeNP(text, out, z, cfg)
    return codec, np.asarray(out, dtype=float)


@pytest.mark.parametrize("fam", FAMILIES)
def test_numpress_arrays_read_back_through_the_reference_decoder(cli, fam):
    ds = create(fam, cli["tmp"] / f"in_{fam}")
    zl = {(o, k): (n, a, t) for o, n, k, a, t in arrays(convert(cli, ds, fam, "standard"))}
    seen = 0
    for owner, n, kind, accs, text in arrays(convert(cli, ds, fam, "smallest")):
        if not CODECS.keys() & accs:
            continue
        codec, got = by_reference(accs, text)
        assert len(got) == n, f"{fam} {owner} {kind} {codec}: {len(got)} values, {n} declared"
        want = plain(zl[(owner, kind)][1], zl[(owner, kind)][2])
        assert len(want) == n
        if codec == "SLOF":
            tol = 1e-3 * np.abs(want) + 1e-3
        elif codec == "PIC":
            tol = 0.5 + 1e-9 * np.abs(want)
        else:
            tol = 1e-6 * np.maximum(1.0, np.abs(want))
        bad = np.flatnonzero(np.abs(got - want) > tol)
        assert bad.size == 0, f"{fam} {owner} {kind} {codec}: at {bad[:5]} {got[bad[:5]]} vs {want[bad[:5]]}"
        seen += 1
    # the writer uses numpress for spectra of at least 3 points (`mzmlwrite/matrix.rs`)
    if any(n >= 3 and o[0] == "spectrum" for (o, _), (n, _, _) in zl.items()):
        assert seen > 0, f"{fam}: no numpress array written"
