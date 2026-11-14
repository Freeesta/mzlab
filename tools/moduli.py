"""Data of the site's .wasm modules: one tools/moduli/<name>.toml each (output, cargo package, features, wasm-opt set, build part, cap, note, last measure).
The only reader of those files: `--elenco <part>` (names, for the shell scripts), `--tetto <name>`, `--controlla [--base <git ref>]`.
Cap rule (docs/agenti/wasm-tetti.md): a cap rises by at most 10 % per new function and never in a workflow; with --base a cap above 1.10 x its base value fails."""
import argparse
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIR = ROOT / "tools" / "moduli"
PARTI = ("core", "a", "b", "lettori")
OPT = ("@motore", "@estesi")
CAMPI = {"uscita": str, "pacchetto": str, "wasm_opt": str, "parte": str, "tetto": int, "nota": str}


def carica(testi: dict[str, str] | None = None) -> dict[str, dict]:
    """name -> record, in file-name order; `testi` (name -> toml text) replaces the files, for a git base."""
    if testi is None:
        testi = {p.stem: p.read_text(encoding="utf-8") for p in sorted(DIR.glob("*.toml"))}
    return {n: tomllib.loads(t) for n, t in sorted(testi.items())}


def da_base(ref: str) -> dict[str, dict]:
    elenco = subprocess.run(["git", "ls-tree", "--name-only", ref, "tools/moduli/"], cwd=ROOT, capture_output=True, text=True, check=False).stdout.split()
    testi = {Path(f).stem: subprocess.run(["git", "show", f"{ref}:{f}"], cwd=ROOT, capture_output=True, text=True, check=True).stdout for f in elenco if f.endswith(".toml")}
    return carica(testi)


def errori(mod: dict[str, dict]) -> list[str]:
    err = []
    uscite = {}
    for n, r in mod.items():
        for c, tipo in CAMPI.items():
            if not isinstance(r.get(c), tipo) or isinstance(r.get(c), bool):
                err.append(f"{n}: field `{c}` missing or not {tipo.__name__}")
        if r.get("parte") not in PARTI:
            err.append(f"{n}: parte must be one of {PARTI}")
        if r.get("wasm_opt") not in OPT:
            err.append(f"{n}: wasm_opt must be one of {OPT}")
        if isinstance(r.get("tetto"), int) and r["tetto"] <= 0:
            err.append(f"{n}: tetto must be positive")
        if "misura" in r and (not isinstance(r["misura"], int) or r["misura"] > r.get("tetto", 0)):
            err.append(f"{n}: misura {r['misura']} above the cap")
        if "\n" in str(r.get("nota", "")):
            err.append(f"{n}: nota is one line")
        if r.get("uscita") in uscite:
            err.append(f"{n}: uscita {r['uscita']} already used by {uscite[r['uscita']]}")
        uscite[r.get("uscita")] = n
    return err


def cambi(mod: dict[str, dict], base: dict[str, dict]) -> list[str]:
    """Every changed cap, one line each; a line starting with FAIL is above the allowed 1.10 x base."""
    out = []
    for n, r in mod.items():
        if n in base and r["tetto"] != base[n]["tetto"]:
            b = base[n]["tetto"]
            out.append(f"{'FAIL ' if r['tetto'] > b * 1.10 else ''}{n}: {b} -> {r['tetto']} ({(r['tetto'] / b - 1) * 100:+.1f} %)")
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--elenco", choices=PARTI, metavar="PART")
    g.add_argument("--tetto", metavar="NAME")
    g.add_argument("--controlla", action="store_true")
    ap.add_argument("--base", metavar="REF")
    a = ap.parse_args()
    mod = carica()
    if a.elenco:
        print("\n".join(n for n, r in mod.items() if r["parte"] == a.elenco))
    elif a.tetto:
        if a.tetto not in mod:
            print(f"unknown module {a.tetto}", file=sys.stderr)
            return 2
        print(mod[a.tetto]["tetto"])
    else:
        err = errori(mod)
        if a.base:
            c = cambi(mod, da_base(a.base))
            print("\n".join(c) if c else "0 caps changed")
            err += [x for x in c if x.startswith("FAIL")]
        print("\n".join(err) if err else f"{len(mod)} modules, data complete")
        return 1 if err else 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
