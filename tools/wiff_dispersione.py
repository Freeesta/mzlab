"""`dispersione.wiff` of tools/wiff_riferimento.py: a method with a counted `MassRangeEx` (the X500R / ZenoTOF layout) that holds what the scans do not say,
the declustering potential and the collision energy spread, and a sample named for another mass. No laboratory data."""
import struct


def counted_range(entries, names=("DP", "CE", "DPS", "CES")) -> bytes:
    """A counted `MassRangeEx` of a scan experiment (the X500R / ZenoTOF layout: `n_params` = 4): per range its start, stop and step, then every parameter as its UTF-16 tag and a float32 start, stop and step."""
    out = bytearray(32) + struct.pack("<II", 2, len(names))
    for start, stop, step, values in entries:
        out += struct.pack("<ffffIH", start, stop, step, 0.1, 0, 0)
        for n, v in zip(names, values):
            tag = n.encode("utf-16le")
            out += struct.pack("<H", len(tag)) + tag + struct.pack("<fff", v, v, 0.0)
    return bytes(out)


def spread(out, writer, header, ion_spray, full, product):
    """`dispersione.wiff`: a method that holds what the scans do not say: a full scan (DP 60 V) and a product-ion experiment of precursor 253.1 with CE 35 eV and a collision energy spread of 15 eV (the data are the sum of the steps 20 / 35 / 50), and a sample named for another mass (`Name_237.1`)."""
    def head(code, prec):
        d = bytearray(header(code, prec))
        d[176:180] = struct.pack("<I", 1)
        return bytes(d)
    method = {"Experiment0/ExperimentHeader": head(15, 30.0), "Experiment1/ExperimentHeader": head(11, 253.1),
              "Experiment0/MassRangeEx/MassRangeEx": counted_range([(100.0, 300.0, 0.06, (60.0, 10.0, 0.0, 0.0))]),
              "Experiment1/MassRangeEx/MassRangeEx": counted_range([(50.0, 300.0, 0.06, (60.0, 35.0, 0.0, 15.0))]),
              "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
    blocks = [("scan", full if e == 0 else product, 1000.0 * c + 100.0 * e, 1.0) for c in range(2) for e in range(2)]
    w = writer()
    w.sample(1, blocks, method)
    # `SampleDABE/DATA` names the sample when it holds four or more counted strings (name, ..., vial)
    w.streams["SampleSubtree/Sample1/SampleDABE/DATA"] = b"".join(struct.pack("<H", 2 * len(t)) + t.encode("utf-16le") for t in ("Name_237.1", "a", "b", "1:A,1"))
    w.save(out, "dispersione")
    return {"precursor": 253.1, "ce": 35.0, "ces": 15.0, "dp": 60.0, "name_mz": 237.1}


def damaged(out, writer, header, full, product):
    """Three damaged methods, one file each: `cut_header.wiff` (the `ExperimentHeader` of the product-ion experiment ends before the scan-type code), `cut_range.wiff` (the `MassRangeEx` announces two ranges and holds one) and `bad_tag.wiff` (a parameter tag of odd length, the garbage a bad copy leaves)."""
    blocks = [("scan", full if e == 0 else product, 1000.0 * c + 100.0 * e, 1.0) for c in range(2) for e in range(2)]

    def head(code, prec, count=1, cut=None):
        d = bytearray(header(code, prec))
        d[176:180] = struct.pack("<I", count)
        return bytes(d[:cut] if cut else d)

    def make(name, headers, ranges):
        method = {"Experiment0/ExperimentHeader": head(15, 30.0), "Experiment1/ExperimentHeader": headers,
                  "Experiment0/MassRangeEx/MassRangeEx": counted_range([(100.0, 300.0, 0.06, (60.0, 10.0, 0.0, 0.0))]),
                  "Experiment1/MassRangeEx/MassRangeEx": ranges}
        w = writer()
        w.sample(1, blocks, method)
        w.save(out, name)

    good = counted_range([(50.0, 300.0, 0.06, (60.0, 35.0, 0.0, 15.0))])
    make("cut_header", head(11, 253.1, cut=100), good)
    make("cut_range", head(11, 253.1, count=2), good)
    make("bad_tag", head(11, 253.1), good.replace(struct.pack("<H", 4) + "CE".encode("utf-16le"), struct.pack("<H", 3) + "CE".encode("utf-16le")[:3] + b"\x00", 1))
    return ["cut_header", "cut_range", "bad_tag"]
