"""Spectral study (docs/agenti/studio-spettri.md): runs the `studio_spettri` example of mzlab-lib in release mode for 3 seeds per scenario and
prints one `MISURA studio_<scenario>: ...` line per arm (mean over the seeds, min-max in brackets for Recall@1 and AUPRC@FDR<=1 %). The arm
set is `base` (the baselines) or `null` (arm A: the decoy kinds and the +1-corrected q, `MISURA studio_null_<scenario>`).

    python3 tools/studio_spettri.py [--bracci base|null] [--scenari e0,e1,...] [--n-lib 20000] [--n-query 2000] [--semi 1,2,3] [--json OUT]
"""
import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCENARI = ["e0", "e1", "e2", "e3", "e4", "e5", "e6"]
METRICHE = ["recall1", "mrr", "pauprc", "ece", "fdp_q01", "recall_q01", "gate", "hit", "score", "q_true", "q_trap", "q_auc"]


def esegui(scenario, n_lib, n_query, seme, bracci):
    cmd = ["cargo", "run", "--release", "-q", "-p", "mzlab-lib", "--example", "studio_spettri", "--",
           scenario, str(n_lib), str(n_query), str(seme), bracci]
    out = subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True, text=True).stdout
    return [json.loads(r) for r in out.splitlines() if r.strip()]


def media(v):
    v = [x for x in v if x is not None]
    return sum(v) / len(v) if v else None


def riga(nome, arm, righe):
    a = [r["all"] for r in righe]
    pezzi = []
    for m in METRICHE:
        x = media([r[m] for r in a])
        if x is None:
            continue
        testo = f"{m} {x:.4f}"
        if m in ("recall1", "pauprc"):
            testo += f" [{min(r[m] for r in a):.4f}-{max(r[m] for r in a):.4f}]"
        pezzi.append(testo)
    ms = media([r["ms"] for r in righe])
    det = all(r["det"] for r in righe)
    return f"MISURA studio_{nome}: arm={arm} n_lib={righe[0]['n_lib']} n={a[0]['n']} semi={len(righe)} " + ", ".join(pezzi) + f", ms {ms:.0f}, det {det}"


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--bracci", default="base", choices=["base", "null"])
    p.add_argument("--scenari", default=None, help="default: all the scenarios for base, e6,e1,e3 for null")
    p.add_argument("--n-lib", type=int, default=20000)
    p.add_argument("--n-query", type=int, default=2000)
    p.add_argument("--semi", default="1,2,3")
    p.add_argument("--json", help="also writes every JSON line of the example to this file")
    a = p.parse_args()
    tutte = []
    scenari = a.scenari or (",".join(SCENARI) if a.bracci == "base" else "e6,e1,e3")
    for sc in scenari.split(","):
        per_arm = {}
        for seme in [int(s) for s in a.semi.split(",")]:
            for r in esegui(sc, a.n_lib, a.n_query, seme, a.bracci):
                per_arm.setdefault(r["arm"], []).append(r)
                tutte.append(r)
        for arm, righe in per_arm.items():
            print(riga(sc if a.bracci == "base" else f"null_{sc}", arm, righe), flush=True)
    if a.json:
        Path(a.json).write_text("".join(json.dumps(r) + "\n" for r in tutte))
    return 0


if __name__ == "__main__":
    sys.exit(main())
