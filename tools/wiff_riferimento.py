"""Reference for the parity test of the Rust `.wiff` / `.wiff.scan` reader (crates/mzlab-core/tests/wiff_parity.rs).

    python3 tools/wiff_riferimento.py OUT

Writes into OUT/ synthetic pairs in the layout of the 3200 QTRAP files (`one.wiff` + `.wiff.scan`: one sample with two experiments, a full scan and a
product-ion scan, with a method; `two.wiff` + `.wiff.scan`: a scan sample and an MRM sample without method; `lonely.wiff`: no `.scan`; `serie.wiff`: 80 profile scans with a chromatographic peak; `prodotto.wiff`: two samples of product-ion scans: one precursor 305.0, and two (229.1, 305.0), no reference entry), built with
tools/cfb_scrivi.py, and OUT/riferimento.json: what the reader must give (spectra with m/z = start + bin * step, intensity = count * scale; MRM
chromatograms), computed here from the inputs with an encoder of the nibble stream that shares no code with the reader. No laboratory data is used.
The variants of the readers module (feature `estesi`): `variante.wiff` (Log model QTRAP 5500; survey scan V1, enhanced product ion V3, precursor-ion scan V1, neutral-loss scan V1,
under "variante" in the JSON) and the files the module refuses with `err.file.wiff` (under "rifiuti": name -> text of the `detail`): two periods, a TOF scan type, a TOF block, a block version 4.
"""
import json
import math
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cfb_scrivi  # noqa: E402

MARKER = b"\xfe\xff\xff\xff\x02\x00\x00\x00"
HEAD = [100.02, 300.0, 0.06, 16666.667, 250.02, 400.0, 0.06, 50000.0]  # start1 end1 step scale1 start2 end2 step2 scale2


def encode(points) -> bytes:
    """Nibble stream: 1..7 a point; 0 + long a point; 8 + long a skip; 9..15 a short skip. A long is one byte, or 0 and a u32, on a byte boundary."""
    nibs = []

    def long(v):
        if len(nibs) % 2:
            nibs.append(0)
        raw = bytes([v]) if 1 <= v <= 255 else b"\0" + struct.pack("<I", v)
        for b in raw:
            nibs.extend((b >> 4, b & 15))

    pos = 0
    for b, c in points:
        gap = b - pos
        if 0 < gap <= 7:
            nibs.append(8 + gap)
        elif gap > 7:
            nibs.append(8)
            long(gap)
        if 1 <= c <= 7:
            nibs.append(c)
        else:
            nibs.append(0)
            long(c)
        pos = b + 1
    if len(nibs) % 2:
        nibs.append(0)
    return bytes(nibs[i] << 4 | nibs[i + 1] for i in range(0, len(nibs), 2))


RANGES = {1: [HEAD[:4]], 2: [HEAD[:4], HEAD[4:]], 3: [HEAD[:4], HEAD[4:], [320.02, 500.0, 0.06, 100000.0]]}  # the ranges of the block versions (the 3200 writes 2)


def scan_block(points, version=2) -> bytes:
    data = encode(points)
    block = MARKER[:4] + struct.pack("<I", version) + b"".join(struct.pack("<4d", *r) for r in RANGES.get(version, RANGES[2])) + struct.pack("<I", len(points)) + data
    return block if points else block + bytes(4)


def expected_scan(points, version=2):
    rs = RANGES[version]
    mz = [rs[0][0] + b * rs[0][2] for b, _ in points]
    return mz, [c * ([rs[0][3]] + [r[3] for r in rs[1:] if m > r[0]])[-1] for (_, c), m in zip(points, mz)]


def idx_row(offset, size, time_ms, tic, n) -> bytes:
    return struct.pack("<IId", offset, size, time_ms) + b"\x01\x00" + struct.pack("<dd", tic, float(n)) + bytes(20)


def header(code, precursor) -> bytes:
    d = bytearray(180)
    d[42:50] = struct.pack("<d", precursor)
    d[122:126] = struct.pack("<I", code)
    return bytes(d)


def ion_spray(volts) -> bytes:
    return bytes(24) + "IS".encode("utf-16le") + struct.pack("<ff", volts, volts) + bytes(4)


class Writer:
    def __init__(self):
        self.scan, self.streams, self.table, self.expect = bytearray(), {}, [], {}

    def sample(self, n, blocks, method=None, log=None):
        """blocks: [("scan", points, time_ms, tic[, version]) | ("mrm", values, time_ms, tic)] in file order; `log`: the model of the Log; a method path that starts with `Period` is taken whole."""
        start = len(self.scan)
        self.scan += bytes(44)
        idx, off = bytearray(32), 0
        for kind, content, t, tic, *ver in blocks:
            block = scan_block(content, *ver) if kind == "scan" else struct.pack(f"<{len(content)}f", *content)
            idx += idx_row(off, len(block), t, tic, len(content) if kind == "scan" else 0)
            self.scan += block
            off += len(block)
        self.table.append((n, start + 20))
        self.streams[f"SampleSubtree/Sample{n}/Idx"] = bytes(idx)
        for path, data in (method or {}).items():
            self.streams[f"MethodSubtree/Method{n}/DeviceMethod0/{path if path.startswith('Period') else 'Period0/' + path}"] = data
        if log:
            self.streams[f"SampleSubtree/Sample{n}/Log"] = b"\0\0" + f"Mass Spectrometer: Config Table, Component ID: {log}, Version: 1".encode("utf-16le") + b"\0\0"

    def save(self, out: Path, name: str):
        t = bytearray(32)
        for n, base in self.table:
            t += struct.pack("<II", n, base) + bytes(8)
        self.streams["SampleSubtree/SampleIdxTable"] = bytes(t)
        cfb_scrivi.write(self.streams, out / f"{name}.wiff")
        (out / f"{name}.wiff.scan").write_bytes(bytes(self.scan))


FULL = [(41, 1), (50, 200), (300, 341), (301, 70000), (1500, 20000000), (3000, 2), (9000, 5000000000 % (1 << 32))]
PRODUCT = [(10, 3), (20, 123), (21, 124), (22, 255), (23, 256), (2000, 65535), (2001, 65536), (2002, (1 << 24) - 1), (2010, 7), (2018, 5)]
LATE = [(1000, 0xFFFFFF), (1400, 0xFFFF)]
MRM = [[1.5, 20000.25, 0.0], [3.0, 4.5e6, 12.0], [0.0, 0.0, 0.0], [7.25, 1.0, 99999.0]]


def spectrum(id_, level, prec, t_ms, pts, version=2, term=None):
    mz, it = expected_scan(pts, version)
    return {"id": id_, "level": level, "precursor": prec, "rt": t_ms / 60000.0, "mz": mz, "intensity": it, "term": term}


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    method = {
        "Experiment0/ExperimentHeader": header(15, 30.0),
        "Experiment1/ExperimentHeader": header(11, 123.4),
        "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0),
    }
    w = Writer()
    w.sample(1, [("scan", FULL, 1180.0, 1.5e6), ("scan", [], 1630.0, 0.0), ("scan", LATE, 2500.0, 3.0), ("scan", PRODUCT, 3000.0, 4.5e3)], method)
    one = {"1": {"polarity": 1, "spectra": [spectrum("sample=1 period=1 cycle=1 experiment=1", 1, None, 1180.0, FULL),
                                             spectrum("sample=1 period=1 cycle=2 experiment=1", 1, None, 2500.0, LATE),
                                             spectrum("sample=1 period=1 cycle=2 experiment=2", 2, 123.4, 3000.0, PRODUCT)], "chromatograms": []}}
    w.save(out, "one")
    w = Writer()
    w.sample(1, [("scan", FULL, 600.0, 1.0)])
    times = [500.0, 1000.0, 1500.0, 2100.5]
    w.sample(2, [("mrm", v, t, sum(v)) for v, t in zip(MRM, times)])
    two = {"1": {"polarity": 0, "spectra": [spectrum("sample=1 period=1 cycle=1 experiment=1", 1, None, 600.0, FULL)], "chromatograms": []},
           "2": {"polarity": 0, "spectra": [], "chromatograms": [{"id": "TIC", "time": [t / 60000.0 for t in times], "intensity": [sum(v) for v in MRM]}]
                 + [{"id": f"SRM SIC sample=2 period=1 experiment=1 transition={j}", "time": [t / 60000.0 for t in times], "intensity": [v[j] for v in MRM]} for j in range(3)]}}
    w.save(out, "two")
    cfb_scrivi.write({"SampleSubtree/Sample1/Idx": bytes(32)}, out / "lonely.wiff")
    w, series = Writer(), []
    for k in range(80):  # a peak at m/z ~229.1 (9 bins wide) eluting at 4 min, on sparse noise: enough points per scan for the profile step
        height = round(60000 * math.exp(-((k - 40) / 8.0) ** 2))
        pts = [(2151 + j, 5 + round(height * math.exp(-(j / 2.5) ** 2))) for j in range(-4, 5)] + [(100 + 37 * j + k % 5, 3 + j % 4) for j in range(40)]
        pts.sort()
        series.append(("scan", pts, 6000.0 * k, float(sum(c for _, c in pts))))
    w.sample(1, series, {"Experiment0/ExperimentHeader": header(15, 30.0), "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)})
    w.save(out, "serie")
    big = {"1": {"polarity": 1, "spectra": [spectrum(f"sample=1 period=1 cycle={k + 1} experiment=1", 1, None, 6000.0 * k, series[k][1]) for k in range(80)], "chromatograms": []}}
    w = Writer()  # product-ion scans: sample 1 one precursor (305.0); sample 2 two (229.1 every cycle, 305.0 in about one cycle of four, the other scans empty), irregular times, dense scans
    rnd = random.Random(7)
    for n, precs in ((1, (305.0,)), (2, (229.1, 305.0))):
        blocks, t = [], 0.0
        for k in range(300):
            t += 1500.0 * (0.5 + rnd.random())
            for e in range(len(precs)):
                height = round(60000 * math.exp(-((k - 150) / 30.0) ** 2) * (1 if e == 0 else 0.5))
                dense = [(100 + 11 * j + rnd.randrange(5), 1 + rnd.randrange(6)) for j in range(300)]
                pts = sorted(set([(1500 + 100 * e + j, 5 + round(height * math.exp(-(j / 2.5) ** 2))) for j in range(-4, 5)] + dense))
                pts = [] if e == 1 and len(precs) == 2 and rnd.random() < 0.75 else pts
                blocks.append(("scan", pts, t + 200 * e, float(sum(c for _, c in pts))))
        w.sample(n, blocks, {f"Experiment{e}/ExperimentHeader": header(11, pr) for e, pr in enumerate(precs)} | {"Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)})
    w.save(out, "prodotto")
    variante, rifiuti = variants(out)
    (out / "riferimento.json").write_text(json.dumps({"one": one, "two": two, "serie": big, "variante": variante, "rifiuti": rifiuti}))


def variants(out: Path):
    """The files of the readers module. A QTRAP 5500 with four experiments (survey V1, enhanced product ion V3, precursor-ion scan V1, neutral-loss scan V1) over two cycles, and the refused files."""
    ex = {"Experiment0/ExperimentHeader": header(15, 30.0), "Experiment1/ExperimentHeader": header(11, 55.5), "Experiment2/ExperimentHeader": header(5, 184.0),
          "Experiment3/ExperimentHeader": header(7, 217.3), "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
    pts = {1: FULL, 3: PRODUCT, 5: [(5, 4), (6, 9), (400, 12000)], 7: [(70, 1), (71, 2), (72, 7), (900, 300)]}
    blocks, want = [], []
    for cycle in range(2):
        for e, (code, level, prec, version, term) in enumerate([(15, 1, None, 1, None), (11, 2, 55.5, 3, None), (5, 2, 184.0, 1, ("MS:1000341", "precursor ion spectrum")),
                                                                (7, 2, 217.3, 1, ("MS:1000326", "constant neutral loss spectrum"))]):
            p = [pts[1], pts[3], pts[5], pts[7]][e]
            p = p if not (e == 3 and cycle == 1) else []  # an empty block (no spectrum)
            t = 1000.0 * cycle + 100.0 * e
            blocks.append(("scan", p, t, float(sum(c for _, c in p)), version))
            if p:
                want.append(spectrum(f"sample=1 period=1 cycle={cycle + 1} experiment={e + 1}", level, prec, t, p, version, term))
    w = Writer()
    w.sample(1, blocks, ex, log="QTRAP 5500")
    w.save(out, "variante")
    variante = {"1": {"polarity": 1, "model": "QTRAP 5500", "spectra": want, "chromatograms": []}}
    rifiuti = {}
    one = [("scan", FULL, 1000.0, 1.0)]
    w = Writer()  # two periods: the reader has no real file with one, so it refuses
    w.sample(1, one, {"Experiment0/ExperimentHeader": header(15, 30.0), "Period1/Experiment0/ExperimentHeader": header(15, 30.0)}, log="QTRAP 6500+")
    w.save(out, "periodi")
    rifiuti["periodi"] = "periods=2"
    w = Writer()  # a TOF MS experiment
    w.sample(1, one, {"Experiment0/ExperimentHeader": header(8, 0.0)}, log="TripleTOF 6600")
    w.save(out, "tof")
    rifiuti["tof"] = "scan type code 8 (experiment 1)"
    w = Writer()  # an unknown code
    w.sample(1, one, {"Experiment0/ExperimentHeader": header(21, 0.0)}, log="QTRAP 5500")
    w.save(out, "codice")
    rifiuti["codice"] = "scan type code 21 (experiment 1)"
    w = Writer()  # a block of the TOF family under a scan experiment
    w.sample(1, one, {"Experiment0/ExperimentHeader": header(15, 30.0)}, log="QTRAP 5500")
    w.scan[44:48] = b"\xff\xff\xff\xff"
    w.save(out, "bloccotof")
    rifiuti["bloccotof"] = "TOF block"
    w = Writer()  # a scan block of a version nobody knows
    w.sample(1, one, {"Experiment0/ExperimentHeader": header(15, 30.0)}, log="QTRAP 5500")
    w.scan[48:52] = struct.pack("<I", 4)
    w.save(out, "versione4")
    rifiuti["versione4"] = "scan block version 4"
    return variante, rifiuti


if __name__ == "__main__":
    main()
