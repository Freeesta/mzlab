"""patRoon (R, GPL-3) run as an external program inside a Docker image pinned by digest (`versioni.PATROON`): `corri_patroon.R` on the mzML (the folder mounted
read-only) -> CSV -> `bench/1`. Optional `--analisi` (analysis,group,blank) and `--sospetti` (name,mz,rt) for blanks and suspect screening. Without Docker it SKIPS
(exit 0, clear message). See `corri.py`. Config = `parametri/patroon/<config>.json` (`openms`, `xcms`)."""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import adatta, corri, versioni  # noqa: E402

R = Path(__file__).with_name("corri_patroon.R")


def immagine() -> str | None:
    """The image reference with its digest, or None when no digest is pinned."""
    p = versioni.PATROON
    return f"{p['nome']}@{p['digest']}" if p["digest"] else None


def comando(img: str, mzml_dir: Path, nat: Path, config: Path, analisi: Path | None, sospetti: Path | None, stems: list[str]) -> list[str]:
    """The `docker run` line: mzML folder and inputs read-only, only the native output folder writable."""
    c = ["docker", "run", "--rm", "-v", f"{mzml_dir.resolve()}:/mzml:ro", "-v", f"{nat.resolve()}:/out", "-v", f"{R.resolve()}:/bench/run.R:ro",
         "-v", f"{config.resolve()}:/bench/par.json:ro"]
    for nome, p in (("analisi", analisi), ("sospetti", sospetti)):
        if p:
            c += ["-v", f"{p.resolve()}:/bench/{nome}.csv:ro"]
    return c + [img, "Rscript", "/bench/run.R", "/bench/par.json", "/out", "/bench/analisi.csv" if analisi else "-", "/bench/sospetti.csv" if sospetti else "-",
                *[f"/mzml/{s}.mzML" for s in stems]]


def main() -> None:
    def extra(ap):
        ap.add_argument("--analisi", help="CSV analysis,group,blank (blank groups are subtracted, x5)")
        ap.add_argument("--sospetti", help="CSV name,mz,rt[,SMILES]: the same list given to mzLab screen.suspects")
        ap.add_argument("--pull", action="store_true", help="pull the pinned image first")
    a = corri.argomenti(__doc__, extra)
    if not shutil.which("docker"):
        print("SKIP: Docker is not available; patRoon runs only in its Docker image")
        return
    img = immagine()
    if not img:
        raise SystemExit("patRoon image not pinned: write the digest in versioni.PATROON first")
    fs = corri.file_mzml(a.mzml_dir, a.solo)
    nat = corri.cartella_nativa(a.out_dir, "patroon", a.config)
    for n in ("featureGroups.csv", "suspects.csv"):
        (nat / n).unlink(missing_ok=True)
    if a.pull:
        subprocess.run(["docker", "pull", img], check=True)
    an, so = (Path(x) if x else None for x in (a.analisi, a.sospetti))
    r = subprocess.run(comando(img, Path(a.mzml_dir), nat, corri.PARAMETRI / "patroon" / f"{a.config}.json", an, so, [f.stem for f in fs]), capture_output=True, text=True)
    (nat / "patroon.log").write_text(r.stdout + r.stderr, encoding="utf-8")
    fg = nat / "featureGroups.csv"
    if r.returncode != 0 or not fg.exists():
        raise SystemExit(f"patRoon failed (exit {r.returncode}):\n{r.stderr[-2000:]}")
    versione = next((l.split()[1] for l in r.stdout.splitlines() if l.startswith("patRoon ")), "unknown")
    sosp = nat / "suspects.csv"
    t = adatta.da_csv("patroon", fg, [f.stem for f in fs], a.dataset, a.config, sospetti=sosp if so and sosp.exists() else None)
    print(corri.salva(t, a.out_dir, f"{versione} ({img})"), len(t["righe"]), "rows")


if __name__ == "__main__":
    main()
