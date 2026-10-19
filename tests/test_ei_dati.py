"""The EI spectra of the Teoria and of the Pratica (mzlab/web/teoria/pratica/ei-dati.js, built by tools/genera_ei.py).

The teaching notes ("keys") are written by hand: these checks are the guard against chemistry mistakes. For every key ion:
nominal mass of its formula (+ delta for an isotope peak) = its m/z; odd/even electrons from the rings-plus-double-bonds value
agree with the charge notation ('+.' radical cation, '+' cation) and with the mechanism; the ion is a sub-formula of the
compound (one extra H allowed for hydrogen-transfer rearrangements); the peak is really in the chosen spectrum.
"""
import json
import re
import sys
from pathlib import Path

import pytest

from mzlab.chem.elements import parse_formula, mass

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from dati_percorsi import percorso  # noqa: E402
DATA = ROOT / "mzlab" / "web" / "teoria" / "pratica" / "ei-dati.js"
NOMINAL = {"H": 1, "C": 12, "N": 14, "O": 16, "F": 19, "Si": 28, "P": 31, "S": 32, "Cl": 35, "Br": 79, "I": 127}
AIUTI = ROOT / "mzlab" / "web" / "teoria" / "pratica" / "ei-aiuti.js"
SKILLS = {"M", "isotopi", "azoto", "rdb", "alpha", "induttiva", "sigma", "mclafferty", "rda", "benzilica", "orto", "serie", "perdita", "riarr", "m-assente"}
LICENSES = {"CC BY-NC-SA", "CC BY-NC", "CC BY-SA", "CC BY", "CC0"}
ODD = {"M", "mclafferty", "rda", "orto"}                 # mechanisms that give odd-electron ions
EVEN = {"alpha", "i", "sigma", "tropilio", "serie"}      # simple cleavages: even-electron ions
TYPES = ODD | EVEN | {"perdita", "riarr", "isotopo"}


def load():
    """ei-dati.js plus the per-level files ei-dati-l1..l5.js (EI_DATA.items.push(...[...]))."""
    txt = DATA.read_text(encoding="utf-8")
    body = txt[txt.index("const EI_DATA = ") + len("const EI_DATA = "):].rstrip().rstrip(";")
    d = json.loads(body)
    for f in sorted(DATA.parent.glob("ei-dati-l*.js")):
        t = f.read_text(encoding="utf-8")
        d["items"] += json.loads(t[t.index("EI_DATA.items.push(...") + len("EI_DATA.items.push(..."):].rstrip().rstrip(";")[:-1])
    return d


def split_ion(ion: str):
    m = re.fullmatch(r"([A-Za-z0-9]+)(\+\.?)", ion)
    assert m, ion
    return dict(parse_formula(m.group(1))), m.group(2) == "+."


def rdb(f: dict) -> float:
    c = f.get("C", 0) + f.get("Si", 0)
    x = f.get("H", 0) + f.get("F", 0) + f.get("Cl", 0) + f.get("Br", 0) + f.get("I", 0)
    n = f.get("N", 0) + f.get("P", 0)
    return c - x / 2 + n / 2 + 1


def test_file_and_sources():
    d = load()
    items = d["items"]
    assert len(items) >= 170 and sum(i["play"] for i in items) >= 150
    for f in [DATA, *DATA.parent.glob("ei-dati-l*.js")]:
        assert f.stat().st_size < 160_000, (f.name, "past ~150 kB: split further, the page loads only the data of the chosen level")
    assert "NIST" not in "".join(f.read_text(encoding="utf-8").upper() for f in DATA.parent.glob("ei-dati-l*.js"))
    ext = d["extra"]
    assert ext and all(any(i["id"] == k for i in items) for k in ext)
    for f in DATA.parent.glob("ei-dati-l*.js"):
        lv = int(f.stem.rsplit("-l", 1)[1])
        t = f.read_text(encoding="utf-8")
        assert all(v == lv for k, v in ext.items() if '"id":"%s"' % k in t), f.name
    assert "NIST" not in DATA.read_text(encoding="utf-8").upper().replace("NIST SPECTRA", "")
    ids = [i["id"] for i in items]
    assert len(ids) == len(set(ids))
    for it in items:
        f = dict(parse_formula(it["formula"]))
        assert it["M"] == sum(NOMINAL[e] * n for e, n in f.items()), it["id"]
        assert abs(mass(parse_formula(it["formula"])) - it["M"]) < 0.5
        assert it["src"]["accession"].startswith("MSBNK-") and it["src"]["license"], it["id"]
        mzs = [p[0] for p in it["peaks"]]
        assert mzs == sorted(mzs) and max(p[1] for p in it["peaks"]) == 999, it["id"]
        assert max(mzs) <= it["M"] + 12, it["id"]                     # nothing far above the molecular ion cluster
        assert it["play"] is False or it["keys"], it["id"]


def test_keys_are_chemically_consistent():
    for it in load()["items"]:
        comp = dict(parse_formula(it["formula"]))
        peaks = dict((m, v) for m, v in it["peaks"])
        for k in it["keys"]:
            assert k["type"] in TYPES, (it["id"], k)
            f, odd = split_ion(k["ion"])
            nominal = sum(NOMINAL[e] * n for e, n in f.items())
            assert nominal + k.get("delta", 0) == k["mz"], (it["id"], k["mz"], k["ion"], nominal)
            r = rdb(f)
            assert r >= 0, (it["id"], k)
            assert (r == int(r)) == odd, (it["id"], k["ion"], "RDB", r)
            if k["type"] in ODD:
                assert odd, (it["id"], k)
            if k["type"] in EVEN:
                assert not odd, (it["id"], k)
            for e, n in f.items():
                assert n <= comp.get(e, 0) + (1 if e == "H" else 0), (it["id"], k["ion"], e)
            need = 2 if k["type"] == "M" else 10
            assert peaks.get(k["mz"], 0) >= need, (it["id"], k["mz"], peaks.get(k["mz"]))
            assert k["text"] and len(k["text"]) < 260


def test_no_compound_of_the_lab_methods():
    """The pollutants of the lab methods must not appear (their names are read from the private data repository, if present)."""
    dam = percorso("dati/3200 qtrap/dam")
    legacy = ROOT.parent / "Data" / "dam - Metodi"          # outside the data repository: not in the layout table
    if legacy.is_dir():
        dam = legacy
    if dam is None:
        pytest.skip("repository privato dei dati non presente")
    names = {m.group(1).lower() for p in dam.glob("*.dam") if (m := re.match(r"Lab_inq_MS2_([A-Za-z]+)\.dam$", p.name))}
    assert names
    text = json.dumps(load(), ensure_ascii=False).lower()
    for n in names:
        assert n[:7] not in text, n[:3] + "…"


def test_every_spectrum_cited_in_the_theory_exists():
    import re as _re
    ids = {i["id"] for i in load()["items"]}
    for page in (ROOT / "mzlab" / "web" / "teoria").glob("*.html"):
        for ref in _re.findall(r'data-ei="([^"]+)"', page.read_text(encoding="utf-8")):
            assert ref in ids, (page.name, ref)


def aiuti():
    return dict(re.findall(r'^  "([^"]+)": "(.*)",$', AIUTI.read_text(encoding="utf-8"), re.M))


def test_molecular_ion_and_nitrogen_rule():
    """Every compound: the M key (if any) is the formula itself as a radical cation; nominal M parity = parity of the N count; RDB integer >= 0."""
    for it in load()["items"]:
        f = dict(parse_formula(it["formula"]))
        assert (it["M"] % 2) == (f.get("N", 0) % 2), (it["id"], "regola dell'azoto")
        r = rdb(f)
        assert r >= 0 and r == int(r), (it["id"], r)
        for k in it["keys"]:
            if k["type"] == "M":
                ion, odd = split_ion(k["ion"])
                assert odd and ion == f and k["mz"] == it["M"], (it["id"], k)
            if k["type"] == "isotopo":
                assert any(e in f for e in ("Cl", "Br", "S")) and k["delta"] in (2, 4), (it["id"], k)


def test_path_of_simple_compounds():
    """Path levels 1-5: ordered, playable, with skill tags derived from the validated keys, licence and accession of MassBank."""
    items = load()["items"]
    path = [i for i in items if "path" in i]
    assert {i["path"] for i in path} == {1, 2, 3, 4, 5}
    for lv in range(1, 6):
        grp = sorted((i for i in path if i["path"] == lv), key=lambda i: i["order"])
        assert len(grp) >= 6, lv
        assert [i["order"] for i in grp] == list(range(len(grp))), lv
    for it in path:
        assert it["play"] and it["keys"], it["id"]
        assert it["skills"] and set(it["skills"]) <= SKILLS and "M" in it["skills"], (it["id"], it["skills"])
        f = dict(parse_formula(it["formula"]))
        assert ("isotopi" in it["skills"]) == any(e in f for e in ("Cl", "Br", "S")), it["id"]
        assert ("azoto" in it["skills"]) == ("N" in f), it["id"]
        assert it["smiles"] and it["inchikey"], it["id"]
        assert it["src"]["license"] in LICENSES and it["src"]["accession"].startswith("MSBNK-"), it["id"]
    # the ids that existed before the path keep their id (progress in localStorage qqq.pratica)
    ids = {i["id"] for i in items}
    assert {"acetofenone", "1-butanolo", "2-esanone", "toluene", "clorobenzene", "dietil-etere", "decano"} <= ids


def test_key_notes_are_not_direct_answers():
    """A note never says the name of its compound (the notes are shown only after the student's own answer)."""
    for it in load()["items"]:
        name = it["name"].lower()
        for k in it["keys"]:
            assert name not in k["text"].lower(), (it["id"], k["text"])


def test_hints_of_the_hard_mode_do_not_reveal_the_structure():
    """Level 3 of the hints: one sentence per compound of the path, no m/z, no name, no formula, no class or group word."""
    h = aiuti()
    forbidden = re.compile(r"\balcol|\bammin|\bchetone|\baldeid|\beter[ei]\b|\bester|\bacid[oi]\b|\bnitril|\bammid|\balogen|\baromatic|\bbenzen|\banell|\banello|\bcicl|\bcarbonil|\bossidril|\bnitro|\btiol|\bsolfur|\bfenol|\balchil|\bcatena|\bsimmetr|\bramific|\bdoppio legame|\bcarbossil|\bfenil", re.I)
    path = [i for i in load()["items"] if "path" in i]
    assert set(h) >= {i["id"] for i in path}
    assert set(h) <= {i["id"] for i in load()["items"]}
    for it in path:
        t = h[it["id"]]
        assert 40 < len(t) < 330, it["id"]
        assert not re.search(r"\d{2,}", t), (it["id"], "a number in the hint")
        assert it["name"].lower() not in t.lower() and it["formula"] not in t, it["id"]
        assert not forbidden.search(t), (it["id"], forbidden.search(t).group(0))
