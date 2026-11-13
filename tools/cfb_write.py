"""Writes an OLE2 compound file (the container of Analyst `.dam` / `.wiff`) from a dict {"Dir/Sub/Stream": bytes}.

Only for tests: version 3 (512-byte sectors, 64-byte mini sectors, 4096 cutoff), no DIFAT sectors (up to about 7 MB), the directory tree is a plain right-leaning
chain, which `mzlab.reader.ole` and the Rust reader walk like any other tree.
"""
from __future__ import annotations

import struct

END, FREE, FATSECT = 0xFFFFFFFE, 0xFFFFFFFF, 0xFFFFFFFD
SS, MSS, CUTOFF = 512, 64, 4096


def _entry(name: str, typ: int, left=FREE, right=FREE, child=FREE, start=0, size=0) -> bytes:
    raw = name.encode("utf-16le")[:62]
    return (raw.ljust(64, b"\0") + struct.pack("<HBB", len(raw) + 2 if name else 0, typ, 1) + struct.pack("<III", left, right, child) + bytes(16) + bytes(4) + bytes(16)
            + struct.pack("<IQ", start, size))


def _tree(streams: dict) -> list:
    """Directory entries as (name, type, parent index, stream path or None); index 0 is the root."""
    ents = [("Root Entry", 5, None, None)]
    where = {"": 0}
    for path in sorted(streams):
        parts = path.split("/")
        for i in range(1, len(parts)):
            sub = "/".join(parts[:i])
            if sub not in where:
                where[sub] = len(ents)
                ents.append((parts[i - 1], 1, where["/".join(parts[:i - 1])], None))
        ents.append((parts[-1], 2, where["/".join(parts[:-1])], path))
    return ents


def write(streams: dict, path) -> None:
    ents = _tree(streams)
    kids: dict[int, list[int]] = {}
    for i, e in enumerate(ents[1:], 1):
        kids.setdefault(e[2], []).append(i)
    links = {}                                   # entry index -> (right, child)
    for parent, ch in kids.items():
        for a, b in zip(ch, ch[1:]):
            links[a] = b
    first = {p: ch[0] for p, ch in kids.items()}
    mini, minifat, start, size = bytearray(), [], {}, {}
    big = {}
    for i, e in enumerate(ents):
        if e[3] is None:
            continue
        d = streams[e[3]]
        size[i] = len(d)
        if len(d) < CUTOFF:
            n = max(1, -(-len(d) // MSS))
            start[i] = len(minifat)
            minifat += list(range(len(minifat) + 1, len(minifat) + n)) + [END]
            mini += d.ljust(n * MSS, b"\0")
        else:
            big[i] = d
    dirsec = -(-len(ents) * 128 // SS)
    mfsec = -(-len(minifat) * 4 // SS)
    minisec = -(-len(mini) // SS)
    bigsec = {i: -(-len(d) // SS) for i, d in big.items()}
    rest = dirsec + mfsec + minisec + sum(bigsec.values())
    nfat = 1
    while nfat * (SS // 4) < nfat + rest:
        nfat += 1
    fat = [FATSECT] * nfat
    cursor = nfat

    def chain(n):
        nonlocal cursor
        ids = list(range(cursor, cursor + n))
        cursor += n
        fat.extend(ids[1:] + [END] if ids else [])
        return ids

    dir_ids = chain(dirsec)
    mf_ids = chain(mfsec)
    mini_ids = chain(minisec)
    big_ids = {i: chain(n) for i, n in bigsec.items()}
    fat += [FREE] * (nfat * (SS // 4) - len(fat))
    start[0], size[0] = (mini_ids[0] if mini_ids else END), len(mini)
    for i, ids in big_ids.items():
        start[i] = ids[0]
    dirs = b""
    for i, e in enumerate(ents):
        dirs += _entry(e[0], e[1], FREE, links.get(i, FREE), first.get(i, FREE), start.get(i, 0 if e[1] != 2 else END), size.get(i, 0))
    dirs = dirs.ljust(dirsec * SS, b"\0")
    blank = _entry("", 0, FREE, FREE, FREE, 0, 0)
    dirs = dirs[:len(ents) * 128] + blank * (dirsec * SS // 128 - len(ents))
    head = (b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1" + bytes(16) + struct.pack("<HHHHH", 0x3E, 3, 0xFFFE, 9, 6) + bytes(6)
            + struct.pack("<IIIIIIIII", 0, nfat, dir_ids[0], 0, CUTOFF, mf_ids[0] if mf_ids else END, mfsec, END, 0))
    difat = list(range(nfat)) + [FREE] * (109 - nfat)
    head += struct.pack("<109I", *difat)
    body = struct.pack(f"<{len(fat)}I", *fat) + dirs
    body += struct.pack(f"<{len(minifat)}I", *minifat).ljust(mfsec * SS, b"\xff") if mfsec else b""
    body += bytes(mini).ljust(minisec * SS, b"\0")
    for i, d in big.items():
        body += d.ljust(bigsec[i] * SS, b"\0")
    with open(path, "wb") as f:
        f.write(head.ljust(SS, b"\0") + body)
