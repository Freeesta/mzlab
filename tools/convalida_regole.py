"""Runs the validation of the fragmentation rules (`docs/studi/regole-frammentazione-piano.md` §3) on public MassBank spectra: the records of the pinned
release of `tools/librerie_pacchetti.py` (SHA-256 checked) with a SMILES and the licence CC0 or CC BY are written to a cache in `.verifica/` (never committed),
`mzlab-cli msn-rules` judges them (decoys included) and `mzlab-cli msn-rules somma` adds the lines up. This script only moves files and starts the
commands: every number is computed in Rust.

    python3 tools/convalida_regole.py                          # downloads the release into .verifica/massbank_regole/, runs, prints the summary
    python3 tools/convalida_regole.py --zip MassBank.zip       # an already downloaded copy
    python3 tools/convalida_regole.py --registri DIR           # records already on disk (the tests use a handful of public ones); no download
"""
import argparse
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import librerie_pacchetti as lp  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / ".verifica" / "massbank_regole"
LICENZE = ("CC0", "CC BY")


def usabile(text: str) -> bool:
    """The record has a SMILES, a licence we may use and is an ESI MS2 (`librerie_pacchetti.keep`)."""
    smiles = lp._field(text, "CH$SMILES:")
    return bool(smiles) and smiles != "N/A" and lp._field(text, "LICENSE:") in LICENZE and lp.keep(text) is not None


def estrai(zpath: Path, dest: Path) -> int:
    """Writes the usable records of the release zip to `dest`, one file each; returns how many."""
    dest.mkdir(parents=True, exist_ok=True)
    n = 0
    with zipfile.ZipFile(zpath) as z:
        for info in z.infolist():
            base = info.filename.rsplit("/", 1)[-1]
            if info.is_dir() or not (base.startswith("MSBNK-") and base.endswith(".txt")):
                continue
            text = z.read(info).decode("utf-8", "replace")
            if usabile(text):
                (dest / base).write_text(text, encoding="utf-8", newline="\n")
                n += 1
    return n


def registri(args) -> Path:
    if args.registri:
        return Path(args.registri)
    marca = CACHE / "registri" / ".pronto"
    if not marca.exists():
        zpath = Path(args.zip) if args.zip else CACHE / "MassBank.zip"
        if not zpath.exists():
            CACHE.mkdir(parents=True, exist_ok=True)
            print(f"downloading MassBank-data {lp.VERSION} (Zenodo {lp.ZENODO_RECORD})", file=sys.stderr)
            lp.download(lp.ZIP_URL, zpath, lp.ZIP_SHA256)
        print(f"{estrai(zpath, CACHE / 'registri')} usable records", file=sys.stderr)
        marca.write_text("ok\n")
    return CACHE / "registri"


def cli(args) -> list[str]:
    if args.cli:
        return [args.cli]
    return ["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--"]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--zip", help="an already downloaded MassBank-data zip")
    ap.add_argument("--registri", help="a folder of MassBank records instead of the release")
    ap.add_argument("--seme", default="1", help="seed of the decoys (1)")
    ap.add_argument("--cli", help="the mzlab-cli binary (default: cargo run --release)")
    ap.add_argument("--out", help="write the summary there instead of the standard output")
    args = ap.parse_args()
    cartella = registri(args)
    with tempfile.TemporaryDirectory(prefix="mzlab-regole-") as tmp:
        lista, righe = Path(tmp) / "lista.txt", Path(tmp) / "righe.jsonl"
        lista.write_text("".join(f"{p}\n" for p in sorted(cartella.glob("*.txt"))))
        base = cli(args)
        for comando in (
            [*base, "msn-rules", "--lista", str(lista), "--regole", "--decoy", args.seme, "--out", str(righe)],
        ):
            subprocess.run(comando, cwd=ROOT, check=True)
        somma = subprocess.run([*base, "msn-rules", "somma", str(righe)], cwd=ROOT, check=True, capture_output=True, text=True).stdout
    if args.out:
        Path(args.out).write_text(somma, encoding="utf-8")
    else:
        sys.stdout.write(somma)
    return 0


if __name__ == "__main__":
    sys.exit(main())
