"""Gold data of mzmine 3.9.0 per stage on four small synthetic mzML files (all numbers invented, fixed seed) -> `tests/data/untargeted/mzmine39/`.

The files carry every case that the stages of mzmine treat differently: isolated Gaussians, two peaks that touch (valley at 40 % and 70 % of the lower apex), a trace
with a hole of one zero, a long background trace with a peak on top, an isotope pair of charge 1 and one of charge 2, a retention-time drift between files (join),
and a peak that one file lacks (gap filling: it is below the minimum height there but present in the raw data). Usage:
    python3 tools/bench/oro_mzmine.py --genera DIR        write the four mzML (they never go into the repository)
    python3 tools/bench/oro_mzmine.py --oro [DIR]         run mzmine 3.9.0 and write the CSV and the LEGGIMI of `tests/data/untargeted/mzmine39/`
    python3 tools/bench/oro_mzmine.py --reale MZML_DIR DEST  ADAP lists of mzmine on the first two mzML of a folder (real-data test of the ADAP builder)"""
from __future__ import annotations

import argparse
import csv
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
from datetime import date
from pathlib import Path

import numpy as np

RADICE = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RADICE / "tools"), str(RADICE / "tools" / "bench")]
import dati_sintetici as ds  # noqa: E402
from valida import sintetici_mzml as sm  # noqa: E402
from bench import corri, corri_mzmine  # noqa: E402

DEST = RADICE / "tests" / "data" / "untargeted" / "mzmine39"
SEME = 20261015
CICLO = 0.01                  # min between two scans
FINE = 6.0                    # min
ISO = 1.003355                # mass of one 13C - 12C
DERIVA = {"oro_1": 0.0, "oro_2": 0.05, "oro_3": -0.05, "oro_4": 0.0}
SCALA = {"oro_1": 1.0, "oro_2": 0.8, "oro_3": 1.2, "oro_4": 0.9}


def _arr(acc, name, data, f4=False) -> str:
    """The array of the writer of the identity series with the true `encodedLength` (the parser of mzmine reads the base 64 text by that length)."""
    cv = '<cvParam cvRef="MS" accession="MS:1000521" name="32-bit float" value=""/>' if f4 else '<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>'
    b = ds._b64(data, f4) if len(data) else ""
    return (f'<binaryDataArray encodedLength="{len(b)}">{cv}<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value=""/><binary>{b}</binary></binaryDataArray>')


sm._arr = _arr


def _valle(sigma: float, rapporto: float) -> float:
    """Distance between the apices of two equal Gaussians whose valley is `rapporto` of the apex."""
    return sigma * float(np.sqrt(-8 * np.log(rapporto / 2)))


FONDO, BUCO = 7, 6            # indices in `componenti` of the long trace and of the trace with a hole
# (m/z, RT, sigma, height): the isolated ones, the touching pairs, the hole, the peak over the background, the isotopes
def componenti(nome: str) -> list[tuple[float, float, float, float]]:
    d, s = DERIVA[nome], SCALA[nome]
    c = [(200.1000, 1.00, 0.05, 5e5), (250.2000, 1.50, 0.04, 2e5 if nome != "oro_3" else 2.5e4)]
    sg = 0.05
    c += [(300.1500, 2.40, sg, 3e5), (300.1500, 2.40 + _valle(sg, 0.4), sg, 3e5)]
    c += [(350.1700, 2.40, sg, 3e5), (350.1700, 2.40 + _valle(sg, 0.7), sg, 3e5)]
    c += [(400.2000, 3.50, 0.06, 4e5), (450.3000, 4.50, 0.05, 5e5)]
    c += [(500.2500, 5.00, 0.05, 6e5), (500.2500 + ISO, 5.00, 0.05, 6e5 * 0.30), (500.2500 + 2 * ISO, 5.00, 0.05, 6e5 * 0.05)]
    c += [(600.3000, 5.40, 0.05, 4e5), (600.3000 + ISO / 2, 5.40, 0.05, 4e5 * 0.60), (600.3000 + ISO, 5.40, 0.05, 4e5 * 0.20)]
    return [(m * (1 + 0.5e-6 * (k % 3 - 1)), rt + d, w, h * s) for k, (m, rt, w, h) in enumerate(c)]


def scrivi(path: Path) -> str:
    nome = path.stem
    rng = np.random.default_rng([SEME, int(nome[-1])])
    comp = componenti(nome)
    rts = np.arange(0.2, FINE, CICLO)
    buco = int(np.argmin(np.abs(rts - (3.50 + DERIVA[nome]))))      # the scan at the apex of the trace with a hole
    specs, tic = [], []
    for n, rt in enumerate(rts):
        mz, it = [], []
        for k, (m, rc, w, h) in enumerate(comp):
            y = h * np.exp(-0.5 * ((rt - rc) / w) ** 2)
            if k == FONDO:
                y += 6e4 * (1 + 0.15 * np.sin(rt * 3))                 # trace present in the whole run, with a peak on top
            if k == BUCO and n == buco:
                y = 0.0
            if y >= 2e3:
                mz.append(m * (1 + rng.normal(0, 0.3e-6)))
                it.append(y * rng.normal(1, 0.02))
        mz += list(rng.uniform(100, 700, 20)); it += list(rng.lognormal(np.log(3e3), 0.5, 20))
        mz, it = np.array(mz), np.array(it)
        o = np.argsort(mz, kind="stable")
        specs.append(sm._scan(n, rt, 1, mz[o], it[o], filt=sm.FILTRO1, res=60000))
        tic.append(float(it.sum()))
    chroms = [ds._chrom(0, "TIC", rts, np.array(tic))]
    path.write_text(ds._hr_head("exploris", False, nome) + f'<spectrumList count="{len(specs)}" defaultDataProcessingRef="pwiz_Reader_Thermo_conversion">' + "".join(specs) + "</spectrumList>"
                    + f'<chromatogramList count="{len(chroms)}" defaultDataProcessingRef="pwiz_Reader_Thermo_conversion">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def genera(cartella: Path) -> dict[str, str]:
    cartella.mkdir(parents=True, exist_ok=True)
    return {f"{n}.mzML": scrivi(cartella / f"{n}.mzML") for n in DERIVA}


def _leggi(path: Path) -> list[dict]:
    with open(path, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def _numero(x: str) -> float | None:
    try:
        return float(x)
    except (TypeError, ValueError):
        return None


def _per_file(fasi: Path, prefisso: str) -> list[tuple[str, list[dict]]]:
    """The exports `<prefix>_<file>.mzML_<list name>.csv` of one stage, by file name."""
    out = []
    for p in sorted(fasi.glob(f"{prefisso}_*.mzML_*.csv")):
        out.append((re.match(rf"{prefisso}_(.+?)\.mzML", p.name).group(1), _leggi(p)))
    return out


def _scrivi_csv(path: Path, colonne: list[str], righe: list[list], chiave: tuple[str, ...]) -> None:
    """Rows sorted by the named columns (numbers as numbers), so that two runs write the same bytes."""
    def k(r):
        return tuple((0, float(v), "") if _numero(v) is not None else (1, 0.0, str(v)) for v in (r[colonne.index(c)] for c in chiave))
    righe = sorted(righe, key=k)
    with open(path, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(colonne)
        w.writerows(righe)


def _fase_per_file(fasi: Path, prefisso: str, area: bool) -> tuple[list[str], list[list]]:
    col = ["file", "id", "mz", "rt", "rt_start", "rt_end", "height"] + (["area"] if area else [])
    righe = [[f, r["id"], r["mz"], r["rt"], r["rt_range:min"], r["rt_range:max"], r["height"]] + ([r["area"]] if area else []) for f, rs in _per_file(fasi, prefisso) for r in rs]
    return col, righe


def _fase_allineata(csv_path: Path) -> tuple[list[str], list[list]]:
    """The aligned table in long form: one line per cell with a feature (detected or estimated)."""
    col = ["row", "mz", "rt", "aligned_features", "score", "file", "mz_file", "rt_file", "height", "area", "state"]
    righe = []
    for r in _leggi(csv_path):
        for k in sorted({m.group(1) for c in r if (m := re.match(r"datafile:(oro_\d)\.mzML:area$", c))}):
            if _numero(r[f"datafile:{k}.mzML:area"]) is None:
                continue
            c = lambda n: r[f"datafile:{k}.mzML:{n}"]
            righe.append([r["id"], r["mz"], r["rt"], r["alignment_scores:aligned_features_n"], r["alignment_scores:weighted_distance_score"], k, c("mz"), c("rt"), c("height"), c("area"), c("feature_state")])
    return col, righe


def _fase_isotopi(fasi: Path) -> tuple[list[str], list[list]]:
    """Per file: the rows that the grouper keeps (charge > 0: principal; charge 0: alone) and the ones it removes (isotopologues, attached to the principal of their charge)."""
    col = ["file", "id", "mz", "rt", "height", "charge", "role", "principal"]
    righe = []
    for f, tenute in _per_file(fasi, "isotopi"):
        tutte = dict(_per_file(fasi, "risolutore"))[f]
        principali = {r["id"]: (float(r["mz"]), int(float(r["charge"] or 0)), float(r["rt"])) for r in tenute}
        for r in tutte:
            mz, rt = float(r["mz"]), float(r["rt"])
            if r["id"] in principali:
                z = principali[r["id"]][1]
                righe.append([f, r["id"], r["mz"], r["rt"], r["height"], z, "principal" if z else "alone", ""])
                continue
            cand = [(abs((mz - m) * z / ISO - round((mz - m) * z / ISO)), i) for i, (m, z, t) in principali.items() if z and m < mz and abs(t - rt) < 0.1 and 0 < round((mz - m) * z / ISO) < 5]
            righe.append([f, r["id"], r["mz"], r["rt"], r["height"], "", "isotopologue", min(cand)[1] if cand else ""])
    return col, righe


def _sostituisci(testo: str, dir_: Path) -> str:
    return testo.replace(str(dir_), "<DIR>")


def scrivi_oro(dest: Path) -> None:
    """Runs mzmine 3.9.0 on the four files and writes the CSV of the stages and the LEGGIMI of provenance in `dest`."""
    mm, jh = corri_mzmine.installa_39()
    par = corri.parametri("mzmine", "predefinita")
    with tempfile.TemporaryDirectory() as t:
        t = Path(t)
        shas = genera(t / "mzml")
        fs = sorted((t / "mzml").glob("*.mzML"))
        fasi = t / "fasi"
        fasi.mkdir()
        xml_a, xml_b = fasi / "fasi.xml", fasi / "isotopi.xml"
        xml_a.write_text(corri_mzmine.batch_xml(fs, fasi / "finale.csv", par, fasi=fasi), encoding="utf-8")
        xml_b.write_text(corri_mzmine.batch_xml(fs, fasi / "finale.csv", par, fasi=fasi, isotopi=True), encoding="utf-8")
        ignoti = corri_mzmine.parametri_non_letti(xml_a, mm) + corri_mzmine.parametri_non_letti(xml_b, mm)
        if ignoti:
            raise SystemExit("mzmine would ignore these parameters: " + "; ".join(ignoti))
        for x in (xml_a, xml_b):
            r = corri_mzmine.esegui_batch(x, mm, jh, t / "tmp")
            if r.returncode or "SEVERE" in r.stdout + r.stderr:
                raise SystemExit(f"mzmine failed on {x.name}:\n{(r.stdout + r.stderr)[-2000:]}")
        dest.mkdir(parents=True, exist_ok=True)
        per_file, allineata = ("file", "id"), ("mz", "rt", "file")
        for nome, (col, righe), chiave in [("adap.csv", _fase_per_file(fasi, "adap", False), per_file), ("resolved.csv", _fase_per_file(fasi, "risolutore", True), per_file),
                                           ("aligned.csv", _fase_allineata(fasi / "join_Aligned_feature_list.csv"), allineata),
                                           ("gapfilled.csv", _fase_allineata(fasi / "riempimento_Aligned_feature_list_gap-filled.csv"), allineata),
                                           ("isotopes.csv", _fase_isotopi(fasi), per_file)]:
            _scrivi_csv(dest / nome, col, righe, chiave)
        java = subprocess.run([str(jh / "bin" / "java"), "-version"], capture_output=True, text=True).stderr.strip().splitlines()
        java = [x for x in java if x.startswith(("openjdk version", "OpenJDK Runtime"))]
        (dest / "LEGGIMI.md").write_text(_leggimi(shas, [java[0], java[1]] if len(java) > 1 else java, _sostituisci(xml_a.read_text(), t), _sostituisci(xml_b.read_text(), t)), encoding="utf-8")


def scrivi_reali(cartella: Path, dest: Path, quanti: int = 2) -> None:
    """Runs mzmine 3.9.0 on the first `quanti` mzML of `cartella` and writes `adap_<file>.csv` (columns of `adap.csv`) in `dest`: the lists the real-data test of the ADAP builder reads."""
    mm, jh = corri_mzmine.installa_39()
    par = corri.parametri("mzmine", "predefinita")
    fs = sorted(cartella.glob("*.mzML"))[:quanti]
    if not fs:
        raise SystemExit(f"no mzML in {cartella}")
    with tempfile.TemporaryDirectory() as t:
        t = Path(t)
        fasi = t / "fasi"
        fasi.mkdir()
        xml = fasi / "fasi.xml"
        xml.write_text(corri_mzmine.batch_xml(fs, fasi / "finale.csv", par, fasi=fasi), encoding="utf-8")
        ignoti = corri_mzmine.parametri_non_letti(xml, mm)
        if ignoti:
            raise SystemExit("mzmine would ignore these parameters: " + "; ".join(ignoti))
        r = corri_mzmine.esegui_batch(xml, mm, jh, t / "tmp")
        if r.returncode or "SEVERE" in r.stdout + r.stderr:
            raise SystemExit(f"mzmine failed:\n{(r.stdout + r.stderr)[-2000:]}")
        dest.mkdir(parents=True, exist_ok=True)
        col, righe = _fase_per_file(fasi, "adap", False)
        for f in sorted({r[0] for r in righe}):
            _scrivi_csv(dest / f"adap_{f}.csv", col, [r for r in righe if r[0] == f], ("id",))


def _leggimi(shas: dict[str, str], java: list[str], xml_a: str, xml_b: str) -> str:
    file = "".join(f"- `{n}`: `{h}`\n" for n, h in shas.items())
    return f"""# Gold data of mzmine 3.9.0 per stage (synthetic files)

Output of mzmine 3.9.0 on four small synthetic mzML files of ours (all numbers invented, fixed seed), one CSV per stage. They are the reference of the parity tests of the stages
rewritten in Rust (`crates/mzlab-feat/src/untargeted/`); the mzML files are not in the repository: `tools/bench/oro_mzmine.py --genera DIR` writes them again, byte for byte.

- **Written on**: {date.today().isoformat()} by `python3 tools/bench/oro_mzmine.py --oro` (it checks first that every parameter name of the batch exists in the parameter classes of the module: mzmine ignores the others without a word).
- **mzmine**: 3.9.0, bioconda package `mzmine-3.9.0-hdfd78af_0.tar.bz2`, sha256 `{corri_mzmine.BIOCONDA_SHA256}`; license MIT, (c) 2004-2022 The mzmine Development Team (`https://github.com/mzmine/mzmine/blob/master/LICENSE.txt`).
- **JDK**: {java[0]}{' / ' + java[1] if len(java) > 1 else ''} (Adoptium OpenJDK 20, `{corri_mzmine.JDK20_URL}`).
- **Command**: `bin/MZmine -b fasi.xml -t TMP` and `bin/MZmine -b isotopi.xml -t TMP` (no interface), parameters of `tools/bench/parametri/mzmine/predefinita.json`.
- **Input files** (sha256 of the mzML written by `oro_mzmine.py`):

{file}
## Content of the files
Four files, 0.2-6 min, one scan every 0.01 min, 60 000 resolving power invented, 20 noise points under 1e4 per scan. Components (m/z, RT of the first file, height of the first file): isolated Gaussians 200.1000 at 1.00 (5e5) and 250.2000 at 1.50 (2e5); two peaks that touch at 300.1500 (RT 2.40 and 2.58, valley 40 % of the apex) and 350.1700 (RT 2.40 and 2.545, valley 70 %); 400.2000 at 3.50 with a hole of one scan (zero) at the apex; 450.3000, a trace present in the whole run (6e4 with a ripple) with a peak at 4.50 (5e5); an isotope pair of charge 1 at 500.2500 (RT 5.00; +1.003355 at 30 %, +2.00671 at 5 %) and one of charge 2 at 600.3000 (RT 5.40; +0.50168 at 60 %, +1.003355 at 20 %). `oro_2` drifts by +0.05 min and `oro_3` by -0.05 min (join); in `oro_3` the peak 250.2000 has height 2.5e4, under the minimum height of the ADAP step (3e4) and so missing (gap filling). Intensities of the files: x1.0, x0.8, x1.2, x0.9.

## Files
- `adap.csv`: chromatograms of the ADAP builder, per file: `file, id, mz, rt, rt_start, rt_end, height` (RT in min; mzmine exports the range of the scans kept, not the intensity of every scan).
- `resolved.csv`: features of the local-minimum resolver, per file: `file, id, mz, rt, rt_start, rt_end, height, area` (area = intensity x min).
- `aligned.csv`: rows of the join aligner in long form, one line per cell with a feature: `row, mz, rt, aligned_features, score, file, mz_file, rt_file, height, area, state`.
- `gapfilled.csv`: the same table after the peak finder; `state` DETECTED or ESTIMATED (the filled cells).
- `isotopes.csv`: per file, the result of the isotope grouper on the resolved features (second batch, no alignment): `file, id, mz, rt, height, charge, role, principal`; `role` = principal, alone or isotopologue (a row that the grouper removes; `principal` is the id of the row it belongs to, found by the mass step n x 1.003355 / z).

## Batch of the stages (`fasi.xml`)
```xml
{xml_a}```

## Batch of the isotopes (`isotopi.xml`)
```xml
{xml_b}```
"""


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--genera", metavar="DIR")
    ap.add_argument("--oro", nargs="?", const=str(DEST), metavar="DIR")
    ap.add_argument("--reale", nargs=2, metavar=("MZML_DIR", "DEST"), help="ADAP lists of mzmine on the first mzML of a folder (for the real-data test)")
    a = ap.parse_args()
    if a.genera:
        for n, h in genera(Path(a.genera)).items():
            print(h, n)
    if a.oro:
        scrivi_oro(Path(a.oro))
    if a.reale:
        scrivi_reali(Path(a.reale[0]), Path(a.reale[1]))


if __name__ == "__main__":
    main()
