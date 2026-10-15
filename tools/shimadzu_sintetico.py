"""Synthetic Shimadzu files (invented numbers, no laboratory data) for tests_e2e/e2e_shimadzu.py: an OLE2 container writer and a `.qgd` / `.lcd` (LCMS-2020) in the layout of
docs/agenti/shimadzu.md. `qgd(n)` and `lcd(n)` return (bytes, tic per scan)."""
import struct

END, FREE, FATSECT = 0xFFFFFFFE, 0xFFFFFFFF, 0xFFFFFFFD


def ole(streams):
    """An OLE2 container of 512-byte sectors, every stream a regular chain (mini-stream cutoff 0); `streams` = [(path, bytes)], a path with `/` makes its storage."""
    ents = [dict(n="Root Entry", t=5, p=0, s=END, d=0, c=FREE, r=FREE)]
    data = []
    for path, b in streams:
        parts, par = path.split("/"), 0
        for i, n in enumerate(parts):
            last = i + 1 == len(parts)
            j = next((k for k in range(1, len(ents)) if ents[k]["p"] == par and ents[k]["n"] == n and ents[k]["t"] == (2 if last else 1)), None)
            if j is None:
                ents.append(dict(n=n, t=2 if last else 1, p=par, s=END, d=0, c=FREE, r=FREE))
                j = len(ents) - 1
                if last:
                    data.append(b)
            par = j
    prev = {}
    for j in range(1, len(ents)):
        p = ents[j]["p"]
        if p in prev:
            ents[prev[p]]["r"] = j
        else:
            ents[p]["c"] = j
        prev[p] = j
    nd = -(-len(ents) // 4)
    sec = lambda n: -(-n // 512)
    nf = 1
    while nf * 128 < nf + nd + sum(sec(len(b)) for b in data):
        nf += 1
    assert nf <= 109
    fat = [FREE] * (nf * 128)
    fat[:nf] = [FATSECT] * nf
    for i in range(nd):
        fat[nf + i] = nf + i + 1 if i + 1 < nd else END
    at, body, k = nf + nd, b"", 0
    for e in ents[1:]:
        if e["t"] != 2:
            continue
        b = data[k]; k += 1
        e["d"] = len(b)
        n = sec(len(b))
        if n:
            e["s"] = at
            for i in range(n):
                fat[at + i] = at + i + 1 if i + 1 < n else END
            body += b + b"\0" * (n * 512 - len(b)); at += n
    h = bytearray(512)
    h[:8] = bytes([0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1])
    struct.pack_into("<HHHHH", h, 0x18, 0x3E, 3, 0xFFFE, 9, 6)
    struct.pack_into("<IIIIII", h, 0x2C, nf, nf, 0, 0, END, 0)
    struct.pack_into("<II", h, 0x3C, END, 0)
    struct.pack_into("<II", h, 0x44, END, 0)
    for i in range(109):
        struct.pack_into("<I", h, 0x4C + 4 * i, i if i < nf else FREE)
    out = bytes(h) + struct.pack("<%dI" % len(fat), *fat)
    d = b""
    for e in ents:
        u = e["n"].encode("utf-16-le")
        r = bytearray(128); r[:len(u)] = u
        struct.pack_into("<HBB", r, 0x40, len(u) + 2, e["t"], 1)
        struct.pack_into("<III", r, 0x44, FREE, e["r"], e["c"])
        struct.pack_into("<II", r, 0x74, e["s"], e["d"])
        d += bytes(r)
    return out + d.ljust(nd * 512, b"\0") + body


def qgd(n=12):
    """`n` EI scans (m/z 50, 51, 55 and a fourth that moves), 2-byte intensities; times 1 s apart."""
    raw, off, tic = b"", [], []
    for i in range(n):
        off.append(len(raw))
        pts = [(1000, 100 + i), (1020, 7), (1100, 3000 + 10 * i), (1200 + i, 1)]
        r = bytearray(32); struct.pack_into("<HH", r, 20, 2, len(pts))
        raw += bytes(r) + b"".join(struct.pack("<HH", m, y) for m, y in pts)
        tic.append(sum(y for _, y in pts))
    return ole([("GCMS Raw Data/MS Raw Data", raw), ("GCMS Raw Data/Spectrum Index", struct.pack("<%dI" % n, *off)),
                ("GCMS Raw Data/Retention Time", struct.pack("<%dI" % n, *[1000 * i for i in range(n)])),
                ("GCMS Raw Data/TIC Data", struct.pack("<%dQ" % n, *tic))]), tic


def lcd(n=12):
    """`n` scans of an LCMS-2020 `.lcd`, event 0, 4-byte intensities."""
    raw, off, tic = b"", [], []
    for i in range(n):
        off.append(len(raw))
        pts = [(2000 + 100 * j, 10 * (j + 1) + i) for j in range(8)]
        r = bytearray(64); struct.pack_into("<H", r, 8, 0); struct.pack_into("<HH", r, 52, 4, len(pts))
        raw += bytes(r) + b"".join(struct.pack("<HI", m, y) for m, y in pts)
        tic.append(sum(y for _, y in pts))
    return ole([("Mass Raw Data/MS Raw Data", raw), ("Mass Raw Data/Spectrum Index", struct.pack("<%dI" % n, *off)),
                ("Mass Raw Data/Retention Time", struct.pack("<%dI" % n, *[1000 * i for i in range(n)])),
                ("Mass Raw Data/TIC Data", struct.pack("<%dQ" % n, *tic)), ("LSS Raw Data/x", b"\1\2\3")]), tic


def qtof():
    """A container with only a `QTFL RawData` storage: recognised and refused."""
    return ole([("QTFL RawData/MS Raw Data", b"\1\2\3\4")])
