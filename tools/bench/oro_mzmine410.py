"""Gold data of mzmine 4.10 per stage on the four synthetic mzML of `oro_mzmine.py` -> `tests/data/untargeted/mzmine410/`.
Same files, same stages and same CSV formats as the 3.9.0 set (`tests/data/untargeted/mzmine39/`), plus the output of the reworked isotope finder. Usage:
    python3 tools/bench/oro_mzmine410.py --oro [DIR]       run mzmine 4.10 and write the CSV and the LEGGIMI (DIR default: the folder above)
    python3 tools/bench/oro_mzmine410.py --oro DIR --versione 4.10.6    the same with another release of the table `VERSIONI`
The mzmine package (portable zip, own JDK) is downloaded once into `~/.cache/mzlab/mzmine<version>` (or `MZMINE410_DIR`) and its sha256 is checked."""
from __future__ import annotations

import argparse
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from datetime import date
from pathlib import Path

RADICE = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RADICE / "tools"), str(RADICE / "tools" / "bench")]
import oro_mzmine as oro  # noqa: E402
from bench import corri, corri_mzmine  # noqa: E402

DEST = RADICE / "tests" / "data" / "untargeted" / "mzmine410"
VERSIONE = "4.10.90"
# portable Linux package of the release `v<version>` of github.com/mzmine/mzmine and its sha256
VERSIONI = {"4.10.90": "15b616c9ce60edaa8c7b8ae8be10c44d49a784542efe68838bfab721b6e8c7ca",
            "4.10.6": "6ac918014058455501368905312c578c5736bfc634cbc0a0b40d5074d0c8ce40"}
URL = "https://github.com/mzmine/mzmine/releases/download/v{v}/mzmine_Linux_portable-{v}.zip"
M = corri_mzmine.M
FL = corri_mzmine.FL
# `isotopes.csv` is the grouper (the module already ported to Rust), `isotopes_finder.csv` the reworked finder
FINDER = M + "dataprocessing.filter_isotopefinder.IsotopeFinderModule"
GROUPER = M + "dataprocessing.filter_isotopegrouper.IsotopeGrouperModule"


def installa(versione: str) -> Path:
    """The folder of the mzmine package (with `bin/mzmine` and its own JDK); downloads and checks the zip when it is not there."""
    casa = Path(os.environ.get("MZMINE410_DIR") or Path.home() / ".cache" / "mzlab" / f"mzmine{versione}")
    if not (casa / "bin" / "mzmine").exists():
        casa.mkdir(parents=True, exist_ok=True)
        zp = casa / "mzmine.zip"
        corri_mzmine._scarica(URL.format(v=versione), zp, VERSIONI[versione])
        with zipfile.ZipFile(zp) as z:
            z.extractall(casa)
        zp.unlink()
        for p in [casa / "bin" / "mzmine", *(casa / "lib" / "runtime" / "bin").glob("*")]:
            p.chmod(0o755)
    return casa


def _finder_passo(fasi: Path) -> str:
    tol = ('<parameter name="m/z tolerance (feature-to-scan)"><absolutetolerance>0.005</absolutetolerance><ppmtolerance>15.0</ppmtolerance></parameter>'
           '<parameter name="Require 13C isotope peak">true</parameter><parameter name="Maximum charge of isotope m/z">3</parameter>')
    return (f'<batchstep method="{FINDER}" parameter_version="2">{FL}<parameter name="Algorithm" selected_item="automatic"><module name="automatic">{tol}</module></parameter></batchstep>\n')


# 4.10 parameters of type ModuleOptionsEnumComboParameter: the selected option is the attribute `selected_item` (the 3.9 ModuleComboParameter wrote `selected`; mzmine 4.10
# ignores it WITHOUT A WORD and takes the default, for the mass detector the AUTO one whose noise level is far below the one of the batch)
MODULO_ENUM = ("Mass detector", "Algorithm")


def controlla_selected(xml: str) -> None:
    """Refuses a batch in which a parameter of type ModuleOptionsEnumComboParameter names its option with `selected` instead of `selected_item`, or is missing it."""
    for nome in MODULO_ENUM:
        for m in re.finditer(r'<parameter name="' + re.escape(nome) + r'"([^>]*)>', xml):
            if "selected_item=" not in m.group(1) or " selected=" in m.group(1):
                raise SystemExit(f"parameter {nome!r}: it must use selected_item=..., found{m.group(1)!r}")


def controlla_filtro(xml: str) -> None:
    """Refuses a batch whose mass detection or ADAP step lacks the 4.10 scan filter: the parameter is `Scan filters` with `selected="true"` (the 3.9 name `Scans` is ignored WITHOUT A WORD
    and the step then runs on every scan, MS2 included, which on interleaved MS1/MS2 files breaks the consecutive-scan rule)."""
    for m in re.finditer(r'<batchstep method="[^"]*\.(MassDetectionModule|ModularADAPChromatogramBuilderModule)".*?</batchstep>', xml, re.S):
        if '<parameter name="Scan filters" selected="true">' not in m.group(0) or '<parameter name="Scans">' in m.group(0):
            raise SystemExit(f"step {m.group(1)}: the scan filter must be <parameter name=\"Scan filters\" selected=\"true\">")


def batch_xml(files: list[Path], fasi: Path, par: dict, versione: str, finder: bool = False, isotopi: bool = False) -> str:
    """The batch of `corri_mzmine.batch_xml` for the 4.10 file format; `finder`: the isotope grouper step is replaced by the reworked finder (it removes no row)."""
    x = corri_mzmine.batch_xml(files, fasi / "finale.csv", par, fasi=fasi, isotopi=isotopi or finder)
    x = x.replace('mzmine_version="3.9.0"', f'mzmine_version="{versione}"')
    # the centroid detector of 4.10 has no `Detect isotope signals below noise level` any more
    x = x.replace('<parameter name="Detect isotope signals below noise level" selected="false"/>', "")
    x = x.replace('<parameter name="Mass detector" selected="Centroid">', '<parameter name="Mass detector" selected_item="Centroid">')
    if finder:
        x = re.sub(r'<batchstep method="' + re.escape(GROUPER) + r'".*?</batchstep>\n', lambda _m: _finder_passo(fasi), x, flags=re.S)
        x = x.replace("/isotopi_{}.csv", "/finder_{}.csv")
    x = x.replace('<parameter name="Scans">', '<parameter name="Scan filters" selected="true">')
    controlla_selected(x)
    controlla_filtro(x)
    return x


def senza_cromatogrammi(cartella: Path) -> dict[str, str]:
    """The parser of 4.10 fails on the TIC chromatogram of the files (its time array comes out empty, `ConversionUtils.removeDuplicateRtDataPoints`; 3.9 read it): drops the `chromatogramList` in place
    and returns the sha256 of the files as mzmine reads them. No stage uses it: the scans and their arrays are the ones of the 3.9.0 set."""
    out = {}
    for f in sorted(cartella.glob("*.mzML")):
        f.write_text(re.sub(r"<chromatogramList.*?</chromatogramList>", "", f.read_text(encoding="utf-8"), flags=re.S), encoding="utf-8", newline="\n")
        out[f.name] = oro.sha_contenuto(f.read_bytes())
    return out


def esegui(xml: Path, mm: Path, tmp: Path) -> subprocess.CompletedProcess:
    """`bin/mzmine -b xml -t tmp` (the package carries its JDK); the log goes next to the batch as `<xml>.log`."""
    # mzmine 4 refuses every batch without a logged-in user: `MZMINE_USER_FILE` is the .mzuser file of a (free) account; the script runs on a machine where the user is logged in, not in the CI
    utente = ["-u", os.environ["MZMINE_USER_FILE"]] if os.environ.get("MZMINE_USER_FILE") else []
    r = subprocess.run([str(mm / "bin" / "mzmine"), "-b", str(xml), "-t", str(tmp), *utente], capture_output=True, text=True)
    xml.with_suffix(".log").write_text(r.stdout + r.stderr, encoding="utf-8")
    return r


ISO = 1.003355      # mass difference of 13C - 12C


def _fase_isotopi(fasi: Path) -> tuple[list[str], list[list]]:
    """Per file: the rows that the grouper keeps and the ones it removes. The 4.10 export has no charge column, so the charge is derived: a removed row is an isotopologue
    of the closest kept row within 0.1 min whose m/z is lower by 1 to 4 times 1.003355/z (z in 1..3, tolerance 0.01); the charge of a kept row is round(1.003355 / spacing of its nearest removed row) and every other removed row is a whole multiple of 1.003355/z.
    Kept rows with removed rows are principal (charge z), the others alone (charge 0)."""
    col = ["file", "id", "mz", "rt", "height", "charge", "role", "principal"]
    righe = []
    for f, tenute in oro._per_file(fasi, "isotopi"):
        tutte = dict(oro._per_file(fasi, "risolutore"))[f]
        ids = {r["id"] for r in tenute}
        base = {r["id"]: (float(r["mz"]), float(r["rt"])) for r in tenute}
        rimosse, figlie = [], {i: [] for i in ids}
        for r in tutte:
            if r["id"] in ids:
                continue
            mz, rt = float(r["mz"]), float(r["rt"])
            # kept rows that can have this row as the k-th isotopologue (k = 1..4) for some charge z in 1..3; the closest in m/z wins
            trovate = [(mz - m, i) for i, (m, t) in base.items() if m < mz and abs(t - rt) < 0.1
                       and any(abs((mz - m) * z / ISO - round((mz - m) * z / ISO)) < 0.01 and 1 <= round((mz - m) * z / ISO) <= 4 for z in (1, 2, 3))]
            i = min(trovate)[1] if trovate else ""
            if i:
                figlie[i].append(mz - base[i][0])
            rimosse.append([f, r["id"], r["mz"], r["rt"], r["height"], "", "isotopologue", i])
        cariche: dict[str, set[int]] = {}
        for i, ds in figlie.items():
            # the charge follows from the nearest removed row (the first isotopologue, at 1.003355/z); every other row must be a whole multiple of that spacing
            z = round(ISO / min(ds)) if ds else 0
            ok = [z] if ds and 1 <= z <= 3 and all(abs(d * z / ISO - round(d * z / ISO)) < 0.01 and round(d * z / ISO) >= 1 for d in ds) else []
            if ds and not ok:
                raise SystemExit(f"{f}: the removed rows of the kept row {i} fit no charge")
            cariche[i] = {ok[0]} if ds else set()
        for r in tenute:
            zs = cariche[r["id"]]
            z = next(iter(zs)) if zs else 0
            righe.append([f, r["id"], r["mz"], r["rt"], r["height"], z, "principal" if zs else "alone", ""])
        righe += rimosse
        # expected by the design of the synthetic data: the kept 500.25 has z=1 and the kept 600.30 has z=2, with at least one removed row each (the M+2 peak is not always detected)
        for mz0, z0 in ((500.25, 1), (600.30, 2)):
            k = [r for r in righe if r[0] == f and r[6] == "principal" and abs(float(r[2]) - mz0) < 0.01]
            n = [r for r in righe if r[0] == f and r[6] == "isotopologue" and k and r[7] == k[0][1]]
            print(f"check {f}: kept {mz0} -> {[(r[1], r[5]) for r in k]}, removed {len(n)}")
            if len(k) != 1 or k[0][5] != z0 or not n:
                raise SystemExit(f"{f}: the kept {mz0} must have charge {z0} and at least one removed row (got {[(r[1], r[5]) for r in k]}, {len(n)})")
    return col, righe


def _fase_finder(fasi: Path) -> tuple[list[str], list[list]]:
    """Per file: what the finder writes on every resolved feature (charge, number of peaks of the pattern, m/z of the peaks)."""
    col = ["file", "id", "mz", "rt", "height", "charge", "pattern_peaks", "pattern_mz"]
    righe = []
    for f, rs in oro._per_file(fasi, "finder"):
        for r in rs:
            mzs = (r.get("isotope_pattern:mz") or "").split(";") if r.get("isotope_pattern:mz") else []
            # 4.10.90 exports the number of peaks of the pattern as `datafile:<file>.mzML:isotopes` (and not the m/z of the peaks: `pattern_mz` stays empty)
            n = r.get("isotope_pattern:n") or r.get(f"datafile:{f}.mzML:isotopes", "")
            righe.append([f, r["id"], r["mz"], r["rt"], r["height"], r.get("charge", ""), n, " ".join(mzs)])
    return col, righe


def scrivi_oro(dest: Path, versione: str) -> None:
    """Runs mzmine on the four files and writes the CSV of the stages and the LEGGIMI of provenance in `dest`."""
    mm = installa(versione)
    par = corri.parametri("mzmine", "predefinita")
    with tempfile.TemporaryDirectory() as t:
        t = Path(t)
        shas = oro.genera(t / "mzml")
        shas_410 = senza_cromatogrammi(t / "mzml")
        fs = sorted((t / "mzml").glob("*.mzML"))
        fasi = t / "fasi"
        fasi.mkdir()
        xmls = {"fasi.xml": batch_xml(fs, fasi, par, versione), "isotopi.xml": batch_xml(fs, fasi, par, versione, isotopi=True), "finder.xml": batch_xml(fs, fasi, par, versione, finder=True)}
        for n, x in xmls.items():
            (fasi / n).write_text(x, encoding="utf-8")
        ignoti = [i for n in xmls for i in corri_mzmine.parametri_non_letti(fasi / n, mm)]
        if ignoti:
            raise SystemExit("mzmine would ignore these parameters: " + "; ".join(ignoti))
        for n in xmls:
            r = esegui(fasi / n, mm, t / "tmp")
            if r.returncode or "SEVERE" in r.stdout + r.stderr:
                raise SystemExit(f"mzmine failed on {n}:\n{(r.stdout + r.stderr)[-3000:]}")
        dest.mkdir(parents=True, exist_ok=True)
        per_file, allineata = ("file", "id"), ("mz", "rt", "file")
        for nome, (col, righe), chiave in [("adap.csv", oro._fase_per_file(fasi, "adap", False), per_file), ("resolved.csv", oro._fase_per_file(fasi, "risolutore", True), per_file),
                                           ("aligned.csv", oro._fase_allineata(fasi / "join_Aligned_feature_list.csv"), allineata),
                                           ("gapfilled.csv", oro._fase_allineata(fasi / "riempimento_Aligned_feature_list_gap-filled.csv"), allineata),
                                           ("isotopes.csv", _fase_isotopi(fasi), per_file), ("isotopes_finder.csv", _fase_finder(fasi), per_file)]:
            oro._scrivi_csv(dest / nome, col, righe, chiave)
        (dest / "LEGGIMI.md").write_text(_leggimi(versione, shas, shas_410, mm, {n: oro._sostituisci(x, t) for n, x in xmls.items()}), encoding="utf-8")


def _leggimi(versione: str, shas: dict[str, str], shas_410: dict[str, str], mm: Path, xmls: dict[str, str]) -> str:
    java = subprocess.run([str(mm / "lib" / "runtime" / "bin" / "java"), "-version"], capture_output=True, text=True).stderr.strip().splitlines()
    java = [x for x in java if x.startswith(("openjdk version", "OpenJDK Runtime"))]
    plataforma = f"generated on macOS arm64 with mzmine.app {versione} and the Temurin JDK of the app" if sys.platform == "darwin" else "generated from the portable Linux package"
    file = "".join(f"- `{n}`: `{h}` (as read by mzmine: `{shas_410[n]}`)\n" for n, h in shas.items())
    lotti = "".join(f"\n## Batch `{n}`\n```xml\n{x}```\n" for n, x in xmls.items())
    return f"""# Gold data of mzmine {versione} per stage (synthetic files)

Output of mzmine {versione} on the four synthetic mzML files of the 3.9.0 set (`tests/data/untargeted/mzmine39/`, same numbers and same hashes), one CSV per stage, same formats. Reference of the parity tests `crates/mzlab-feat/tests/untargeted_*_410.rs`.
The mzML files are not in the repository: `tools/bench/oro_mzmine.py --genera DIR` writes them again, byte for byte.

- **Written on**: {date.today().isoformat()} by `python3 tools/bench/oro_mzmine410.py --oro` (it checks first that every parameter name of the batch exists in the parameter classes of `mzmine-community-{versione}.jar`: mzmine ignores the others without a word).
- **mzmine**: {versione}, `{URL.format(v=versione)}`, sha256 `{VERSIONI[versione]}`; license MIT, (c) 2004-2022 The mzmine Development Team (`https://github.com/mzmine/mzmine/blob/master/LICENSE.txt`, branch master checked).
- **JDK**: the one inside the package (`lib/runtime`): {java[0]}{' / ' + java[1] if len(java) > 1 else ''}.
- **Platform**: {plataforma}
- **Charge in `isotopes.csv`**: the 4.10 export of the grouper has no charge column, so `role`, `charge` and `principal` are derived (`_fase_isotopi`): a removed row is an isotopologue of the kept row within 0.1 min whose m/z is lower by 1 to 4 times 1.003355/z (z in 1..3, tolerance 0.01), the charge of a kept row is round(1.003355 / spacing of its nearest removed row) and every other removed row is a whole multiple of 1.003355/z; kept rows with removed rows are principal (charge z), the others alone (charge 0). `isotopes_finder.csv` is written as exported: `pattern_peaks` is the column `isotopes` of the finder export (number of peaks of the pattern, empty when no pattern was found), `pattern_mz` is empty because 4.10.90 does not export the m/z of the peaks, and an empty `charge` means that the finder found no charge.
- **Command**: `bin/mzmine -b fasi.xml -t TMP`, `bin/mzmine -b isotopi.xml -t TMP`, `bin/mzmine -b finder.xml -t TMP` (no interface), parameters of `tools/bench/parametri/mzmine/predefinita.json`.
- **Input files** (sha256 of the mzML written by `oro_mzmine.py`, equal to the ones of the 3.9.0 set, with every array decoded and rounded to 6 significant digits; mzmine {versione} reads them after `senza_cromatogrammi`, which drops the TIC chromatogram: its parser fails on it, no stage uses it:

{file}
## Files
`adap.csv`, `resolved.csv`, `aligned.csv`, `gapfilled.csv`, `isotopes.csv` have the columns of the 3.9.0 set (see its LEGGIMI); `isotopes.csv` is the output of the isotope grouper («13C isotope filter»).
{lotti}"""


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--oro", nargs="?", const=str(DEST), metavar="DIR", required=True)
    ap.add_argument("--versione", default=VERSIONE, choices=sorted(VERSIONI))
    a = ap.parse_args()
    scrivi_oro(Path(a.oro), a.versione)


if __name__ == "__main__":
    main()
