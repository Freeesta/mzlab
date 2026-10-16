"""Synthetic look-alikes of the lab's "B" series (flufenacet), for tests where the real mzML are not available
(cloud sessions, CI). The real data never go into the repository: this script WRITES files with the same names and
the same structure as the lab files, but every number is invented.

    python3 tools/dati_sintetici.py OUT_DIR            # writes B_FullMass-t0..t60, B_FullMass-neg-t0 (ESI-), C_IDA / C_MRM-EPI / C_POLALT (mixed files), HR_DDA-Exploris-t30 / HR_DDA-Fusion-t30 / HR_rotto-t30 (Orbitrap DDA, see hr_dda), B_MS2-t15, B_MRM-*, B_MRM-STD_*

What is built in (so the e2e tests find what they expect):
- Full Scan (7 times, RT 0.5-20 min, ~1100 scans, m/z 120-480, +0.3 Da offset like the instrument): parent 364.4 at
  14.3 min decaying with time, its isotopes (365.4, 366.4), in-source fragments 194.2, 152.2, 124.3 at the same RT and
  constant ratio, Na adduct 386.3, products 305.3 (14.7 min) and 229.1 (7.07 min) rising then falling, 224.2 (13.0 min),
  a contaminant 391.3 at 18.3 min in every file, chemical noise; TIC, BPC and a PDA "TWC" chromatogram.
- MS2 (B_MS2-t15, t45, t60): product-ion scans of precursors 229.1 and 305.0 (CE 10), many empty scans (as in the real files).
- MRM (7 samples + standards 0.06, 0.6, 2.4, 7.2, 12, 18 ppm, names with "_" as decimal separator like the lab files):
  SRM chromatograms 364.1>194.1 (Quant) and 364.1>152.1 (Qual), peak at 14.3 min, area proportional to concentration.
No .dam files: tests that need a method file are skipped by tools/verifica.py.
"""
from __future__ import annotations

import base64
import math
import sys
import zlib
from pathlib import Path

import numpy as np

OFF = 0.3                       # instrument m/z offset seen in the lab files
TIMES = [0, 5, 10, 15, 30, 45, 60]
STANDARDS = {"0_06": 0.06, "0_6": 0.6, "2_4": 2.4, "7_2": 7.2, "12": 12.0, "18": 18.0}


def _libm(f):
    """numpy's exp/log/sin/power choose SIMD code by CPU (AVX-512 or not) and may differ in the last bit; the scalar libm is the same
    on every runner, so the files, their SHA-256 (the keys of tests/golden/e2e) and the golden answers do not depend on the machine."""
    v = np.frompyfunc(f, 1, 1)
    return lambda x: np.asarray(v(np.asarray(x, float)), float) if np.ndim(x) else f(float(x))


_exp, _log, _sin = _libm(math.exp), _libm(math.log), _libm(math.sin)


def _b64(a, f4=False) -> str:
    return base64.b64encode(zlib.compress(np.asarray(a, "<f4" if f4 else "<f8").tobytes())).decode()


def _arr(acc, name, data, unit="") -> str:
    return (f'<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value=""{unit}/><binary>{_b64(data) if len(data) else ""}</binary></binaryDataArray>')


def _spec(i, rt, mz, it, level=1, prec=None, ce=None, neg=False, profile=False) -> str:
    p = ""
    if prec is not None:
        p = (f'<precursorList count="1"><precursor><selectedIonList count="1"><selectedIon>'
             f'<cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{prec}"/></selectedIon></selectedIonList>'
             f'<activation><cvParam cvRef="MS" accession="MS:1000045" name="collision energy" value="{ce}"/></activation></precursor></precursorList>')
    return (f'<spectrum index="{i}" id="sample=1 period=1 cycle={i + 1} experiment=1" defaultArrayLength="{len(mz)}">'
            f'<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="{level}"/>'
            + ('<cvParam cvRef="MS" accession="MS:1000128" name="profile spectrum" value=""/>' if profile else '')
            + ('<cvParam cvRef="MS" accession="MS:1000129" name="negative scan" value=""/>' if neg else '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>') +
            f'<cvParam cvRef="MS" accession="MS:1000285" name="total ion current" value="{float(np.sum(it)):.6g}"/>'
            f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt:.6f}" '
            f'unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>{p}<binaryDataArrayList count="2">'
            + _arr("MS:1000514", "m/z array", mz) + _arr("MS:1000515", "intensity array", it) + "</binaryDataArrayList></spectrum>")


def _chrom(i, cid, t, y, extra="", yunit="") -> str:
    return (f'<chromatogram index="{i}" id="{cid}" defaultArrayLength="{len(t)}">{extra}<binaryDataArrayList count="2">'
            + _arr("MS:1000595", "time array", t, ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
            + _arr("MS:1000515", "intensity array", y, f' unitName="{yunit}"' if yunit else "") + "</binaryDataArrayList></chromatogram>")


def pressure_trace(rt_end: float = 15.0, n: int = 6000):
    """Pump pressure like a Thermo file converted with msconvert: declared in pascal, values ten times too high (278 bar at 1 min really shows 2.78e8)."""
    t = np.linspace(0, rt_end, n)
    bar = 270 + 8 * t + 20 * _sin(t * 3)
    return t, bar * 1e5 * 10


def _pressure_chrom(i: int) -> str:
    t, y = pressure_trace()
    return _chrom(i, "Pump Pressure 1", t, y, '<cvParam cvRef="MS" accession="MS:1003019" name="pressure chromatogram" value=""/>', "pascal")


def _write(path: Path, specs: list[str], chroms: list[str]) -> None:
    head = ('<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
            '<instrumentConfigurationList count="1"><instrumentConfiguration id="IC1">'
            '<cvParam cvRef="MS" accession="MS:1000651" name="3200 QTRAP" value=""/></instrumentConfiguration></instrumentConfigurationList>'
            f'<run id="{path.stem}">')
    body = (f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>") if specs else ""
    path.write_text(head + body + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>",
                    encoding="utf-8")


def _g(rt, c, s, tau=0.04):
    """Slightly tailed Gaussian (exponential tail on the right), max ~1."""
    x = np.asarray(rt, float) - c
    return _exp(-0.5 * (x / s) ** 2) * np.where(x > 0, _exp(-x / max(tau, 1e-6) * 0.15), 1.0)


def _pda(rng, total, rt_end=22.0):
    t = np.arange(0, rt_end, 0.004)
    y = 2.0e3 + 1.5e2 * _sin(t / 3) + rng.normal(0, 30, t.size)
    for c, a in ((14.3, 2.2e4 * total), (7.07, 3e3), (18.3, 4e3), (2.1, 6e3)):
        y += a * _g(t, c, 0.07)
    return t, y


def full_scan(path: Path, t: float, rng, neg: bool = False, pressure: bool = False) -> None:
    k = 0.035
    P = 2.0e7 * _exp(-k * t)
    comps = [  # (m/z observed, RT, sigma, amplitude)
        (364.07 + OFF, 14.3, 0.06, P), (365.07 + OFF, 14.3, 0.06, 0.16 * P), (366.07 + OFF, 14.3, 0.06, 0.055 * P),
        (194.0 + 0.2, 14.3, 0.06, 0.8 * P), (152.0 + 0.2, 14.3, 0.06, 0.95 * P), (124.0 + 0.3, 14.3, 0.06, 0.3 * P),
        (386.05 + OFF, 14.3, 0.06, 0.03 * P),
        (305.0 + 0.3, 14.7, 0.06, 2.5e6 * (1 - _exp(-0.06 * t)) * _exp(-0.01 * t)),
        (229.1, 7.07, 0.06, 1.2e6 * (1 - _exp(-0.04 * t))),
        (224.2, 13.0, 0.06, 8e5 * (1 - _exp(-0.03 * t)) ** 2),
        (391.3, 18.3, 0.08, 6e5), (279.2, 2.1, 0.1, 3e5),
    ]
    if neg:                                           # negative mode: [M-H]- is 2.0146 lighter than [M+H]+ (everything else the same, only to test the sign and the default adduct)
        comps = [(m - 2.0146, c, s, a) for m, c, s, a in comps]
    rts = np.linspace(0.5087, 20.0076, 1111)
    specs, tic, bpc = [], [], []
    for i, rt in enumerate(rts):
        mz = list(rng.uniform(120, 480, 45)); it = list(rng.lognormal(_log(4e3), 0.9, 45))
        for m, c, s, a in comps:
            v = a * _g(rt, c, s) * rng.normal(1, 0.03)
            if v > 200:
                mz.append(m + rng.normal(0, 0.03)); it.append(v)
        o = np.argsort(mz); mz = np.array(mz)[o]; it = np.array(it)[o]
        specs.append(_spec(i, rt, mz, it, neg=neg)); tic.append(it.sum()); bpc.append(it.max())
    tp, yp = _pda(rng, _exp(-k * t))
    _write(path, specs, [_chrom(0, "TIC", rts, tic), _chrom(1, "BPC", rts, bpc), _chrom(2, "TWC", tp, yp)] + ([_pressure_chrom(3)] if pressure else []))


def ms2(path: Path, rng, t: float = 15) -> None:
    f = (1 - _exp(-0.04 * t)) / (1 - _exp(-0.6))                 # products grow with the treatment time
    prods = {229.1: ([123.1, 171.2, 211.1, 229.1], [0.05, 0.8, 0.25, 1.0], 7.07, 9e5 * f),
             305.0: ([152.2, 194.2, 263.1, 305.2], [0.3, 1.0, 0.2, 0.6], 14.7, 7e5 * f)}
    rts = np.linspace(0.0197, 20.0009, 1005)
    specs, tic, bpc = [], [], []
    for i, rt in enumerate(rts):
        prec = 229.1 if i % 2 == 0 else 305.0
        mzs, rel, c, a = prods[prec]
        sig = a * _g(rt, c, 0.06)
        if sig < 300 and rng.random() < 0.75:                       # empty scans, as in the lab files
            mz = it = np.zeros(0)
        else:
            mz = np.array(mzs) + rng.normal(0, 0.03, len(mzs)); it = np.array(rel) * max(sig, 300) * rng.normal(1, 0.05, len(mzs))
            noise_mz = rng.uniform(60, prec, 6); noise = rng.lognormal(_log(150), 0.6, 6)
            mz = np.concatenate([mz, noise_mz]); it = np.concatenate([it, noise]); o = np.argsort(mz); mz, it = mz[o], it[o]
        specs.append(_spec(i, rt, mz, it, level=2, prec=prec, ce=10.0)); tic.append(float(np.sum(it))); bpc.append(float(np.max(it)) if len(it) else 0.0)
    tp, yp = _pda(rng, 0.6, 20.0)
    _write(path, specs, [_chrom(0, "TIC", rts, tic), _chrom(1, "BPC", rts, bpc), _chrom(2, "TWC", tp, yp)])


def mrm(path: Path, amount: float, rng, sample: int) -> None:
    t = np.linspace(0, 20.0034, 19995)
    q = amount * _g(t, 14.3, 0.05) + rng.normal(0, 150, t.size).clip(0) + 200
    ql = 1.7 * amount * _g(t, 14.3, 0.05) + rng.normal(0, 150, t.size).clip(0) + 200
    pol = '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
    srm = lambda n, q3, name: f"SRM SIC Q1=364.1 Q3={q3} sample={sample} period=1 experiment=1 transition={n} ce=20 name={name}"
    tp, yp = _pda(rng, amount / 1.3e5, 20.0)
    _write(path, [], [_chrom(0, "TIC", t, q + ql), _chrom(1, "BPC", t, np.maximum(q, ql)),
                      _chrom(2, srm(0, "194.1", "Quant"), t, q, pol), _chrom(3, srm(1, "152.1", "Qual"), t, ql, pol),
                      _chrom(4, "TWC", tp, yp)])


def _scan_ions(rt, rng, ions, neg=False):
    mz = list(rng.uniform(120, 480, 12)); it = list(rng.lognormal(_log(3e3), 0.8, 12))
    for m, c, sg, a in ions:
        v = a * _g(rt, c, sg) * rng.normal(1, 0.03)
        if v > 200:
            mz.append(m + rng.normal(0, 0.03)); it.append(v)
    o = np.argsort(mz); return np.array(mz)[o], np.array(it)[o]


def mixed_ida(path: Path, rng) -> None:
    """IDA / DDA look-alike: a survey scan (level 1) and two product-ion scans (level 2) at every cycle, the precursors change from cycle to cycle."""
    ions = [(364.4, 14.3, 0.06, 2e7), (305.3, 14.7, 0.06, 2e6), (229.1, 7.07, 0.06, 1e6), (224.2, 13.0, 0.06, 6e5), (391.3, 18.3, 0.08, 6e5)]
    rts = np.linspace(0.5, 20.0, 160); specs, tic, i = [], [], 0
    for c, rt in enumerate(rts):
        mz, it = _scan_ions(rt, rng, ions); specs.append(_spec(i, rt, mz, it)); tic.append(it.sum()); i += 1
        for k in range(2):
            top = sorted(ions, key=lambda x: -x[3] * _g(rt, x[1], x[2]))[k]
            prec = round(top[0] + rng.normal(0, 0.15), 1) if c % 3 else round(top[0] + 0.0, 1)
            frag = np.sort(rng.uniform(60, prec - 10, 5)); it2 = rng.lognormal(_log(2e3), 0.6, 5) * (1 + 3 * _g(rt, top[1], top[2]))
            specs.append(_spec(i, rt + 0.004 * (k + 1), frag, it2, level=2, prec=prec, ce=25.0)); i += 1
    _write(path, specs, [_chrom(0, "TIC", rts, tic)])


def mixed_mrm_epi(path: Path, rng) -> None:
    """MRM + EPI look-alike: SRM chromatograms of two transitions plus product-ion scans (level 2) triggered on the MRM peak."""
    t = np.linspace(0, 20.0, 4000)
    q = 1e5 * _g(t, 14.3, 0.05) + rng.normal(0, 150, t.size).clip(0) + 200
    ql = 1.7e5 * _g(t, 14.3, 0.05) + rng.normal(0, 150, t.size).clip(0) + 200
    pol = '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
    srm = lambda n, q3, name: f"SRM SIC Q1=364.1 Q3={q3} sample=1 period=1 experiment=1 transition={n} ce=20 name={name}"
    specs = []
    for i, rt in enumerate(np.linspace(14.0, 14.7, 20)):
        frag = np.array([152.2, 194.2, 224.1, 305.2]); it = np.array([0.5, 1.0, 0.2, 0.3]) * 1e5 * _g(rt, 14.3, 0.1) * rng.normal(1, 0.05, 4)
        specs.append(_spec(i, rt, frag, it, level=2, prec=364.1 + (i % 2), ce=30.0))       # two triggering precursors alternate: not one block of a single precursor (that would be an infusion)
    _write(path, specs, [_chrom(0, "TIC", t, q + ql), _chrom(1, srm(0, "194.1", "Quant"), t, q, pol), _chrom(2, srm(1, "152.1", "Qual"), t, ql, pol)])


def mixed_polarity(path: Path, rng) -> None:
    """Positive and negative scans alternate at every cycle (Full Scan only)."""
    ions = [(364.4, 14.3, 0.06, 2e7), (362.4, 14.3, 0.06, 1e7)]
    rts = np.linspace(0.5, 20.0, 200); specs, i = [], 0
    for rt in rts:
        for neg in (False, True):
            mz, it = _scan_ions(rt, rng, [ions[1] if neg else ions[0]]); specs.append(_spec(i, rt + (0.003 if neg else 0), mz, it, neg=neg)); i += 1
    _write(path, specs, [])


def full_scan_profile(path: Path, t: float, rng) -> None:
    """The same kind of Full Scan as `full_scan`, but as PROFILE: points every 0.06 Da, peaks about 0.7 Da wide (FWHM) and one ion (305)
    with a flat top that a centroiding algorithm would split in two. Zeros are left out, as MSConvert does."""
    k = 0.035
    P = 2.0e7 * _exp(-k * t)
    comps = [(364.07 + OFF, 14.3, 0.06, P, 0.3), (365.07 + OFF, 14.3, 0.06, 0.16 * P, 0.3), (366.07 + OFF, 14.3, 0.06, 0.055 * P, 0.3),
             (194.0 + 0.2, 14.3, 0.06, 0.8 * P, 0.3), (152.0 + 0.2, 14.3, 0.06, 0.95 * P, 0.3),
             (305.0 + 0.3, 14.7, 0.06, 2.5e6 * (1 - _exp(-0.06 * t)) * _exp(-0.01 * t), -1),     # sigma < 0: flat top
             (229.1, 7.07, 0.06, 1.2e6 * (1 - _exp(-0.04 * t)), 0.3)]
    grid = np.round(np.arange(120.0, 400.0, 0.06), 2)
    rts = np.linspace(0.5087, 20.0076, 400)
    specs, tic, bpc = [], [], []
    for i, rt in enumerate(rts):
        y = rng.lognormal(_log(30), 0.8, grid.size) * (rng.random(grid.size) < 0.25)      # chemical noise: sparse small points
        for m, c, s, a, sg in comps:
            v = a * _g(rt, c, s) * rng.normal(1, 0.03)
            if v > 200:
                x = grid - m
                y = y + v * (_exp(-0.5 * (x / sg) ** 2) if sg > 0 else _exp(-np.square(np.square(np.square(x / 0.42)))))
        keep = y > 50
        mz, it = grid[keep], y[keep]
        specs.append(_spec(i, rt, mz, it, profile=True)); tic.append(it.sum()); bpc.append(it.max() if len(it) else 0.0)
    _write(path, specs, [_chrom(0, "TIC", rts, tic), _chrom(1, "BPC", rts, bpc)])


# ---------------------------------------------------------------------------------------------- high resolution (Orbitrap) DDA
HR_PARENT = "C14H13F4N3O2S"       # formula of the invented "parent" of the HR files (m/z computed with mzlab.chem.elements)
HR_NAMES = {"exploris": "HR_DDA-Exploris-t30.mzML", "fusion": "HR_DDA-Fusion-t30.mzML", "broken": "HR_rotto-t30.mzML"}


def hr_parent_mz() -> float:
    """[M+H]+ of HR_PARENT rounded to 1e-6: mass() adds with sum(), whose last bit changes from Python 3.12 on (compensated sum),
    and the files and the URLs of the golden e2e must not depend on the Python version."""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    from mzlab.chem.elements import ion_mz, mass, parse_formula
    return round(ion_mz(mass(parse_formula(HR_PARENT)), "[M+H]+"), 6)


def _cvp(acc, name, value="", unit=""):
    return f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value="{value}"{unit}/>'


_U_MZ = ' unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"'


def _hr_head(kind: str, broken: bool, stem: str) -> str:
    """Header of a Thermo-like mzML: model in a referenceableParamGroup, instrumentConfiguration(s) with the components."""
    fus = kind == "fusion"
    model = (_cvp("MS:1002416", "Orbitrap Fusion") if fus else _cvp("MS:1003095", "Orbitrap Exploris 120"))
    if broken:
        model = _cvp("MS:1009999", "Modello sconosciuto")
    def conf(ic, ana, det):
        return (f'<instrumentConfiguration id="{ic}"><referenceableParamGroupRef ref="CommonInstrumentParams"/><componentList count="4">'
                f'<source order="1">{_cvp("MS:1000073", "electrospray ionization")}{_cvp("MS:1000057", "electrospray inlet")}</source>'
                f'<analyzer order="2">{_cvp("MS:1000081", "quadrupole")}</analyzer><analyzer order="3">{ana}</analyzer>'
                f'<detector order="4">{det}</detector></componentList><softwareRef ref="Xcalibur"/></instrumentConfiguration>')
    ics = conf("IC1", _cvp("MS:1000484", "orbitrap"), _cvp("MS:1000624", "inductive detector"))
    if fus:
        ics += conf("IC2", _cvp("MS:1000083", "radial ejection linear ion trap"), _cvp("MS:1000253", "electron multiplier"))
    return ('<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
            '<cvList count="2"><cv id="MS" fullName="Proteomics Standards Initiative Mass Spectrometry Ontology" version="4.1.163" URI="x"/>'
            '<cv id="UO" fullName="Unit Ontology" version="09:04:2014" URI="x"/></cvList>'
            f'<fileDescription><fileContent>{_cvp("MS:1000579", "MS1 spectrum")}{_cvp("MS:1000580", "MSn spectrum")}</fileContent>'
            f'<sourceFileList count="1"><sourceFile id="RAW1" name="{stem}.raw" location="file:///sintetico">{_cvp("MS:1000768", "Thermo nativeID format")}'
            f'{_cvp("MS:1000563", "Thermo RAW format")}</sourceFile></sourceFileList></fileDescription>'
            f'<referenceableParamGroupList count="1"><referenceableParamGroup id="CommonInstrumentParams">{model}'
            f'{_cvp("MS:1000529", "instrument serial number", "SINTETICO")}</referenceableParamGroup></referenceableParamGroupList>'
            '<softwareList count="2"><software id="Xcalibur" version="0.0">' + _cvp("MS:1000532", "Xcalibur") + '</software>'
            '<software id="pwiz" version="0.0">' + _cvp("MS:1000615", "ProteoWizard software") + '</software></softwareList>'
            f'<instrumentConfigurationList count="{2 if fus else 1}">{ics}</instrumentConfigurationList>'
            '<dataProcessingList count="1"><dataProcessing id="pwiz_Reader_Thermo_conversion"><processingMethod order="0" softwareRef="pwiz">'
            + _cvp("MS:1000544", "Conversion to mzML") + '</processingMethod><processingMethod order="1" softwareRef="pwiz">'
            + _cvp("MS:1000035", "peak picking") + '</processingMethod></dataProcessing></dataProcessingList>'
            f'<run id="{stem}" defaultInstrumentConfigurationRef="IC1">')


def hr_dda(path: Path, rng, kind: str = "exploris", broken: bool = False, profile: bool = False) -> dict:
    """Thermo-like Orbitrap DDA file (all numbers invented). kind = "exploris" (HCD NCE 30, MS2 in the Orbitrap, isolation +-0.75, 4 decimals)
    or "fusion" (CID 35 in the ion trap, MS2 with 2 decimals and no resolution, isolation +-1.5, +3 ppm systematic error on the MS1);
    broken=True removes everything an HR reader might rely on (spectrumRef, isolation window, resolution, known model, normal filter string).
    profile=True writes the survey scans as PROFILE (every centroid becomes a Gaussian of nine points 0.004 Da apart; the MS2 stay centroids).

    Cycle every 0.03 min: one Full Scan (centroids, ~150 noise peaks) + the top-3 ions above 5e4 with 0.3 min dynamic exclusion.
    Ground truth (masses are invented, m/z of the parent from its formula):
    parent [M+H]+ at RT 14.3 (with 13C and 34S isotopes); isobars 305.0702 (RT 9.0) and 305.1066 (RT 12.0); isobars at the same RT
    412.1000 and 412.1364 (RT 16.0); co-isolation: 229.0500 (weak, 4e4: never picked) beside 229.6000 (5x) at RT 7.07;
    250.1234 at 3e4 (RT 11.0), never fragmented; a few other ions and two constant background ions to keep the cycles busy."""
    fus, sd = kind == "fusion", 0.8e-6
    mz_par = hr_parent_mz()
    sys_ppm = 3e-6 if fus else 0.0
    # (m/z, RT, sigma, amplitude, selectable); the background ions have sigma = 0 -> constant
    ions = [(mz_par, 14.3, 0.07, 3.0e7, True), (305.0702, 9.0, 0.07, 4.0e6, True), (305.1066, 12.0, 0.07, 3.0e6, True),
            (412.1000, 16.0, 0.07, 2.0e6, True), (412.1364, 16.0, 0.07, 1.5e6, True),
            (229.0500, 7.07, 0.07, 4.4e4, True), (229.6000, 7.07, 0.07, 2.2e5, True), (250.1234, 11.0, 0.07, 3.0e4, True),
            (195.0877, 5.0, 0.07, 8.0e5, True), (346.1218, 10.5, 0.07, 6.0e5, True), (505.2005, 18.0, 0.07, 5.0e5, True),
            (279.1591, 0.0, 0.0, 1.5e5, True), (391.2843, 0.0, 0.0, 3.0e5, True)]
    frag_rng = np.random.default_rng(99)
    frags = {}
    for m, *_ in ions:         # invented fragments: fractions of the precursor, with a mass defect
        f = np.sort(m * frag_rng.uniform(0.25, 0.93, 4))
        frags[m] = (np.round(f, 4), frag_rng.uniform(0.15, 1.0, 4))
    frags[mz_par] = (np.round(mz_par - np.array([33.9950, 61.9896, 97.9765, 125.0000]), 4), np.array([0.3, 1.0, 0.6, 0.2]))
    iso = lambda m: [(m + 1.00335, 0.13)] + ([(m + 1.99580, 0.045)] if m == mz_par else [])        # 13C (+34S for the parent)
    def level(m, rt, c, s, a):
        if s <= 0:
            return a * (1 + 0.1 * _sin(rt / 2.0))
        return a * float(_g(rt, c, s))
    pre, ex = "controllerType=0 controllerNumber=1 scan=", {}
    if kind == "fusion":
        ms2_filter = lambda p: f"ITMS + c ESI d Full ms2 {p:.4f}@cid35.00 [50.0000-{p + 10:.4f}]"; act, ce, hw = "MS:1000133", 35.0, 1.5
    else:
        ms2_filter = lambda p: f"FTMS + p ESI d Full ms2 {p:.4f}@hcd30.00 [50.0000-{p + 10:.4f}]"; act, ce, hw = "MS:1000422", 30.0, 0.75
    ms1_filter, res1, res2 = "FTMS + p ESI Full ms [100.0000-900.0000]", (60000 if fus else 45000), 15000
    if broken:
        ms1_filter = "FTMS {1,2} + p ESI w Full ms [100.0000-900.0000]"
        ms2_filter = lambda p: f"FTMS {{1,2}} + p ESI d w Full ms2 {p:.4f}@hcd30.00 [50.0000-{p + 10:.4f}]"
    specs, tic, rts_ms1, i, n_ms2 = [], [], [], 0, 0

    def build(idx, rt, level_, mz, it, extra_scan="", extra_prec=""):
        prof = profile and level_ == 1
        if prof:
            mz = np.asarray(mz, float); it = np.asarray(it, float)
            off = np.arange(-4, 5) * 0.004
            mz = (mz[:, None] + off[None, :]).ravel(); it = (it[:, None] * _exp(-0.5 * (off / 0.008) ** 2)[None, :]).ravel()
            o = np.argsort(mz, kind="stable"); mz, it = mz[o], it[o]
        n = len(mz)
        base = int(np.argmax(it)) if n else 0
        return (f'<spectrum index="{idx}" id="{pre}{idx + 1}" defaultArrayLength="{n}">'
                + _cvp("MS:1000579" if level_ == 1 else "MS:1000580", "MS1 spectrum" if level_ == 1 else "MSn spectrum")
                + _cvp("MS:1000511", "ms level", level_) + _cvp("MS:1000130", "positive scan") + (_cvp("MS:1000128", "profile spectrum") if prof else _cvp("MS:1000127", "centroid spectrum"))
                + _cvp("MS:1000504", "base peak m/z", f"{mz[base]:.6f}" if n else 0, _U_MZ) + _cvp("MS:1000505", "base peak intensity", f"{it[base]:.6g}" if n else 0)
                + _cvp("MS:1000285", "total ion current", f"{float(np.sum(it)):.6g}")
                + f'<scanList count="1">{_cvp("MS:1000795", "no combination")}<scan{extra_scan}>'
                + _cvp("MS:1000016", "scan start time", f"{rt:.6f}", ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
                + (extra_prec[0] if extra_prec else "") + '<scanWindowList count="1"><scanWindow>' + _cvp("MS:1000501", "scan window lower limit", 50.0, _U_MZ)
                + _cvp("MS:1000500", "scan window upper limit", 900.0, _U_MZ) + '</scanWindow></scanWindowList></scan></scanList>'
                + (extra_prec[1] if extra_prec else "") + '<binaryDataArrayList count="2">'
                + _arr("MS:1000514", "m/z array", mz) + _arr("MS:1000515", "intensity array", it) + "</binaryDataArrayList></spectrum>")

    cyc = 0.03
    for rt in np.arange(0.5, 20.0, cyc):
        vals = [(m, level(m, rt, c, s, a) * rng.normal(1, 0.03), sel) for m, c, s, a, sel in ions]
        mz = list(rng.uniform(100, 900, 150)); it = list(rng.lognormal(_log(8e3), 0.7, 150))
        meas = {}
        for m, v, _ in vals:
            if v < 2e3:
                continue
            e = m * (1 + sys_ppm + rng.normal(0, sd)); meas[m] = (e, v); mz.append(e); it.append(v)
            for d, r in iso(m):
                mz.append((m + (d - m)) * (1 + sys_ppm + rng.normal(0, sd))); it.append(v * r)
        o = np.argsort(mz); mz = np.array(mz)[o]; it = np.array(it)[o]
        idx1 = i
        res_xml = "" if broken else _cvp("MS:1000800", "mass resolving power", res1)
        specs.append(build(i, rt, 1, mz, it, "", (res_xml + _cvp("MS:1000512", "filter string", ms1_filter), ""))); tic.append(float(it.sum())); rts_ms1.append(rt); i += 1
        # data-dependent selection: the most intense ions above 5e4 that are not on the exclusion list (0.3 min)
        cand = sorted([(v, m) for m, v, sel in vals if sel and v > 5e4 and m in meas and not (m in ex and rt < ex[m])], reverse=True)[:3]
        for k, (v, m) in enumerate(cand):
            ex[m] = rt + 0.3
            e, vint = meas[m]
            w = [(mm, vv) for mm, vv, _ in vals if abs(mm - m) <= hw and mm in meas]                    # co-isolated ions
            fm, fi = [], []
            for mm, vv in w:
                fm += list(frags[mm][0] * (1 + rng.normal(0, sd, 4))); fi += list(frags[mm][1] * vv * 0.2 * rng.normal(1, 0.05, 4))
            fm += list(rng.uniform(60, m, 8)); fi += list(rng.lognormal(_log(1.5e3), 0.5, 8))
            o = np.argsort(fm); fm = np.array(fm)[o]; fi = np.array(fi)[o]
            if fus:
                fm = np.round(fm + rng.normal(0, 0.05, fm.size), 2)
            res2_xml = "" if (broken or fus) else _cvp("MS:1000800", "mass resolving power", res2)
            sc = (res2_xml + _cvp("MS:1000512", "filter string", ms2_filter(m)))
            cfg = ' instrumentConfigurationRef="IC2"' if fus else ""
            win = ("" if broken else "<isolationWindow>" + _cvp("MS:1000827", "isolation window target m/z", f"{m:.6f}", _U_MZ)
                   + _cvp("MS:1000828", "isolation window lower offset", hw, _U_MZ) + _cvp("MS:1000829", "isolation window upper offset", hw, _U_MZ) + "</isolationWindow>")
            ref = "" if broken else f' spectrumRef="{pre}{idx1 + 1}"'
            prec = (f'<precursorList count="1"><precursor{ref}>{win}<selectedIonList count="1"><selectedIon>' + _cvp("MS:1000744", "selected ion m/z", f"{e:.6f}", _U_MZ)
                    + _cvp("MS:1000041", "charge state", 1) + _cvp("MS:1000042", "peak intensity", f"{vint:.6g}") + "</selectedIon></selectedIonList><activation>"
                    + _cvp(act, "collision-induced dissociation") + _cvp("MS:1000045", "collision energy", ce, ' unitCvRef="UO" unitAccession="UO:0000266" unitName="electronvolt"')
                    + "</activation></precursor></precursorList>")
            specs.append(build(i, rt + 0.004 * (k + 1), 2, fm, fi, cfg, (sc, prec))); i += 1; n_ms2 += 1
    chroms = [_chrom(0, "TIC", np.array(rts_ms1), np.array(tic)), _chrom(1, "Pump Pressure", np.array(rts_ms1), 250.0 + 5 * rng.normal(0, 1, len(rts_ms1)))]
    stem = Path(path).stem
    Path(path).write_text(_hr_head(kind, broken, stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                          + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")
    return {"n_ms1": len(rts_ms1), "n_ms2": n_ms2, "parent_mz": mz_par}


def hr_series(path: Path, rng, features: list[tuple], rt_end: float = 12.0, cyc: float = 0.04) -> None:
    """Orbitrap-like Full Scan file (MS1 only, centroids, ~100 noise peaks per scan) whose ions are given by the caller.
    features: (m/z, RT, sigma in min, amplitude); an amplitude of 0 leaves the ion out. Used by the tests of the series statistics."""
    pre, specs, rts, tic = "controllerType=0 controllerNumber=1 scan=", [], [], []
    filt = "FTMS + p ESI Full ms [100.0000-900.0000]"
    for i, rt in enumerate(np.arange(0.5, rt_end, cyc)):
        mz = list(rng.uniform(100, 900, 100)); it = list(rng.lognormal(_log(3e3), 0.5, 100))
        for m, c, sg, a in features:
            v = a * float(_g(rt, c, sg)) * rng.normal(1, 0.02) if a > 0 else 0.0
            if v >= 1e3:
                mz.append(m * (1 + rng.normal(0, 0.8e-6))); it.append(v)
        o = np.argsort(mz); mz = np.array(mz)[o]; it = np.array(it)[o]
        n, b = len(mz), int(np.argmax(it))
        specs.append(f'<spectrum index="{i}" id="{pre}{i + 1}" defaultArrayLength="{n}">' + _cvp("MS:1000579", "MS1 spectrum") + _cvp("MS:1000511", "ms level", 1)
                     + _cvp("MS:1000130", "positive scan") + _cvp("MS:1000127", "centroid spectrum")
                     + _cvp("MS:1000504", "base peak m/z", f"{mz[b]:.6f}", _U_MZ) + _cvp("MS:1000505", "base peak intensity", f"{it[b]:.6g}")
                     + _cvp("MS:1000285", "total ion current", f"{float(np.sum(it)):.6g}")
                     + f'<scanList count="1">{_cvp("MS:1000795", "no combination")}<scan>'
                     + _cvp("MS:1000016", "scan start time", f"{rt:.6f}", ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
                     + _cvp("MS:1000800", "mass resolving power", 60000) + _cvp("MS:1000512", "filter string", filt)
                     + '<scanWindowList count="1"><scanWindow>' + _cvp("MS:1000501", "scan window lower limit", 100.0, _U_MZ)
                     + _cvp("MS:1000500", "scan window upper limit", 900.0, _U_MZ) + '</scanWindow></scanWindowList></scan></scanList>'
                     + '<binaryDataArrayList count="2">' + _arr("MS:1000514", "m/z array", mz) + _arr("MS:1000515", "intensity array", it) + "</binaryDataArrayList></spectrum>")
        rts.append(rt); tic.append(float(it.sum()))
    chroms = [_chrom(0, "TIC", np.array(rts), np.array(tic))]
    Path(path).write_text(_hr_head("exploris", False, Path(path).stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                          + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")


def subtraction_series(out: Path, blank: bool = True, seed: int = 41) -> list[Path]:
    """Files SUB_t0, SUB_t10, SUB_t20 (+ SUB_blank) to try the subtraction of a reference: a parent at m/z 253.3 (RT 5.2) that decays, a product at 301.3
    (RT 6.4) that appears, a background line at 150.2 (RT 3.0) in every file, an RT drift of 0.05 min in t10 and t20 and a gain of 1.3 in t20 (the TIC
    grows without the sample changing). Every number is invented."""
    out = Path(out); out.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(seed)
    rts = np.arange(0.5, 10.0, 0.05)
    plan = {"t0": (8e5, 0.0, 0.0, 1.0), "t10": (4e5, 3e5, 0.05, 1.0), "t20": (2e5, 6e5, 0.05, 1.3), "blank": (0.0, 0.0, 0.0, 1.0)}
    paths = []
    for name, (par, prod, drift, gain) in plan.items():
        if name == "blank" and not blank:
            continue
        specs, tic = [], []
        for i, rt in enumerate(rts):
            mz = list(rng.uniform(100, 400, 20)); it = list(rng.lognormal(_log(3e3), 0.8, 20))
            for m, c, a in ((253.3, 5.2 + drift, par), (301.3, 6.4 + drift, prod), (150.2, 3.0, 2e5)):
                v = a * float(_g(rt, c, 0.08, 1e6)) * rng.normal(1, 0.01)
                if v > 100:
                    mz.append(m + rng.normal(0, 0.02)); it.append(v)
            o = np.argsort(mz); mz = np.array(mz)[o]; it = np.array(it)[o] * gain
            specs.append(_spec(i, float(rt), mz, it)); tic.append(it.sum())
        p = out / f"SUB_{name}.mzML"
        _write(p, specs, [_chrom(0, "TIC", rts, tic)])
        paths.append(p)
    return paths


def make(out: Path) -> Path:
    out = Path(out); out.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(2026)
    for t in TIMES:                                   # the lab's t30 file is called "B_FullMass-t30 (2)": same name here
        full_scan(out / (f"B_FullMass-t{t}.mzML" if t != 30 else "B_FullMass-t30 (2).mzML"), t, rng)
    for t in (15, 45, 60):
        ms2(out / f"B_MS2-t{t}.mzML", rng, t)
    for i, t in enumerate((0, 5, 10, 15, 30, 45, 60)):
        mrm(out / f"B_MRM-t{t}.mzML", 1.3e5 * _exp(-0.035 * t), rng, 6 + i)
    for i, (name, c) in enumerate(STANDARDS.items()):
        mrm(out / f"B_MRM-STD_{name}ppm.mzML", 1.0e4 * c, rng, 1 + i)
    full_scan(out / "B_FullMass-neg-t0.mzML", 0, np.random.default_rng(7), neg=True)      # one NEGATIVE Full Scan (written last: the other files do not change)
    rng2 = np.random.default_rng(11)                  # mixed files (several experiments in ONE file), written last: nothing else changes
    mixed_ida(out / "C_IDA-t0.mzML", rng2); mixed_mrm_epi(out / "C_MRM-EPI-t0.mzML", rng2); mixed_polarity(out / "C_POLALT-t0.mzML", rng2)
    rng3 = np.random.default_rng(5)                   # profile files, written last: nothing else changes
    for t in (0, 15, 60):
        full_scan_profile(out / f"P_FullMass-t{t}.mzML", t, rng3)
    rng4 = np.random.default_rng(31)                  # high-resolution DDA files, written last: nothing else changes
    hr_dda(out / HR_NAMES["exploris"], rng4, "exploris"); hr_dda(out / HR_NAMES["fusion"], rng4, "fusion"); hr_dda(out / HR_NAMES["broken"], rng4, "exploris", broken=True)
    subtraction_series(out)                           # written last: nothing else changes
    (out / "SINTETICI.txt").write_text("Dati sintetici scritti da tools/dati_sintetici.py: NON sono misure del laboratorio.\n", encoding="utf-8")
    return out


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    print(make(Path(sys.argv[1])))
