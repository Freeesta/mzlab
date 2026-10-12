"""Synthetic Agilent MassHunter .D folder (MSScan.bin + MSPeak.bin + the small files), written with the layout of MSScan.xsd: no laboratory data.
    python3 tools/agilent_sintetico.py <out folder> [name.D] [scans]
A chromatographic peak of m/z 84 and 86 (ratio 3:2) at 3.2 min on a flat background of m/z 51 and 207, 3.0 to 3.4 min."""
import math
import struct
import sys
from pathlib import Path

REC = struct.Struct("<iiidiidddiiiiddddddd" + "ii" + "i" + "q" + "ii" + "dddd")


def scans(n):
    out = []
    for k in range(n):
        t = 3.0 + 0.4 * k / max(1, n - 1)
        g = math.exp(-((t - 3.2) / 0.03) ** 2)
        out.append((t, [(51.0, 40.0), (84.0, 20.0 + 5000.0 * g), (86.0, 15.0 + 3300.0 * g), (207.0, 60.0)]))
    return out


def write(root: Path, name: str = "synt.D", n: int = 120) -> Path:
    d = root / name
    (d / "AcqData").mkdir(parents=True, exist_ok=True)
    peak = bytearray(68)
    head = bytearray(160)
    head[0:2] = b"\x01\x01"
    head[88:92] = struct.pack("<i", 160)
    recs = bytearray()
    for k, (t, pts) in enumerate(scans(n)):
        off = len(peak)
        peak += b"".join(struct.pack("<d", p[0]) for p in pts) + b"".join(struct.pack("<d", float(p[1])) for p in pts)
        tic = sum(p[1] for p in pts)
        bp = max(pts, key=lambda p: p[1])
        recs += REC.pack(k + 1, 1, 0, t, 1, 1, tic, bp[0], bp[1], k + 1, 0, 2, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 1050.0, 0.0, 0, 0, 1, off, len(pts) * 16, len(pts), 1.7e308, bp[1], 0.0, 0.0)
    (d / "AcqData/MSScan.bin").write_bytes(bytes(head + recs))
    (d / "AcqData/MSPeak.bin").write_bytes(bytes(peak))
    (d / "AcqData/MSTS.xml").write_text(f'<?xml version="1.0"?><TimeSegments><Version>3</Version><TimeSegment TimeSegmentID="1"><StartTime>3</StartTime><EndTime>3.4</EndTime><NumOfScans>{n}</NumOfScans></TimeSegment></TimeSegments>', encoding="utf-8")
    (d / "AcqData/Devices.xml").write_text('<Devices><Device DeviceID="1"><Name>SingleQuadrupole</Name><ModelNumber>5977</ModelNumber><SerialNumber>SYNTH0001</SerialNumber></Device></Devices>', encoding="utf-8")
    (d / "AcqData/Contents.xml").write_text("<Contents><AcquiredTime>2025-01-02T10:11:12.5+01:00</AcquiredTime><SeparationTechnique>2</SeparationTechnique><AcqSoftwareVersion>MassHunter GC/MS Acquisition  10.0.1.1</AcqSoftwareVersion></Contents>", encoding="utf-8")
    return d


if __name__ == "__main__":
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    print(write(out, sys.argv[2] if len(sys.argv) > 2 else "synt.D", int(sys.argv[3]) if len(sys.argv) > 3 else 120))
