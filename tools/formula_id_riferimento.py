"""Independent reference of the tests of `untargeted::formula_id`: [M+H]+ and doubly charged ions of known molecules with the isotope pattern computed from the natural
abundances (exact convolution of the isotopes of every atom, merged per nominal mass, weighted mean m/z), so the spectra do not come from data.
Run: python3 tools/formula_id_riferimento.py -> tests/data/untargeted/formula_id/casi.json"""
import json
import pathlib

ISO = {
    "H": [(1.00782503207, 0.999885), (2.0141017778, 0.000115)],
    "C": [(12.0, 0.9893), (13.0033548378, 0.0107)],
    "N": [(14.0030740048, 0.99636), (15.0001088982, 0.00364)],
    "O": [(15.99491461956, 0.99757), (16.99913170, 0.00038), (17.9991610, 0.00205)],
    "S": [(31.97207100, 0.9499), (32.97145876, 0.0075), (33.96786690, 0.0425)],
    "Cl": [(34.96885268, 0.7576), (36.96590259, 0.2424)],
    "Br": [(78.9183371, 0.5069), (80.9162906, 0.4931)],
}
ELECTRON = 0.00054857990946


def ion_pattern(ion, z, floor=1e-3):
    """(m/z, relative height) of the isotope peaks of the ion given as `{element: count}` and its charge z."""
    dist = {0: (0.0, 1.0)}
    mono = 0.0
    for el, n in ion.items():
        iso = ISO[el]
        mono += n * iso[0][0]
        for _ in range(n):
            new = {}
            for off, (mw, ab) in dist.items():
                for m, a in iso:
                    k = off + round(m - iso[0][0])
                    cur = new.get(k, (0.0, 0.0))
                    new[k] = (cur[0] + (mw + ab * (m - iso[0][0])) * a, cur[1] + ab * a)
            dist = {k: v for k, v in new.items() if v[1] > 1e-9}
    top = max(v[1] for v in dist.values())
    out = []
    for k in sorted(dist):
        mw, ab = dist[k]
        if ab / top >= floor:
            out.append(((mono + mw / ab - z * ELECTRON) / z, ab / top))
    return out


def case(name, ion, z, neutral):
    pat = ion_pattern(ion, z)
    return {"name": name, "ion": ion, "z": z, "neutral": neutral, "mz": [p[0] for p in pat], "y": [p[1] * 1e6 for p in pat]}


cases = [
    case("glucose", {"C": 6, "H": 13, "O": 6}, 1, "C6H12O6"),
    case("caffeine", {"C": 8, "H": 11, "N": 4, "O": 2}, 1, "C8H10N4O2"),
    case("paracetamol", {"C": 8, "H": 10, "N": 1, "O": 2}, 1, "C8H9NO2"),
    case("atrazine", {"C": 8, "H": 15, "Cl": 1, "N": 5}, 1, "C8H14ClN5"),
    case("bromazepam", {"C": 14, "H": 11, "Br": 1, "N": 3, "O": 1}, 1, "C14H10BrN3O"),
    case("peptide_2plus", {"C": 34, "H": 52, "N": 8, "O": 9}, 2, None),
]
out = pathlib.Path(__file__).resolve().parent.parent / "tests/data/untargeted/formula_id/casi.json"
out.write_text(json.dumps(cases, separators=(",", ":")))
print(out, len(cases))
