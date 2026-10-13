"""asari (command line): `asari process` on the mzML directory -> `full_Feature_table.tsv` -> `bench/1`. See `corri.py`."""
from __future__ import annotations

import shutil
import subprocess
import sys
from importlib import metadata
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import adatta, corri  # noqa: E402


def main() -> None:
    a = corri.argomenti(__doc__)
    par = corri.parametri("asari", a.config)
    fs = corri.file_mzml(a.mzml_dir, a.solo)
    nat = corri.cartella_nativa(a.out_dir, "asari", a.config)
    entrata = nat / "input"                                     # asari reads a directory: link the chosen files
    shutil.rmtree(entrata, ignore_errors=True)
    entrata.mkdir()
    for f in fs:
        (entrata / f.name).symlink_to(f.resolve())
    for vecchia in nat.glob("uscita*"):
        shutil.rmtree(vecchia, ignore_errors=True)
    uscita = nat / "uscita"                                     # asari writes "<output>_asari_project_<time>" next to it
    exe = shutil.which("asari") or Path(sys.executable).parent / "asari"
    cmd = [str(exe), "process", "-m", par["mode"], "-i", str(entrata), "-o", str(uscita), "--ppm", str(par["mz_tolerance_ppm"]),
           "--min_peak_height", str(par["min_peak_height"]), "--cal_min_peak_height", str(par["cal_min_peak_height"])]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit(f"asari failed:\n{(r.stderr or r.stdout)[-1500:]}")
    tabelle = sorted(nat.glob("uscita*/**/full_Feature_table.tsv"))
    if not tabelle:
        raise SystemExit("asari wrote no full_Feature_table.tsv")
    t = adatta.da_csv("asari", tabelle[0], [f.stem for f in fs], a.dataset, a.config, sep="\t")
    print(corri.salva(t, a.out_dir, metadata.version("asari-metabolomics")), len(t["righe"]), "rows")


if __name__ == "__main__":
    main()
