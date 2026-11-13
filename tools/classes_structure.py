"""Structure classes (`structure:<value>`) of a molecule read from its SMILES alone, by fixed rules: only what the graph shows for certain.

Used by tools/generate_molecules.py for the catalogue (mzlab/web/molecole/catalogo.csv). A rule never guesses a use or an effect: a halogen is in the formula,
a carboxylic acid is a carbonyl carbon bound to carbon or hydrogen and to an OH, a nitro group
sits on C or N, a sulfonamide is C-SO2-N. Tests: tests/test_classi_struttura.py.
"""
import re

VOCABULARY = ("brominated", "carboxylic_acid", "chlorinated", "ester", "fluorinated", "iodinated", "nitro", "quaternary_ammonium", "sulfonamide")
_TOKEN = re.compile(r"\[([^\]]+)\]|(Cl|Br|[BCNOPSFI]|[bcnops])|(%\d\d|\d)|([-=#:/\\$.])|([()])")
_ORDER = {"-": 1, "/": 1, "\\": 1, "=": 2, "#": 3, "$": 4, ":": 1.5}


def parse(smiles):
    """Atoms [(element, aromatic, charge, explicit H or None)] and bonds {(i, j): order} of a SMILES; ValueError on a token it does not know."""
    atoms, bonds, stack, rings = [], {}, [], {}
    prev, pending, pos = None, None, 0
    while pos < len(smiles):
        m = _TOKEN.match(smiles, pos)
        if not m:
            raise ValueError(f"SMILES not read at {pos}: {smiles}")
        pos = m.end()
        bracket, plain, ring, bond, paren = m.groups()
        if bracket or plain:
            if bracket:
                b = re.fullmatch(r"\d*([A-Z][a-z]?|[a-z]{1,2})(@*)(?:H(\d?))?([+-]+\d*)?(?::\d+)?", bracket)
                if not b:
                    raise ValueError(f"bracket atom not read: [{bracket}]")
                el, h, ch = b.group(1), b.group(3), b.group(4) or ""
                hs = (int(h) if h else 1) if "H" in bracket[len(b.group(1)):] else 0
                charge = (int(ch[1:]) if ch[1:].isdigit() else len(ch)) * (1 if ch[:1] == "+" else -1) if ch else 0
            else:
                el, hs, charge = plain, None, 0
            atoms.append((el.capitalize(), el.islower(), charge, hs))
            i = len(atoms) - 1
            if prev is not None:
                bonds[(prev, i)] = _ORDER[pending] if pending else (1.5 if atoms[prev][1] and atoms[i][1] else 1)
            prev, pending = i, None
        elif ring:
            if ring in rings:
                j, o = rings.pop(ring)
                bonds[(j, prev)] = _ORDER[pending or o] if (pending or o) else (1.5 if atoms[j][1] and atoms[prev][1] else 1)
            else:
                rings[ring] = (prev, pending)
            pending = None
        elif bond:
            if bond == ".":
                prev = None
            else:
                pending = bond
        elif paren == "(":
            stack.append(prev)
        else:
            prev = stack.pop()
    if rings or stack:
        raise ValueError(f"open ring or branch: {smiles}")
    return atoms, bonds


def classes(smiles):
    """The `structure:` classes of a SMILES, sorted; [] for an empty SMILES."""
    if not smiles.strip():
        return []
    atoms, bonds = parse(smiles.strip())
    nb = [[] for _ in atoms]                                     # (neighbour, order)
    for (i, j), o in bonds.items():
        nb[i].append((j, o))
        nb[j].append((i, o))
    el = lambda i: atoms[i][0]                                   # noqa: E731
    neutral_oh = lambda i: el(i) == "O" and atoms[i][2] == 0 and len(nb[i]) == 1 and atoms[i][3] in (None, 1)   # noqa: E731
    out = {name for name, sym in (("fluorinated", "F"), ("chlorinated", "Cl"), ("brominated", "Br"), ("iodinated", "I")) if any(a[0] == sym for a in atoms)}
    for i, (e, arom, charge, hs) in enumerate(atoms):
        double_o = [j for j, o in nb[i] if o == 2 and el(j) == "O" and len(nb[j]) == 1]
        single = [j for j, o in nb[i] if o == 1]
        if e == "C" and not arom and len(double_o) == 1 and len(nb[i]) == len(single) + 1:
            rest = [j for j in single if el(j) != "O"]
            o_single = [j for j in single if el(j) == "O"]
            if len(o_single) == 1 and all(el(j) == "C" for j in rest):
                o = o_single[0]
                if neutral_oh(o):
                    out.add("carboxylic_acid")
                elif atoms[o][2] == 0 and len(nb[o]) == 2 and all(el(j) == "C" for j, _ in nb[o]) and all(oo == 1 for _, oo in nb[o]):
                    out.add("ester")
        if e == "S" and len(double_o) == 2 and len(single) == 2 and sorted(el(j) for j in single) == ["C", "N"]:
            out.add("sulfonamide")                               # C-SO2-N only: a sulfamate (O-SO2-N) or a sulfamic acid is not one
        if e == "N":
            os_ = [(j, o) for j, o in nb[i] if el(j) == "O" and len(nb[j]) == 1]
            charged = charge == 1 and sorted(o for _, o in os_) == [1, 2] and any(atoms[j][2] == -1 for j, o in os_ if o == 1)
            third = [el(j) for j, _ in nb[i] if (j, _) not in os_]
            if len(nb[i]) == 3 and third in (["C"], ["N"]) and (charged or (charge == 0 and [o for _, o in os_] == [2, 2])):   # a nitrate ester (O-NO2) is not nitro
                out.add("nitro")
            if charge == 1 and not arom and len(nb[i]) == 4 and all(o == 1 and el(j) != "H" for j, o in nb[i]) and not hs:
                out.add("quaternary_ammonium")
    return sorted("structure:" + c for c in out)
