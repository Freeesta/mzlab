"""WP-H11: the list of known contaminants (m/z computed from the formulas), the series and the matching engine (web/liste.js run by node), the CSV of the laboratory."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import genera_contaminanti as g  # noqa: E402
from mzlab.chem.contaminants import builtin, expand  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
CHEM = ROOT / "mzlab" / "chem"


def one(i, ad, n=None):
    r = [x["mz"] for x in builtin()["items"] if x["id"] == i and x["adduct"] == ad and (n is None or (x["series"] or {}).get("n") == n)]
    assert len(r) == 1, (i, ad, r)
    return r[0]


def test_valori_calcolati_del_prompt():
    assert one("siloxano_ciclico", "[M+H]+", 6) == pytest.approx(445.1200, abs=5e-5)
    assert one("siloxano_ciclico", "[M+NH4]+", 6) == pytest.approx(462.1466, abs=5e-5)
    assert one("erucamide", "[M+H]+") == pytest.approx(338.3417, abs=5e-5)
    assert one("dbp", "[M+H]+") == pytest.approx(279.1591, abs=5e-5)
    assert one("dehp", "[M+H]+") == pytest.approx(391.2843, abs=5e-5)
    assert one("dehp", "[M+Na]+") == pytest.approx(413.2662, abs=5e-5)
    assert one("peg", "[M+NH4]+", 8) == pytest.approx(388.2541, abs=5e-5)


def test_serie_peg_e_doppia_carica():
    a, b = one("peg", "[M+NH4]+", 8), one("peg", "[M+NH4]+", 9)
    assert b - a == pytest.approx(44.02621, abs=1e-4)
    # [M+2H]2+ of n=12: (M + 2 H+) / 2
    m = 12 * 44.0262 + 18.0106
    assert one("peg", "[M+2H]2+", 12) == pytest.approx((m + 2 * 1.007276) / 2, abs=1e-3)
    assert one("siloxano_ciclico", "[M+H]+", 7) - one("siloxano_ciclico", "[M+H]+", 6) == pytest.approx(74.0188, abs=1e-3)


def test_polarita_negativa_e_solo_mz():
    it = [x for x in builtin()["items"] if x["id"] == "phosphoric_acid"][0]
    assert it["pol"] == -1 and it["mz"] == pytest.approx(96.9696, abs=5e-4)
    assert one("tfa", "[M-H]-") == pytest.approx(112.9856, abs=5e-4)
    sm = expand({"id": "x", "name": "solo m/z", "formula": None, "mz": 123.4567, "polarity": "negative", "source": "prova"})
    assert len(sm) == 1 and sm[0]["mzonly"] and sm[0]["pol"] == -1 and sm[0]["mz"] == 123.4567
    assert {x["pol"] for x in builtin()["items"]} == {1, -1}


def test_voci_keller_si_espandono_e_combaciano():
    """Every entry comes from Keller 2008 and expands; masses computed by us (not copied from the table) of some of the entries added after the comparison."""
    d = json.loads((ROOT / "mzlab/chem/contaminants.json").read_text(encoding="utf-8"))
    assert all(e["source"] == "Keller 2008" for e in d["entries"])
    assert len({e["id"] for e in d["entries"]}) == len(d["entries"])
    for e in d["entries"]:
        its = expand(e)
        assert its and all(x["mz"] > 0 for x in its), e["id"]
    assert one("tween_c18h34o6", "[M+Na]+", 10) == pytest.approx(809.4869, abs=2e-3)
    assert one("tween_c24h44o6", "[M+Na]+", 20) == pytest.approx(1331.8273, abs=2e-3)
    assert one("trit_c15h24o", "[M+H]+", 5) == pytest.approx(441.3211, abs=2e-3)
    assert one("siloxano_ciclico", "[M+H-CH4]+", 6) == pytest.approx(429.0887, abs=2e-3)
    assert one("pep_slpr", "[M+H]+") == pytest.approx(472.2878, abs=2e-3)
    assert one("diisoottilftalato_dimero", "[M+Na]+") == pytest.approx(803.5432, abs=2e-3)
    assert one("dehp", "[M+CH3CN+Na]+") == pytest.approx(454.2928, abs=2e-3)


def test_ogni_voce_ha_fonte_e_ioni():
    d = json.loads((ROOT / "mzlab/chem/contaminants.json").read_text(encoding="utf-8"))
    for e in d["entries"]:
        assert e["source"] and e["id"]
        if e.get("formula") is not None:
            assert e["ions"], e["id"]
    assert len(expand(d["entries"][0])) > 100                      # a polymer gives one set of ions for each n


NODE = shutil.which("node")


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_motore_liste_con_node(tmp_path):
    code = r"""
const L = require(process.argv[1]);
const items = JSON.parse(require('fs').readFileSync(process.argv[2], 'utf8'));
const idx = L.buildIndex([{ id: "keller", name: "K", items }, L.labList([{ mz: 123.4567, name: "mio", formula: "", polarity: 1, note: "bianco" }])]);
const out = {};
out.hit = L.find(idx, "positive", 445.1200, 5).map(x => x.name);
out.miss = L.find(idx, "positive", 445.2200, 5).length;
out.neg = L.find(idx, "negative", 445.1200, 5).length;
out.lab = L.find(idx, "positive", 123.4567, 5).map(x => x.list);
out.tolLow = L.tolDa(100, 5);
const peg = items.filter(x => x.id === "peg" && x.adduct === "[M+NH4]+").sort((a, b) => a.series.n - b.series.n);
const mzs = peg.filter(x => x.series.n >= 7 && x.series.n <= 11).map(x => x.mz).concat([500.5]);
const runs = L.seriesRuns(idx, "positive", mzs, 5);
out.runs = [...new Set(runs.values())];
out.two = L.seriesRuns(idx, "positive", peg.filter(x => x.series.n === 7 || x.series.n === 9).map(x => x.mz), 5).size;
out.tip = L.tipHtml(L.find(idx, "positive", 445.1200, 5), runs).includes("Compatibile con un contaminante noto");
out.none = L.tipHtml([], runs);
const csv = L.csvOut([{ mz: 100.1, name: 'a, "b"', formula: "C2H6", polarity: -1, note: "x" }]);
out.csv = L.csvParse(csv);
out.csv2 = L.csvParse("mz;name\n150,5;punto e virgola");
console.log(JSON.stringify(out));
"""
    items = builtin()["items"]
    f = tmp_path / "items.json"; f.write_text(json.dumps(items), encoding="utf-8")
    r = subprocess.run([NODE, "-e", code, str(ROOT / "mzlab/web/liste.js"), str(f)], capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    o = json.loads(r.stdout)
    assert any("polisilossano" in n for n in o["hit"]) and o["miss"] == 0 and o["neg"] == 0
    assert o["lab"] == ["laboratorio"] and o["tolLow"] == 0.001
    assert o["runs"] == ["serie PEG (n = 7-11)"] and o["two"] == 0          # three consecutive members are needed
    assert o["tip"] and o["none"] == ""
    assert o["csv"] == [{"mz": 100.1, "name": 'a, "b"', "formula": "C2H6", "polarity": -1, "note": "x"}]
    assert o["csv2"][0]["mz"] == 150.5 and o["csv2"][0]["name"] == "punto e virgola"


def test_static_file_equals_the_route():
    from mzlab.api import dispatch
    from mzlab.app import App
    code, _ctype, body, _h = dispatch(App(), "GET", "/api/contaminants", {})
    assert code == 200
    assert g.OUT.read_text(encoding="utf-8") == body.decode("utf-8") == g.text()
    assert len(json.loads(g.OUT.read_text(encoding="utf-8"))["lists"]) == 1


def test_every_entry_has_english_name_and_class():
    d = json.loads((CHEM / "contaminants.json").read_text(encoding="utf-8"))
    en = json.loads((CHEM / "contaminants_en.json").read_text(encoding="utf-8"))
    ids = [e["id"] for e in d["entries"]]
    assert not [i for i in ids if not en["names"].get(i)], "entries without an English name"
    assert not [e["class"] for e in d["entries"] if e["class"] not in en["classes"]], "classes without an English name"
    assert not set(en["names"]) - set(ids), "English names of entries that do not exist"
    assert en["list"]["name"] and en["list"]["license"]


def test_builtin_carries_english_fields():
    b = builtin()
    assert b["name_en"] and all(i["name_en"] for i in b["items"] if not i.get("mzonly")) and all(i["cls_en"] for i in b["items"])
