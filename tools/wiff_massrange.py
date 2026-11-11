"""Synthetic MRM `.wiff` pairs whose `MassRangeEx` is written by the counted grammar (crates/mzlab-core/tests/wiff_massrange.rs).

    python3 tools/wiff_massrange.py OUT

`tabella`: one MRM experiment whose header announces 4 entries (`ExperimentHeader` +176) and a stream of exactly 4 (40-byte head, 5 parameter
blocks each): a named row, an unnamed row, a row whose name is not ASCII and one whose name is 151 characters (longer than one length byte holds). `conta`: the header announces 5, the stream holds 4. `coda`: the
stream has 4 bytes after its 4 entries. OUT/massrange.json: the transitions expected of `tabella` and the refusals of the others. Built with the
Writer of wiff_riferimento.py and an encoder written here from the grammar; the numbers are invented, no laboratory data is used.
"""
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wiff_riferimento import Writer, header, ion_spray  # noqa: E402

PARAMS = ("DP", "EP", "CEP", "CE", "CXP")
ROWS = [(364.1, 194.1, 25.0, "Quant", (50.0, 10.0, 22.5, 20.0, 3.0)), (364.1, 152.1, 25.0, "", (50.0, 10.0, 22.5, 31.0, 4.0)),
        (250.25, 100.5, 40.0, "Qual-é", (60.0, 9.0, 15.0, 27.5, 2.0)), (412.5, 201.25, 30.0, "L" * 151, (55.0, 10.0, 18.0, 33.0, 5.0))]


def utf16(t: str) -> bytes:
    return t.encode("utf-16le")


def entry(q1, q3, dwell, name, values) -> bytes:
    b = struct.pack("<ffffI", q1, 0.0, q3, dwell, 0) + struct.pack("<H", len(utf16(name))) + utf16(name)
    for tag, v in zip(PARAMS, values):
        b += struct.pack("<H", len(utf16(tag))) + utf16(tag) + struct.pack("<fff", v, v, 0.0)
    return b


def stream(rows) -> bytes:
    return bytes(32) + struct.pack("<II", 2, len(PARAMS)) + b"".join(entry(*r) for r in rows)


def counted_header(n) -> bytes:
    h = bytearray(header(4, 0.0))
    h[176:180] = struct.pack("<I", n)
    return bytes(h)


def pair(out: Path, name: str, announced: int, table: bytes):
    method = {"Experiment0/ExperimentHeader": counted_header(announced), "Experiment0/MassRangeEx/MassRangeEx": table,
              "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
    w = Writer()
    w.sample(1, [("mrm", [100.0 + c, 200.0 + c, 300.0 + c, 400.0 + c], 60000.0 * c, 600.0 + 3 * c) for c in range(4)], method)
    w.save(out, name)


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    pair(out, "tabella", 4, stream(ROWS))
    pair(out, "conta", 5, stream(ROWS))
    pair(out, "coda", 4, stream(ROWS) + bytes(4))
    want = {"tabella": [{"q1": round(q1, 2), "q3": round(q3, 2), "dwell": dw, "name": nm, "ce": v[3], "dp": v[0], "cxp": v[4]} for q1, q3, dw, nm, v in ROWS],
            "rifiuti": {"conta": 5, "coda": 4}}
    (out / "massrange.json").write_text(json.dumps(want, ensure_ascii=False))


if __name__ == "__main__":
    main()
