"""`metodocond.wiff` (synthetic): three samples that share one `Method1`; see `shared_method`. usage: wiff_metodo_condiviso.py <out dir>"""
import struct
import sys
from pathlib import Path

from wiff_riferimento import Writer, header, ion_spray, mrm_method


def shared_method(out: Path):
    """`metodocond.wiff`: a batch of three samples that keep one `Method1` (a dense and a scheduled MRM experiment, its `GLPTables`) as Analyst writes it when the
    samples share their method; sample 2 has an optical detector. What `method/1` says beyond the 3200 (`estesi`): the borrowed method, the scheduled window, the GLP table, the DAD header."""
    dense = [(300.0, 100.0, 50.0, "d0", 30.0, 9.0, 15.0, 4.0), (301.0, 101.0, 50.0, "d1", 30.0, 9.0, 16.0, 4.0)]
    sched = [(364.5, 176.0, 0.5, "s0", 40.0, 10.0, 25.0, 5.0), (250.25, 100.5, 2.5, "s2", 30.0, 9.0, 15.0, 4.0)]
    method = {"Experiment0/ExperimentHeader": header(4, 0.0), "Experiment0/MassRangeEx/MassRangeEx": mrm_method(dense), "Experiment1/ExperimentHeader": header(4, 0.0),
              "Experiment1/MassRangeEx/MassRangeEx": mrm_method(sched), "Experiment1/sMRM": bytes(36) + struct.pack("<II", 1, 60) + bytes(8),
              "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
    dense_blocks = [("mrm", [100.0 + c, 200.0 + c], 60000.0 * c, 300.0 + 2 * c) for c in range(4)]
    ts = [0.05 * k for k in range(8)]
    rows = [[5.0 + j + t for j in range(11)] for t in ts]
    w = Writer()
    w.sample(1, dense_blocks, method, log="Triple Quad 5500")
    w.sample(2, dense_blocks, None, dad=(ts, 210.0, 1.0, rows))
    w.sample(3, dense_blocks, None)
    text = "Calibration table\tmass\tsigma\n300.0\t0.01\nFile was created with the software version: 1.7.1"
    w.streams["MethodSubtree/Method1/GLPTables"] = bytes(8) + text.encode("utf-16le")
    w.save(out, "metodocond")


if __name__ == "__main__":
    shared_method(Path(sys.argv[1]))
