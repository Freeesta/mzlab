"""`dispositivi.wiff` of tools/wiff_riferimento.py: a 3200 QTRAP full scan with version-1 blocks (the site sends it to the readers module) and the real-time
device traces of `Devices/Device_N/{Channel, DevRealTimeHeader, DevData}` as the reader parses them: Device_0 a column pressure in psi and a pump flow,
Device_1 a second column pressure in bar (same name: the reader tells it apart by the device) and a channel with no points (not listed). No laboratory data."""
import math
import struct

PSI, BAR = 6_894.757_293_168, 1e5  # pascal per unit, written out here so the expectation shares no table with the reader


def channel(name, unit, step_s, idx):
    n, u = name.encode("utf-16le"), f"({unit})".encode("utf-16le")
    return bytes(4) + struct.pack("<H", len(n)) + n + struct.pack("<H", len(u)) + u + struct.pack("<dHff", step_s, idx, 0.0, 1000.0) + bytes(4)


def device(streams, n, dev, chans, points):
    """chans: [(name, unit, step in s)]; points: one row of values (one per channel) per time step."""
    base = f"SampleSubtree/Sample{n}/Devices/Device_{dev}/"
    streams[base + "Channel"] = bytes(36) + struct.pack("<H", len(chans)) + b"".join(channel(c, u, s, i) for i, (c, u, s) in enumerate(chans))
    total = len(points) * len(chans)
    streams[base + "DevRealTimeHeader"] = bytes(32) + struct.pack("<I", total)
    streams[base + "DevData"] = bytes(32) + b"".join(struct.pack(f"<{len(r)}d", *r) for r in points)


def devices_file(out, writer, header):
    """Writes the pair and returns what the reader must list after the TIC: [{id, kind, unit, time (min), intensity}]."""
    blocks = [("scan", [(100 + k, 50 + k)], 1000.0 * k, 50.0 + k, 1) for k in range(30)]
    w = writer()
    w.sample(1, blocks, {"Experiment0/ExperimentHeader": header(15, 30.0)}, log="3200 QTRAP")
    pres = [[2000.0 + 50 * math.sin(k / 4), 300.0 + k * 0.5] for k in range(40)]  # psi, µL/min every 0.5 s
    device(w.streams, 1, 0, [("Column Pressure", "psi", 0.5), ("Pump A Flowrate", "µL/min", 0.5)], pres)
    bar = [[140.0 + k * 0.25, 0.0] for k in range(10)]
    device(w.streams, 1, 1, [("Column Pressure", "bar", 2.0), ("Oven Temperature", "°C", 2.0)], bar)
    w.streams["SampleSubtree/Sample1/Devices/Device_2/Channel"] = bytes(36) + struct.pack("<H", 1) + channel("Empty", "V", 1.0, 0)
    w.save(out, "dispositivi")
    t0, t1 = [k * 0.5 / 60 for k in range(40)], [k * 2.0 / 60 for k in range(10)]
    return [
        {"id": "Column Pressure", "kind": "pressure", "unit": "pascal", "time": t0, "intensity": [r[0] * PSI for r in pres]},
        {"id": "Pump A Flowrate", "kind": "analog", "unit": "µL/min", "time": t0, "intensity": [r[1] for r in pres]},
        {"id": "Column Pressure #1", "kind": "pressure", "unit": "pascal", "time": t1, "intensity": [r[0] * BAR for r in bar]},
        {"id": "Oven Temperature", "kind": "analog", "unit": "°C", "time": t1, "intensity": [r[1] for r in bar]},
    ]
