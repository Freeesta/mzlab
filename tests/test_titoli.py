"""Panel titles (mzlab/web/titoli.js): one naming scheme, every panel type x signal, IT and EN."""
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
ENV = {**os.environ, "NODE_OPTIONS": ""}  # the page catalogs are loaded by the script itself
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

JS = """
globalThis.window = globalThis;
const fs = require("fs"), path = require("path");
const lang = process.argv[1];
globalThis.I18N_LANG = lang;
require(%(web)r + "/i18n.js");
for (const f of fs.readdirSync(path.join(%(web)r, "lang", lang)).sort()) new Function("I18N", fs.readFileSync(path.join(%(web)r, "lang", lang, f), "utf8"))(globalThis.I18N);
globalThis.ESP = new Function(fs.readFileSync(%(web)r + "/esperimenti.js", "utf8") + "\\nreturn ESP;")();  // the page loads esperimenti.js before titoli.js
require(%(titoli)r);
const cases = JSON.parse(process.argv[2]);
console.log(JSON.stringify(cases.map(([p, ctx]) => TITOLI.nome(p, ctx))));
"""

CASES = {
    "ms2_single_chrom": ({"type": "chrom", "tab": "ms2", "kind": "tic", "prec": 414.1, "title": "@ms2tic"}, {"nPrec": 1}),
    "ms2_several_all": ({"type": "chrom", "tab": "ms2", "kind": "tic", "prec": None, "title": "@ms2tic"}, {"nPrec": 3}),
    "ms2_several_one": ({"type": "chrom", "tab": "ms2", "kind": "tic", "prec": 414.1, "title": "@ms2prec:414.1"}, {"nPrec": 3}),
    "ms2_spec": ({"type": "spec", "tab": "ms2", "level": 2, "prec": 414.1, "title": "@product:414.1"}, {}),
    "ms1_chrom": ({"type": "chrom", "tab": "full", "kind": "tic", "title": "@chrom"}, {}),
    "ms1_spec": ({"type": "spec", "tab": "full", "level": 1, "title": "@spec"}, {}),
    "mrm_all": ({"type": "mrm", "tab": "mrm", "tr": "", "title": "@mrm"}, {}),
    "mrm_one": ({"type": "mrm", "tab": "mrm", "tr": ["414.1>195.0"], "title": "@mrm"}, {}),
    "pda_all": ({"type": "chrom", "kind": "pda", "mz0": None, "title": "@chrom"}, {}),
    "pda_one": ({"type": "chrom", "kind": "pda", "mz0": 254, "mz1": 254, "title": "@chrom"}, {}),
    "pda_range": ({"type": "chrom", "kind": "pda", "mz0": 250, "mz1": 260, "title": "@chrom"}, {}),
    "uv": ({"type": "spec", "uv": True, "level": 1, "title": "@uv"}, {}),
    "typed": ({"type": "chrom", "kind": "tic", "title": "My trace"}, {}),
}
EXPECT = {
    "en": ["TIC · MS²", "TIC · MS² (all precursors)", "TIC · MS² · precursor 414.1", "MS² of 414.1", "TIC", "MS¹", "MRM", "MRM · 414.1>195.0",
           "PDA (all λ)", "PDA 254 nm", "PDA 250–260 nm", "UV spectrum", "My trace"],
    "it": ["TIC · MS²", "TIC · MS² (tutti i precursori)", "TIC · MS² · precursore 414.1", "MS² di 414.1", "TIC", "MS¹", "MRM", "MRM · 414.1>195.0",
           "PDA (tutte le λ)", "PDA 254 nm", "PDA 250–260 nm", "Spettro UV", "My trace"],
}


@pytest.mark.parametrize("lang", ["it", "en"])
def test_titles(lang):
    code = JS % {"web": str(WEB), "titoli": str(WEB / "titoli.js")}
    out = subprocess.run(["node", "-e", code, lang, json.dumps(list(CASES.values()))], check=True, capture_output=True, encoding="utf-8", env=ENV).stdout
    assert json.loads(out) == EXPECT[lang]


def test_chrom_and_spectrum_of_one_experiment_differ():
    code = JS % {"web": str(WEB), "titoli": str(WEB / "titoli.js")}
    cs = [CASES["ms2_single_chrom"], CASES["ms2_spec"]]
    top, spec = json.loads(subprocess.run(["node", "-e", code, "en", json.dumps(cs)], check=True, capture_output=True, encoding="utf-8", env=ENV).stdout)
    assert top != spec and "414.1" in spec and "414.1" not in top
