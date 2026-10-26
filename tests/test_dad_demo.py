"""The UV matrix of the Full Scan demos (tools/dati_sintetici_dad.py): the five files carry it in the TWC chromatogram, the index and the checksum agree, and the file stays small."""
import base64
import re
import sys
import zlib
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402
import dati_sintetici_dad as dd  # noqa: E402

ESEMPI = ROOT / "mzlab" / "web" / "esempi"


def _matrix(path: Path):
    b = path.read_bytes()
    a = re.search(rb'<binaryDataArray [^>]*>(?:(?!</binaryDataArray>).)*?value="dad_matrix".*?</binaryDataArray>', b, re.S).group(0)
    p = {k.decode(): float(v) for k, v in re.findall(rb'<userParam name="(\w+)" value="([^"]+)"', a)}
    raw = zlib.decompress(base64.b64decode(re.search(rb"<binary>(.*?)</binary>", a, re.S).group(1)))
    return np.frombuffer(raw, "<f4").reshape(int(p["n_rt"]), int(p["n_wl"])), p


def test_demos_carry_a_small_consistent_matrix():
    for t in dd.DEMOS:
        f = ESEMPI / f"FullScan_t{t}.mzML"
        assert f.stat().st_size <= dd.STEP_MAX, f.name
        assert dd.check(f) == [], f.name
        m, p = _matrix(f)
        assert m.shape == (600, 201) and p["wl_start"] == 200 and p["wl_stop"] == 400 and p["rt_stop"] > p["rt_start"]


def test_bands_follow_the_compounds_and_the_kinetics():
    m, p = _matrix(ESEMPI / "FullScan_t10.mzML")
    rt = np.linspace(p["rt_start"], p["rt_stop"], int(p["n_rt"]))
    at = lambda c: dd.WL[m[int(np.argmin(abs(rt - c)))].argmax()]
    assert abs(at(14.30) - 224) <= 4 and abs(at(7.07) - 248) <= 4
    peak = lambda t, c: (lambda m, p: m[int(np.argmin(abs(np.linspace(p["rt_start"], p["rt_stop"], int(p["n_rt"])) - c)))].max())(*_matrix(ESEMPI / f"FullScan_t{t}.mzML"))
    assert peak(0, 14.30) > peak(30, 14.30)                  # the parent decays
    assert peak(0, 7.07) < peak(30, 7.07)                    # TP 1 grows


def test_injection_keeps_the_index_and_is_idempotent(tmp_path):
    body = b'<spectrumList count="1"><spectrum index="0" id="s"></spectrum></spectrumList>'
    chrom = ds._chrom(0, "TWC", np.linspace(0, 10, 50), np.zeros(50)).encode().replace(b'id="TWC"', b'id="TWC"').replace(b'index="0"', b'index="2"')
    a = b'<?xml version="1.0"?><indexedmzML><mzML><run>' + body + b'<chromatogramList count="1">'
    b = a + chrom + b"</chromatogramList></run></mzML>"
    idx = b'<indexList count="2"><index name="spectrum"><offset idRef="s">%d</offset></index><index name="chromatogram"><offset idRef="TWC">%d</offset></index></indexList>'
    f = tmp_path / "x.mzML"
    head = b + idx % (a.index(b"<spectrum "), len(a)) + b"<indexListOffset>%d</indexListOffset><fileChecksum>" % len(b)
    f.write_bytes(head + b"0</fileChecksum></indexedmzML>")
    dd.inject_dad(f, 10)
    once = f.read_bytes()
    assert dd.check(f) == [] and once.count(b"dad_matrix") == 1
    dd.inject_dad(f, 10)
    assert f.read_bytes() == once
