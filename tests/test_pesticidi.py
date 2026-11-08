"""The built-in list of pesticides and environmental contaminants (mzlab/chem/pesticides.csv + pesticides_meta.json, expanded by contaminants.py):
ids, InChIKeys, formula against SMILES and mass, tags from the closed vocabulary, IT+EN names, and its place next to Keller in contaminanti.json."""
import collections
import csv
import json
import re
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from mzlab.chem.contaminants import ion_shift, lists, pesticides  # noqa: E402
from mzlab.chem.elements import fmt, mass, parse_formula  # noqa: E402

CHEM = ROOT / "mzlab" / "chem"
ROWS = list(csv.DictReader((CHEM / "pesticides.csv").open(encoding="utf-8"), delimiter=";"))
META = json.loads((CHEM / "pesticides_meta.json").read_text(encoding="utf-8"))
IONS = {"[M+H]+", "[M+Na]+", "[M+NH4]+", "[M-H]-"}


def heavy_atoms(smiles):
    """Element counts of a SMILES without hydrogens (bracket atoms, the organic subset and its aromatic lower case)."""
    out = collections.Counter()
    for bracket, plain in re.findall(r"\[([^\]]+)\]|(Cl|Br|[BCNOPSFI]|[bcnops])", smiles):
        el = re.match(r"\d*([A-Z][a-z]?|[a-z]{1,2})", bracket).group(1) if bracket else plain
        if el != "H":
            out[el.capitalize()] += 1
    return out


def test_counts_ids_and_inchikeys():
    assert len(ROWS) == 192
    for k in ("id", "inchikey", "pubchem_cid", "name_en", "name"):
        vals = [r[k].lower() for r in ROWS]
        assert len(set(vals)) == len(vals), (k, [v for v, n in collections.Counter(vals).items() if n > 1])
    assert all(re.fullmatch(r"[a-z0-9_]+", r["id"]) for r in ROWS)
    assert all(re.fullmatch(r"[A-Z]{14}-[A-Z]{10}-[A-Z]", r["inchikey"]) for r in ROWS)
    assert all(r["pubchem_cid"].isdigit() for r in ROWS)


@pytest.mark.parametrize("r", ROWS, ids=lambda r: r["id"])
def test_formula_matches_the_structure(r):
    f = parse_formula(r["formula"])
    assert heavy_atoms(r["smiles"]) == collections.Counter({k: v for k, v in f.items() if k != "H"})
    assert fmt(f) == r["formula"]                                   # PubChem writes Hill order, the same as the engine


def test_tags_come_from_the_closed_vocabulary():
    used_cls, used_use = {r["class"] for r in ROWS}, {r["use"] for r in ROWS}
    assert used_cls <= set(META["classes"]) and used_use <= set(META["uses"])
    assert set(META["classes"]) == used_cls and set(META["uses"]) == used_use, "a word of the vocabulary that no row uses"
    assert all(len(v) == 2 and all(v) for v in [*META["classes"].values(), *META["uses"].values()]), "every class and use in Italian and English"
    assert all(set(r["ions"].split("|")) <= IONS for r in ROWS)
    for cls in ("organophosphates", "carbamates", "triazines", "neonicotinoids", "strobilurins", "azoles", "phenylureas", "pfas"):
        assert sum(r["class"] == cls for r in ROWS) >= 5, cls


def test_the_ions_carry_the_engine_masses():
    lst = pesticides()
    assert lst["id"] == "pesticides" and lst["background"] is False and lst["builtin"]
    assert lst["license"] and lst["license_en"] and "PubChem" in lst["source"]
    assert len(lst["items"]) == sum(len(r["ions"].split("|")) for r in ROWS)
    for it in lst["items"]:
        sh, z = ion_shift(it["adduct"])
        assert it["mz"] == pytest.approx((mass(parse_formula(it["formula"])) + sh) / z, abs=1e-5)
        assert it["pol"] == (1 if it["adduct"].endswith("+") else -1)
        assert it["name"] and it["name_en"] and it["cls"] and it["cls_en"] and it["smiles"] and it["inchikey"]
        assert [t.split(":")[0] for t in it["tags"]] == ["chemical", "use"]
    atz = [i["mz"] for i in lst["items"] if i["id"] == "atrazine" and i["adduct"] == "[M+H]+"]
    assert atz == [pytest.approx(216.10105, abs=2e-5)]
    pfoa = [i["mz"] for i in lst["items"] if i["id"] == "pfoa"]
    assert pfoa == [pytest.approx(412.96643, abs=2e-5)]


def test_the_list_sits_next_to_keller():
    assert [x["id"] for x in lists()] == ["keller", "pesticides"]
    assert lists()[0].get("background", True)


NODE = __import__("shutil").which("node")


@pytest.mark.skipif(not NODE, reason="node non disponibile")
@pytest.mark.parametrize("lang", ["it", "en"])
def test_the_formula_dialog_finds_neonicotinoids_by_name(lang):
    """liste-nomi.js (formula dialog, XIC ion field) over the shipped lists: «imidacloprid» is one entry with its neutral formula and [M+H]+."""
    code = r"""
const fs = require('fs'), vm = require('vm'), web = process.argv[1];
globalThis.I18N = { lang: process.argv[3], t: (k, o) => k + JSON.stringify(o || {}) };
globalThis.NOMI = require(web + '/nomi.js');
const L = require(web + '/liste.js');
const lists = JSON.parse(fs.readFileSync(process.argv[2], 'utf8')).lists;
globalThis.LISTE = { ensure: () => L.buildIndex(lists, null) };
vm.runInThisContext(fs.readFileSync(web + '/liste-nomi.js', 'utf8') + ';globalThis.LISTENOMI = LISTENOMI;');
const names = ['imidacloprid', 'thiamethoxam', 'clothianidin', 'acetamiprid', 'thiacloprid', 'dinotefuran'];
console.log(JSON.stringify({ hit: LISTENOMI.byName('imidacloprid'), plus: LISTENOMI.byName('Imidacloprid [M+Na]+'),
  sug: LISTENOMI.suggest('imidaclo').map(r => r.name), all: names.map(n => !!LISTENOMI.byName(n)) }));
"""
    import subprocess
    r = subprocess.run([NODE, "-e", code, str(ROOT / "mzlab" / "web"), str(ROOT / "mzlab" / "web" / "contaminanti.json"), lang],
                       capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    o = json.loads(r.stdout)
    assert o["hit"] == {"formula": "C9H10ClN5O2", "adduct": "[M+H]+"}
    assert o["plus"] == {"formula": "C9H10ClN5O2", "adduct": "[M+Na]+"}
    assert o["sug"] == ["imidacloprid"] and all(o["all"])


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_a_name_with_its_bracket_is_found_by_either_part():
    """«dibutilftalato (DBP)» of Keller: the name before the bracket and the short name inside it are the same entry; a bracket word shared by
    many entries («dimero») finds none; the full name still works."""
    code = r"""
const fs = require('fs'), vm = require('vm'), web = process.argv[1];
globalThis.I18N = { lang: 'it', t: (k, o) => k + JSON.stringify(o || {}) };
globalThis.NOMI = require(web + '/nomi.js');
const L = require(web + '/liste.js');
globalThis.LISTE = { ensure: () => L.buildIndex(JSON.parse(fs.readFileSync(process.argv[2], 'utf8')).lists, null) };
vm.runInThisContext(fs.readFileSync(web + '/liste-nomi.js', 'utf8') + ';globalThis.LISTENOMI = LISTENOMI;');
const f = t => (LISTENOMI.byName(t) || {}).formula || null;
console.log(JSON.stringify([f('dibutilftalato (DBP)'), f('dibutilftalato'), f('DBP'), f('dibutyl phthalate'), f('dimero'), f('DBP [M+Na]+')]));
"""
    import subprocess
    r = subprocess.run([NODE, "-e", code, str(ROOT / "mzlab" / "web"), str(ROOT / "mzlab" / "web" / "contaminanti.json")],
                       capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    assert json.loads(r.stdout) == ["C16H22O4"] * 4 + [None, "C16H22O4"]
