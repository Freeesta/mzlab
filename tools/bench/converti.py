"""Conversion of Thermo `.raw` files to centroid mzML with ThermoRawFileParser (Apache-2.0, Linux), once, for every tool of the benchmark.

All tools receive the SAME mzML (peak picking by the vendor library, indexed mzML); the SHA-256 of every input and output goes into a small JSON record
(`converti.json` in the output directory) that the private repository keeps next to the manifest. The converter is found with `--parser`, the environment
variable `THERMORAWFILEPARSER` or `ThermoRawFileParser` on PATH; its version is read from `--version` and written in the record.

    python3 tools/bench/converti.py RAW_DIR MZML_DIR [--parser PATH] [--solo SA1.raw ...] [--elimina-raw]
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

try:
    from bench.scarica import sha256_file
except ImportError:                                    # run as a script
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from bench.scarica import sha256_file

FORMATO_MZML_INDICIZZATO = "2"                          # ThermoRawFileParser -f: 0 MGF, 1 mzML, 2 indexed mzML, 3 Parquet
SCHEMA = "bench/conversione/1"


def trova_parser(esplicito: str | None = None) -> str:
    p = esplicito or os.environ.get("THERMORAWFILEPARSER") or shutil.which("ThermoRawFileParser") or shutil.which("thermorawfileparser")
    if not p:
        raise SystemExit("ThermoRawFileParser not found: pass --parser or set THERMORAWFILEPARSER")
    return p


def versione(parser: str) -> str:
    r = subprocess.run([parser, "--version"], capture_output=True, text=True, env=_env(), timeout=120)
    return (r.stdout or r.stderr).strip().splitlines()[-1] if (r.stdout or r.stderr).strip() else "unknown"


def _env() -> dict:
    e = dict(os.environ)
    e.setdefault("DOTNET_SYSTEM_GLOBALIZATION_INVARIANT", "1")   # no ICU needed: the converter does not format locale-dependent text
    return e


def converti(parser: str, raw: Path, uscita: Path) -> Path:
    """Convert one `.raw` into `uscita/<stem>.mzML` (peak picking on, no compression); return the mzML path."""
    uscita.mkdir(parents=True, exist_ok=True)
    dest = uscita / (raw.stem + ".mzML")
    dest.unlink(missing_ok=True)
    r = subprocess.run([parser, "-i", str(raw), "-o", str(uscita), "-f", FORMATO_MZML_INDICIZZATO], capture_output=True, text=True, env=_env())
    if r.returncode != 0 or not dest.exists() or dest.stat().st_size == 0:
        raise SystemExit(f"conversion of {raw.name} failed (exit {r.returncode}):\n{(r.stderr or r.stdout)[-800:]}")
    return dest


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("raw_dir")
    ap.add_argument("mzml_dir")
    ap.add_argument("--parser")
    ap.add_argument("--solo", nargs="*", help="only these .raw names")
    ap.add_argument("--elimina-raw", action="store_true", help="delete each .raw after its mzML is written (disk)")
    a = ap.parse_args(argv)
    parser = trova_parser(a.parser)
    raws = sorted(p for p in Path(a.raw_dir).glob("*.raw") if not a.solo or p.name in a.solo)
    if not raws:
        raise SystemExit("no .raw files to convert")
    out = Path(a.mzml_dir)
    out.mkdir(parents=True, exist_ok=True)
    rec_path = out / "converti.json"
    rec = json.loads(rec_path.read_text()) if rec_path.exists() else {"schema": SCHEMA, "file": {}}
    rec["convertitore"] = {"nome": "ThermoRawFileParser", "versione": versione(parser), "opzioni": f"-f {FORMATO_MZML_INDICIZZATO} (indexed mzML, peak picking on)"}
    for raw in raws:
        print(f"{raw.name} ...", flush=True)
        sha_raw = sha256_file(raw)
        mz = converti(parser, raw, out)
        rec["file"][raw.name] = {"raw_sha256": sha_raw, "raw_byte": raw.stat().st_size, "mzml": mz.name, "mzml_sha256": sha256_file(mz), "mzml_byte": mz.stat().st_size}
        rec_path.write_text(json.dumps(rec, indent=1, sort_keys=True) + "\n")
        print(f"  ok {mz.stat().st_size} B", flush=True)
        if a.elimina_raw:
            raw.unlink()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
