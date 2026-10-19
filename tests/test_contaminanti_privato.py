"""Private run (skipped without the data repository): how many of the 100 most intense peaks of 10 MS1 spectra of the real Orbitrap files are flagged by the built-in list,
and which entries come up most often (it tells which background the instruments of the laboratory show). The numbers are printed (pytest -s), nothing is asserted but sanity."""
import bisect
import sys
from collections import Counter
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tools"))
from dati_percorsi import percorso  # noqa: E402


def _hrms():
    d = percorso("dati/HRMS")
    return d if d and list(d.glob("*.mzML")) else None


@pytest.mark.skipif(_hrms() is None, reason="MZLAB_DATI / QQQ_DATI with HRMS/ not available")
def test_contaminanti_nei_file_orbitrap():
    from mzlab.chem.contaminants import builtin
    from mzlab.reader.mzml import Run
    items = sorted((i for i in builtin()["items"] if i["pol"] == 1), key=lambda i: i["mz"]); mzs = [i["mz"] for i in items]
    tot = Counter(); flagged = n = 0
    for f in sorted(_hrms().glob("*.mzML")):
        r = Run(str(f)); ms1 = [s for s in r.scans if s.level == 1 and getattr(s, "polarity", 1) != -1][::max(1, len(r.scans) // 40)][:5]
        for s in ms1:
            a, b = r.read(s.index)
            top = sorted(range(len(a)), key=lambda i: -b[i])[:100]
            for i in top:
                m = float(a[i]); tol = max(m * 5e-6, 0.001 if m < 200 else 0); n += 1
                lo = bisect.bisect_left(mzs, m - tol); hit = [items[j] for j in range(lo, len(items)) if items[j]["mz"] <= m + tol]
                if hit:
                    flagged += 1; tot[hit[0]["name"] + " " + str(hit[0]["adduct"])] += 1
    print(f"\npeaks looked at: {n}; flagged: {flagged}; most frequent: {tot.most_common(8)}")
    assert n > 0 and flagged <= n
