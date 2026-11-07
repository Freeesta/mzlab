"""Converts the files of the instruments to mzML, the only format the program reads (no reader of proprietary formats is built in).

    python tools/converti.py CARTELLA [-o USCITA] [--trfp PERCORSO] [--prova]

* .raw (Thermo): converted here with ThermoRawFileParser (centroids of the manufacturer, indexed mzML, zlib). The program is looked for in --trfp, in
  the variable THERMORAWFILEPARSER, in the PATH (ThermoRawFileParser, ThermoRawFileParser.exe; the .exe runs with mono outside Windows).
* .wiff (Sciex): the SCIEX reader only exists for Windows, so the command of MSConvert in Docker is printed (to run it by hand).
* .d (Agilent, Bruker, Waters): the MSConvert command is printed.

--prova prints what would be run without running anything. The files already converted (same name .mzML in the output folder) are skipped.
"""
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

PWIZ = "chambm/pwiz-skyline-i-agree-to-the-vendor-licenses"       # MSConvert with the readers of the manufacturers, in Docker (it runs under wine)
FILTER = 'peakPicking vendor msLevel=1-'


def trova_trfp(percorso: str | None = None) -> list[str] | None:
    """The command that starts ThermoRawFileParser, or None."""
    for c in (percorso, os.environ.get("THERMORAWFILEPARSER"), shutil.which("ThermoRawFileParser"), shutil.which("ThermoRawFileParser.exe")):
        if not c:
            continue
        p = Path(c)
        if p.is_dir():
            p = next((q for q in (p / "ThermoRawFileParser", p / "ThermoRawFileParser.exe") if q.is_file()), p)
        if not p.is_file():
            continue
        if p.suffix.lower() == ".exe" and os.name != "nt":
            m = shutil.which("mono")
            return [m, str(p)] if m else None
        return [str(p)]
    return None


def command_trfp(trfp: list[str], raw: Path, uscita: Path) -> list[str]:
    """One .raw -> indexed mzML (-f 2), zlib (-z), centroids of the manufacturer (peak picking is on unless -p is given)."""
    return [*trfp, "-i", str(raw), "-o", str(uscita), "-f", "2", "-z"]


def command_docker(cartella: Path, nome: str, uscita: str = "/dati/mzML") -> str:
    """MSConvert in Docker for a .wiff (it needs the .wiff.scan next to it) or a .d folder."""
    return (f'docker run -it --rm -e WINEDEBUG=-all -v "{cartella}":/dati {PWIZ} wine msconvert "/dati/{nome}" --mzML --zlib '
            f'--filter "{FILTER}" -o {uscita}')


def converti(cartella: Path, uscita: Path, trfp_path: str | None = None, prova: bool = False, out=print) -> dict:
    """Walks the folder. Returns {"fatti": [...], "saltati": [...], "da_fare": [comandi stampati], "errori": [...]}"""
    res = {"fatti": [], "saltati": [], "da_fare": [], "errori": []}
    cartella = cartella.resolve(); uscita = uscita.resolve()
    raws = sorted(p for p in cartella.rglob("*") if p.is_file() and p.suffix.lower() == ".raw")
    wiffs = sorted(p for p in cartella.rglob("*") if p.is_file() and p.suffix.lower() in (".wiff", ".wiff2"))
    ds = sorted(p for p in cartella.rglob("*.d") if p.is_dir())
    if not (raws or wiffs or ds):
        out(f"Nessun file .raw, .wiff o .d in {cartella}.")
        return res
    if raws:
        trfp = trova_trfp(trfp_path)
        if trfp is None and not prova:
            out("ThermoRawFileParser non trovato. Scaricalo da https://github.com/CompOmics/ThermoRawFileParser/releases e indicane il percorso con --trfp "
                "(o nella variabile THERMORAWFILEPARSER). Fuori da Windows serve anche mono, oppure la versione per Linux/Mac.")
            res["errori"].append("trfp")
        else:
            uscita.mkdir(parents=True, exist_ok=True) if not prova else None
            for r in raws:
                dest = uscita / (r.stem + ".mzML")
                if dest.exists():
                    out(f"già convertito: {r.name}"); res["saltati"].append(r.name); continue
                cmd = command_trfp(trfp or ["ThermoRawFileParser"], r, uscita)
                if prova:
                    out(" ".join(f'"{c}"' if " " in c else c for c in cmd)); res["da_fare"].append(cmd); continue
                out(f"converto {r.name} …")
                p = subprocess.run(cmd, capture_output=True, text=True)
                if p.returncode == 0 and dest.exists():
                    res["fatti"].append(r.name)
                else:
                    res["errori"].append(r.name); out(f"  non riuscito: {(p.stderr or p.stdout).strip().splitlines()[-1:] or ''}")
    for w in wiffs:
        c = command_docker(w.parent, w.name); out(c); res["da_fare"].append(c)
    for d in ds:
        c = command_docker(d.parent, d.name); out(c); res["da_fare"].append(c)
    if wiffs or ds:
        out("\nI file .wiff / .d si convertono con MSConvert (ProteoWizard): il comando qui sopra usa Docker. Sul .wiff deve stare accanto il suo .wiff.scan. "
            "In alternativa: MSConvert per Windows, Output format mzML, 64-bit, zlib, Numpress spento, filtro «peakPicking vendor msLevel=1-».")
    return res


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("cartella", type=Path, help="cartella con i file dello strumento")
    ap.add_argument("-o", "--uscita", type=Path, help="cartella degli mzML (predefinita: CARTELLA/mzML)")
    ap.add_argument("--trfp", help="percorso di ThermoRawFileParser (o della sua cartella)")
    ap.add_argument("--prova", action="store_true", help="mostra i comandi senza eseguirli")
    a = ap.parse_args(argv)
    if not a.cartella.is_dir():
        print(f"{a.cartella} non è una cartella."); return 2
    r = converti(a.cartella, a.uscita or a.cartella / "mzML", a.trfp, a.prova)
    return 1 if r["errori"] else 0


if __name__ == "__main__":
    sys.exit(main())
