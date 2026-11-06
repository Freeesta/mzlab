"""Large synthetic HR DDA series for profiling the Flow engine (bench only: the bytes depend on numpy's SIMD exp, so no golden answer reads them).

    python3 tools/dati_sintetici_grandi.py OUT [--files 12] [--scans 3000] [--compounds 3000] [--noise 1500] [--top 4] [--seed 5] [--frags 8]

writes OUT/G_000.mzML ... : Orbitrap-like centroid MS1 every cycle plus top-N MS2 (isolation +-0.8, 0.2 min dynamic exclusion). Each invented
compound has [M+H]+ with 13C/13C2 isotopes, half of them [M+Na]+, a quarter an in-source fragment; abundance varies per file (10 % absent), the RT drifts
linearly with the injection order, m/z jitters 1 ppm. Every number is invented.
"""
import argparse
import base64
import zlib
from pathlib import Path

import numpy as np

NA, H2O, C13 = 21.981943, 18.010565, 1.003355


def b64(a, f4):
    return base64.b64encode(zlib.compress(np.asarray(a, "<f4" if f4 else "<f8").tobytes(), 1)).decode()


def arr(acc, name, data, f4):
    bits = ("MS:1000521", "32-bit float") if f4 else ("MS:1000523", "64-bit float")
    return (f'<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="{bits[0]}" name="{bits[1]}" value=""/>'
            '<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value=""/><binary>{b64(data, f4) if len(data) else ""}</binary></binaryDataArray>')


def spec(i, scan_ms1, rt, mz, it, level, prec=None):
    p, ref = "", ""
    if prec is not None:
        p = ('<precursorList count="1"><precursor spectrumRef="controllerType=0 controllerNumber=1 scan=%d"><isolationWindow>'
             '<cvParam cvRef="MS" accession="MS:1000827" name="isolation window target m/z" value="%.5f"/>'
             '<cvParam cvRef="MS" accession="MS:1000828" name="isolation window lower offset" value="0.8"/>'
             '<cvParam cvRef="MS" accession="MS:1000829" name="isolation window upper offset" value="0.8"/></isolationWindow>'
             '<selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="%.5f"/>'
             '</selectedIon></selectedIonList><activation><cvParam cvRef="MS" accession="MS:1000422" name="beam-type collision-induced dissociation" value=""/>'
             '<cvParam cvRef="MS" accession="MS:1000045" name="collision energy" value="30"/></activation></precursor></precursorList>') % (scan_ms1, prec, prec)
    tic = float(np.sum(it)) if len(it) else 0.0
    filt = "FTMS + p ESI Full ms [100.0000-1000.0000]" if level == 1 else "FTMS + p ESI d Full ms2 %.4f@hcd30.00 [50.0000-1000.0000]" % prec
    return (f'<spectrum index="{i}" id="controllerType=0 controllerNumber=1 scan={i + 1}" defaultArrayLength="{len(mz)}">'
            f'<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="{level}"/>'
            '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
            '<cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000285" name="total ion current" value="{tic:.6g}"/>'
            f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt:.6f}" '
            'unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/>'
            f'<cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="{filt}"/></scan></scanList>'
            f'{p}<binaryDataArrayList count="2">' + arr("MS:1000514", "m/z array", mz, False)
            + arr("MS:1000515", "intensity array", it, True) + "</binaryDataArrayList></spectrum>")


def compounds(rng, n, nfrag=8):
    m = np.sort(rng.uniform(120, 900, n))
    rt = rng.uniform(0.8, 14.0, n)
    sig = rng.uniform(0.025, 0.08, n)
    amp = 10 ** rng.uniform(5.0, 8.0, n)
    ions = []                                   # (compound, m/z, relative amplitude)
    for k in range(n):
        nc = m[k] / 14.0
        ions += [(k, m[k], 1.0), (k, m[k] + C13, 0.011 * nc), (k, m[k] + 2 * C13, (0.011 * nc) ** 2 / 2)]
        if k % 2 == 0:
            ions.append((k, m[k] + NA, 0.3))
        if k % 4 == 1 and m[k] > 200:
            ions.append((k, m[k] - H2O, 0.5))
    a = np.array(ions)
    frags = [np.sort(m[k] * rng.uniform(0.2, 0.95, nfrag)) for k in range(n)]
    return m, rt, sig, amp, a[:, 0].astype(int), a[:, 1], a[:, 2], frags


def write_file(path, f, n_files, a, rng, scans, noise, top):
    m, rt, sig, amp, ck, imz, irel, frags = a
    drift = 0.002 * (f - n_files / 2)
    present = rng.uniform(0, 1, len(m)) > 0.1
    gain = amp * rng.lognormal(0, 0.4, len(m)) * present
    crt = rt + drift
    o = np.argsort(crt[ck]); ck, imz, irel = ck[o], imz[o], irel[o]
    lo_rt = crt[ck] - 5 * sig[ck]
    order = np.argsort(lo_rt)
    cyc = 15.0 / scans
    excl = np.full(len(m), -1e9)
    out, i, s1 = [], 0, 0
    hi_rt = crt[ck] + 5 * sig[ck]
    for s in range(scans):
        t = 0.3 + s * cyc
        act = np.nonzero((lo_rt <= t) & (hi_rt >= t))[0]
        k = ck[act]
        y = gain[k] * irel[act] * np.exp(-0.5 * ((t - crt[k]) / sig[k]) ** 2) * rng.normal(1, 0.03, len(act))
        keep = y >= 1e3
        mz = np.concatenate([imz[act][keep] * (1 + rng.normal(0, 1e-6, keep.sum())), rng.uniform(100, 1000, noise)])
        it = np.concatenate([y[keep], rng.lognormal(np.log(4e3), 0.6, noise)])
        so = np.argsort(mz, kind="stable")
        out.append(spec(i, i + 1, t, mz[so], it[so], 1)); s1 = i; i += 1
        cand = [(y[j], k[j]) for j in np.nonzero(keep)[0] if irel[act][j] == 1.0 and y[j] > 5e4 and t - excl[k[j]] > 0.2]
        cand.sort(reverse=True)
        for yv, kk in cand[:top]:
            excl[kk] = t
            fz = frags[kk]
            fi = yv * 0.05 * rng.uniform(0.1, 1, len(fz))
            out.append(spec(i, s1 + 1, t + 0.001, fz, fi, 2, prec=float(m[kk]))); i += 1
    head = ('<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
            '<cvList count="2"><cv id="MS" fullName="MS" version="4.1" URI="x"/><cv id="UO" fullName="UO" version="1" URI="x"/></cvList>'
            '<fileDescription><fileContent><cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum" value=""/></fileContent></fileDescription>'
            '<referenceableParamGroupList count="1"><referenceableParamGroup id="C"><cvParam cvRef="MS" accession="MS:1003095" name="Orbitrap Exploris 120" value=""/>'
            '</referenceableParamGroup></referenceableParamGroupList><softwareList count="1"><software id="s" version="0"/></softwareList>'
            '<instrumentConfigurationList count="1"><instrumentConfiguration id="IC1"><referenceableParamGroupRef ref="C"/></instrumentConfiguration></instrumentConfigurationList>'
            '<dataProcessingList count="1"><dataProcessing id="d"/></dataProcessingList>'
            f'<run id="{path.stem}" defaultInstrumentConfigurationRef="IC1"><spectrumList count="{len(out)}">')
    path.write_text(head + "".join(out) + "</spectrumList></run></mzML>", encoding="utf-8")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out", type=Path)
    ap.add_argument("--files", type=int, default=12)
    ap.add_argument("--scans", type=int, default=3000)
    ap.add_argument("--compounds", type=int, default=3000)
    ap.add_argument("--noise", type=int, default=1500)
    ap.add_argument("--top", type=int, default=4)
    ap.add_argument("--seed", type=int, default=5)
    ap.add_argument("--frags", type=int, default=8, help="fragments of every MS2 spectrum")
    g = ap.parse_args()
    g.out.mkdir(parents=True, exist_ok=True)
    a = compounds(np.random.default_rng(g.seed), g.compounds, g.frags)
    for f in range(g.files):
        write_file(g.out / f"G_{f:03d}.mzML", f, g.files, a, np.random.default_rng(g.seed * 1000 + f), g.scans, g.noise, g.top)


if __name__ == "__main__":
    main()
