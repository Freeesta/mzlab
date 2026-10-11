"""Real exports of an untargeted campaign (mzMine, FBMN, GNPS2): ONLY with MZLAB_UNTARGET pointing to a local clone of the private data repository of the exports (never copied here).
The counts are those of the specification; nothing is printed that names a file, a site or a compound."""
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB, JS = ROOT / "mzlab" / "web", ROOT / "TP_Mine" / "js"


def _base():
    c = os.environ.get("MZLAB_UNTARGET")
    if not c:
        return None
    for p in Path(c).glob("*/02 Data/*Exploris*"):
        if (p / "1 - mzMine" / "DDA_POS" / "merged").is_dir():
            return p
    return None


pytestmark = [pytest.mark.skipif(_base() is None, reason="MZLAB_UNTARGET (clone of the exports) not available"), pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")]


def _find(d, name):
    low = name.lower()
    for f in Path(d).rglob("*"):
        if f.is_file() and f.name.lower() == low:
            return f
    raise FileNotFoundError(name)


def test_counts_of_the_real_export_and_the_import_time():
    b = _base(); m = b / "1 - mzMine" / "DDA_POS" / "merged"
    paths = {"full": _find(m, "merged_pos.csv"), "ann": _find(m, "merged_pos_annotations.csv"), "quant": _find(m, "merged_pos_GNPS_quant.csv"), "gnps": _find(b / "3 - GNPS", "merged_results_with_gnps.tsv")}
    code = f"""globalThis.window = globalThis; const store = {{}}; globalThis.localStorage = {{ getItem: k => (k in store ? store[k] : null), setItem: (k, v) => {{ store[k] = String(v); }} }};
      require({str(WEB / 'livelli-1.js')!r}); require({str(WEB / 'livelli.js')!r}); const U = require({str(JS / '25_untarget.js')!r}); const fs = require('fs'); const P = {json.dumps({k: str(v) for k, v in paths.items()})};
      const rd = k => fs.readFileSync(P[k], 'utf8'); const out = {{}};
      let t0 = Date.now(); const q = U.readMzmineQuant(rd('quant'), {{ pol: 1 }}); out.quant = q.features.length; out.quantMs = Date.now() - t0;
      t0 = Date.now(); const t = U.readMzmineFull(rd('full'), {{ pol: 1 }}); out.full = t.features.length; out.fullMs = Date.now() - t0;
      U.mergeMzmineAnnotations(t, rd('ann')); out.lipids = t.stats.lipidRowsDropped;
      U.readGnpsLibrary(q, rd('gnps')); out.gnpsRows = q.stats.gnpsRows; out.gnpsAnalogs = q.stats.gnpsAnalogs;
      console.log(JSON.stringify(out));"""
    t0 = time.time()
    p = subprocess.run(["node", "--max-old-space-size=4096", "-e", code], check=True, capture_output=True, encoding="utf-8")
    r = json.loads(p.stdout)
    assert r["quant"] == 16367 and r["gnpsRows"] == 1128 and r["gnpsAnalogs"] == 1006 and r["lipids"] == 52, r
    assert r["full"] == r["quant"], r
    print("import of the full table:", r["fullMs"], "ms; total", round(time.time() - t0, 1), "s")
