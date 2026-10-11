# TPMINE-PRIVATE
"""Identities of a feature table (untargeted): the features that are the same compound, with the evidence of every link.

An edge joins two features only if ALL hold: they co-elute (apex within `apex_scans` scans of `cycle_s` seconds), their areas are correlated over
the samples they share (Pearson r >= `r_min`, at least `min_common` samples) and their m/z differ by a relation that is explained (isotope,
adduct, in-source loss, dimer) within the tolerance. The relations come from `iimn.relations` (one table of adducts, isotopes and losses for the
whole of mzFinder); the table of mzMine's own ion identity networking (`ion_identities:*`, FBMN `MS1 annotation` edges) enters as evidence "mzMine
IIN" and only the steps mzMine did not group are computed. Connected components are the identities: the neutral mass that explains most members is
the main hypothesis, the others are kept as alternatives. Nothing here is a verdict: every edge carries dm/z, r, dRT and the shared samples.

Also: pairs of the two polarities (`polarity_pairs`: same neutral mass under the usual adduct pairs and RT within a window).
Pure numpy; input and output are plain JSON-able dicts (the page sends the table, see `29_untarget_identity.js`)."""
from __future__ import annotations

import numpy as np

from mzlab.chem import elements as E

from . import edge_evidence, iimn

PROTON = E.PROTON
# ion forms of a neutral M: (name, m/z shift from M, multiplier of M, is the base ion). The shifts are those of `chem.elements` / `iimn`.
FORMS = {
    1: [("[M+H]+", PROTON, 1, True), ("[M+NH4]+", iimn.ADDUCT_FROM_H["[M+NH4]+"] + PROTON, 1, False), ("[M+Na]+", iimn.ADDUCT_FROM_H["[M+Na]+"] + PROTON, 1, False),
        ("[M+K]+", iimn.ADDUCT_FROM_H["[M+K]+"] + PROTON, 1, False), ("[M+H+ACN]+", iimn.ADDUCT_FROM_H["[M+H+ACN]+"] + PROTON, 1, False), ("[2M+H]+", PROTON, 2, False)],
    -1: [("[M-H]-", -PROTON, 1, True), ("[M+FA-H]-", iimn.ADDUCT_FROM_DEPROT["[M+FA-H]-"] - PROTON, 1, False), ("[M+Cl]-", iimn.ADDUCT_FROM_DEPROT["[M+Cl]-"] - PROTON, 1, False),
         ("[M+Na-2H]-", iimn.ADDUCT_FROM_DEPROT["[M+Na-2H]-"] - PROTON, 1, False), ("[2M-H]-", -PROTON, 2, False)],
}
ADDUCTS = {1: iimn.ADDUCT_FROM_H, -1: iimn.ADDUCT_FROM_DEPROT}
BASE = {1: "[M+H]+", -1: "[M-H]-"}
# usual pairs of the two polarities (positive form, negative form) for the same molecule
PAIR_POS = ["[M+H]+", "[M+Na]+", "[M+NH4]+"]
PAIR_NEG = ["[M-H]-", "[M+FA-H]-", "[M+Cl]-"]
DEFAULTS = {"ppm": 5.0, "abs_da": 0.002, "r_min": 0.85, "apex_scans": 2, "cycle_s": 1.0, "min_common": 3, "pair_ppm": 5.0, "pair_rt_min": 0.15, "max_hyp_members": 12}
IIN_KIND = "mzMine IIN"


def _form_table(pol: int) -> dict:
    return {name: (shift, mult) for name, shift, mult, _ in FORMS[pol]}


def _cross_pairs(pol: int) -> list[tuple[str, float]]:
    """Differences between two adducts that are not the base ion ([M+Na]+ vs [M+NH4]+ ...): (label, shift of the second from the first)."""
    ad = [(n, s) for n, s, m, b in FORMS[pol] if not b and m == 1]
    return [(f"{b} vs {a}", sb - sa) for a, sa in ad for b, sb in ad if a != b]


def shifts(pol: int) -> np.ndarray:
    """Every signed m/z difference a relation can give (the prefilter of the pair search)."""
    ad = list(ADDUCTS[pol].values())
    out = ad + [-v for v in ad] + [d for _, d in _cross_pairs(pol)]
    for table in (iimn.ISOTOPES, iimn.LOSSES):
        out += list(table.values()) + [-v for v in table.values()]
    return np.unique(np.array(out, float))


def pair_relations(delta: float, mz_ref: float, pol: int, ppm: float, abs_da: float) -> list[dict]:
    """What the difference `delta` (= m/z of Y minus m/z of X) can be, as a list of {role, name, label, expected} (expected = the shift, signed)."""
    out = []
    for r in iimn.relations(delta, mz_ref, ppm, abs_da, adducts=ADDUCTS[pol]):
        base = BASE[pol]
        label = {"adduct": f"{r['name']} / {base}", "isotope": f"{r['name']} isotope", "loss": f"-{r['name']} in source"}[r["role"]]
        out.append({"role": r["role"], "name": r["name"], "label": label, "expected": float(r["delta"]), "of": r["of"]})
    tol = max(mz_ref * ppm * 1e-6, abs_da)
    for label, d in _cross_pairs(pol):
        if abs(delta - d) <= tol:
            out.append({"role": "adduct", "name": label, "label": label, "expected": float(d), "of": None})
    return out


def _shape_r(a: np.ndarray, b: np.ndarray, min_common: int):
    """Pearson r of two area rows over the samples where both are detected (> 0); (None, n) when it cannot be told."""
    k = (a > 0) & (b > 0)
    n = int(k.sum())
    if n < min_common:
        return None, n, k
    x, y = a[k], b[k]
    sx, sy = x.std(), y.std()
    if sx <= 0 or sy <= 0:
        return None, n, k
    return float(np.mean((x - x.mean()) * (y - y.mean())) / (sx * sy)), n, k


class _DSU:
    def __init__(self, n):
        self.p = list(range(n))

    def find(self, i):
        while self.p[i] != i:
            self.p[i] = self.p[self.p[i]]
            i = self.p[i]
        return i

    def union(self, a, b):
        a, b = self.find(a), self.find(b)
        if a != b:
            self.p[max(a, b)] = min(a, b)


def build(payload: dict) -> dict:
    """payload: {pol: 1|-1, samples: [names], features: [{k, id, mz, rt, a: [[sample index, area], ...], ms2?: {mz, it}, iin?: {id, ions, neutral}}],
    edges?: [{a, b, type, score, annotation}] (mzMine's FBMN edges, by feature id), opt?: {ppm, abs_da, r_min, apex_scans, cycle_s, min_common}}.
    Returns {identities, edges, stats}; every identity has `members` (keys), `neutral` (main hypothesis) and `alternatives`."""
    opt = {**DEFAULTS, **(payload.get("opt") or {})}
    pol = 1 if int(payload.get("pol") or 1) >= 0 else -1
    feats = payload["features"]
    n = len(feats)
    samples = payload.get("samples") or []
    S = len(samples)
    mz = np.array([f["mz"] for f in feats], float)
    rt = np.array([f["rt"] for f in feats], float)
    keys = [f["k"] for f in feats]
    A = np.zeros((n, max(S, 1)), np.float64)
    for i, f in enumerate(feats):
        for s, v in f.get("a") or []:
            if 0 <= s < S:
                A[i, s] = v
    order = np.argsort(rt, kind="stable")
    rts = rt[order]
    apex_min = opt["apex_scans"] * opt["cycle_s"] / 60.0
    sh = shifts(pol)
    ppm, abs_da = opt["ppm"], opt["abs_da"]
    dsu = _DSU(n)
    edges: list[dict] = []
    seen: set = set()
    index = {k: i for i, k in enumerate(keys)}
    ids = {f["id"]: i for i, f in enumerate(feats)}

    # ---- (a) the ion identities that the table already carries
    groups: dict = {}
    for i, f in enumerate(feats):
        g = f.get("iin")
        if g and g.get("id") not in (None, ""):
            groups.setdefault(str(g["id"]), []).append(i)
    iin_of = {i: gid for gid, ms in groups.items() for i in ms}
    n_iin = 0
    for gid, ms in groups.items():
        if len(ms) < 2:
            continue
        ms = sorted(ms, key=lambda i: mz[i])
        for j in ms[1:]:
            i = ms[0]
            g = feats[j].get("iin") or {}
            ions = " ~ ".join(x for x in (feats[i].get("iin", {}).get("ions"), g.get("ions")) if x)
            edges.append(_edge(i, j, keys, mz, rt, kind=IIN_KIND, label=ions or f"mzMine group {gid}", source="mzMine table", group=gid, neutral=g.get("neutral") or feats[i].get("iin", {}).get("neutral")))
            dsu.union(i, j)
            seen.add((i, j) if i < j else (j, i))
            n_iin += 1
    for e in payload.get("edges") or []:
        t = str(e.get("type") or "")
        if "cosine" in t.lower() or not t:
            continue                                                  # MS2 similarity edges are not ion identities
        a, b = ids.get(str(e["a"])), ids.get(str(e["b"]))
        if a is None or b is None or a == b:
            continue
        pair = (a, b) if a < b else (b, a)
        if pair in seen:
            for ed in edges:
                if {ed["a"], ed["b"]} == {keys[a], keys[b]}:
                    ed["note"] = (ed.get("note") or "") + (f" · {t}: {e.get('annotation')}" if e.get("annotation") else f" · {t}")
            continue
        seen.add(pair)
        edges.append(_edge(a, b, keys, mz, rt, kind=IIN_KIND, label=str(e.get("annotation") or t), source="mzMine edges", score=e.get("score")))
        dsu.union(a, b)
        n_iin += 1

    # ---- (b) the steps mzMine did not group: co-eluting, correlated, explained
    n_pairs = n_hits = 0
    hi = np.searchsorted(rts, rts + apex_min * (1 + 1e-9), side="right")
    for pi in range(n):
        i = int(order[pi])
        js = order[pi + 1:hi[pi]]
        if len(js) == 0:
            continue
        mzj = mz[js]
        delta = mzj - mz[i]
        tol = np.maximum(mzj * ppm * 1e-6, abs_da)
        near = np.abs(delta[:, None] - sh[None, :]).min(1) <= tol
        dim = (np.abs(mzj - (2 * (mz[i] - pol * PROTON) + pol * PROTON)) <= tol) | (np.abs(mz[i] - (2 * (mzj - pol * PROTON) + pol * PROTON)) <= tol)
        for j in js[near | dim]:
            j = int(j)
            n_pairs += 1
            pair = (i, j) if i < j else (j, i)
            if pair in seen or (i in iin_of and iin_of.get(i) == iin_of.get(j)):
                continue
            lo, up = (i, j) if mz[i] <= mz[j] else (j, i)           # x: lighter ion, y: heavier ion
            rels = pair_relations(float(mz[up] - mz[lo]), float(mz[lo]), pol, ppm, abs_da)
            if iimn.dimer_of(float(mz[lo]), float(mz[up]), ppm, abs_da, pol):
                rels.append({"role": "dimer", "name": BASE_DIMER[pol], "label": f"{BASE_DIMER[pol]} / {BASE[pol]}", "expected": float(mz[up] - mz[lo]), "of": "x"})
            if not rels:
                continue
            r, ncom, common = _shape_r(A[lo], A[up], opt["min_common"])
            if r is None or r < opt["r_min"]:
                continue
            n_hits += 1
            seen.add(pair)
            ed = _edge(lo, up, keys, mz, rt, kind=rels[0]["role"], label=" | ".join(x["label"] for x in rels), source="computed", r=r, n_common=ncom,
                       samples=[samples[s] for s in np.flatnonzero(common)[:12]], apex_min=apex_min, cycle_s=opt["cycle_s"],
                       relations=[{k: v for k, v in x.items() if k != "of"} | {"ppm": float((mz[up] - mz[lo] - x["expected"]) / mz[lo] * 1e6)} for x in rels])
            edges.append(ed)
            dsu.union(lo, up)

    # ---- the evidence of every edge: adduct or in-source fragment? (a signal for the chemist, `edge_evidence`)
    for ed in edges:
        i, j = index[ed["a"]], index[ed["b"]]
        lo, up = (i, j) if mz[i] <= mz[j] else (j, i)
        al, au = edge_evidence.feature_areas(feats[lo], feats[up], S)
        ed["evidence"] = edge_evidence.edge_evidence(float(mz[lo]), float(mz[up]), pol, ms2_lo=feats[lo].get("ms2"), ms2_up=feats[up].get("ms2"),
                                                     areas_lo=al, areas_up=au, drt_min=abs(float(rt[up] - rt[lo])))

    # ---- connected components → identities
    comp: dict = {}
    for i in range(n):
        comp.setdefault(dsu.find(i), []).append(i)
    by_edge: dict = {}
    for ei, ed in enumerate(edges):
        by_edge.setdefault(dsu.find(index[ed["a"]]), []).append(ei)
    identities = []
    for c, members in sorted(comp.items(), key=lambda kv: (float(np.median(rt[kv[1]])), kv[0])):
        ms = sorted(members, key=lambda i: mz[i])
        maxa = {i: float(A[i].max()) for i in ms}
        hyp = hypotheses(ms, mz, maxa, pol, opt, [feats[i].get("iin") for i in ms])
        main, alts = (hyp[0], hyp[1:5]) if hyp else (None, [])
        identities.append({"id": f"ID{len(identities) + 1}", "members": [keys[i] for i in ms], "rt": float(np.median(rt[ms])), "neutral": _hyp_out(main, keys, mz),
                           "alternatives": [_hyp_out(h, keys, mz) for h in alts], "edges": by_edge.get(c, []), "mzmine_groups": sorted({str(iin_of[i]) for i in ms if i in iin_of})})
    return {"identities": identities, "edges": edges,
            "stats": {"features": n, "identities": len(identities), "grouped": sum(1 for x in identities if len(x["members"]) > 1), "edges": len(edges), "edges_mzmine": n_iin,
                      "edges_computed": len(edges) - n_iin, "pairs_tested": n_pairs, "apex_window_min": apex_min}}


BASE_DIMER = {1: "[2M+H]+", -1: "[2M-H]-"}


def _edge(a, b, keys, mz, rt, kind, label, source, **kw) -> dict:
    d = {"a": keys[a], "b": keys[b], "kind": kind, "label": label, "source": source, "dmz": float(mz[b] - mz[a]), "drt": float(rt[b] - rt[a])}
    if "apex_min" in kw:
        am, cy = kw.pop("apex_min"), kw.pop("cycle_s")
        d["dscans"] = abs(d["drt"]) * 60.0 / cy
    d.update({k: v for k, v in kw.items() if v is not None})
    return d


def hypotheses(ms, mz, maxa, pol: int, opt: dict, iin_list=()) -> list[dict]:
    """Neutral masses that explain the members of an identity, best first: (members explained, a base ion among them, small error, area).
    A member is explained as an adduct of the neutral mass, or as an isotopologue / in-source fragment of an explained one."""
    forms = FORMS[pol]
    ppm, abs_da = opt["ppm"], opt["abs_da"]
    cap = sorted(ms, key=lambda i: -maxa[i])[:opt["max_hyp_members"]]
    cands = []
    for i in cap:
        for name, shift, mult, _ in forms:
            cands.append(((mz[i] - shift) / mult, "t"))
    for g in iin_list:
        if g and g.get("neutral"):
            cands.append((float(g["neutral"]), "mzmine"))
    cands.sort()
    clusters = []
    for m, src in cands:
        if clusters and abs(m - clusters[-1]["m"]) <= max(m * ppm * 1e-6, abs_da):
            clusters[-1]["n"] += 1
            clusters[-1]["src"].add(src)
        else:
            clusters.append({"m": m, "n": 1, "src": {src}})
    out = []
    for cl in clusters:
        M = cl["m"]
        role = {}
        for j in ms:
            best = None
            for name, shift, mult, base in forms:
                exp = mult * M + shift
                err = mz[j] - exp
                if abs(err) <= max(exp * ppm * 1e-6, abs_da) and (best is None or abs(err) < abs(best[1])):
                    best = (name, err, base)
            if best:
                role[j] = {"label": best[0], "err": best[1], "kind": "adduct", "base": best[2]}
        if not role:
            continue
        for _ in range(3):
            grew = False
            for j in ms:
                if j in role:
                    continue
                for e, r in list(role.items()):
                    d = mz[j] - mz[e]
                    tol = max(mz[e] * ppm * 1e-6, abs_da)
                    ref = r.get("ref", r["label"])
                    hit = next((("isotope", f"{nm} isotope of {ref}", d - v, ref) for nm, v in iimn.ISOTOPES.items() if abs(d - v) <= tol), None) \
                        or next((("loss", f"-{nm} in source from {ref}", d + v, f"the -{nm} fragment") for nm, v in iimn.LOSSES.items() if abs(d + v) <= tol), None)
                    if hit:
                        role[j] = {"label": hit[1], "err": hit[2], "kind": hit[0], "base": False, "ref": hit[3]}
                        grew = True
                        break
            if not grew:
                break
        n_base = sum(1 for r in role.values() if r["base"])
        rms = float(np.sqrt(np.mean([r["err"] ** 2 for r in role.values()])))
        area = sum(maxa[j] for j in role)
        anchor = max(role, key=lambda j: (role[j]["kind"] == "adduct", role[j]["base"], maxa[j]))
        out.append({"mass": M, "n": len(role), "n_base": n_base, "rms": rms, "area": area, "roles": role, "anchor": anchor, "mzmine": "mzmine" in cl["src"]})
    out.sort(key=lambda h: (-h["n"], -h["n_base"], h["rms"], -h["area"]))
    # a hypothesis whose members are all explained by a better one adds nothing (it is the same neutral mass seen through another adduct)
    final = []
    for h in out:
        if any(set(h["roles"]) <= set(g["roles"]) and abs(h["mass"] - g["mass"]) / g["mass"] < 1e-9 for g in final):
            continue
        final.append(h)
    return final


def _hyp_out(h, keys, mz) -> dict | None:
    if h is None:
        return None
    first = h["roles"][h["anchor"]]["label"]
    return {"mass": round(h["mass"], 5), "explained": h["n"], "anchor": keys[h["anchor"]], "form": first, "from_mzmine": h["mzmine"],
            "roles": {keys[j]: {"label": r["label"], "kind": r["kind"], "dmz": round(float(r["err"]), 5), "ppm": round(float(r["err"] / mz[j] * 1e6), 2)} for j, r in h["roles"].items()}}


# ---------------------------------------------------------------------------------------------------------------------- the two polarities
def polarity_pairs(pos: list[dict], neg: list[dict], ppm: float = 5.0, rt_min: float = 0.15) -> list[dict]:
    """Features of the positive and negative tables that can be the same molecule: the neutral mass agrees within `ppm` under the usual pairs
    ([M+H]+, [M+Na]+, [M+NH4]+ against [M-H]-, [M+FA-H]-, [M+Cl]-) and the RT within `rt_min`. Evidence for the chemist, not a verdict.
    Features: [{k, mz, rt}]. Returns [{pos, neg, pos_form, neg_form, mass, ppm, drt}]."""
    ft = {1: _form_table(1), -1: _form_table(-1)}
    rows = []
    for fe in neg:
        for name in PAIR_NEG:
            sh, mult = ft[-1][name]
            rows.append(((fe["mz"] - sh) / mult, name, fe))
    rows.sort(key=lambda t: t[0])
    Ms = np.array([r[0] for r in rows])
    out = []
    for fe in pos:
        for name in PAIR_POS:
            sh, mult = ft[1][name]
            M = (fe["mz"] - sh) / mult
            tol = M * ppm * 1e-6
            a, b = np.searchsorted(Ms, M - tol), np.searchsorted(Ms, M + tol, side="right")
            for j in range(a, b):
                Mn, nname, nf = rows[j]
                if abs(nf["rt"] - fe["rt"]) > rt_min:
                    continue
                out.append({"pos": fe["k"], "neg": nf["k"], "pos_form": name, "neg_form": nname, "mass": round(float(M), 5), "ppm": round(float((Mn - M) / M * 1e6), 2), "drt": round(float(nf["rt"] - fe["rt"]), 3)})
    out.sort(key=lambda r: (r["pos"], abs(r["ppm"])))
    return out
