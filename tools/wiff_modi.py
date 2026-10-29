"""Matrix of the scan modes of a SCIEX QTRAP as synthetic `.wiff` pairs (reference for crates/mzlab-core/tests/wiff_modi.rs).

    python3 tools/wiff_modi.py OUT

Writes OUT/modi/<SIGLA>.wiff + .wiff.scan for 14 modes (one 3200 QTRAP experiment, 4 cycles, version 2 blocks; EPI also a version 3 sample, EMS also a
version 1 one) and OUT/modi/modi.json: per mode the code (ExperimentHeader +122), whether it is confirmed on real files, what the reader does today
("letto" with level / precursor / CV type, or "rifiuto" with the exact `detail`) and what it should do once supported ("futuro"). Code 0 (Q1, three samples with one shared Method1) is seen in real files and read like EMS; codes 1-3, 6, 12-14, 16 are
guesses from the Analyst ScanType enum and the Q1MI / Q3MI blocks (one f32 per ion) are a hypothesis. A mode that gets supported changes its entry in
modi.json: the test is the executable matrix. No laboratory data is used.
"""
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wiff_riferimento as w  # noqa: E402

CYCLES = 4
PREC = {"EPI": 55.5, "PI": 55.5, "PREC": 184.0, "NL": 217.3, "ER": 55.5, "EMC": 55.5, "MS3": 55.5, "TDF": 55.5}
CV = {"PREC": "MS:1000341", "NL": "MS:1000326"}
IONS = [100.5, 200.25, 300.0]
TRS = [(364.5, 176.0, 50.0, "tpA", 40.0, 10.0, 25.0, 5.0), (364.5, 159.0, 25.5, "tpB", 41.0, 10.5, 35.0, 6.0), (250.25, 100.5, 100.0, "ref", 30.0, 9.0, 15.0, 4.0)]
# sigla: (code, confirmed on real files, kind of block, level once supported, CV type once supported)
MODES = {
    "EMS": (15, True, "scan", 1, None), "EMC": (16, False, "scan", 1, "MS:1000789"), "EPI": (11, True, "scan", 2, None),
    "ER": (12, False, "scan", 1, "MS:1000791"), "MRM": (4, True, "mrm", None, None), "MS3": (13, False, "scan", 3, None),
    "NL": (7, True, "scan", 2, "MS:1000326"), "PREC": (5, True, "scan", 2, "MS:1000341"), "PI": (6, False, "scan", 2, None),
    "Q1": (0, True, "scan", 1, None), "Q1MI": (1, False, "ions", None, None), "Q3": (2, False, "scan", 1, None),
    "Q3MI": (3, False, "ions", None, None), "TDF": (14, False, "scan", 2, None),
}
READ = {"EMS", "EPI", "MRM", "NL", "PREC", "Q1"}


def pts(k):
    """Sparse profile of cycle k: a few peaks on bins that stay inside the version 1 range."""
    return [(10 + 3 * k, 5), (200, 1200 + k), (201, 90), (1500 + k, 777), (2400, 20)]


def method(sigla, code):
    header = bytearray(w.header(code, PREC.get(sigla, 0.0)))
    header[86:90] = struct.pack("<I", 0)  # positive
    if sigla == "MRM":
        range_ex = w.mrm_method(TRS)
    elif sigla in ("Q1MI", "Q3MI"):
        range_ex = bytes(8) + struct.pack(f"<{len(IONS)}f", *IONS)  # hypothesis: the masses of the ions as float32
    elif MODES[sigla][2] == "scan":
        range_ex = w.mass_range(25.0) if sigla in PREC else bytes(8)
    else:
        range_ex = bytes(8)
    return {"Experiment0/ExperimentHeader": bytes(header), "Experiment0/MassRangeEx/MassRangeEx": range_ex,
            "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": w.ion_spray(5500.0)}


def blocks(sigla, kind, version):
    out = []
    for k in range(CYCLES):
        t = 1000.0 * k
        if kind == "scan":
            p = pts(k)
            out.append(("scan", p, t, float(sum(c for _, c in p)), version))
        elif kind == "mrm":
            v = [100.0 + k, 200.0 + k, 300.0 + k]
            out.append(("mrm", v, t, sum(v)))
        else:
            v = [10.0 * (j + 1) + k for j in range(len(IONS))]
            out.append(("mrm", v, t, sum(v)))  # one float32 per ion, written like a dense MRM block
    return out


def oggi(sigla, code, kind, version):
    if sigla not in READ:
        return {"esito": "rifiuto", "detail": f"scan type code {code} (experiment 1)"}
    if kind == "mrm":
        return {"esito": "letto", "spettri": 0, "cromatogrammi": 1 + len(TRS), "tipo": None}
    sp = [w.spectrum(f"sample=1 period=1 cycle={k + 1} experiment=1", 1 if sigla in ("EMS", "Q1") else 2, PREC.get(sigla) if sigla not in ("EMS", "Q1") else None, 1000.0 * k, pts(k), version)
          for k in range(CYCLES)]
    return {"esito": "letto", "spettri": CYCLES, "livello": sp[0]["level"], "precursore": sp[0]["precursor"], "tipo": CV.get(sigla),
            "mz": sp[0]["mz"], "intensita": sp[0]["intensity"]}


def main(out=None):
    out = Path(out if out is not None else sys.argv[1]) / "modi"
    out.mkdir(parents=True, exist_ok=True)
    info = {}
    for sigla, (code, confirmed, kind, level, cv) in MODES.items():
        wr = w.Writer()
        samples = [(1, 2)]
        if sigla == "EPI":
            samples.append((2, 3))
        if sigla == "EMS":
            samples.append((2, 1))
        if sigla == "Q1":
            # a batch run: only Method1 exists, samples 2 and 3 take the experiment from it
            samples += [(2, 2), (3, 2)]
        for n, version in samples:
            own = None if sigla == "Q1" and n > 1 else method(sigla, code)
            wr.sample(n, blocks(sigla, kind, version), own, log="3200 QTRAP")
        wr.save(out, sigla)
        info[sigla] = {"codice": code, "codice_confermato": confirmed, "polarita": 1,
                       "campioni": [{"n": n, "versione": v, "oggi": oggi(sigla, code, kind, v)} for n, v in samples],
                       "futuro": {"livello": level, "tipo_cv": cv, "precursore": PREC.get(sigla), "sim": sigla in ("Q1MI", "Q3MI"),
                                  "spettri": kind == "scan", "ioni": IONS if kind == "ions" else None}}
    (out / "modi.json").write_text(json.dumps(info))


if __name__ == "__main__":
    main()
