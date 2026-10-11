"""Reference answers of mzlab/project.py for crates/mzlab-core/tests/project_parity.rs.

    python3 tools/project_riferimento.py OUT.json

Writes [{name, label, time, type, conc, cunit}] for a few thousand generated file names (no compound names).
"""
import itertools
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.project import guess_conc, guess_sample  # noqa: E402


def names() -> list[str]:
    heads = ["", "s_", "Sample-", "std_", "STD", "stds ", "calib_", "standard-", "cal", "taratura_", "curva ", "blank", "BLK_", "bianco_", "mrm-", "full_", "x"]
    cores = ["t0", "t7.5", "t30", "t7,5", "5", "0.5", "10", "120min", "2h", "1d", "3days", "3day", "2 ore", "4gg", "1giorno", "2giorni", "30 min", "7_2ppm", "5ppm", "0.5ppm",
             "10mgL", "5_ug-L", "2 mg/L", "3p5ppb", "1ng/mL", "1,5 ppt", "5 g/l", "2mg/l-1", "4 mgml", "9µg/L", "8μg/l", "1d2", "t5m", "5ms", "20 xyz", "", "12", "3.", "3,"]
    tails = ["", "_1", "-2", " rep", "_t3", "ppm", "_A", "5", "mgL"]
    exts = ["", ".mzML", ".MZML", ".raw", ".d", ".wiff"]
    out = []
    for h, c, t, e in itertools.product(heads, cores, tails, exts):
        out.append(h + c + t + e)
    return out[:: max(1, len(out) // 20000)] + ["a.b.c", ".mzML", "..", "std", "blank", "t", "_", "5_", "_5", "t_5", "std_", "std_5.", "std_5p5", "std_5x"]


if __name__ == "__main__":
    res = []
    for n in names():
        label, t, typ = guess_sample(n)
        c = guess_conc(n)
        res.append({"name": n, "label": label, "time": t, "type": typ, "conc": c[0] if c else None, "cunit": c[1] if c else None})
    Path(sys.argv[1]).write_text(json.dumps(res), encoding="utf-8")
    print(len(res))
