"""Write mzlab/web/elements.js (data of the "Tavola periodica" dialog).

Sources, all already in the program (no internet):
- exact isotope masses: the isotope table inside OpenChemLib (web/vendor/openchemlib.js);
- standard atomic weights: the element list inside Ketcher (web/vendor/ketcher/assets/index-*.js);
- natural abundances (%): IUPAC/CIAAW representative isotopic compositions, typed below only for the elements
  that matter in environmental/organic LC-MS. Elements not listed are shown without abundances.
Run from the project root:  python3 tools/genera_elementi.py
"""
import glob, json, re
from pathlib import Path

WEB = Path(__file__).resolve().parents[1] / "mzlab" / "web"

NAMES = ("Idrogeno Elio Litio Berillio Boro Carbonio Azoto Ossigeno Fluoro Neon Sodio Magnesio Alluminio Silicio Fosforo Zolfo "
         "Cloro Argon Potassio Calcio Scandio Titanio Vanadio Cromo Manganese Ferro Cobalto Nichel Rame Zinco Gallio Germanio "
         "Arsenico Selenio Bromo Kripton Rubidio Stronzio Ittrio Zirconio Niobio Molibdeno Tecnezio Rutenio Rodio Palladio Argento "
         "Cadmio Indio Stagno Antimonio Tellurio Iodio Xeno Cesio Bario Lantanio Cerio Praseodimio Neodimio Promezio Samario Europio "
         "Gadolinio Terbio Disprosio Olmio Erbio Tulio Itterbio Lutezio Afnio Tantalio Tungsteno Renio Osmio Iridio Platino Oro "
         "Mercurio Tallio Piombo Bismuto Polonio Astato Radon Francio Radio Attinio Torio Protoattinio Uranio Nettunio Plutonio "
         "Americio Curio Berkelio Californio Einsteinio Fermio Mendelevio Nobelio Laurenzio Rutherfordio Dubnio Seaborgio Bohrio "
         "Hassio Meitnerio Darmstadtio Roentgenio Copernicio Nihonio Flerovio Moscovio Livermorio Tennesso Oganesson").split()
SYMBOLS = ("H He Li Be B C N O F Ne Na Mg Al Si P S Cl Ar K Ca Sc Ti V Cr Mn Fe Co Ni Cu Zn Ga Ge As Se Br Kr Rb Sr Y Zr Nb Mo "
           "Tc Ru Rh Pd Ag Cd In Sn Sb Te I Xe Cs Ba La Ce Pr Nd Pm Sm Eu Gd Tb Dy Ho Er Tm Yb Lu Hf Ta W Re Os Ir Pt Au Hg Tl "
           "Pb Bi Po At Rn Fr Ra Ac Th Pa U Np Pu Am Cm Bk Cf Es Fm Md No Lr Rf Db Sg Bh Hs Mt Ds Rg Cn Nh Fl Mc Lv Ts Og").split()

# natural abundance, % (mass number: abundance)
AB = {
    "H": {1: 99.9885, 2: 0.0115}, "He": {3: 0.000134, 4: 99.999866}, "Li": {6: 7.59, 7: 92.41}, "Be": {9: 100},
    "B": {10: 19.9, 11: 80.1}, "C": {12: 98.93, 13: 1.07}, "N": {14: 99.636, 15: 0.364}, "O": {16: 99.757, 17: 0.038, 18: 0.205},
    "F": {19: 100}, "Ne": {20: 90.48, 21: 0.27, 22: 9.25}, "Na": {23: 100}, "Mg": {24: 78.99, 25: 10.00, 26: 11.01}, "Al": {27: 100},
    "Si": {28: 92.223, 29: 4.685, 30: 3.092}, "P": {31: 100}, "S": {32: 94.99, 33: 0.75, 34: 4.25, 36: 0.01}, "Cl": {35: 75.76, 37: 24.24},
    "Ar": {36: 0.3336, 38: 0.0629, 40: 99.6035}, "K": {39: 93.2581, 40: 0.0117, 41: 6.7302},
    "Ca": {40: 96.941, 42: 0.647, 43: 0.135, 44: 2.086, 46: 0.004, 48: 0.187}, "Sc": {45: 100},
    "Ti": {46: 8.25, 47: 7.44, 48: 73.72, 49: 5.41, 50: 5.18}, "V": {50: 0.250, 51: 99.750}, "Cr": {50: 4.345, 52: 83.789, 53: 9.501, 54: 2.365},
    "Mn": {55: 100}, "Fe": {54: 5.845, 56: 91.754, 57: 2.119, 58: 0.282}, "Co": {59: 100},
    "Ni": {58: 68.077, 60: 26.223, 61: 1.1399, 62: 3.6346, 64: 0.9255}, "Cu": {63: 69.15, 65: 30.85},
    "Zn": {64: 49.17, 66: 27.73, 67: 4.04, 68: 18.45, 70: 0.61}, "Ga": {69: 60.108, 71: 39.892},
    "Ge": {70: 20.57, 72: 27.45, 73: 7.75, 74: 36.50, 76: 7.73}, "As": {75: 100}, "Se": {74: 0.89, 76: 9.37, 77: 7.63, 78: 23.77, 80: 49.61, 82: 8.73},
    "Br": {79: 50.69, 81: 49.31}, "Rb": {85: 72.17, 87: 27.83}, "Sr": {84: 0.56, 86: 9.86, 87: 7.00, 88: 82.58}, "Y": {89: 100},
    "Nb": {93: 100}, "Mo": {92: 14.53, 94: 9.15, 95: 15.84, 96: 16.67, 97: 9.60, 98: 24.39, 100: 9.82}, "Rh": {103: 100},
    "Ag": {107: 51.839, 109: 48.161}, "Cd": {106: 1.25, 108: 0.89, 110: 12.49, 111: 12.80, 112: 24.13, 113: 12.22, 114: 28.73, 116: 7.49},
    "Sn": {112: 0.97, 114: 0.66, 115: 0.34, 116: 14.54, 117: 7.68, 118: 24.22, 119: 8.59, 120: 32.58, 122: 4.63, 124: 5.79},
    "Sb": {121: 57.21, 123: 42.79}, "I": {127: 100}, "Cs": {133: 100},
    "Ba": {130: 0.106, 132: 0.101, 134: 2.417, 135: 6.592, 136: 7.854, 137: 11.232, 138: 71.698}, "Pr": {141: 100}, "Tb": {159: 100},
    "Ho": {165: 100}, "Tm": {169: 100}, "Au": {197: 100}, "Hg": {196: 0.15, 198: 9.97, 199: 16.87, 200: 23.10, 201: 13.18, 202: 29.86, 204: 6.87},
    "Tl": {203: 29.52, 205: 70.48}, "Pb": {204: 1.4, 206: 24.1, 207: 22.1, 208: 52.4}, "Bi": {209: 100},
}

def layout(z):
    """(row, column) in the usual 18-column table; lanthanides and actinides in rows 9 and 10."""
    if z == 1: return 1, 1
    if z == 2: return 1, 18
    for start, period in ((3, 2), (11, 3)):
        if start <= z < start + 8:
            i = z - start; return period, i + 1 if i < 2 else i + 11
    for start, period in ((19, 4), (37, 5)):
        if start <= z < start + 18: return period, z - start + 1
    for start, period in ((55, 6), (87, 7)):
        if not start <= z < start + 32: continue
        i = z - start
        if i < 2: return period, i + 1
        if 2 <= i < 17: return period + 3, i + 1          # La..Lu / Ac..Lr: separate rows, columns 3..17
        return period, i - 14 + 1
    raise ValueError(z)

def ocl_masses():
    """Exact isotope masses by atomic number - 1: {neutron count: mass}, read from the table inside OpenChemLib."""
    ocl = (WEB / "vendor" / "openchemlib.js").read_text()
    pos = ocl.index("Bl=b(k(Ht,2),fA,16,0,[null,") + len("Bl=b(k(Ht,2),fA,16,0,[null,")
    pat, iso = re.compile(r"b\(k\(Ht,1\),Vt,3,0,\[(.*?)\]\)"), []
    while (m := pat.match(ocl, pos)):
        iso.append({int(n): float(x) for n, x in re.findall(r"new u\((\d+),([\d.]+)\)", m.group(1))})
        pos = m.end() + 1
        if ocl[m.end()] != ",": break
    return iso

def main():
    iso = ocl_masses()
    ket = Path(glob.glob(str(WEB / "vendor" / "ketcher" / "assets" / "index-*.js"))[0]).read_text()
    a = ket.index('eu=[{number:1,label:"H"'); b = ket.index("}]", a)
    weight = {int(n): float(w) for n, w in re.findall(r"\{number:(\d+),[^}]*?mass:([\d.]+)", ket[a:b])}
    out = []
    for z, (sym, name) in enumerate(zip(SYMBOLS, NAMES), start=1):
        row, col = layout(z)
        masses = iso[z - 1] if z - 1 < len(iso) else {}
        ab = AB.get(sym)
        isotopes = []
        if ab:
            for A, pct in sorted(ab.items()):
                n = A - z
                isotopes.append([A, masses.get(n), pct])
        out.append({"z": z, "s": sym, "n": name, "r": row, "c": col, "w": weight.get(z), "iso": isotopes})
    text = ("// Generated by tools/genera_elementi.py: element data for the \"Tavola periodica\" dialog (do not edit by hand).\n"
            "// iso = [mass number, exact mass, natural abundance %]; only for the elements typical of LC-MS.\n"
            "const ELEMENTS = " + json.dumps(out, separators=(",", ":")) + ";\n")
    (WEB / "elements.js").write_text(text, encoding="utf-8")
    print("written", WEB / "elements.js", len(out), "elements")

if __name__ == "__main__":
    main()
