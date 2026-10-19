"""Technical summary of high-resolution mzML files (Orbitrap, Q-TOF): numbers only, no sample or compound names.

    python3 tools/prova_hr.py CARTELLA_O_FILE [...]

For every .mzML it prints: size, scans per level, MS2 per cycle, resolution, isolation window, activation, spectrum mode,
and the size / time of the MS1 peak table (what the browser has to hold in memory). `tools/verifica.py` runs it on the
two cuttings of the private data repository (dati/HRMS) and, on Federico's Mac, on ../Data/HRMS (the whole files).
Written with regular expressions on the headers only, so it works on any file the reader opens (and says what is missing).
"""
from __future__ import annotations

import re
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.reader.mzml import Run  # noqa: E402


def _med(v):
    return float(np.median(v)) if len(v) else float("nan")


def summarize(path: Path) -> dict:
    t0 = time.time()
    run = Run(path)
    out = {"file": path.name, "MB": round(path.stat().st_size / 1e6, 1), "load_s": round(time.time() - t0, 2)}
    lv = np.array([s.level for s in run.scans])
    out["MS1"], out["MS2"] = int((lv == 1).sum()), int((lv == 2).sum())
    out["MS2_per_MS1"] = round(out["MS2"] / max(out["MS1"], 1), 2)
    res = {1: [], 2: []}
    iso, act = [], set()
    mm = run._mm
    for s in run.scans[:: max(1, len(run.scans) // 400)]:                  # a regular sample of the scans is enough
        h = mm[s.start:s.end].decode("utf-8", "replace")
        h = h[:h.find("<binaryDataArrayList")]
        m = re.search(r'accession="MS:1000800"[^>]*value="([^"]*)"', h)
        if m:
            res[min(s.level, 2)].append(float(m.group(1)))
        if s.level > 1:
            lo, hi = re.search(r'MS:1000828"[^>]*value="([^"]*)"', h), re.search(r'MS:1000829"[^>]*value="([^"]*)"', h)
            if lo and hi:
                iso.append((float(lo.group(1)), float(hi.group(1))))
            act.update(re.findall(r'accession="(MS:100(?:0422|0133|1880|0262))"', h))
    out["res_MS1"], out["res_MS2"] = _med(res[1]), _med(res[2])
    out["isolation"] = sorted(set(iso))[:3]
    out["activation"] = sorted(act)
    out["mode_MS1"] = "profile" if any(s.profile for s in run.scans if s.level == 1) else "centroid"
    t0 = time.time()
    tb = run.table(1)
    out["MS1_peaks"], out["table_s"] = int(len(tb.mz)), round(time.time() - t0, 1)
    out["table_MB"] = round((tb.mz.nbytes + tb.inten.nbytes + tb.pos.nbytes) / 1e6)
    out["parent_refs"] = sum(1 for s in run.scans if s.level > 1 and b"spectrumRef" in mm[s.start:s.end])
    run.close()
    return out


def main(args: list[str]) -> int:
    files = []
    for a in args:
        p = Path(a)
        files += sorted(p.glob("*.mzML")) if p.is_dir() else [p]
    if not files:
        print("prova_hr: nessun file mzML in", args); return 0
    for f in files:
        try:
            r = summarize(f)
        except Exception as e:                                       # noqa: BLE001
            print(f"{f.name}: ERRORE {e}"); continue
        print(f"{r['file']}: {r['MB']} MB · {r['MS1']} MS1 + {r['MS2']} MS2 ({r['MS2_per_MS1']} per ciclo) · risoluzione {r['res_MS1']:g}/{r['res_MS2']:g} · "
              f"isolamento {r['isolation']} · attivazione {r['activation']} · MS1 {r['mode_MS1']} · {r['MS1_peaks']} picchi MS1, tabella {r['table_MB']} MB in {r['table_s']} s · "
              f"MS2 con spectrumRef: {r['parent_refs']}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
