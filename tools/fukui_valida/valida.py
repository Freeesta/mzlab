#!/usr/bin/env python3
"""Validation of the condensed Fukui indices of mzlab-chem (reatt/1) on simple textbook cases, against the committed xtb 6.7.1
reference outputs, and on timolol. Reference/tooling only: runs the Rust runner `tools/fukui_valida` (built on demand) and prints
markdown tables. Usage: python3 tools/fukui_valida/valida.py [--seeds 4]. Study: docs/studi/FUKUI_COSA_DICONO.md."""
import argparse, csv, json, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT / "tools/fukui_valida/target/release/fukui_valida"

# name -> (SMILES, preferred heavy atoms for electrophilic substitution, disfavoured ones); heavy index = SMILES order, 0-based.
CASES = {
    "phenol": ("Oc1ccccc1", {2, 4, 6}, {3, 5}),
    "anisole": ("COc1ccccc1", {3, 5, 7}, {4, 6}),
    "aniline": ("Nc1ccccc1", {2, 4, 6}, {3, 5}),
    "toluene": ("Cc1ccccc1", {2, 4, 6}, {3, 5}),
    "chlorobenzene": ("Clc1ccccc1", {2, 4, 6}, {3, 5}),
    "acetanilide": ("CC(=O)Nc1ccccc1", {5, 7, 9}, {6, 8}),
    "nitrobenzene": ("[O-][N+](=O)c1ccccc1", {5, 7}, {4, 6, 8}),
    "benzoic_acid": ("OC(=O)c1ccccc1", {5, 7}, {4, 6, 8}),
    "benzaldehyde": ("O=Cc1ccccc1", {4, 6}, {3, 5, 7}),
    "acetophenone": ("CC(=O)c1ccccc1", {5, 7}, {4, 6, 8}),
    "benzonitrile": ("N#Cc1ccccc1", {4, 6}, {3, 5, 7}),
    "naphthalene": ("c1ccc2ccccc2c1", {2, 4, 7, 9}, {0, 1, 5, 6}),
    "pyrrole": ("c1cc[nH]c1", {2, 4}, {0, 1}),
    "furan": ("c1ccoc1", {2, 4}, {0, 1}),
    "thiophene": ("c1ccsc1", {2, 4}, {0, 1}),
    "indole": ("c1ccc2[nH]ccc2c1", {6}, {5}),
    "pyridine": ("n1ccccc1", {2, 4}, {1, 5, 3}),
}

TIMOLOL = "CC(C)(C)NCC(O)COc1nsnc1N1CCOCC1"
# heavy index -> role in timolol
TIM_ROLES = {0: "CH3 (tBu)", 1: "C quaternary (tBu)", 2: "CH3 (tBu)", 3: "CH3 (tBu)", 4: "NH (tBuNH)", 5: "CH2 alpha to N (chain)",
             6: "CH(OH)", 7: "OH", 8: "CH2 alpha to O (chain)", 9: "O ether", 10: "C thiadiazole (O)", 11: "N thiadiazole",
             12: "S thiadiazole", 13: "N thiadiazole", 14: "C thiadiazole (N)", 15: "N morpholine", 16: "CH2 alpha to N (morph.)",
             17: "CH2 alpha to O (morph.)", 18: "O morpholine", 19: "CH2 alpha to O (morph.)", 20: "CH2 alpha to N (morph.)"}
TIM_MORPH_C = {16, 17, 19, 20}
TIE = 0.005  # same threshold as mzlab-chem fukui.rs


def run(items):
    if not BIN.exists():
        subprocess.run(["cargo", "build", "--release", "--manifest-path", str(ROOT / "tools/fukui_valida/Cargo.toml")], check=True)
    out = subprocess.run([str(BIN)] + [f"{n}|{s}|{o}" for n, s, o in items], capture_output=True, text=True, check=True).stdout
    res = {}
    for line in out.splitlines():
        j = json.loads(line)
        res[j["name"]] = j
    return res


def cls_heavy(site):
    return {int(c[1:]) - 1 for c in site["class"].split(", ")}


def rankcorr(a, b):
    def rk(v):
        o = sorted(range(len(v)), key=lambda i: v[i])
        r = [0.0] * len(v)
        for k, i in enumerate(o):
            r[i] = k
        return r
    ra, rb = rk(a), rk(b)
    n = len(a)
    ma, mb = sum(ra) / n, sum(rb) / n
    num = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    den = (sum((x - ma) ** 2 for x in ra) * sum((y - mb) ** 2 for y in rb)) ** 0.5
    return num / den if den else float("nan")


def coherence(res):
    print("\n## A. Internal coherence (water, default options)\n")
    print("| molecule | atoms | Σq | Σf+ | Σf− | max abs(f0−(f++f−)/2) | f− < 0 on heavy atoms | SCC |")
    print("|---|---|---|---|---|---|---|---|")
    worst = [0, 0, 0]
    for n, j in res.items():
        d = j["reatt"]
        at = d["atoms"]
        sq = sum(a["q"] for a in at)
        sp = sum(a["f_plus"] for a in at)
        sm = sum(a["f_minus"] for a in at)
        z = max(abs(a["f_zero"] - 0.5 * (a["f_plus"] + a["f_minus"])) for a in at)
        neg = sum(1 for a in at if a["heavy_index"] is not None and a["f_minus"] < 0)
        heavy = sum(1 for a in at if a["heavy_index"] is not None)
        worst = [max(worst[0], abs(sq)), max(worst[1], abs(sp - 1)), max(worst[2], abs(sm - 1))]
        print(f"| {n} | {len(at)} | {sq:+.4f} | {sp:.4f} | {sm:.4f} | {z:.4f} | {neg}/{heavy} | {d['scc']['converged']} |")
    print(f"\nWorst deviation over the set: |Σq| {worst[0]:.4f}, |Σf+ − 1| {worst[1]:.4f}, |Σf− − 1| {worst[2]:.4f} (4-decimal rounding of {len(at)} atoms).")


def sear(res, seeds):
    print("\n## B. Textbook electrophilic substitution: first C–H class of f(−) (water, seed 42)\n")
    print("| molecule | expected C | first class (f−) | rank of best expected | margin | verdict | top-1 over seeds |")
    print("|---|---|---|---|---|---|---|")
    t1 = t3 = dec = 0
    for n, (smi, pref, dis) in CASES.items():
        d = res[f"{n}@42"]["reatt"]
        sites = d["ch_sites"]
        rank = next((s["rank"] for s in sites if cls_heavy(s) & pref), None)
        fm = {a["heavy_index"]: a["f_minus"] for a in d["atoms"] if a["heavy_index"] is not None}
        # margin: best expected minus best disfavoured f(-); within TIE it is a tie, not a call
        margin = max(fm[i] for i in pref) - max(fm[i] for i in dis)
        verdict = "right" if margin >= TIE else ("tie" if margin > -TIE else "wrong")
        t1 += rank == 1
        t3 += rank is not None and rank <= 3
        dec += verdict == "right"
        stab = sum(1 for sd in seeds if (cls_heavy(res[f"{n}@{sd}"]["reatt"]["ch_sites"][0]) & pref))
        print(f"| {n} | {sorted(i + 1 for i in pref)} | {sites[0]['class']} ({sites[0]['f_minus']:+.3f}) | {rank} | {margin:+.3f} | {verdict} | {stab}/{len(seeds)} |")
    print(f"\nFirst class is an expected one {t1}/{len(CASES)}, in the first three {t3}/{len(CASES)}; decisive (margin ≥ {TIE}) {dec}/{len(CASES)}. C-numbers are SMILES order, 1-based, as in `ch_sites`.")


def xtb_ref():
    print("\n## C. mzLab (own ETKDG geometry) against the committed xtb 6.7.1 `--vfukui` outputs (water, MMFF geometry)\n")
    print("| molecule | heavy atoms | Spearman f− | Spearman f+ | max abs Δf− | first C–H class equal |")
    print("|---|---|---|---|---|---|")
    ref = ROOT / "tests/data/xtb"
    mols = {}
    for p in sorted(ref.glob("*_fukui.csv")):
        if p.stem.endswith("_gas_fukui"):
            continue
        rows = list(csv.DictReader(open(p)))
        mols[p.stem[:-6]] = (rows[0]["smiles"], {(int(r["atom"]), r["quantity"]): float(r["value"]) for r in rows})
    res = run([(n, s, "") for n, (s, _) in mols.items()])
    eq = tot = 0
    for n, (smi, v) in mols.items():
        d = res[n]["reatt"]
        fm = {a["heavy_index"]: a["f_minus"] for a in d["atoms"] if a["heavy_index"] is not None}
        fp = {a["heavy_index"]: a["f_plus"] for a in d["atoms"] if a["heavy_index"] is not None}
        idx = sorted(fm)
        xm = [v[(i, "f_minus")] for i in idx]
        xp = [v[(i, "f_plus")] for i in idx]
        om = [fm[i] for i in idx]
        op = [fp[i] for i in idx]
        dmax = max(abs(a - b) for a, b in zip(om, xm))
        # first C-H class of the reference: unsaturated C with H, same grouping as mzLab classes
        sites = d["ch_sites"]
        ok = "n/a"
        if sites:
            best = max(sites, key=lambda s: sum(v[(i, "f_minus")] for i in cls_heavy(s)) / len(cls_heavy(s)))
            first = [s for s in sites if s["tie_first"]]
            ok = "yes" if best["class"] in {s["class"] for s in first} or best["rank"] == 1 else "no"
            tot += 1
            eq += ok == "yes"
        print(f"| {n} | {len(idx)} | {rankcorr(om, xm):.2f} | {rankcorr(op, xp):.2f} | {dmax:.3f} | {ok} |")
    print(f"\nFirst class equal (or tied with mzLab's first): {eq}/{tot} molecules with C–H sites.")


def timolol(seeds):
    print("\n## D. Timolol (water; heavy atoms numbered as in the SMILES, 0-based)\n")
    r = run([("tim", TIMOLOL, "")])["tim"]["reatt"]
    at = r["atoms"]
    hs = {}
    for b in r["bonds"]:
        for x, y in ((b["a"], b["b"]), (b["b"], b["a"])):
            if at[x]["heavy_index"] is not None and at[y]["el"] == "H":
                hs.setdefault(at[x]["heavy_index"], []).append(y)
    print("| atom | role | q | f+ | f0 | f− | f− (C+H) | f−/heavy rank |")
    print("|---|---|---|---|---|---|---|---|")
    heavy = [a for a in at if a["heavy_index"] is not None]
    order = sorted(heavy, key=lambda a: -a["f_minus"])
    rk = {a["heavy_index"]: k + 1 for k, a in enumerate(order)}
    for a in heavy:
        h = a["heavy_index"]
        fch = a["f_minus"] + sum(at[j]["f_minus"] for j in hs.get(h, []))
        print(f"| {h} {a['el']} | {TIM_ROLES[h]} | {a['q']:+.3f} | {a['f_plus']:+.3f} | {a['f_zero']:+.3f} | {a['f_minus']:+.3f} | {fch:+.3f} | {rk[h]} |")
    print(f"\nC–H sites of `ch_sites` (unsaturated carbons with H): {len(r['ch_sites'])}. f− spread over the 4 lowest conformers: {r['conformers']['f_minus_spread']}.")
    sp3 = [h for h in TIM_ROLES if TIM_ROLES[h].startswith("CH")]
    print("\nRanking of the sp3 C–H carbons by f−(C+H) over embedding seeds (morpholine carbons are marked *):\n")
    print("| seed | order (highest first) | best morpholine C rank among 10 | abs diff of equivalent pairs C16-C20 / C17-C19 |")
    print("|---|---|---|---|")
    res = run([(f"tim@{s}", TIMOLOL, json.dumps({"seed": s})) for s in seeds])
    for s in seeds:
        d = res[f"tim@{s}"]["reatt"]
        a2 = d["atoms"]
        hs2 = {}
        for b in d["bonds"]:
            for x, y in ((b["a"], b["b"]), (b["b"], b["a"])):
                if a2[x]["heavy_index"] is not None and a2[y]["el"] == "H":
                    hs2.setdefault(a2[x]["heavy_index"], []).append(y)
        v = {h: a2[[a["i"] for a in a2 if a["heavy_index"] == h][0]]["f_minus"] + sum(a2[j]["f_minus"] for j in hs2.get(h, [])) for h in sp3}
        o = sorted(v, key=lambda h: -v[h])
        best = min(o.index(h) + 1 for h in TIM_MORPH_C)
        print(f"| {s} | {' > '.join(('*' if h in TIM_MORPH_C else '') + str(h) for h in o)} | {best} | {abs(v[16] - v[20]):.3f} / {abs(v[17] - v[19]):.3f} |")
    g = run([("tim_gas", TIMOLOL, json.dumps({"solvent": "none"}))])["tim_gas"]["reatt"]
    top = sorted([a for a in g["atoms"] if a["heavy_index"] is not None], key=lambda a: -a["f_minus"])[:4]
    print("\nGas phase, 4 highest f− heavy atoms: " + ", ".join(f"{a['heavy_index']} {a['el']} ({a['f_minus']:+.3f})" for a in top))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seeds", type=int, default=4)
    n = ap.parse_args().seeds
    seeds = [42] + list(range(1, n))
    items = [(f"{k}@{s}", smi, json.dumps({"seed": s})) for k, (smi, _, _) in CASES.items() for s in seeds]
    res = run(items)
    bad = [k for k, v in res.items() if "error" in v]
    if bad:
        sys.exit(f"engine errors: {bad}")
    print("# Fukui validation (mzlab-chem, GFN2-xTB + ALPB water)")
    coherence({k.split("@")[0]: v for k, v in res.items() if k.endswith("@42")})
    sear(res, seeds)
    xtb_ref()
    timolol(seeds)


if __name__ == "__main__":
    main()
