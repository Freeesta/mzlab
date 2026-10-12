"""Reference answers of `tpmine.hr.features.detect` for the parity test of the Rust crate (crates/mzlab-feat/tests/trace_parity.rs).

    python3 tools/feat_riferimento.py OUT [--golden]

Writes in OUT/ the synthetic mzML files of `tools/dati_sintetici.py` (Orbitrap-like MS1 series with invented ions: weak, intense, wobbling in
m/z, side lobes, continuous background), their `.mzidx` built by the Python builder and `detect_<name>.json`: the features of
`features.detect` with the first and last scan of every trace. With --golden the JSON files go to tests/golden/feat/ as well.
No compound names: every ion is a number.

    python3 tools/feat_riferimento.py OUT --solo allinea [--golden]

writes `allinea_sintetico.json` (alignment and RT map of a synthetic series, see `tools/feat_allinea.py`).

    python3 tools/feat_riferimento.py OUT --solo riempimento

writes `riempimento.json` and the `.mzidx` of a small series (alignment, RT map and `fill_gaps` with and without the map, see `tools/feat_riempimento.py`).
    python3 tools/feat_riferimento.py OUT --solo ipotesi [--golden]

writes `ipotesi_regole.json`: the hypotheses (parent times the rules of mzFinder, 1-2 steps) of invented parent formulas, see `tools/feat_ipotesi.py`.

    python3 tools/feat_riferimento.py OUT --solo cinetica [--golden]

writes `cinetica_forme.json`, `cinetica_generazione.json` (shapes and generation of synthetic profiles) and `cinetica_v1.json` (areas of the identity series with known kinetics, noise and random-kinetics decoys), see `tools/feat_cinetica.py`.

    python3 tools/feat_riferimento.py OUT --solo identita [--golden]

writes only `iso_ms2_identita.json`: on one file of the synthetic identity series (isotopologues, co-isolated MS2) the features of `features.detect` with
the in-file isotope groups (`isotopi_riferimento`, the rules of crates/mzlab-feat/src/isotopes.rs) and the MS2 scans linked to every peak with the purity
of the precursor (`ms2_riferimento`, crates/mzlab-feat/src/ms2.rs), written in plain loops: the Rust crate must answer with the same indices.
"""
import json
import sys
from pathlib import Path

import numpy as np

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI))
sys.path.insert(0, str(AQUI.parent))
sys.path.insert(0, str(AQUI.parent / "TP_Mine" / "py"))
import dati_sintetici as ds  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402
from tpmine.hr import features  # noqa: E402
from mzlab.chem import isoscore  # noqa: E402

D13C, DPIU2 = 1.003355, (2.00671, 1.995796, 1.99705, 1.997954)
PROTONE, AVERAGINE, BIN_MASSA = 1.007276, 111.1254, 25.0


def _attesi(massa: float, cache: dict) -> tuple[float, float]:
    """(M+1/M, M+2/M) of an averagine of the neutral mass, one value per bin of 25 Da."""
    b = int(max(massa, 0.0) / BIN_MASSA)
    if b not in cache:
        u = (b + 0.5) * BIN_MASSA / AVERAGINE
        n = lambda x: max(int(np.floor(x * u + 0.5)), 0)      # noqa: E731  (round half up, like f64::round on positives)
        pat = isoscore.fine_pattern({"C": max(n(4.9384), 1), "H": n(7.7583), "N": n(1.3577), "O": n(1.4773), "S": n(0.0417)}, 1, 0.001)
        nom = [0.0, 0.0, 0.0]
        for q in pat:
            k = int(np.floor(q["mz"] - pat[0]["mz"] + 0.5))
            if k < 3:
                nom[k] += q["rel"]
        cache[b] = (nom[1] / nom[0], nom[2] / nom[0]) if nom[0] > 0 else (0.0, 0.0)
    return cache[b]


def isotopi_riferimento(mz, rt, area, rt_tol: float = 0.05, tol_ppm: float = 5.0, zmax: int = 3) -> tuple[list[int], list[int], list[int]]:
    """(iso_group, charge, isotopologue) per feature, features sorted by m/z; plain loops over the same rules as crates/mzlab-feat/src/isotopes.rs."""
    n, cache = len(mz), {}
    rt32 = np.asarray(rt, dtype=np.float32)
    gid, ch, iso = [0] * n, [0] * n, [0] * n
    passi = [(1, D13C)] + [(2, d) for d in DPIU2]
    for j in range(n):
        best = None
        for z in range(1, zmax + 1):
            for k, d in passi:
                want = mz[j] - d / z
                tol = mz[j] * tol_ppm * 1e-6 + 3e-4
                a, b = np.searchsorted(mz, want - tol, "left"), np.searchsorted(mz, want + tol, "right")
                for i in range(a, min(b, j)):
                    if abs(float(rt32[i] - rt32[j])) > rt_tol or (ch[i] != 0 and ch[i] != z):
                        continue
                    r = area[j] / max(area[i], 1e-300)
                    e1, e2 = _attesi((mz[i] - PROTONE) * z, cache)
                    if not ((r >= 0.25 * e1 - 0.005 and r <= 2.5 * e1 + 0.05) if k == 1 else (r >= 0.2 * e2 and r <= 1.2)):
                        continue
                    err = abs((mz[i] + d / z - mz[j]) / mz[j]) * 1e6
                    if best is None or err < best[0] or (err == best[0] and z < best[1]):
                        best = (err, z, i)
        if best is None:
            continue
        _, z, i = best
        g = gid[i] if gid[i] else i + 1
        gid[i] = gid[j] = g
        ch[i] = ch[j] = z
        iso[j] = 1
    return gid, ch, iso


def area_finestra(t, f, mezza: float = 0.05, tol_ppm: float = 6.0) -> np.ndarray:
    """`area_win`: raw trace (peaks within 6 ppm, no floor) within 0.05 min of the apex scan, counts x seconds (the isotope ratios use it)."""
    rt = np.asarray(t.rt)
    out = np.zeros(len(f))
    for i in range(len(f)):
        y = t.xic(float(f.mz[i]), float(f.mz[i]) * tol_ppm * 1e-6)
        out[i] = float(y[np.abs(rt - rt[f.apex[i]]) <= mezza].sum()) * f.dt
    return out


def ms2_riferimento(run, t, f, mezza: float = 0.6, tol_ppm: float = 6.0) -> tuple[list[list[int]], list[list[float]]]:
    """For every feature the MS2 scans (file indices) whose MS1 parent lies between the edges and whose isolation window holds its m/z, with the purity."""
    pos_of = {int(g): k for k, g in enumerate(t.scan_ids)}
    cand = []
    for g, sc in enumerate(run.scans):
        pp = pos_of.get(sc.parent) if sc.parent is not None else None
        if sc.level != 2 or sc.polarity < 0 or pp is None:
            continue
        if sc.iso is not None:
            cand.append((g, pp, sc.iso[0], sc.iso[1]))
        elif sc.precursor:
            cand.append((g, pp, sc.precursor - mezza, sc.precursor + mezza))
    mz32, it, pos = np.asarray(t.mz), np.asarray(t.inten, dtype=float), np.asarray(t.pos)
    janela = {}
    for g, pp, lo, hi in cand:
        a, b = np.searchsorted(mz32, np.float32(lo), "left"), np.searchsorted(mz32, np.float32(hi), "right")
        sel = pos[a:b] == pp
        janela[g] = (mz32[a:b][sel].astype(float), it[a:b][sel])
    scans, pur = [], []
    for i in range(len(f)):
        s_i, p_i = [], []
        for g, pp, lo, hi in sorted(cand):
            if f.scan_lo[i] <= pp <= f.scan_hi[i] and lo <= f.mz[i] <= hi:
                wm, wi = janela[g]
                own = float(wi[np.abs(wm - f.mz[i]) <= f.mz[i] * tol_ppm * 1e-6].sum())
                if own > 0 and wi.sum() > 0:
                    s_i.append(g)
                    p_i.append(own / float(wi.sum()))
        scans.append(s_i)
        pur.append(p_i)
    return scans, pur


def un_file_identita(out: Path, golden: Path | None) -> int:
    """`iso_ms2_identita.json`: SIN_t10min of the identity series (seed 7, small profile) with the references of isotopes and MS2."""
    from valida import sintetici_identita as si
    d = out / "identita"
    si.genera(7, "piccolo", d)
    mz = d / "SIN_t10min.mzML"
    ix = out / "identita.mzidx"
    run = Run(mz, index_path=ix)
    mzidx.build(run, ix)
    t = run.table(1, 1)
    f = features.detect(t)
    srt = np.asarray(f.scan_rt)
    lo, hi = np.searchsorted(srt, f.left), np.searchsorted(srt, f.right)
    f.scan_lo, f.scan_hi = lo, hi
    area_win = area_finestra(t, f)
    gid, ch, iso = isotopi_riferimento(f.mz, f.rt, area_win)
    scans, pur = ms2_riferimento(run, t, f)
    doc = {"name": "identita", "n": len(f), "mz": f.mz.tolist(), "apex": f.apex.tolist(), "scan_lo": lo.tolist(), "scan_hi": hi.tolist(), "area_win": area_win.tolist(), "iso_group": gid,
           "charge": ch, "isotope": iso, "ms2_scans": scans, "ms2_purity": pur}
    txt = json.dumps(doc, separators=(",", ":"))
    (out / "iso_ms2_identita.json").write_text(txt, encoding="utf-8")
    if golden is not None:
        golden.mkdir(parents=True, exist_ok=True)
        (golden / "iso_ms2_identita.json").write_text(txt, encoding="utf-8")
    run.close()
    return len(f)


def un_file_famiglie(out: Path, golden: Path | None) -> int:
    """`famiglie_coppie.json` and `famiglie_identita.json` (two windows of retention time of SIN_t10min, see tools/feat_famiglie.py)."""
    import feat_famiglie as ff
    from valida import sintetici_identita as si
    d = out / "identita"
    si.genera(7, "piccolo", d)
    ix = out / "identita.mzidx"
    run = Run(d / "SIN_t10min.mzML", index_path=ix)
    mzidx.build(run, ix)
    t = run.table(1, 1)
    f = features.detect(t)
    srt = np.asarray(f.scan_rt)
    f.scan_lo, f.scan_hi = np.searchsorted(srt, f.left), np.searchsorted(srt, f.right)
    area_win = area_finestra(t, f)
    gid, ch, iso = isotopi_riferimento(f.mz, f.rt, area_win)
    scans, pur = ms2_riferimento(run, t, f)
    keep = [i for i in range(len(f)) if 0.9 <= f.rt[i] <= 1.15 or 3.1 <= f.rt[i] <= 3.7]
    new = {g: k for k, g in enumerate(keep)}
    rows = {"mz": [float(f.mz[i]) for i in keep], "rt": [float(np.float32(f.rt[i])) for i in keep], "apex": [int(f.apex[i]) for i in keep],
            "fwhm": [(int(f.scan_hi[i]) - int(f.scan_lo[i]) + 1) / 2 for i in keep], "height": [float(np.float32(f.height[i])) for i in keep],
            "isotope": [bool(iso[i]) for i in keep], "iso_group": [new.get(gid[i] - 1, -1) + 1 if gid[i] else 0 for i in keep],
            "excluded": [bool(f.background[i] or f.artefact[i]) for i in keep], "dt": float(f.dt), "ms2_n": [len(scans[i]) for i in keep]}
    spectra = {}
    for k, i in enumerate(keep):
        if scans[i]:
            g = scans[i][int(np.argmax(pur[i]))]
            m, y = run.read(g)
            order = np.argsort(-y, kind="stable")[:40] if len(y) > 40 else np.arange(len(y))
            order = np.sort(order)
            spectra[k] = {"mz": m[order].tolist(), "it": y[order].tolist()}
    traces = trazas(t, rows, ff, len(srt))
    want = ff.riferimento_famiglie(rows, {k: v for k, v in traces.items()}, spectra)
    doc = {"name": "famiglie_identita", "params": ff.PARAMS, "rows": rows, "traces": {str(k): v for k, v in traces.items()},
           "spectra": {str(k): v for k, v in spectra.items()}, "want": want}
    txt = json.dumps(doc, separators=(",", ":"))
    pair = json.dumps(ff.coppie_json(), separators=(",", ":"))
    (out / "famiglie_identita.json").write_text(txt, encoding="utf-8")
    (out / "famiglie_coppie.json").write_text(pair, encoding="utf-8")
    if golden is not None:
        golden.mkdir(parents=True, exist_ok=True)
        (golden / "famiglie_identita.json").write_text(txt, encoding="utf-8")
        (golden / "famiglie_coppie.json").write_text(pair, encoding="utf-8")
    run.close()
    return len(keep)


def trazas(t, rows: dict, ff, nscan: int) -> dict:
    """Trace (first scan, values) of every row in a candidate pair, from the peak table (summed within 6 ppm), the window of `family/traces.rs`."""
    n = len(rows["mz"])
    ids = set()
    for j in range(n):
        for i in range(j):
            if ff_candidate(rows, ff, i, j):
                ids.update((i, j))
    out = {}
    for k in sorted(ids):
        half = min(max(int(np.ceil(rows["fwhm"][k])) + 3, 5), 40)
        w0, w1 = max(rows["apex"][k] - half, 0), min(rows["apex"][k] + half, nscan - 1)
        y = t.xic(rows["mz"][k], rows["mz"][k] * ff.PARAMS["trace_ppm"] * 1e-6)
        out[k] = [w0, [float(v) for v in y[w0:w1 + 1]]]
    return out


def ff_candidate(rows: dict, ff, i: int, j: int) -> bool:
    """Is (i, j) worth a trace: not an isotopologue or lobe, co-eluting by the loose gate (the reference applies the exact one)."""
    if rows["excluded"][i] or rows["excluded"][j] or rows["isotope"][i] or rows["isotope"][j]:
        return False
    return abs(rows["rt"][i] - rows["rt"][j]) <= 0.3


def piani() -> dict[str, tuple[int, list[tuple], float]]:
    """name -> (seed, ions (m/z, RT, sigma, amplitude), RT end)."""
    rng = np.random.default_rng(11)
    deboli = [(float(m), float(c), 0.05, float(a)) for m, c, a in zip(rng.uniform(150, 800, 40), rng.uniform(1, 9, 40), 10 ** rng.uniform(5.3, 7, 40))]
    intensi = [(300.1234, 4.0, 0.06, 4e8), (300.1234 * (1 + 60e-6), 4.0, 0.06, 5e6),       # a side lobe 60 ppm from an intense ion
               (455.2, 6.0, 0.05, 9e7), (455.2 * (1 + 3e-6), 6.02, 0.05, 2e7),           # two ions 3 ppm apart
               (610.31, 2.5, 0.2, 3e7)]
    fondo = [(250.05, 0.5 + 0.001, 40.0, 3e6), (420.4, 1.0, 40.0, 2.5e5)]                 # continuous background (sigma 40 min = whole run)
    vicini = [(500.0 + 0.0012 * k, 5.0 + 0.3 * (k % 3), 0.04, 5e5 * (1 + k)) for k in range(6)]
    return {"deboli": (21, deboli, 12.0), "intensi": (22, intensi + deboli[:10], 12.0), "fondo": (23, fondo + deboli[:15], 6.0),
            "vicini": (24, vicini, 10.0), "vuoto": (25, [], 3.0)}


def un_file(out: Path, nome: str, seed: int, ioni: list, fine: float, golden: Path | None) -> int:
    mz = out / f"{nome}.mzML"
    ds.hr_series(mz, np.random.default_rng(seed), ioni, rt_end=fine, cyc=0.008 if nome == "fondo" else 0.04)   # "fondo": > 400 scans
    ix = out / f"{nome}.mzidx"
    run = Run(mz, index_path=ix)
    mzidx.build(run, ix)
    t = run.table(1, 1)
    kw = {"long_trace": 60} if nome == "fondo" else {}          # the synthetic traces are shorter than 400 scans: a lower limit marks the background
    f = features.detect(t, **kw)
    srt = np.asarray(f.scan_rt)
    lo = np.searchsorted(srt, f.left)
    hi = np.searchsorted(srt, f.right)
    doc = {"name": nome, "n": len(f), "dt": f.dt, "params": kw,
           "mz": f.mz.tolist(), "rt": f.rt.tolist(), "apex": f.apex.tolist(), "area": f.area.tolist(), "height": f.height.tolist(),
           "npts": f.npts.tolist(), "left": f.left.tolist(), "right": f.right.tolist(), "scan_lo": lo.tolist(), "scan_hi": hi.tolist(),
           "background": f.background.astype(int).tolist(), "artefact": f.artefact.astype(int).tolist()}
    txt = json.dumps(doc, separators=(",", ":"))
    (out / f"detect_{nome}.json").write_text(txt, encoding="utf-8")
    if golden is not None:
        golden.mkdir(parents=True, exist_ok=True)
        (golden / f"detect_{nome}.json").write_text(txt, encoding="utf-8")
    run.close()
    return len(f)


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    golden = AQUI.parent / "tests" / "golden" / "feat" if "--golden" in argv else None
    if "--solo" in argv and argv[argv.index("--solo") + 1] == "famiglie":
        print("famiglie", un_file_famiglie(out, golden))
        return 0
    if "--solo" in argv and argv[argv.index("--solo") + 1] == "riempimento":
        import feat_riempimento  # noqa: E402
        print("riempimento", feat_riempimento.un_file_riempimento(out, golden))
        return 0
    if "--solo" in argv and argv[argv.index("--solo") + 1] == "ipotesi":
        import feat_ipotesi  # noqa: E402
        print("ipotesi", feat_ipotesi.un_file_ipotesi(out, golden))
        return 0
    if "--solo" in argv and argv[argv.index("--solo") + 1] == "allinea":
        import feat_allinea  # noqa: E402
        print("allinea", feat_allinea.un_file_allinea(out, golden))
        return 0
    if "--solo" in argv and argv[argv.index("--solo") + 1] == "cinetica":
        import feat_cinetica  # noqa: E402
        print("cinetica", feat_cinetica.scrivi(out, golden), feat_cinetica.scrivi_v1(out, golden))
        return 0
    if "--solo" in argv and argv[argv.index("--solo") + 1] == "identita":
        print("identita", un_file_identita(out, golden))
        return 0
    for nome, (seed, ioni, fine) in piani().items():
        print(nome, un_file(out, nome, seed, ioni, fine, golden))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
