"""Extended synthetic series V1X (tools/valida/sintetici_esteso*.py): V1 unchanged, determinism, counts, chains, in-source fragments of the products, halogens, close isomers."""
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
from valida import sintetici_esteso as X  # noqa: E402
from valida import sintetici_esteso_verita as XV  # noqa: E402
from valida import sintetici_identita as SI  # noqa: E402
from mzlab.chem import elements as E  # noqa: E402

SEME = 7
V1 = {("piccolo", 7): ("cf1136fa5b4f525304eb524cc03f24e120da8f04af3eea7011b006b6e128f7e6", "a7775fb611a931e530016ba553771a5f5a17aef89c807a4f18d3c32fde16b860"),
      ("completo", 7): ("28ed1fb05944bd3fdf43ffbf82f9ed15db25d9af2f9e876a5383313113abb7d5", "2a4bb2658124cd6c10e20f14791306fdd9ef7990f24e74ade5a204d8e6dc2b12")}      # SHA-256 of verita.csv / verita.json before the extension


def _sha(d: Path) -> dict:
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(d.iterdir())}


@pytest.fixture(scope="module")
def piccolo(tmp_path_factory):
    return SI.genera(SEME, "V1X-piccolo", tmp_path_factory.mktemp("v1xp"))


@pytest.fixture(scope="module")
def completo():
    return SI.costruisci(SEME, "V1X")


@pytest.fixture(scope="module")
def verita(piccolo):
    d = json.loads((piccolo / "verita_tp.json").read_text(encoding="utf-8"))
    est = json.loads((piccolo / "righe_esterne.json").read_text(encoding="utf-8")) if (piccolo / "righe_esterne.json").exists() else {}
    return {k: est[k] if v == "righe_esterne" else v for k, v in d.items()}


@pytest.mark.parametrize("profilo", ["piccolo", "completo"])
def test_v1_is_unchanged(profilo, tmp_path):
    c = SI.costruisci(SEME, profilo)
    SI.scrivi_verita(tmp_path, c, SEME, profilo)
    assert c.vx is None and (_sha(tmp_path)["verita.csv"], _sha(tmp_path)["verita.json"]) == V1[(profilo, SEME)]


def test_v1x_keeps_the_v1_model_ions(completo):
    base = SI.costruisci(SEME, "completo")
    por = {o.id: o for o in completo.ioni}
    assert all(o.id in por and por[o.id].mz == o.mz for o in base.ioni if o.ruolo in ("progenitore", "tp1", "tp2", "isomero", "chimerica"))      # the same molecules, same m/z
    assert (len(completo.ioni) - len(base.ioni)) < 0.3 * len(base.ioni)                                     # the new rows are less than 30% of the existing ones


def test_two_runs_same_bytes(piccolo, tmp_path):
    assert _sha(SI.genera(SEME, "V1X-piccolo", tmp_path)) == _sha(piccolo)
    assert {"verita_tp.json", "manifest.json"} <= set(_sha(piccolo))


def test_counts_generator_expected_and_file(completo, verita, piccolo):
    for c, att in ((SI.costruisci(SEME, "V1X-piccolo"), verita["metriche_attese"]), (completo, XV.contatori(completo))):
        assert XV.contatori(c) == att
    c = SI.costruisci(SEME, "V1X-piccolo")
    ric = Counter(t["gen_attesa"] for t in verita["tps"])
    assert {int(g): n for g, n in att_gen(verita).items()} == dict(ric)
    iso = Counter(f"{g[0]['rt_separabile']:g}" for g in _gruppi(verita).values())
    assert dict(iso) == verita["metriche_attese"]["isomeri_per_delta_rt"]
    ere = verita["ereditati"]
    assert sum(1 for e in ere if e["tipo"] == "isf" and e["rapporto_costante"] and e["padre"] != "parent") >= c.vx.cont["isf_costanti"]
    assert sum(1 for e in ere if e["tipo"] == "isf" and not e["rapporto_costante"]) == verita["metriche_attese"]["isf_tp"]["variabili"] == c.vx.cont["isf_variabili"]
    assert json.loads((piccolo / "manifest.json").read_text())["sha256"]["verita_tp.json"] == hashlib.sha256((piccolo / "verita_tp.json").read_bytes()).hexdigest()
    assert verita["meta"]["budget_mb"] > 0


def att_gen(v):
    return v["metriche_attese"]["tps_per_generazione"]


def _gruppi(v):
    g = {}
    for r in v["righe_isomeri"]:
        g.setdefault(r["gruppo"], []).append(r)
    return g


def test_chains(completo):
    st, por = completo.vx, {o.id: o for o in completo.ioni}
    assert st.cont["catene3"] >= 6 and st.cont["catene2"] >= 12
    gen3 = [o for o in completo.ioni if o.ruolo == "tp3"]
    assert len(gen3) >= 6 and any(st.meta[o.id].get("debole") for o in gen3)
    for o in completo.ioni:
        if o.ruolo not in ("tp2", "tp3") or o.id not in st.meta:
            continue
        pred = por[o.progenitore]
        assert pred.generazione == o.generazione - 1 and o.generazione in (2, 3)
        delta = {k: v for k, v in XV.delta_ioni(o, pred).items()}
        assert XV.classe_di(delta) is not None or st.meta[o.id]["via"] == "scissione"                      # each step is a rule of the table
        tot = st.tot[o.id]
        assert X.chiave(tot) not in X.UN_PASSO                                                              # never one rule from the progenitor
        if o.generazione == 3:
            assert X.chiave(tot) not in X.DUE_PASSI and st.meta[o.id]["raggiungibile"] is False         # nor two: the observed network is needed
    tm = {o.id: st.meta[o.id]["t_max"] for o in completo.ioni if o.id in st.meta and st.meta[o.id].get("t_max") is not None and o.ruolo in ("tp1", "tp2", "tp3")}
    for o in gen3:                                                                                          # delayed maxima
        assert tm[o.id] > tm[o.progenitore] > tm[por[o.progenitore].progenitore]
    debole = next(o for o in gen3 if st.meta[o.id].get("debole"))
    assert 3 * 6e5 * 0.999 <= debole.amp.max() <= 10 * 6e5 * 1.001                                         # 3-10 times the noise
    fam = [o for o in completo.ioni if o.id in st.meta and st.meta[o.id].get("via") == "scissione" and o.ruolo == "tp1" and o.generazione == 1]
    assert fam and sum(1 for o in completo.ioni if o.ruolo == "tp2" and o.progenitore == fam[0].id) >= 3      # the cleavage family


def test_isf_of_the_products(completo):
    por = {o.id: o for o in completo.ioni}
    costanti = [o for o in completo.ioni if o.ruolo == "isf" and o.id.endswith("_ISF1") and por[o.progenitore].ruolo in ("tp1", "tp2", "tp3")]
    assert len(costanti) >= 20
    for o in costanti:
        b = por[o.progenitore]
        m = (b.amp > 0)
        r = o.amp[m] / b.amp[m]
        assert 0.03 <= r[0] <= 0.3 and r.std() / r.mean() <= 0.1 and abs(o.rt - b.rt) <= 0.01
        assert o.isf_in_ms2 == 1 and np.any(np.abs(b.frammenti[0] - o.mz) < 1e-6)
    var = [o for o in completo.ioni if o.forma == "isf_variabile"]
    assert len(var) == 6
    for v in var:
        t = por[completo.vx.ere[v.id]["coeluisce_con_tp"]]
        b = por[v.progenitore]
        assert t.ion == v.ion and abs(t.rt - v.rt) < 1e-9 and abs(v.rt - b.rt) <= 0.01
        m = b.amp > 0
        r = t.amp[m] / b.amp[m]
        assert r.std() / r.mean() > 0.2                                                                     # not constant: a real product under the fragment
    p = por["SIN-001"]
    casc = sorted((o for o in completo.ioni if "_CISF" in o.id), key=lambda o: o.id)
    assert len(casc) >= 3 and all(abs(o.rt - p.rt) <= 0.004 for o in casc)


def test_halogens(completo):
    st = completo.vx
    pat = {"Cl": ["Cl"], "Br": ["Br"], "ClBr": ["Br", "Cl"], "Cl2": ["Cl", "Cl"]}
    for nome, alog in pat.items():
        pars = [o for o in completo.ioni if o.ruolo == "progenitore_alog" and st.meta[o.id]["specie"] == nome]
        assert len(pars) >= 2 and all(st.meta[o.id]["alogeni"] == alog for o in pars)
        de = [o for o in completo.ioni if o.ruolo == "tp1" and st.meta.get(o.id, {}).get("specie") == nome and st.meta[o.id]["classe"].startswith("dealogenazione")]
        assert len(de) >= 3 and {st.meta[o.id]["classe"].rsplit("_", 1)[1] for o in de} == {"H", "OH"}
        for o in de:
            assert len(st.meta[o.id]["alogeni"]) == len(alog) - 1 and st.meta[o.id]["delta"] in ("-Cl+H", "-Cl+OH", "-Br+H", "-Br+OH")


def _nominale(formula: str) -> np.ndarray:
    """Relative abundance of the nominal groups M..M+3 of a formula, convolved from the isotope abundances (independent of the program's isotope model)."""
    iso = {"C": [.9893, .0107], "H": [.999885, .000115], "N": [.99636, .00364], "O": [.99757, .00038, .00205], "S": [.9499, .0075, .0425, 0, .0001], "Cl": [.7576, 0, .2424], "Br": [.5069, 0, .4931]}
    p = np.array([1.0])
    for e, n in E.parse_formula(formula).items():
        for _ in range(n):
            p = np.convolve(p, np.array(iso[e]))[:6]
    return p[:4] / p[:4].sum()


def test_isotope_patterns(completo):
    for o in completo.ioni:
        if o.ruolo not in ("progenitore_alog", "tp1") or not any(e in E.parse_formula(o.ion) for e in ("Cl", "Br")):
            continue
        m0 = min(m for m, _ in o.picchi)
        g = np.zeros(4)
        for m, r in o.picchi:
            if int(round(m - m0)) < 4:
                g[int(round(m - m0))] += r
        g /= g.sum()
        ref = _nominale(o.ion)
        assert np.all(np.abs(g[:3] - ref[:3]) / ref[:3] < 0.02), (o.ion, g, ref)


def test_close_isomers(completo):
    por = {o.id: o for o in completo.ioni}
    gruppi = {}
    for o in completo.ioni:
        if o.ruolo == "isomero":
            gruppi.setdefault(o.famiglia, []).append(o)
    nuovi = [m for m in gruppi.values() if "fwhm" in m[0].note]
    assert Counter(round(m[1].rt - m[0].rt, 2) for m in nuovi) == {d: 3 for d in (0.05, 0.1, 0.15, 0.2, 0.3)}
    for mem in nuovi:
        d = mem[1].rt - mem[0].rt
        note = dict(p.split("=") for p in mem[0].note.split(";"))
        assert 0.08 <= float(note["fwhm"]) <= 0.15 and abs(float(note["sigma"]) * 2.3548 - float(note["fwhm"])) < 1e-3
        assert len({o.mz for o in mem}) == 1 and all(abs((b.rt - a.rt) - d) < 1e-9 for a, b in zip(mem, mem[1:]))
        t = np.linspace(mem[0].rt - 0.5, mem[-1].rt + 0.5, 20001)
        y = sum(o.amp[-1] / mem[0].amp[-1] * np.exp(-0.5 * ((t - o.rt) / o.sg) ** 2) for o in mem)
        picchi = int(np.sum((y[1:-1] > y[:-2]) & (y[1:-1] >= y[2:])))
        assert (picchi == len(mem)) == bool(int(note["separabile"]))                                        # declared separable <=> as many apexes as isomers
        if d >= 0.1:
            assert note["separabile"] == "1"
        if d == 0.05:
            assert note["separabile"] == "0"                                                                 # the FWHM is wider than the distance
        f = [set(np.round(o.frammenti[0], 4)) for o in mem]
        assert len(f[0]) == 10 and len(f[1] & f[0]) / 10 >= 0.6
        if len(mem) == 3:
            assert len(f[2] & f[0]) / 10 <= 0.3 + 1e-9
    assert sum(1 for m in nuovi if len(m) == 3) >= 3 and por


def test_inherited_that_look_like_products(piccolo, verita):
    ere = {e["id"]: e for e in verita["ereditati"]}
    trav = [e for e in verita["ereditati"] if e.get("travestito")]
    assert trav and {e["delta_rt"] for e in trav} <= {0.0, 0.02}
    assert {e["tipo"] for e in trav} >= {"isf", "addotto", "isotopologo"}
    tps = {t["id"]: t for t in verita["tps"]}
    for e in trav:
        if e["tipo"] != "isotopologo":
            padre = tps[e["padre"]]
            assert abs(e["rt_atteso"][0] - padre["rt_obs"][0] - e["delta_rt"]) < 1e-3
    assert all(e["rapporto_costante"] for e in trav) and not any(e["id"] in tps for e in ere.values())


def test_no_compound_names(piccolo):
    nomi = set()
    for riga in (ROOT / "tools" / "composti_nomi.txt").read_text(encoding="utf-8").splitlines():
        if riga and not riga.startswith("#"):
            nomi |= {x.strip().lower() for x in riga.split(";") if len(x.strip()) >= 5}
    testo = "\n".join(p.read_text(encoding="utf-8") for p in list(piccolo.glob("*.json")) + list((ROOT / "tools" / "valida").glob("sintetici_esteso*.py"))).lower()
    assert [n for n in nomi if re.search(r"(?<![a-z])" + re.escape(n) + r"(?![a-z])", testo)] == []
