"""mzML writer of the synthetic identity series (tools/valida/sintetici_identita.py): Orbitrap-like DDA files, all numbers invented.

One file = survey scans every CICLO min (centroids, noise, continuous background with FT lobes, the ions of the model) and, at the apex of every
selectable ion, an MS2 in which all the ions inside the isolation window are fragmented together (this is what makes an MS2 chimeric)."""
from __future__ import annotations

from pathlib import Path

import numpy as np

import dati_sintetici as ds

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
    rts = np.arange(RT0, c.rt_fine, CICLO)
    pian = piano_ms2(ioni, k)
    specs, tic, idx, n_ms2 = [], [], 0, 0
    for n, rt in enumerate(rts):
        lo, hi = np.searchsorted(cr, [rt - FIN_RT, rt + FIN_RT])
        v = np.zeros(len(cm))
        v[lo:hi] = np.minimum(ch[lo:hi] * np.exp(-0.5 * ((rt - cr[lo:hi]) / cs[lo:hi]) ** 2) * rng.normal(1, 0.03, hi - lo), cap[lo:hi])
        keep = v >= 2e3
        mz = [rng.uniform(100, 900, RUMORE), cm[keep] * (1 + rng.normal(0, SD_PPM, int(keep.sum())))]
        it = [rng.lognormal(np.log(8e3), 0.7, RUMORE), v[keep]]
        if len(bg):
            hb = bg[:, 1] * (1 + 0.1 * np.sin(rt / 2.0 + bg[:, 2])) * rng.normal(1, 0.03, len(bg))
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
