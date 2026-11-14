"""The structure bridge of mzFinder in WebAssembly (crates/mzlab-feat-grafo-wasm) answers a SMILES with the formula, the groups and the cleavage products that
the Rust library gives natively, the piece labels (also with the delta of a bracket), the bond cuts and the analysis card (`anal/1`, every golden molecule) the selection info (`sel/1`) and the lookup URLs (`link/1`, every golden case) of the Drawing byte for byte like the CLI (checked by the formula and the sets), and a bad SMILES with a JSON error. Skipped without cargo, the wasm32 target,
wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import build, cli

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
PARENT = "CC(C)(C)NCC(COc1nsnc1N1CCOCC1)O"


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    out = tmp_path_factory.mktemp("grafo")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-grafo-wasm", runner=run)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_feat_grafo_wasm.wasm")
    return out


def analyse(pkg, smiles):
    return json.loads(run("node", "tests/feat_grafo_node.js", str(pkg), smiles))


def test_parent_through_wasm(pkg):
    r = analyse(pkg, PARENT)
    assert r["format"] == "grafo/1" and r["formula"] == "C13H24N4O3S"
    assert {"n_tbutyl", "ring_hetero_saturated", "alcohol", "ether", "sulfide"} <= set(r["groups"])
    assert "C4H11N" in r["fragments"] and r["fragments"] == sorted(set(r["fragments"]))
    assert all(c["kind"] in ("cut", "ether", "ester", "amide", "sulfonamide") for c in r["cuts"])


def test_error_is_json(pkg):
    assert analyse(pkg, "C1CC")["error_key"] == "error.grafo.smiles"
    assert analyse(pkg, "C.C")["error_key"] == "error.grafo.mixture"


ETHANOL = "\n  qqq\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n" + "".join(f"{x:10.4f}{0:10.4f}{0:10.4f} {el:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n" for x, el in ((0, "C"), (1, "C"), (2, "O"))) + "  1  2  1  0  0  0  0\n  2  3  1  0  0  0  0\nM  END"


def test_fragments_of_a_molfile(pkg):
    r = json.loads(run("node", "tests/feat_grafo_node.js", str(pkg), ETHANOL, "1", "2"))
    (cut,) = r["cuts"]
    assert r["format"] == "frag/1" and (cut["a"], cut["b"]) == (1, 2)
    assert [s["formula"] for s in cut["sides"]] == ["C2H6", "H2O"] and cut["sides"][0]["atoms"] == [0, 1]
    assert cut["sides"][1]["mz"] == pytest.approx(19.0178, abs=1e-3)
    assert len(json.loads(run("node", "tests/feat_grafo_node.js", str(pkg), ETHANOL, "-1", "-1"))["cuts"]) == 2


def test_fragments_keep_charge_and_isotope(pkg):
    mol = lambda els, bonds, props: "\n  qqq\n\n%3d%3d  0  0  0  0  0  0  0  0999 V2000\n" % (len(els), len(bonds)) + "".join(f"{i:10.4f}{0:10.4f}{0:10.4f} {el:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n" for i, el in enumerate(els)) + "".join("%3d%3d%3d  0  0  0  0\n" % b for b in bonds) + "".join(p + "\n" for p in props) + "M  END"
    (cut,) = json.loads(run("node", "tests/feat_grafo_node.js", str(pkg), mol("CN", [(1, 2, 1)], ["M  CHG  1   2   1"]), "0", "1"))["cuts"]
    assert [(s["formula"], s["mz"]) for s in cut["sides"]] == [("CH4", pytest.approx(17.0386, abs=1e-4)), ("H4N+", pytest.approx(18.0338, abs=1e-4))]
    (cut,) = json.loads(run("node", "tests/feat_grafo_node.js", str(pkg), mol("CCOO", [(1, 2, 1), (2, 3, 2), (2, 4, 1)], ["M  ISO  1   1  13"]), "0", "1"))["cuts"]
    assert cut["sides"][0]["formula"] == "13CH4" and cut["sides"][0]["mass"] == pytest.approx(17.0347, abs=1e-4)


def test_fragments_of_a_radical_equal_the_app_fixture(pkg):
    fx = json.loads((Path(__file__).parent / "dati" / "frag_radicale.json").read_text(encoding="utf-8"))
    assert run("node", "tests/feat_grafo_node.js", str(pkg), fx["molfile"], "-1", "-1").strip() == fx["frag"]   # crates/mzlab-desktop/tests/labels.rs reads the same bytes
    assert '"formula":"CH3O","mass":31.0184,"mz":32.0257,"odd":true' in fx["frag"]


def test_mechanism_check_through_wasm_equals_the_app_fixture_and_the_cli(pkg, tmp_path):
    fx = json.loads((Path(__file__).parent / "dati" / "mecc_mclafferty.json").read_text(encoding="utf-8"))
    req = tmp_path / "req.json"
    req.write_text(json.dumps(fx["request"]), encoding="utf-8")
    site = run("node", "tests/feat_grafo_node.js", str(pkg), req.read_text(encoding="utf-8"), "mech")
    assert site.strip() == fx["mecc"]   # crates/mzlab-desktop/tests/labels.rs reads the same bytes
    assert site == cli("mol-mech", str(req), runner=run)
    r = json.loads(site)
    assert [a["ok"] for a in r["arrows"]] == [True] * 5 + [False] and r["arrows"][5]["problem"] == "pairFromRadical"


LABELS = {"structures": [[{"el": "C", "iso": 13, "h": 4}], [{"el": "C", "h": 3}, {"el": "O", "h": 1}], [{"el": "Pt"}, {"el": "Cl"}, {"el": "Cl"}, {"el": "N", "h": 2}, {"el": "N", "h": 2}],
                         [{"el": "C", "h": 2}, {"el": "C"}, {"el": "O", "q": -1}, {"el": "O", "q": -1}, {"el": "O"}, {"el": "O"}], [{"el": "R#"}]], "arrows": [[1, 0], [0, 4]]}


def test_labels_through_wasm_equal_the_cli_byte_for_byte(pkg, tmp_path):
    req = tmp_path / "req.json"
    req.write_text(json.dumps(LABELS), encoding="utf-8")
    site = run("node", "tests/feat_grafo_node.js", str(pkg), req.read_text(encoding="utf-8"), "labels")
    app = cli("mol-label", str(req), runner=run)
    assert site == app
    r = json.loads(site)
    assert [s.get("text") for s in r["structures"]] == ["[13C]H4", "CH4O", "H4Cl2N2Pt", "[C2H2O4]2-", None] and r["arrows"][1] is None


# 4'-fluoroacetanilide, CC(=O)Nc1ccc(F)cc1, Kekulé ring; atom 3 is the amide N, atom 4 the aryl carbon
ANILIDE = ("\n  qqq\n\n 11 11  0  0  0  0  0  0  0  0999 V2000\n"
           + "".join(f"{x:10.4f}{y:10.4f}{0:10.4f} {el:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n" for x, y, el in (
               (0, 0, "C"), (0.87, 0.5, "C"), (0.87, 1.5, "O"), (1.73, 0, "N"), (2.6, 0.5, "C"), (3.46, 0, "C"), (4.33, 0.5, "C"), (4.33, 1.5, "C"),
               (5.2, 2, "F"), (3.46, 2, "C"), (2.6, 1.5, "C")))
           + "".join("%3d%3d%3d  0  0  0  0\n" % b for b in ((1, 2, 1), (2, 3, 2), (2, 4, 1), (4, 5, 1), (5, 6, 2), (6, 7, 1), (7, 8, 2), (8, 9, 1), (8, 10, 1), (10, 11, 2), (11, 5, 1)))
           + "M  END")


@pytest.mark.parametrize("sets", [[], ["mode=heterolytic", "cation=4"], ["mode=hydrogen", "gap=0.5"]])
def test_cut_through_wasm_equals_the_cli_byte_for_byte(pkg, tmp_path, sets):
    mf = tmp_path / "a.mol"
    mf.write_text(ANILIDE, encoding="utf-8")
    req = {"molfile": ANILIDE, "bonds": [[3, 4]]}
    for kv in sets:
        k, v = kv.split("=")
        req[k] = [int(v)] if k == "cation" else float(v) if k == "gap" else v
    site = run("node", "tests/feat_grafo_node.js", str(pkg), json.dumps(req), "cut")
    app = cli("mol-cut", str(mf), "--set", "bonds=3-4", *[a for kv in sets for a in ("--set", kv)], runner=run)
    assert site == app
    labels = {tuple(p["atoms"]): p["label"] for p in json.loads(site)["pieces"]}
    aryl, amide = labels[(4, 5, 6, 7, 8, 9, 10)], labels[(0, 1, 2, 3)]
    if not sets:
        assert (aryl["formula"], round(aryl["mass"], 4), aryl["kind"]) == ("C6H4F", 95.0297, "radical")
        assert (amide["formula"], round(amide["mass"], 4), amide["odd"]) == ("C2H4NO", 58.0293, True)
    elif sets[0] == "mode=heterolytic":
        assert (aryl["kind"], round(aryl["mz"], 4), amide["kind"]) == ("cation", 95.0292, "anion")


def test_labels_with_a_radical_mark_equal_the_cli(pkg, tmp_path):
    req = tmp_path / "req.json"
    req.write_text(json.dumps({"structures": [[{"el": "C", "h": 3, "r": 2}], [{"el": "N", "h": 3, "q": 1, "r": 2}]], "arrows": []}), encoding="utf-8")
    site = run("node", "tests/feat_grafo_node.js", str(pkg), req.read_text(encoding="utf-8"), "labels")
    assert site == cli("mol-label", str(req), runner=run)
    methyl, aminium = json.loads(site)["structures"]
    assert (methyl["unpaired"], methyl["odd"], methyl["kind"]) == (1, True, "radical")
    assert (aminium["kind"], aminium["electrons"]) == ("radicalCation", 9) and aminium["odd"]


def test_labels_with_a_bracket_delta_equal_the_cli(pkg, tmp_path):
    req = tmp_path / "req.json"   # the Draw label of a structure in brackets with «+O»: formula and masses of structure plus delta
    req.write_text(json.dumps({"structures": [[{"el": "C", "h": 3}, {"el": "C", "h": 3}], [{"el": "C", "h": 4}]], "deltas": ["+O", "-O"], "arrows": []}), encoding="utf-8")
    site = run("node", "tests/feat_grafo_node.js", str(pkg), req.read_text(encoding="utf-8"), "labels")
    assert site == cli("mol-label", str(req), runner=run)
    ethanol, bad = json.loads(site)["structures"]
    assert (ethanol["formula"], ethanol["nominal"], ethanol["delta"]) == ("C2H6O", 46, "+O") and bad["error"] == "error.grafo.delta.neg"


@pytest.mark.parametrize("part", [False, True])
def test_sel_through_wasm_equals_the_cli_byte_for_byte(pkg, tmp_path, part):
    req = tmp_path / "req.json"
    req.write_text(json.dumps({"molfile": ANILIDE, "part": part}), encoding="utf-8")
    site = run("node", "tests/feat_grafo_node.js", str(pkg), req.read_text(encoding="utf-8"), "sel")
    assert site == cli("mol-sel", str(req), runner=run)
    r = json.loads(site)
    assert (r["format"], r["part"], r["label"]["formula"], r["heavy"], r["rings"], r["aromaticRings"], r["aromaticAtoms"]) == ("sel/1", part, "C8H8FNO", 11, 1, 1, 6)
    assert round(r["label"]["mass"], 4) == 153.0590


def test_analysis_through_wasm_equals_the_cli_byte_for_byte(pkg, tmp_path):
    # every golden molecule of tools/analisi_riferimento.py, plus CuCl2(NH3)2 (Cu: the abundances of `isotopi-tutti`, on in the .wasm as in the app)
    cases = [c["request"] for c in json.loads((ROOT / "tests/dati/anal_golden.json").read_text(encoding="utf-8"))]
    cases.append({"atoms": [{"el": "Cl"}, {"el": "Cu"}, {"el": "Cl"}, {"el": "N", "h": 3}, {"el": "N", "h": 3}]})
    for i, c in enumerate(cases):
        req = tmp_path / f"anal{i}.json"
        req.write_text(json.dumps(c), encoding="utf-8")
        site = run("node", "tests/feat_grafo_node.js", str(pkg), req.read_text(encoding="utf-8"), "analysis")
        assert site == cli("mol-anal", str(req), runner=run), c
    assert [c["el"] for c in json.loads(site)["composition"]] == ["H", "Cl", "Cu", "N"]


def test_link_through_wasm_equals_the_cli_and_the_reference(pkg, tmp_path):
    # every case of tools/link_reference.py: charges, brackets, stereo, isotopes, #, + and / percent-encoded the same in the page and the app
    for i, c in enumerate(json.loads((ROOT / "tests/dati/link_golden.json").read_text(encoding="utf-8"))):
        req = tmp_path / f"link{i}.json"
        req.write_text(json.dumps(c["request"]), encoding="utf-8")
        site = run("node", "tests/feat_grafo_node.js", str(pkg), req.read_text(encoding="utf-8"), "link")
        assert site == cli("mol-link", str(req), runner=run) == c["link"] + "\n", c
