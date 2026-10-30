"""Writes the synthetic mini suspect index and the answers of the Python reader (for the Rust parity test, crates/mzlab-feat/tests/suspects.rs).

    python3 tools/sospetti_riferimento.py DIR      # DIR/mini.mzsus and DIR/casi.json
"""
import json
import struct
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import genera_sospetti as gs  # noqa: E402
from sospetti_prova import make_sources  # noqa: E402


FORM_NAMES = ["", "as_listed", "parent", "salt", "anion"]

# invented rows of the regulatory extension: (mass, key14, full key, name, [(list, form, bits, entry)])
LISTING_ROWS = [
    (59.037114, "ZORBAMIDEXXXAA", "ZORBAMIDEXXXAA-UHFFFAOYSA-N", "Zorbamide", [(0, 2, 0, 0), (1, 1, 1, 1)]),
    (60.021129, "QUILLICACIDXAA", "QUILLATEANIONA-UHFFFAOYSA-M", "Quillate", [(0, 4, 4, 2)]),
    (60.021129, "WENDOLENEXXXAA", "", "Wendolene", []),
    (146.0579, "MIRELLAXXXXXAA", "MIRELLAXXXXXAA-UHFFFAOYSA-N", "Mirella \u00e9", [(1, 3, 2, 3), (1, 1, 0, 4)]),
]
LISTING_ENTRIES = [
    {"ref": "Act 1, Annex I, no. 1", "name": "Zorbamide", "cas": "9100-11-3", "ec": "901-001-1", "attr": "AA-EQS inland=0,5"},
    {"ref": "Act 2, Table 1, no. 7", "name": "Zorbamide", "cas": "9100-11-3", "ec": "", "attr": ""},
    {"ref": "Act 1, Annex I, no. 2", "name": "Quillate", "cas": "9100-12-4", "ec": "901-002-2", "attr": "AA-EQS inland=1,2"},
    {"ref": "Act 2, Table 1, no. 9", "name": "Mirella \u00e9", "cas": "", "ec": "", "attr": "MCL 4 ng/L"},
    {"ref": "Act 2, Table 2, no. 1", "name": "Mirella", "cas": "9100-14-6", "ec": "", "attr": ""},
]


def pad8(b: bytes) -> bytes:
    return b + b"\0" * (-len(b) % 8)


def strings(items: list[bytes]) -> bytes:
    offs = [0]
    for b in items:
        offs.append(offs[-1] + len(b))
    return pad8(struct.pack(f"<{len(offs)}I", *offs)) + pad8(b"".join(items))


def write_listings(rows=LISTING_ROWS, entries=LISTING_ENTRIES, nl: int = 2) -> bytes:
    """A v1 file (flag bit 0 set) with the `listings` extension, written from scratch here: the reference does not share code with the Rust writer."""
    n = len(rows)
    formulas = ["C2H5NO", "C2H4O2", "C6H6"]
    man = json.dumps({"formato": "mzsus/1", "righe": n, "normative/1": {"lists": [{"id": "L0"}, {"id": "L1"}]}}, sort_keys=True, separators=(",", ":")).encode()
    members = [m for r in rows for m in r[4]]
    entry_json = [json.dumps(e, ensure_ascii=False, separators=(",", ":")).encode() for e in entries]
    offs, c = [], 0
    for r in rows:
        offs.append(c)
        c += len(r[4])
    offs.append(c)
    blob = b"".join(f.encode() for f in formulas)
    fo = [0]
    for f in formulas:
        fo.append(fo[-1] + len(f))
    out = [pad8(struct.pack("<4sHHIIIIII", b"MZSU", 1, 1, n, len(formulas), 0, len(man), 1, 0)), pad8(man),
           pad8(struct.pack(f"<{n}d", *[r[0] for r in rows])), pad8(b"".join(r[1].encode() for r in rows)),
           pad8(struct.pack(f"<{n}I", *[i % len(formulas) for i in range(n)])), pad8(bytes([8] * n)),
           pad8(bytes(2 * n)), pad8(bytes(2 * n)), b"", pad8(struct.pack(f"<{len(fo)}I", *fo)), pad8(blob),
           pad8(struct.pack("<IIII", nl, len(members), len(entries), sum(len(e) for e in entry_json))),
           pad8(b"".join(r[2].ljust(27).encode() for r in rows)), strings([r[3].encode() for r in rows]),
           pad8(struct.pack(f"<{n + 1}I", *offs)), pad8(b"".join(struct.pack("<HBBI", *m) for m in members)), strings(entry_json)]
    return b"".join(out)


def read_listings(data: bytes) -> dict:
    """Parses the extension of `write_listings` by offsets, the way docs/agenti/sospetti.md lays it out, and returns the JSON of `Listings::to_json`."""
    _, _, _, n, nf, nr, nman, flags, _ = struct.unpack_from("<4sHHIIIIII", data)
    assert flags & 1
    p = 32

    def take(size: int) -> bytes:
        nonlocal p
        b = data[p:p + size]
        assert len(b) == size, "truncated"
        p += size + (-size % 8)
        return b

    for size in (nman, 8 * n, 14 * n, 4 * n, n, 2 * n, 2 * n, 8 * nr):
        take(size)
    fo = struct.unpack(f"<{nf + 1}I", take(4 * (nf + 1)))
    take(fo[-1])
    nl, m, ne, _ = struct.unpack("<IIII", take(16))
    full = take(27 * n)

    def strs(count: int) -> list[str]:
        o = struct.unpack(f"<{count + 1}I", take(4 * (count + 1)))
        b = take(o[-1])
        return [b[o[i]:o[i + 1]].decode("utf-8") for i in range(count)]

    names = strs(n)
    mo = struct.unpack(f"<{n + 1}I", take(4 * (n + 1)))
    mb = take(8 * m)
    ents = strs(ne)
    # the InChIKey-14 column starts after the manifest and the mass column
    k0 = 32 + len(pad8(b"x" * nman)) + len(pad8(b"x" * (8 * n)))
    keys = data[k0:k0 + 14 * n]
    rows = []
    for i in range(n):
        mem = []
        for j in range(mo[i], mo[i + 1]):
            lst, form, bits, ent = struct.unpack_from("<HBBI", mb, 8 * j)
            mem.append({"list": lst, "form": FORM_NAMES[form], "bits": bits, "entry": json.loads(ents[ent])})
        rows.append({"key": keys[14 * i:14 * i + 14].decode(), "full_key": full[27 * i:27 * i + 27].decode().rstrip(), "name": names[i], "in": mem})
    return {"lists": nl, "rows": rows}


def main(out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        fonti, keys, mass, _ = make_sources(Path(tmp))
        raw = gs.genera(fonti, Path(tmp), scarica=False)
    (out / "mini.mzsus").write_bytes(raw)
    s = gs.leggi_sospetti(raw)
    qs = [(mass[k] * f, ppm) for k in keys for f, ppm in ((1.0, 0.0), (1.0, 5.0), (1 + 4e-6, 5.0), (1 + 6e-6, 5.0), (1.0, 5000.0))] + [(50.0, 5.0), (700.0, 5.0)]
    casi = [{"mass": m, "ppm": ppm, "hits": [{"key": h.inchikey14, "mass": h.mass, "formula": h.formula, "sources": sum(gs.SOURCE_BITS[x] for x in h.sources),
                                              "pubmed": h.pubmed, "patents": h.patents, "reaction": h.reaction} for h in s.cerca(m, ppm)]} for m, ppm in qs]
    (out / "casi.json").write_text(json.dumps({"rows": s.n, "casi": casi}), encoding="utf-8")
    ext = write_listings()
    (out / "listings.mzsus").write_bytes(ext)
    (out / "listings.json").write_text(json.dumps(read_listings(ext), ensure_ascii=False, separators=(",", ":")), encoding="utf-8")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
