# TPMINE-PRIVATE
"""Confidence of the predictions: how much supports the transformation that derives a candidate from the parent (four levels after Löffler et al.,
ES&T 59:19095, 2025, who grade TPs predicted in silico). A second scale, next to the confidence level of the identification (5, 4, 3 of Schymanski
et al. 2014): the level of the identification says how well the structure is established, the level of the prediction says how well the
transformation is. Prediction level (1 = strongest):
  1  confirmed by the data: the MS2 of the candidate has fragments shifted by the mass difference of the rule (Schollée et al., JASMS 2017) and the
     ion is absent in the reference samples;
  2  observed in the literature: the rule is in the list of transformations with a reference (comment `lit: <reference>`);
  3  specific to the class: the rule needs a group (a halogen, a sulfonyl, a carboxyl...) and the parent has it;
  4  generic: a rule that applies to any compound (hydroxylation, dehydrogenation, demethylation...).
A derivation of several steps takes the weakest of its steps; level 1 is a property of the data, not of the rules. The program never writes 2b."""
from __future__ import annotations

import re

import numpy as np

from . import formula as F

PRED_TEXT = {
    1: "confirmed by the data: MS2 fragments shifted by the mass difference of the rule",
    2: "observed in the literature (reference in the list of transformations)",
    3: "specific to the class: the parent has the group the rule needs",
    4: "generic rule",
}
# rules that need a group of the parent: name -> {"all": element counts needed, "any": at least one of these elements, "smiles": at least one of these
# substrings in the SMILES when the parent has one}
CLASS_RULES = {
    "reductive dechlorination": {"any": ["Cl"]},
    "hydrolytic dechlorination": {"any": ["Cl"]},
    "reductive debromination": {"any": ["Br"]},
    "hydrolytic defluorination": {"any": ["F"]},
    "loss of SO2": {"all": {"S": 1, "O": 2}},
    "decarboxylation": {"all": {"O": 2}, "smiles": ["C(=O)O", "OC(=O)", "C(O)=O", "O=C(O)", "C(=O)[O-]"]},
    "oxidative deamination": {"all": {"N": 1}},
    "loss of propene (N-deisopropylation)": {"all": {"N": 1}},
}
CLEAVAGE = "cleavage"
LIT = re.compile(r"^\s*lit\s*:\s*(.*)$", re.I)


def literature(extra, els: list[str]) -> dict:
    """{signed element vector (tuple): reference} of the rows of the user's list whose comment starts with `lit:`."""
    out = {}
    for t in extra or []:
        m = LIT.match(t.get("comment") or "")
        if m and m.group(1).strip():
            out[tuple(int(t["delta"].get(e, 0)) for e in els)] = m.group(1).strip()
    return out


def steps(derivation: str | None) -> list[str]:
    """Names of the transformations of a derivation ('a + b' = two steps; a cleavage is one step called 'cleavage')."""
    if not derivation:
        return []
    if derivation.startswith(CLEAVAGE):
        return [CLEAVAGE]
    return derivation.split(" + ")


def has_group(name: str, parent_vec: np.ndarray, els: list[str], smiles: str | None) -> bool:
    """Does the parent have what the class rule `name` needs?"""
    need = CLASS_RULES[name]
    n = {e: int(v) for e, v in zip(els, parent_vec)}
    if any(n.get(e, 0) < k for e, k in need.get("all", {}).items()):
        return False
    if need.get("any") and not any(n.get(e, 0) > 0 for e in need["any"]):
        return False
    if need.get("smiles") and smiles and not any(s in smiles for s in need["smiles"]):
        return False
    return True


def step_info(name: str, table, parent_vec, els, smiles, lit: dict) -> dict:
    """{rule, scope 'literature' | 'class' | 'generic', level 2 | 3 | 4, source} of one step."""
    vec = next((tuple(int(x) for x in v) for n, v in table if n == name), None)
    if vec is not None and vec in lit:
        return {"rule": name, "scope": "literature", "level": 2, "source": lit[vec]}
    if name in CLASS_RULES and has_group(name, parent_vec, els, smiles):
        return {"rule": name, "scope": "class", "level": 3, "source": ""}
    return {"rule": name, "scope": "generic", "level": 4, "source": ""}


MIN_SHIFTED = 2             # shifted fragments (or common neutral losses) that confirm the rule
MIN_SHIFTED_COS = 0.05      # ... and their share of the modified cosine


def confirmed(sim: dict | None, kinetics: dict | None) -> tuple[bool, str]:
    """Does the MS2 confirm the transformation? At least MIN_SHIFTED peaks moved by the mass difference (shifted fragments or common neutral losses),
    carrying at least MIN_SHIFTED_COS of the cosine, and the ion not present in the reference samples."""
    if not sim:
        return False, "no MS2 for this ion"
    n = sim["n"]["shifted"] + sim["n"]["loss"]
    share = sim["cos"]["shifted"] + sim["cos"]["loss"]
    if n < MIN_SHIFTED or share < MIN_SHIFTED_COS:
        return False, f"{n} shifted fragments, {share:.2f} of the cosine: not enough"
    if kinetics is not None and kinetics.get("ok") and not kinetics.get("absent_in_reference", True):
        return False, "present in the reference samples"
    return True, f"{n} fragments shifted by {sim['delta']:+.4f} (share of the cosine {share:.2f})"


def prediction(c: dict, table, parent_vec, els, smiles, lit: dict) -> dict | None:
    """Prediction level of a candidate: from its derivation (weakest step), raised to 1 when the data confirm it. None without a derivation."""
    names = steps(c.get("derivation"))
    if not names:
        return None
    infos = [step_info(n, table, parent_vec, els, smiles, lit) for n in names]
    level = max(i["level"] for i in infos)
    ok, why = confirmed((c.get("ms2") or {}).get("sim"), c.get("kinetics")) if names != [CLEAVAGE] else (False, "a cleavage has no single mass difference")
    if ok:
        level = 1
    evidence = [f"{i['rule']}: " + {"literature": f"observed in the literature ({i['source']})", "class": "specific to the class",
                                    "generic": "generic rule"}[i["scope"]] for i in infos]
    evidence.append(("data: " if ok else "data not confirming: ") + why)
    return {"level": level, "text": PRED_TEXT[level], "steps": infos, "confirmed": ok, "evidence": evidence}
