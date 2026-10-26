"""mzML writer of the synthetic identity series (tools/valida/sintetici_identita.py): Orbitrap-like DDA files, all numbers invented.

One file = survey scans every CICLO min (centroids, noise, continuous background with FT lobes, the ions of the model) and, at the apex of every
selectable ion, an MS2 in which all the ions inside the isolation window are fragmented together (this is what makes an MS2 chimeric).

`genera_larghi` writes the bench lot of smeared peaks (blank + samples, `verita.json`): broad Gaussian/EMG bands (FWHM 60-240 s), double plateaus and
bands with narrow peaks on top, over a common gradient background; used by tools/bench (`python3 tools/valida/sintetici_mzml.py OUT --seme N`)."""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import dati_sintetici as ds  # noqa: E402

CICLO = 0.01                  # min between two survey scans
RT0 = 0.5
ISO_SEMI = 0.6                # half of the isolation window (1.2 m/z)
SOGLIA_MS2 = 5e4              # minimum height of the apex to trigger a fragmentation
TOP_N = 3
SD_PPM = 0.8e-6
RUMORE = 100                  # random noise peaks per survey scan
FIN_RT = 0.5                  # components farther than this (min) from the scan are not evaluated (5 sigma of the widest peak is 0.5)
FILTRO1, FILTRO2 = "FTMS + p ESI Full ms [100.0000-900.0000]", "FTMS + p ESI d Full ms2 {p:.4f}@hcd30.00 [50.0000-{q:.4f}]"


def _arr(acc, name, data, f4=False) -> str:
    cv = ('<cvParam cvRef="MS" accession="MS:1000521" name="32-bit float" value=""/>' if f4 else '<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>')
    return (f'<binaryDataArray encodedLength="0">{cv}<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value=""/><binary>{ds._b64(data, f4) if len(data) else ""}</binary></binaryDataArray>')


def _scan(idx, rt, level, mz, it, prec="", filt="", res=0) -> str:
    n = len(mz)
    b = int(np.argmax(it)) if n else 0
    cv = ds._cvp
    return (f'<spectrum index="{idx}" id="controllerType=0 controllerNumber=1 scan={idx + 1}" defaultArrayLength="{n}">'
            + cv("MS:1000579" if level == 1 else "MS:1000580", "MS1 spectrum" if level == 1 else "MSn spectrum") + cv("MS:1000511", "ms level", level)
            + cv("MS:1000130", "positive scan") + cv("MS:1000127", "centroid spectrum")
            + cv("MS:1000504", "base peak m/z", f"{mz[b]:.6f}" if n else 0, ds._U_MZ) + cv("MS:1000505", "base peak intensity", f"{it[b]:.6g}" if n else 0)
            + cv("MS:1000285", "total ion current", f"{float(np.sum(it)):.6g}")
            + f'<scanList count="1">{cv("MS:1000795", "no combination")}<scan>'
            + cv("MS:1000016", "scan start time", f"{rt:.6f}", ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
            + cv("MS:1000800", "mass resolving power", res) + cv("MS:1000512", "filter string", filt)
            + '<scanWindowList count="1"><scanWindow>' + cv("MS:1000501", "scan window lower limit", 50.0, ds._U_MZ) + cv("MS:1000500", "scan window upper limit", 900.0, ds._U_MZ)
            + '</scanWindow></scanWindowList></scan></scanList>' + prec + '<binaryDataArrayList count="2">'
            + _arr("MS:1000514", "m/z array", mz) + _arr("MS:1000515", "intensity array", it, f4=True) + "</binaryDataArrayList></spectrum>")


def _precursore(idx_ms1, m, e, vint) -> str:
    cv = ds._cvp
    return ('<precursorList count="1">' + f'<precursor spectrumRef="controllerType=0 controllerNumber=1 scan={idx_ms1 + 1}"><isolationWindow>'
            + cv("MS:1000827", "isolation window target m/z", f"{m:.6f}", ds._U_MZ) + cv("MS:1000828", "isolation window lower offset", ISO_SEMI, ds._U_MZ)
            + cv("MS:1000829", "isolation window upper offset", ISO_SEMI, ds._U_MZ) + '</isolationWindow><selectedIonList count="1"><selectedIon>'
            + cv("MS:1000744", "selected ion m/z", f"{e:.6f}", ds._U_MZ) + cv("MS:1000041", "charge state", 1) + cv("MS:1000042", "peak intensity", f"{vint:.6g}")
            + '</selectedIon></selectedIonList><activation>' + cv("MS:1000422", "beam-type collision-induced dissociation")
            + cv("MS:1000045", "collision energy", 30.0, ' unitCvRef="UO" unitAccession="UO:0000266" unitName="electronvolt"') + '</activation></precursor></precursorList>')


def componenti_file(c, k: int):
    """Arrays of the components present in file k (sorted by RT): m/z, RT, sigma, height, cap of the saturated ones (inf = none), the ions and the index of the
    most abundant component of each ion. The matrix noise features (c.rumore) are components of no ion."""
    ioni = [o for o in c.ioni if o.amp is not None and o.amp[k] >= 2e3 and o.picchi]
    cm = [m for o in ioni for m, _ in o.picchi]
    cr = [o.rt for o in ioni for _ in o.picchi]
    cs = [o.sg for o in ioni for _ in o.picchi]
    ch = [o.amp[k] * r / 100.0 for o in ioni for _, r in o.picchi]
    cap = [(o.clip[k] if o.clip is not None and r >= 99.9 else np.inf) for o in ioni for _, r in o.picchi]
    mono, pos = [], 0
    for o in ioni:
        mono.append(pos)
        pos += len(o.picchi)
    rm, rr, rs, rh = c.rumore
    cm, cr, cs, ch, cap = (np.concatenate([np.array(a, float), b]) for a, b in ((cm, rm), (cr, rr), (cs, rs), (ch, rh), (cap, np.full(len(rm), np.inf))))
    perm = np.argsort(cr, kind="stable")
    inv = np.empty(len(perm), int)
    inv[perm] = np.arange(len(perm))
    return ioni, cm[perm], cr[perm], cs[perm], ch[perm], cap[perm], inv[np.array(mono, int)] if mono else np.zeros(0, int)


def piano_ms2(ioni, k: int) -> dict:
    """{cycle: [index of ion]}: the apex cycle of every selectable ion, at most TOP_N per cycle (the rest slips to the next cycles)."""
    carico, pian = {}, {}
    for j, o in enumerate(ioni):
        if o.selezionabile and o.amp[k] >= SOGLIA_MS2:
            n = int(round((o.rt - RT0) / CICLO))
            while carico.get(n, 0) >= TOP_N:
                n += 1
            carico[n] = carico.get(n, 0) + 1
            pian.setdefault(n, []).append(j)
    return pian


def spettro_ms2(rng, j, ioni, imz, v, mono, i1, idx, rt, s, e):
    """MS2 of ion o: fragments of every ion in the isolation window, in proportion to its current in the survey scan."""
    o = ioni[j]
    win = np.flatnonzero((np.abs(imz - o.mz) <= ISO_SEMI) & (v[mono] >= 2e3)).tolist()
    fm, fi = [], []
    for i in win:
        fr = ioni[i].frammenti
        if fr is None:
            continue
        fm.append(fr[0] * (1 + rng.normal(0, SD_PPM, len(fr[0]))))
        fi.append(fr[1] * v[mono[i]] * 0.2 * rng.normal(1, 0.05, len(fr[0])))
    fm.append(rng.uniform(60, o.mz, 8)); fi.append(rng.lognormal(np.log(1.5e3), 0.5, 8))
    fm, fi = np.concatenate(fm), np.concatenate(fi)
    q = np.argsort(fm, kind="stable")
    return _scan(idx, rt + 0.0015 * (s + 1), 2, fm[q], fi[q], prec=_precursore(i1, o.mz, e, float(v[mono[j]])), filt=FILTRO2.format(p=o.mz, q=o.mz + 10), res=15000)


def scrivi_file(path: Path, k: int, c, seme: int) -> dict:
    """One file of the series (k = column of the plan)."""
    rng = np.random.default_rng([seme, 1000 + k])
    ioni, cm, cr, cs, ch, cap, mono = componenti_file(c, k)
    imz = np.array([o.mz for o in ioni])
    bg = np.array([[m, a, ph] for _, m, a, ph in c.fondo]) if c.fondo else np.zeros((0, 3))
    lob_mz = np.array([m * (1 + d) for m in bg[:, 0] for d in (-2.5e-5, 2.5e-5)])
    rts = np.arange(RT0, c.rt_fine, getattr(c, "ciclo", CICLO))
    fin, minimo = getattr(c, "fin", FIN_RT), getattr(c, "minimo", 2e3)                  # hooks of the bench lot (`Lotto`); the defaults are the series above
    prof, fondo_t = getattr(c, "profilo", None), getattr(c, "fondo_t", None)
    pian = piano_ms2(ioni, k)
    specs, tic, idx, n_ms2 = [], [], 0, 0
    for n, rt in enumerate(rts):
        lo, hi = np.searchsorted(cr, [rt - fin, rt + fin])
        v = np.zeros(len(cm))
        forma_ = prof(rt, lo, hi) if prof else np.exp(-0.5 * ((rt - cr[lo:hi]) / cs[lo:hi]) ** 2)
        v[lo:hi] = np.minimum(ch[lo:hi] * forma_ * rng.normal(1, 0.03, hi - lo), cap[lo:hi])
        keep = v >= minimo
        mz = [rng.uniform(100, 900, RUMORE), cm[keep] * (1 + rng.normal(0, SD_PPM, int(keep.sum())))]
        it = [rng.lognormal(np.log(8e3), 0.7, RUMORE), v[keep]]
        if len(bg):
            hb = (fondo_t(rt, bg[:, 1], bg[:, 2]) if fondo_t else bg[:, 1] * (1 + 0.1 * np.sin(rt / 2.0 + bg[:, 2]))) * rng.normal(1, 0.03, len(bg))
            mz += [bg[:, 0] * (1 + rng.normal(0, SD_PPM, len(bg))), lob_mz * (1 + rng.normal(0, SD_PPM, len(lob_mz)))]
            it += [hb, np.repeat(hb, 2) * 0.02 * rng.normal(1, 0.05, len(lob_mz))]
        mz, it = np.concatenate(mz), np.concatenate(it)
        o_ = np.argsort(mz, kind="stable")
        i1 = idx
        specs.append(_scan(idx, rt, 1, mz[o_], it[o_], filt=FILTRO1, res=60000))
        tic.append(float(it.sum())); idx += 1
        for s, j in enumerate(pian.get(n, [])):
            e = float(cm[mono[j]] * (1 + rng.normal(0, SD_PPM)))
            specs.append(spettro_ms2(rng, j, ioni, imz, v, mono, i1, idx, rt, s, e))
            idx += 1; n_ms2 += 1
    chroms = [ds._chrom(0, "TIC", rts, np.array(tic))]
    path.write_text(ds._hr_head("exploris", False, path.stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                    + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")
    return {"file": path.name, "n_ms1": len(rts), "n_ms2": n_ms2}


# ------------------------------------------------------------------------------------------------------------ bench lot of smeared peaks
FWHM_K = 2 * math.sqrt(2 * math.log(2))      # FWHM = FWHM_K * sigma of a Gaussian
N0 = 8e3                                     # median height of the noise peaks: S/N of a compound = its apex height / N0
CICLO_LARGHI = 0.02                          # min between two survey scans of the lot (1.2 s: 6 points on a 8 s FWHM peak)
RT_FINE_LARGHI = 30.0
MORFOLOGIE = ("a_gauss", "a_emg", "b_doppio", "c_banda")
_erfc = np.frompyfunc(math.erfc, 1, 1)
_FMAX: dict = {}


def _emg(t, mu, sg, tau):
    """Unit-area exponentially modified Gaussian (tau > 0)."""
    z = (sg / tau - (t - mu) / sg) / math.sqrt(2.0)
    return ds._exp(sg ** 2 / (2 * tau ** 2) - (t - mu) / tau) * _erfc(z).astype(float) / (2 * tau)


def _emg_max(sg: float, tau: float) -> float:
    """Height of the apex of the unit-area EMG (numeric, the apex is smooth so the grid error is far below 1e-6)."""
    k = (round(sg, 9), round(tau, 9))
    if k not in _FMAX:
        _FMAX[k] = float(_emg(np.linspace(-4 * sg, 4 * sg + 8 * tau, 4001), 0.0, sg, tau).max())
    return _FMAX[k]


def forma(t, mu, sg, tau):
    """Unit-apex profile at times t (min) of components (mu, sg, tau): tau = 0 a Gaussian, tau > 0 an EMG (same sigma, exponential tail of time constant tau)."""
    t, mu, sg, tau = np.broadcast_arrays(*(np.asarray(x, float) for x in (t, mu, sg, tau)))
    y = ds._exp(-0.5 * ((t - mu) / sg) ** 2)
    m = tau > 0
    if m.any():
        y = np.where(m, 0.0, y)
        y[m] = _emg(t[m], mu[m], sg[m], tau[m]) / np.array([_emg_max(s, u) for s, u in zip(sg[m], tau[m])])
    return y


def area_unitaria(sg: float, tau: float) -> float:
    """Exact integral (min) of the unit-apex profile."""
    return sg * math.sqrt(2 * math.pi) if tau <= 0 else 1.0 / _emg_max(sg, tau)


def _curva(comps: list, t):
    return sum(c["h"] * forma(t, c["mu"], c["sigma"], c["tau"]) for c in comps)


def _griglia(comps: list):
    return np.arange(min(c["mu"] - 6 * c["sigma"] for c in comps), max(c["mu"] + 6 * c["sigma"] + 10 * c["tau"] for c in comps), 0.002)


def _fwhm_s(comps: list) -> float:
    t = _griglia(comps)
    y = _curva(comps, t)
    i = np.flatnonzero(y >= y.max() / 2)
    return float((t[i[-1]] - t[i[0]]) * 60.0)


def _sigma_emg(fwhm_s: float, rapporto: float = 0.4) -> float:
    """Sigma (min) of an EMG with tau = rapporto * sigma whose FWHM is fwhm_s (the shape depends on tau / sigma only)."""
    return fwhm_s / _fwhm_s([{"mu": 0.0, "sigma": 1.0, "tau": rapporto, "h": 1.0}])


def _sigma_sella(d: float, h2: float, frac: float) -> float:
    """Sigma of two equal Gaussians (heights 1 and h2, centres d apart) whose saddle is frac of the lower apex of the sum (bisection)."""
    def rapporto(s):
        x = np.linspace(-4 * s, d + 4 * s, 4001)
        y = ds._exp(-0.5 * (x / s) ** 2) + h2 * ds._exp(-0.5 * ((x - d) / s) ** 2)
        m = np.flatnonzero((y[1:-1] > y[:-2]) & (y[1:-1] >= y[2:])) + 1
        return 1.0 if len(m) < 2 else float(y[m[0]:m[-1] + 1].min() / min(y[m[0]], y[m[-1]]))
    lo, hi = d / 6, d / 1.5
    for _ in range(40):
        mid = (lo + hi) / 2
        lo, hi = (mid, hi) if rapporto(mid) < frac else (lo, mid)
    return (lo + hi) / 2


def _riga(id_: str, ruolo: str, morf: str, mz: float, comps: list, nfile: int, **extra) -> dict:
    """Truth row of one compound: apex RT (min; of a double plateau the midpoint of its two apexes), FWHM (s) and central 99 % interval of the area of the sum of
    its components, exact area (intensity * min, one value per file, 0 in the blank)."""
    t = _griglia(comps)
    y = _curva(comps, t)
    ap = np.flatnonzero((y[1:-1] > y[:-2]) & (y[1:-1] >= y[2:])) + 1
    rt = float((t[ap[0]] + t[ap[-1]]) / 2) if morf == "b_doppio" and len(ap) > 1 else float(t[int(np.argmax(y))])
    cum = np.cumsum(y)
    cum /= cum[-1]
    area = sum(c["h"] * area_unitaria(c["sigma"], c["tau"]) for c in comps)
    return dict({"id": id_, "ruolo": ruolo, "morfologia": morf, "mz": round(mz, 6), "rt": round(rt, 4), "fwhm_s": round(_fwhm_s(comps), 2),
                 "rt_min": round(float(t[np.searchsorted(cum, 0.005)]), 4), "rt_max": round(float(t[np.searchsorted(cum, 0.995)]), 4),
                 "area_attesa": [0.0] + [round(float(area), 3)] * nfile, "presente": 1,
                 "componenti": [{k: round(float(c[k]), 6) for k in ("mu", "sigma", "tau", "h")} for c in comps]}, **extra)


def costruisci_larghi(seme: int, n_larghi: int = 18, n_stretti: int = 12, n_fondo: int = 200, n_campioni: int = 3, rt_fine: float = RT_FINE_LARGHI):
    """(truth rows, groups, background): the groups are the compounds as the instrument sees them (one m/z, the sum of the components of the row(s) on it)."""
    rng = np.random.default_rng([seme, 77])
    bg_mz = np.sort(rng.uniform(150, 800, n_fondo))
    usati: list = []

    def mz_libero() -> float:
        while True:
            m = float(rng.uniform(150, 800))
            if all(abs(m - x) >= 0.5 for x in usati) and np.all(np.abs(bg_mz - m) >= 0.02):
                usati.append(m)
                return m

    def posto(ext: float) -> float:
        return float(rng.uniform(RT0 + 1.0 + ext, rt_fine - 1.0 - ext))

    def comp(mu, fwhm_s, snr, emg=False):
        sg = _sigma_emg(fwhm_s) if emg else fwhm_s / 60.0 / FWHM_K
        return {"mu": mu, "sigma": sg, "tau": 0.4 * sg if emg else 0.0, "h": snr * N0}

    n_a, n_b = round(n_larghi * 0.45), round(n_larghi * 0.275)
    morfs = ["a_gauss", "a_emg"] * (n_a // 2) + ["a_gauss"] * (n_a % 2) + ["b_doppio"] * n_b + ["c_banda"] * (n_larghi - n_a - n_b)
    righe, gruppi, n_nsb = [], [], 0
    for i, morf in enumerate(morfs):
        mz = mz_libero()
        if morf.startswith("a_"):
            f = float(rng.uniform(60, 240))
            c = comp(0.0, f, float(rng.uniform(4, 12)), emg=morf == "a_emg")
            c["mu"] = posto(3.5 * c["sigma"] + 5 * c["tau"])
            cs = [c]
        elif morf == "b_doppio":
            d, h2 = 1.5, 0.85
            sg = _sigma_sella(d, h2, 0.60)
            mu = posto(d / 2 + 3.5 * sg)
            snr = float(rng.uniform(4, 12))
            cs = [{"mu": mu - d / 2, "sigma": sg, "tau": 0.0, "h": snr * N0}, {"mu": mu + d / 2, "sigma": sg, "tau": 0.0, "h": snr * N0 * h2}]
        else:
            band = comp(posto(3.5 * 150 / 60 / FWHM_K), 150.0, float(rng.uniform(6, 12)))
            cs = [band]
        righe.append(_riga(f"BRD-{i + 1:03d}", "broad", morf, mz, cs, n_campioni))
        todos = list(cs)
        if morf == "c_banda":
            for _ in range(int(rng.integers(1, 3))):
                st = comp(float(cs[0]["mu"] + rng.uniform(-0.35, 0.35) * 150 / 60), 8.0, float(rng.uniform(8, 25)))
                n_nsb += 1
                righe.append(_riga(f"NSB-{n_nsb:03d}", "narrow", "stretto_su_banda", mz * (1 + float(rng.uniform(-3, 3)) * 1e-6), [st], n_campioni, su=f"BRD-{i + 1:03d}"))
                todos.append(st)
        gruppi.append({"mz": mz, "comps": todos})
    for i in range(n_stretti):
        mz = mz_libero()
        st = comp(posto(1.0), float(rng.uniform(6, 12)), float(rng.uniform(15, 60)))
        righe.append(_riga(f"NAR-{i + 1:03d}", "narrow", "stretto", mz, [st], n_campioni))
        gruppi.append({"mz": mz, "comps": [st]})
    fondo = [(f"LAR-BG-{i + 1:03d}", float(m), float(rng.lognormal(np.log(6e4), 0.8)), float(rng.uniform(0, 6.28))) for i, m in enumerate(bg_mz)]
    return righe, gruppi, fondo


def gradiente(rt: float) -> float:
    """Common gradient background G(t): the organic share rises along the run (0.15 -> 1), every background channel follows it."""
    return 0.15 + 0.85 / (1 + ds._exp(-(rt - 16.0) / 5.0))


class Lotto:
    """What `scrivi_file` reads of a series, for the bench lot: no ions, one 'noise' component per compound (a single centroid whatever its sub-peaks) and the
    hooks `profilo` (absolute intensity of the compounds at a time), `fondo_t` (gradient on the background channels), `ciclo`, `fin`, `minimo`."""
    ioni: list = []

    def __init__(self, gruppi: list, fondo: list, rt_fine: float, ciclo: float):
        gruppi = sorted(gruppi, key=lambda g: g["comps"][0]["mu"])
        self.n, self.fondo, self.rt_fine, self.ciclo, self.minimo = len(gruppi), fondo, rt_fine, ciclo, 100.0
        flat = [(i, c) for i, g in enumerate(gruppi) for c in g["comps"]]
        self.owner = np.array([i for i, _ in flat], int)
        self.mu, self.sg, self.tau, self.h = (np.array([c[k] for _, c in flat], float) for k in ("mu", "sigma", "tau", "h"))
        chiave = np.array([g["comps"][0]["mu"] for g in gruppi], float)
        self.rumore = (np.array([g["mz"] for g in gruppi], float), chiave, np.ones(self.n), np.ones(self.n))
        self.fin = max([0.5] + [max(abs(t.min() - k), abs(t.max() - k)) + 0.1 for g, k in zip(gruppi, chiave) for t in (_griglia(g["comps"]),)])

    def profilo(self, rt: float, lo: int, hi: int):
        return np.bincount(self.owner, weights=self.h * forma(rt, self.mu, self.sg, self.tau), minlength=self.n)[lo:hi]

    @staticmethod
    def fondo_t(rt, amp, ph):
        return amp * gradiente(rt) * (1 + 0.1 * ds._sin(rt / 2.0 + ph))


def genera_larghi(out: Path, seme: int = 11, n_campioni: int = 3, n_larghi: int = 18, n_stretti: int = 12, n_fondo: int = 200, ciclo: float = CICLO_LARGHI,
                  rt_fine: float = RT_FINE_LARGHI) -> dict:
    """Writes `blank_01.mzML`, `camp_01..NN.mzML` and `verita.json` in out (same seed, same bytes). The truth is `bench/1`-style: `ioni` (id, ruolo broad|narrow,
    morfologia, mz, rt, fwhm_s, rt_min, rt_max, area_attesa per file, componenti) plus `file`. Units: RT min, FWHM s, area intensity * min. Compounds on a band
    (`su`) have a truth m/z within 3 ppm of it; in the file both are one centroid at the band m/z, as the instrument would merge them."""
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    righe, gruppi, fondo = costruisci_larghi(seme, n_larghi, n_stretti, n_fondo, n_campioni, rt_fine)
    nomi = ["blank_01.mzML"] + [f"camp_{k:02d}.mzML" for k in range(1, n_campioni + 1)]
    for k, nome in enumerate(nomi):
        scrivi_file(out / nome, k, Lotto(gruppi if k else [], fondo, rt_fine, ciclo), seme)
    verita = {"formato": "spal-bench/1", "seme": seme, "file": nomi, "blank": nomi[0], "n0": N0, "unita": {"rt": "min", "fwhm_s": "s", "area": "intensity*min"},
              "fondo": {"canali": n_fondo, "gradiente": "G(t)=0.15+0.85/(1+exp(-(t-16)/5))", "ampiezza_mediana": 6e4}, "ioni": righe}
    (out / "verita.json").write_text(json.dumps(verita, sort_keys=True, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    return verita


def main() -> None:
    ap = argparse.ArgumentParser(description="Bench lot of smeared peaks: blank + samples + verita.json")
    ap.add_argument("out")
    ap.add_argument("--seme", type=int, default=11)
    ap.add_argument("--campioni", type=int, default=3)
    a = ap.parse_args()
    v = genera_larghi(Path(a.out), a.seme, a.campioni)
    print(a.out, len(v["ioni"]), "truth rows,", len(v["file"]), "files")


if __name__ == "__main__":
    main()
