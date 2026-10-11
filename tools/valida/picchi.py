#!/usr/bin/env python3
"""Compare the table of `mzlab-feat` (`feat --analyse --tsv DIR`) with the truth of the synthetic identity series.

    python3 tools/valida/picchi.py DIR_VERITA DIR_TSV

DIR_VERITA = the folder written by `sintetici_identita.py` (verita.json), DIR_TSV = the `<file>.tsv` of the Rust example, one per `SIN_<file>.mzidx`.
Prints, over all the files: purity of the MS2 (true value of tp1 / chimeric rows against the best purity of the peak, share within +-0.05), isotopologues M+1 and
M+2 grouped with their base ion (share), isomers and ions found. Numbers only; the thresholds are in the prompts, not here.
"""
import csv
import json
import sys
from pathlib import Path

PPM, RT_TOL = 5e-6, 0.1


def leggi(p: Path) -> list[dict]:
    with open(p, newline="", encoding="utf-8") as f:
        return [{k: float(v) for k, v in r.items()} for r in csv.DictReader(f, delimiter="\t")]


def trova(feat: list[dict], mz: float, rt: float) -> int | None:
    """Row of the feature at (mz, rt): the nearest in m/z among those within the tolerances, else None."""
    best = None
    for i, r in enumerate(feat):
        if abs(r["mz"] - mz) <= mz * PPM and abs(r["rt"] - rt) <= RT_TOL and (best is None or abs(r["mz"] - mz) < abs(feat[best]["mz"] - mz)):
            best = i
    return best


def main(argv) -> int:
    verita = json.loads((Path(argv[0]) / "verita.json").read_text(encoding="utf-8"))
    ioni = {r["id"]: r for r in verita["ioni"]}
    pur = {"tp1": [0, 0], "chimerica": [0, 0]}
    iso = {1: [0, 0], 2: [0, 0]}
    errori = []
    for f in verita["file"]:
        tsv = Path(argv[1]) / f"{f['nome']}.tsv"
        feat = leggi(tsv)
        for r in verita["ioni"]:
            if not r["rt_obs"] or not r["presente"]:
                continue
            i = trova(feat, r["mz"], r["rt_obs"][0])
            if r["ruolo"] in ("tp1", "chimerica") and r["purezza"] and i is not None and feat[i]["ms2_n"] > 0:
                vero = r["purezza"] if r["ruolo"] == "tp1" else 1 - r["purezza"]      # the truth of a partner is the share of the TARGET
                pur[r["ruolo"]][1] += 1
                pur[r["ruolo"]][0] += abs(feat[i]["purity"] - vero) <= 0.05
                if abs(feat[i]["purity"] - vero) > 0.05:
                    errori.append((f["nome"], r["id"], round(vero, 3), round(feat[i]["purity"], 3)))
            if r["ruolo"] == "isotopo" and i is not None:
                k = int(r["id"][-1])
                base = trova(feat, ioni[r["famiglia"]]["mz"], r["rt_obs"][0])
                if base is None:
                    continue
                iso[k][1] += 1
                iso[k][0] += feat[i]["iso_group"] != 0 and feat[i]["iso_group"] == feat[base]["iso_group"]
    for ruolo, (a, n) in pur.items():
        print(f"purity within 0.05 ({ruolo}; the partner counts its own isotopologues in the window): {a}/{n}")
    for k, (a, n) in iso.items():
        print(f"isotopologues M+{k} grouped with the base: {a}/{n} ({100 * a / max(n, 1):.1f} %)")
    for e in errori[:10]:
        print("  purity off:", e)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
