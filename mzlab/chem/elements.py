"""Monoisotopic masses, formulas and adducts. Plain Python, no dependencies."""
from __future__ import annotations

import re

from ..i18n import UserError

MASS = {
    "H": 1.00782503223, "C": 12.0, "N": 14.00307400443, "O": 15.99491461957, "F": 18.99840316273,
    "P": 30.97376199842, "S": 31.9720711744, "Cl": 34.968852682, "Br": 78.9183376, "I": 126.9044719,
    "Na": 22.989769282, "K": 38.9637064, "Si": 27.9769265350, "Se": 79.9165218,
}
ELECTRON = 0.000548579909
PROTON = MASS["H"] - ELECTRON

# ion m/z = neutral mass + ADDUCT_SHIFT[adduct]; the sign of the charge follows the adduct
ADDUCT_SHIFT = {
    "[M+H]+": MASS["H"] - ELECTRON,
    "[M+NH4]+": MASS["N"] + 4 * MASS["H"] - ELECTRON,
    "[M+Na]+": MASS["Na"] - ELECTRON,
    "[M+K]+": MASS["K"] - ELECTRON,
    "[M-H]-": -MASS["H"] + ELECTRON,
    "[M+Cl]-": MASS["Cl"] + ELECTRON,
    "[M+HCOO]-": MASS["C"] + MASS["H"] + 2 * MASS["O"] + ELECTRON,
}
DEFAULT_ADDUCT = {"positive": "[M+H]+", "negative": "[M-H]-"}

_TOKEN = re.compile(r"([A-Z][a-z]?)(\d*)")


# two-letter symbols that a lower-case spelling may mean (organic / environmental chemistry). Others (co, no, cn, ni, pb, sn, hf...) are read as two elements.
COMMON2 = {"Cl", "Br", "Si", "Na", "Se", "Li", "Mg", "Ca", "Al", "Fe", "Zn", "Cu", "Mn", "As", "Hg", "Cd"}


def normalize_formula(s: str) -> str:
    """Fix the capitals of a formula typed in lower case ('c9h10cl2n2o' -> 'C9H10Cl2N2O').

    A correctly written symbol is kept ('Co' is cobalt, 'CO' is C + O). A lower-case letter followed by a lower-case letter makes a two-letter
    symbol only if it is a common one (Cl, Br, Si, Na, Se...); otherwise they are two elements ('co' = C + O, not cobalt).
    """
    s = re.sub(r"\s+", "", s or "")
    out, i, n = [], 0, len(s)
    while i < n:
        c, nx = s[i], s[i + 1] if i + 1 < n else ""
        if not c.isalpha():
            out.append(c); i += 1
        elif c.isupper() and nx.islower() and (c + nx) in MASS:
            out.append(c + nx); i += 2                                # as typed
        elif c.islower() and nx.islower() and (c.upper() + nx) in COMMON2:
            out.append(c.upper() + nx); i += 2
        else:
            out.append(c.upper()); i += 1
    return "".join(out)


def parse_formula(s: str) -> dict[str, int]:
    """'C15H12N2O' -> {'C': 15, 'H': 12, 'N': 2, 'O': 1}. Parentheses are allowed: 'C(OH)2'. Lower case is understood (see normalize_formula)."""
    s = normalize_formula(s)
    if not s:
        raise UserError("err.formula.empty", text="empty formula")
    stack: list[dict[str, int]] = [{}]
    pos = 0
    tok = re.compile(r"([A-Z][a-z]?)(\d*)|(\()|\)(\d*)")
    while pos < len(s):
        m = tok.match(s, pos)
        if not m:
            raise UserError("err.formula.unreadable", {"formula": s}, f"cannot read the formula {s!r}")
        if m.group(1):
            el = m.group(1)
            if el not in MASS:
                raise UserError("err.formula.element", {"element": el, "formula": s}, f"unknown element {el!r} in {s!r}")
            stack[-1][el] = stack[-1].get(el, 0) + (int(m.group(2)) if m.group(2) else 1)
        elif m.group(3):
            stack.append({})
        else:
            if len(stack) < 2:
                raise UserError("err.formula.parens", {"formula": s}, f"unbalanced parentheses in {s!r}")
            inner = stack.pop()
            k = int(m.group(4)) if m.group(4) else 1
            for el, n in inner.items():
                stack[-1][el] = stack[-1].get(el, 0) + n * k
        pos = m.end()
    if len(stack) != 1 or not stack[0]:
        raise UserError("err.formula.unreadable", {"formula": s}, f"cannot read the formula {s!r}")
    return stack[0]


def round_half_up(x: float, digits: int = 0) -> float:
    """Round half up (not banker's rounding): 364.05 -> 364.1, 200.5 -> 201."""
    from decimal import Decimal, ROUND_HALF_UP
    q = Decimal(1).scaleb(-digits)
    return float(Decimal(repr(round(x, 9))).quantize(q, rounding=ROUND_HALF_UP))


def formula_mz(formula: str, adduct: str | None = None) -> dict:
    """Formula -> neutral mass and ion m/z, exact, at 1 decimal (unit-resolution instrument) and nominal (integer)."""
    f = parse_formula(formula)
    m = mass(f)
    rows = {}
    for a in ADDUCT_SHIFT:
        z = ion_mz(m, a)
        rows[a] = {"mz": round(z, 4), "mz5": round(z, 5), "mz1": round_half_up(z, 1), "nominal": int(round_half_up(z, 0))}      # mz5: the decimals of a high-resolution profile
    out = {"formula": fmt(f), "neutral": round(m, 4), "nominal_neutral": int(round_half_up(m, 0)), "adducts": rows}
    if adduct:
        if adduct not in ADDUCT_SHIFT:
            raise UserError("err.adduct.unknown", {"adduct": adduct}, f"unknown adduct {adduct!r}")
        out["adduct"] = adduct
        out.update(rows[adduct])
    return out


def mass(f: dict[str, int]) -> float:
    return sum(MASS[e] * n for e, n in f.items())


def fmt(f: dict[str, int]) -> str:
    """Hill order: C, H, then alphabetical."""
    order = [e for e in ("C", "H") if e in f] + sorted(e for e in f if e not in ("C", "H"))
    return "".join(e + (str(f[e]) if f[e] != 1 else "") for e in order if f[e])


def ion_mz(neutral: float, adduct: str) -> float:
    return neutral + ADDUCT_SHIFT[adduct]
