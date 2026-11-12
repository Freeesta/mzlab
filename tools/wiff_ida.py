"""The IDA (dependent) files of the `.wiff` reference (tools/wiff_riferimento.py, which passes itself as `r`): product-ion experiments whose precursor
the survey picks at every cycle, stored in `SampleSubtree/SampleN/DDERealTimeData` (crates/mzlab-core/src/wiff/dde.rs). No laboratory data is used."""
import struct
from pathlib import Path


def dde(rows) -> bytes:
    """A `DDERealTimeData` stream: a 32-byte head, then one 32-byte row per dependent slot: precursor m/z (f64, +0), survey intensity (f64, +16), charge (u16, +24)."""
    return bytes(32) + b"".join(struct.pack("<d8sdHH4s", mz, bytes(8), 1e5, z, 0, bytes(4)) for mz, z in rows)


def ida_file(out: Path, r):
    """`ida.wiff`: two samples of one method (survey, two product-ion experiments whose headers hold 500.0 and 321.0) over three cycles. Sample 1 was
    acquired by IDA (a `DDERealTimeData` stream, one row per dependent slot `cycle * 2 + experiment - 1`, masses 200.0 + slot), so each product-ion scan
    carries the mass of its row, never the one of the header, and its index rows hold no point count at +26 (finding WF-14b); sample 2 ran the same method
    without the stream, so its product-ion scans carry none (counted, `warn.wiff.idaPrecursor`).
    `ida_epi.wiff`: survey EMS + EPI whose header holds 188.40 over three cycles: sample 1 with rows 493.27 (z 1), 547.30 (z 2), 687.85 (no charge); sample 2
    without the stream; sample 3 with a stream one row short (refused whole); sample 4 with a second row that is not an m/z (NaN). `epi_fisso.wiff`: the same
    method and no stream anywhere: a targeted EPI keeps 188.40."""
    method = {"Experiment0/ExperimentHeader": r.header(15, 30.0), "Experiment1/ExperimentHeader": r.header(11, 500.0), "Experiment2/ExperimentHeader": r.header(11, 321.0),
              "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": r.ion_spray(5500.0)}
    blocks = [("scan", r.FULL if e == 0 else r.PRODUCT, 1000.0 * c + 100.0 * e, 1.0) for c in range(3) for e in range(3)]
    w = r.Writer()
    w.sample(1, blocks, method)
    w.sample(2, blocks, method)
    w.streams["SampleSubtree/Sample1/DDERealTimeData"] = dde([(200.0 + k, 0) for k in range(6)])
    # the IDA sample's index rows hold at +26 a value that is not the point count of the block (here its highest intensity): the reader must not take it as one
    idx = bytearray(w.streams["SampleSubtree/Sample1/Idx"])
    for i, (_, content, *_rest) in enumerate(blocks):
        struct.pack_into("<d", idx, 32 + 54 * i + 26, float(max(v for _, v in content)))
    w.streams["SampleSubtree/Sample1/Idx"] = bytes(idx)
    w.save(out, "ida")

    def rows_of(n, precs, charges=None):
        charges = charges or [None] * 3
        return [{"id": f"sample={n} period=1 cycle={c + 1} experiment={e + 1}", "level": 1 if e == 0 else 2,
                 "precursor": None if e == 0 else precs[c], "charge": None if e == 0 else charges[c]} for c in range(3) for e in range(2)]

    ida = {"1": [{"id": f"sample=1 period=1 cycle={c + 1} experiment={e + 1}", "level": 1 if e == 0 else 2, "precursor": None if e == 0 else 200.0 + 2 * c + e - 1, "charge": None}
                 for c in range(3) for e in range(3)],
           "2": [{"id": f"sample=2 period=1 cycle={c + 1} experiment={e + 1}", "level": 1 if e == 0 else 2, "precursor": None, "charge": None} for c in range(3) for e in range(3)]}
    epi = {"Experiment0/ExperimentHeader": r.header(15, 30.0), "Experiment1/ExperimentHeader": r.header(11, 188.40),
           "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": r.ion_spray(5500.0)}
    blocks = [("scan", r.FULL if e == 0 else r.PRODUCT, 1000.0 * c + 100.0 * e, 1.0) for c in range(3) for e in range(2)]
    good = [(493.27, 1), (547.30, 2), (687.85, 0)]
    w = r.Writer()
    for n in (1, 2, 3, 4):
        w.sample(n, blocks, epi)
    w.streams["SampleSubtree/Sample1/DDERealTimeData"] = dde(good)
    w.streams["SampleSubtree/Sample3/DDERealTimeData"] = dde(good[:2])
    w.streams["SampleSubtree/Sample4/DDERealTimeData"] = dde([good[0], (float("nan"), 1), good[2]])
    w.save(out, "ida_epi")
    w = r.Writer()
    w.sample(1, blocks, epi)
    w.save(out, "epi_fisso")
    none = [None] * 3
    return {"ida": ida, "unread": {"1": 0, "2": 6},
            "ida_epi": {"1": rows_of(1, [493.27, 547.30, 687.85], [1, 2, None]), "2": rows_of(2, none), "3": rows_of(3, none),
                        "4": rows_of(4, [493.27, None, 687.85], [1, None, None])},
            "ida_epi_unread": {"1": 0, "2": 3, "3": 3, "4": 1},
            "epi_fisso": {"1": rows_of(1, [188.40] * 3)}}
