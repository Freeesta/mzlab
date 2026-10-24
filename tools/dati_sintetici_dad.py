"""Synthetic UV (DAD / PDA) matrix 200-400 nm for the Full Scan demos `FullScan_t{0,5,10,15,30}.mzML` and for the synthetic Full Scan files.

The matrix sits in the PDA `<chromatogram id="TWC">` as one extra `<binaryDataArray>` (`MS:1000786` `dad_matrix`, f32 little-endian, zlib, row-major
rows x wavelengths, axes in `userParam`s) that `crates/mzlab-core/src/mzml_dad.rs` reads. Spectra and the 1D arrays are not touched; the offsets of the
`<indexList>` and the checksum are recomputed. Every number is invented: bands and kinetics follow the compounds of `full_scan` in dati_sintetici.py.

    python3 tools/dati_sintetici_dad.py --esempi          # adds the matrix to the five demos in mzlab/web/esempi/ (idempotent)
"""
from __future__ import annotations

import base64
import hashlib
import math
import re
import sys
import zlib
from pathlib import Path

import numpy as np

WL = np.arange(200.0, 401.0)            # 201 channels, 1 nm
N_RT = 600
DEMOS = (0, 5, 10, 15, 30)
STEP_MAX = 6_000_000                    # bytes: the demos stay well under the 10 MiB of tests/test_esempi_anonimi.py
_exp = np.frompyfunc(math.exp, 1, 1)    # scalar libm: the same bytes on every machine (see dati_sintetici._libm)


def _gauss(x, c, s):
    return np.asarray(_exp(-0.5 * ((np.asarray(x, float) - c) / s) ** 2), float)


def _kinetics(t: float):
    """(RT, sigma_t, amplitude in mAU, [(lambda, sigma, weight), ...]) of the four compounds at treatment time t."""
    e = math.exp
    return [
        (14.30, 0.07, 1800 * e(-0.035 * t), [(224, 14, 1.0), (272, 18, 0.65)]),
        (7.07, 0.06, 950 * (1 - e(-0.04 * t)), [(248, 16, 1.0), (212, 11, 0.45)]),
        (13.00, 0.06, 800 * (1 - e(-0.03 * t)) ** 2, [(284, 19, 1.0), (220, 13, 0.50)]),
        (14.70, 0.06, 900 * (1 - e(-0.06 * t)) * e(-0.01 * t), [(260, 17, 1.0), (218, 12, 0.55)]),
    ]


BACKGROUND = ((1.15, 210), (2.10, 215), (18.30, 235))      # constant peaks: (RT, lambda), the same in every file


def build_dad_matrix(t_min: float, rt_grid) -> tuple[np.ndarray, dict]:
    """`(len(rt_grid), 201)` float32 absorbance in mAU and the parameters of its axes."""
    rt = np.asarray(rt_grid, float)
    mat = np.full((rt.size, WL.size), 20.0)
    for c, s, a, bands in _kinetics(t_min):
        band = sum(w * _gauss(WL, l, sl) for l, sl, w in bands)
        mat += a * np.outer(_gauss(rt, c, s), band)
    for c, l in BACKGROUND:
        mat += 600.0 * np.outer(_gauss(rt, c, 0.08), _gauss(WL, l, 15.0))
    mat += np.random.default_rng(int(t_min) + 101).normal(0, 0.4, mat.shape)
    mat = np.round(mat, 1).astype("<f4")                     # a tenth of a mAU: the zlib stream stays small
    return mat, {"wl_start": 200, "wl_stop": 400, "wl_step": 1, "n_rt": rt.size, "n_wl": WL.size,
                 "rt_start": float(rt[0]), "rt_stop": float(rt[-1])}


def dad_array_xml(mat: np.ndarray, p: dict) -> str:
    users = "".join(f'<userParam name="{k}" value="{v!r}"/>' if isinstance(v, float) else f'<userParam name="{k}" value="{v}"/>' for k, v in p.items())
    data = base64.b64encode(zlib.compress(np.asarray(mat, "<f4").tobytes(), 9)).decode()
    return ('<binaryDataArray encodedLength="%d"><cvParam cvRef="MS" accession="MS:1000521" name="32-bit float" value=""/>'
            '<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            '<cvParam cvRef="MS" accession="MS:1000786" name="non-standard data array" value="dad_matrix"/>%s<binary>%s</binary></binaryDataArray>'
            % (len(data), users, data))


def _twc_times(chrom: bytes) -> np.ndarray:
    """Minutes of the time array of a chromatogram block (zlib or not, 32 or 64 bit, seconds or minutes)."""
    for m in re.finditer(rb"<binaryDataArray .*?</binaryDataArray>", chrom, re.S):
        a = m.group(0)
        if b"MS:1000595" not in a:
            continue
        raw = base64.b64decode(re.search(rb"<binary>(.*?)</binary>", a, re.S).group(1))
        raw = zlib.decompress(raw) if b"MS:1000574" in a else raw
        t = np.frombuffer(raw, "<f4" if b"MS:1000521" in a else "<f8").astype(float)
        return t / 60 if b'unitName="second"' in a or b"UO:0000010" in a else t
    raise ValueError("no time array in the TWC chromatogram")


def _splice(b: bytes, at: int, cut: int, ins: bytes) -> bytes:
    """`b` with `cut` bytes at `at` replaced by `ins`; the offsets of the index that point after `at` follow, then the checksum."""
    new = b[:at] + ins + b[at + cut:]
    delta = len(ins) - cut
    shift = lambda m: m.group(1) + (str(int(m.group(2)) + delta).encode() if int(m.group(2)) > at else m.group(2)) + m.group(3)
    new = re.sub(rb"(<offset [^>]*>)(\d+)(</offset>)", shift, new)
    return re.sub(rb"(<indexListOffset>)(\d+)(</indexListOffset>)", shift, new)


def _count_step(b: bytes, a: int, step: int) -> bytes:
    """The `count` of the first `<binaryDataArrayList` after `a`, changed by `step`."""
    m = re.compile(rb'<binaryDataArrayList count="(\d+)"').search(b, a)
    return _splice(b, m.start(1), len(m.group(1)), str(int(m.group(1)) + step).encode())


def inject_dad(path: Path, t_min: float) -> int:
    """Adds the matrix to the TWC chromatogram of `path` (replacing an older one); returns the new size in bytes."""
    b = Path(path).read_bytes()
    a = b.index(b'<chromatogram index="2" id="TWC"')
    old = re.compile(rb'<binaryDataArray [^>]*>(?:(?!</binaryDataArray>).)*?value="dad_matrix".*?</binaryDataArray>', re.S).search(b, a)
    if old:
        b = _count_step(_splice(b, old.start(), old.end() - old.start(), b""), a, -1)
    t = _twc_times(b[a:b.index(b"</chromatogram>", a)])
    mat, p = build_dad_matrix(t_min, np.linspace(t[0], t[-1], N_RT))
    close = b.index(b"</binaryDataArrayList>", a)
    new = _count_step(_splice(b, close, 0, dad_array_xml(mat, p).encode()), a, 1)
    tag = b"<fileChecksum>"
    if tag in new:
        k = new.index(tag) + len(tag)
        new = new[:k] + hashlib.sha1(new[:k]).hexdigest().encode() + new[new.index(b"</fileChecksum>"):]
    Path(path).write_bytes(new)
    return len(new)


def check(path: Path) -> list[str]:
    """What is wrong with the index and the checksum of `path` (empty when all is right)."""
    b, bad = Path(path).read_bytes(), []
    for m in re.finditer(rb'<offset idRef="([^"]*)">(\d+)</offset>', b):
        if not b[int(m.group(2)):].startswith((b"<spectrum ", b"<chromatogram ")):
            bad.append(f"offset of {m.group(1).decode()} points at {b[int(m.group(2)):][:20]!r}")
    m = re.search(rb"<indexListOffset>(\d+)</indexListOffset>", b)
    if m and not b[int(m.group(1)):].startswith(b"<indexList"):
        bad.append("indexListOffset")
    if b"<fileChecksum>" in b:
        k = b.index(b"<fileChecksum>") + 14
        if hashlib.sha1(b[:k]).hexdigest().encode() != b[k:b.index(b"</fileChecksum>")]:
            bad.append("checksum")
    return bad


def esempi(root: Path) -> None:
    for t in DEMOS:
        p = root / "mzlab" / "web" / "esempi" / f"FullScan_t{t}.mzML"
        n = inject_dad(p, t)
        bad = check(p)
        print(f"{p.name}: {n / 1e6:.2f} MB", "OK" if not bad and n <= STEP_MAX else f"PROBLEM {bad} {n}")


if __name__ == "__main__":
    if sys.argv[1:] != ["--esempi"]:
        sys.exit(__doc__)
    esempi(Path(__file__).resolve().parent.parent)
