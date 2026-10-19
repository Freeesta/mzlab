"""Ground truth `verita/1` of a Thermo `.raw` from ThermoRawFileParser: its metadata JSON (`-m 0`) and its mzML (`-f 2`), run on the Mac or on Windows.
A tool of the test bench (docs/studi/ROSINA.md F.1). The JSON is flattened to `name -> value` without assuming its layout, so a new TRFP version does not break it:
    python3 tools/rosina/verita_trfp.py FILE.mzML FILE-metadata.json [--trfp-versione V] [--nativo FILE.raw] > FILE.verita.json"""
import argparse
import json
import sys
from pathlib import Path

import importlib.util

# the sibling `verita.py`, by path: tools/bench has a module of the same name
_spec = importlib.util.spec_from_file_location("rosina_verita", Path(__file__).resolve().parent / "verita.py")
verita = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(verita)


def appiattisci(x, prefisso="") -> dict:
    """Scalars of a JSON document by dotted path; an object of {Name, Value} pairs is keyed by its Name."""
    out = {}
    if isinstance(x, dict):
        if set(x) >= {"Name", "Value"} and not isinstance(x["Value"], (dict, list)):
            out[f"{prefisso}{x['Name']}"] = x["Value"]
            return out
        for k, v in x.items():
            out.update(appiattisci(v, f"{prefisso}{k}."))
    elif isinstance(x, list):
        for i, v in enumerate(x):
            if isinstance(v, dict) and set(v) >= {"Name", "Value"}:
                out.update(appiattisci(v, prefisso))
            else:
                out.update(appiattisci(v, f"{prefisso}{i}."))
    elif x is not None:
        out[prefisso.rstrip(".")] = x
    return out


def da_trfp(mzml: Path, meta: Path, versione: str, nativo: Path | None) -> dict:
    d = verita.da_mzml(mzml, ("ThermoRawFileParser", versione, "-f 2 -m 0"), nativo, None)
    for nome, v in appiattisci(json.loads(meta.read_text(encoding="utf-8-sig"))).items():
        d["metadati"][f"trfp:{nome}"] = {"valore": v if isinstance(v, (str, int, float, bool)) else str(v), "produttore": "ThermoRawFileParser"}
    return d


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("mzml", type=Path)
    ap.add_argument("metadati", type=Path, help="the JSON of `-m 0`")
    ap.add_argument("--trfp-versione", default="?")
    ap.add_argument("--nativo", type=Path)
    a = ap.parse_args(argv)
    json.dump(da_trfp(a.mzml, a.metadati, a.trfp_versione, a.nativo), sys.stdout, indent=1, ensure_ascii=False)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
