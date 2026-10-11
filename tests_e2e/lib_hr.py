"""Helpers of the high-resolution e2e tests (e2e_hr_*): synthetic Orbitrap DDA files and, when they are there, the two real cuttings of the private data repository."""
import sys
from pathlib import Path
import numpy as np
from lib import ROOT, HERE

sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402


def synth(name="hr"):
    """Folder with the three synthetic HR files (Exploris, Fusion, broken) and one Full Scan of the 3200 QTRAP look-alike."""
    d = HERE / "shots" / f"{name}_tmp"; d.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(31)
    ds.hr_dda(d / ds.HR_NAMES["exploris"], rng, "exploris"); ds.hr_dda(d / ds.HR_NAMES["fusion"], rng, "fusion"); ds.hr_dda(d / ds.HR_NAMES["broken"], rng, "exploris", broken=True)
    ds.full_scan(d / "B_FullMass-t0.mzML", 0, np.random.default_rng(2026))
    return d


def real_cuttings():
    """The two real Orbitrap cuttings of QqQ-lab-dati/HRMS (private repository), or []."""
    import verifica
    out = []
    for d in verifica.hrms_dirs():
        out += sorted(d.glob("*.mzML"))
    return [p for p in out if p.stat().st_size < 100e6]


def load(pg, files, wait=6000, n=None, settle=1500):
    """Open files in the start screen of the page and press «Carica dati»; n = number of entries E.files must reach (a DDA file gives two: MS1 and MS2)."""
    pg.set_input_files("#pick", [str(f) for f in files])
    pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length>={len(files)}", timeout=180000); pg.wait_for_timeout(300)
    pg.click("text=Carica dati"); pg.wait_for_timeout(wait)
    if n:
        pg.wait_for_function(f"E.files.length>={n}", timeout=90000); pg.wait_for_timeout(1500)


def hr_variants(src, d):
    """From a synthetic HR DDA file make three small HR files in folder d: only the Full Scans (HR_full-t5), only the MS2 scans (HR_ms2-t10: no parent scan,
    so not a DDA) and a single scan (HR_inf-t15, an infusion). Returns the three paths."""
    import re
    s = Path(src).read_text(encoding="utf-8")
    head, rest = s.split("<spectrumList", 1)
    lst, tail = rest.split("</spectrumList>", 1)
    attrs, body = lst.split(">", 1)
    spectra = re.findall(r"<spectrum .*?</spectrum>", body, flags=re.S)
    lvl = lambda sp: 2 if 'name="ms level" value="2"' in sp else 1
    ms1, ms2 = [sp for sp in spectra if lvl(sp) == 1], [sp for sp in spectra if lvl(sp) == 2]
    out = []
    for name, keep in (("HR_full-t5", ms1), ("HR_ms2-t10", ms2), ("HR_inf-t15", ms1[:1])):
        k = [re.sub(r'index="\d+"', f'index="{i}"', sp, count=1) for i, sp in enumerate(keep)]
        p = Path(d) / f"{name}.mzML"
        p.write_text(head + f'<spectrumList count="{len(k)}">' + "".join(k) + "</spectrumList>" + tail, encoding="utf-8")
        out.append(p)
    return out
