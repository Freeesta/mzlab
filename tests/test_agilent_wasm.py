"""The Agilent `.D` reader in WebAssembly (mzlab-wasm, `agilent_to_mzml`, through Node): a synthetic folder written here with an encoder of its own (the layout of
`MSScan.xsd`, no laboratory data) comes out as an mzML with the same spectra, times and TIC. Skipped without cargo, wasm-bindgen or node."""
import base64
import re
import shutil
import struct
import subprocess
import zlib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")

REC = struct.Struct("<iiidiidddiiiiddddddd" + "ii" + "i" + "q" + "ii" + "dddd")
SCANS = [(3.0, [(50.5, 10.0), (84.0, 500.0), (86.25, 320.5)]), (3.01, [(51.0, 20.0), (84.0, 1000.0)]), (3.02, []), (3.03, [(60.0, 5.0), (61.0, 7.0), (62.5, 9.0), (70.0, 1.0)])]


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


def write_folder(root: Path) -> Path:
    d = root / "x.D"
    (d / "AcqData").mkdir(parents=True)
    peak = bytearray(68)
    head = bytearray(160)
    head[0:2] = b"\x01\x01"
    head[88:92] = struct.pack("<i", 160)
    recs = bytearray()
    for k, (t, pts) in enumerate(SCANS):
        off = len(peak)
        peak += b"".join(struct.pack("<d", p[0]) for p in pts) + b"".join(struct.pack("<d", p[1]) for p in pts)
        tic = sum(p[1] for p in pts)
        recs += REC.pack(k + 1, 1, 0, t, 1, 1, tic, 0.0, 0.0, k + 1, 0, 2, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 1050.0, 0.0, 0, 0, 1, off, len(pts) * 16, len(pts), 1.7e308, 0.0, 0.0, 0.0)
    (d / "AcqData/MSScan.bin").write_bytes(bytes(head + recs))
    (d / "AcqData/MSPeak.bin").write_bytes(bytes(peak))
    return d


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    out = tmp_path_factory.mktemp("pkg")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    return out


def test_the_wasm_writes_the_same_spectra(pkg, tmp_path):
    d = write_folder(tmp_path)
    out = tmp_path / "x.mzML"
    info = run("node", str(ROOT / "tests/agilent_wasm_node.js"), str(pkg), str(d), str(out))
    assert '"last":[3,3]' in info
    x = out.read_text(encoding="utf-8")
    specs = re.findall(r'<spectrum index="(\d+)" id="([^"]+)" defaultArrayLength="(\d+)">(.*?)</spectrum>', x, re.S)
    kept = [(k, s) for k, s in enumerate(SCANS) if s[1]]
    assert len(specs) == len(kept)
    for (_, sid, n, body), (k, (t, pts)) in zip(specs, kept):
        arr = re.findall(r"<binary>([^<]*)</binary>", body)
        mz = struct.unpack(f"<{len(pts)}d", zlib.decompress(base64.b64decode(arr[0])))
        it = struct.unpack(f"<{len(pts)}f", zlib.decompress(base64.b64decode(arr[1])))
        assert sid == f"scan={k + 1}" and int(n) == len(pts)
        assert list(mz) == [p[0] for p in pts] and list(it) == [p[1] for p in pts]
        assert float(re.search(r'scan start time" value="([^"]+)"', body).group(1)) == t
    assert 'id="TIC" defaultArrayLength="4"' in x


def test_an_index_that_is_not_understood_is_an_error_with_a_key(pkg, tmp_path):
    d = write_folder(tmp_path)
    p = d / "AcqData/MSScan.bin"
    p.write_bytes(p.read_bytes()[:-5])
    r = subprocess.run(["node", str(ROOT / "tests/agilent_wasm_node.js"), str(pkg), str(d), str(tmp_path / "y.mzML")], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode != 0 and "err.file.agilent" in r.stderr


def write_data_ms(root: Path) -> Path:
    """A folder with only the ChemStation data.ms (an older run): big-endian records of twentieths of m/z."""
    d = root / "old.D"
    d.mkdir()
    b = bytearray(0x140)
    b[0:2] = b"\x01\x32"
    for o, s in ((0x04, "GC / MS Data File"), (0x18, "old run")):
        b[o] = len(s)
        b[o + 1 : o + 1 + len(s)] = s.encode()
    b[0x10A:0x10C] = struct.pack(">H", (0x140 + 2) // 2)
    scans = [(60000, [(1019, 10), (1680, 500)]), (60300, [(1680, 20), (4138, 30)])]
    b[0x116:0x11A] = struct.pack(">I", len(scans))
    for ms, peaks in scans:
        b += struct.pack(">HIHHHH", (24 + 4 * (len(peaks) + 1)) // 2, ms, 12, 1, 0, len(peaks))
        top = max(peaks, key=lambda p: p[1])
        b += struct.pack(">HH", *top)
        for m, a in reversed(peaks):
            b += struct.pack(">HH", m, a)
        b += struct.pack(">HHHI", 5, 4, 0, sum(a for _, a in peaks))
    (d / "data.ms").write_bytes(bytes(b))
    return d


def test_a_folder_with_only_data_ms_is_converted(pkg, tmp_path):
    d = write_data_ms(tmp_path)
    out = tmp_path / "old.mzML"
    run("node", str(ROOT / "tests/agilent_wasm_node.js"), str(pkg), str(d), str(out))
    x = out.read_text(encoding="utf-8")
    specs = re.findall(r'<spectrum index="(\d+)" id="([^"]+)" defaultArrayLength="(\d+)">', x)
    assert [s[2] for s in specs] == ["2", "2"] and 'id="TIC" defaultArrayLength="2"' in x
