"""The display rule of the Method window's numbers is one rule in two places: `methodview::shown` (Rust, the text the site, the app and the CLI print) and its mirror
`METODO_DOC.shown` (mzlab/web/metodo-doc.js, used only for the legacy method/1 view). Both must give the same text for every value of tests/data/shown_numbers.json
(the Rust side is checked against the same file in crates/mzlab-core/src/methodview_tests.rs), ties to the even digit included, where JS toFixed(2) alone differs."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS = json.loads((ROOT / "tests" / "data" / "shown_numbers.json").read_text(encoding="utf-8"))

NODE = """
const vm = require("vm"), fs = require("fs");
const sandbox = { window: {}, EH: s => String(s), I18N: { t: k => k }, METODO_INCL: { bind() {}, html: () => "" }, console };
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(process.argv[1], "utf8") + "\\n;globalThis.__m = METODO_DOC;", sandbox);
const cases = JSON.parse(process.argv[2]);
console.log(JSON.stringify(cases.map(([v]) => sandbox.__m.shown(v))));
"""


def test_the_js_mirror_prints_every_value_of_the_shared_corpus_as_rust_does():
    out = subprocess.run(["node", "-e", NODE, str(ROOT / "mzlab" / "web" / "metodo-doc.js"), json.dumps(CORPUS)], check=True, capture_output=True, encoding="utf-8").stdout
    got = json.loads(out)
    assert got == [want for _, want in CORPUS], [(v, g, w) for (v, w), g in zip(CORPUS, got) if g != w]


def test_the_corpus_keeps_the_tie_cases_that_make_the_two_rules_differ():
    values = {v for v, _ in CORPUS}
    assert {0.125, 0.375, 0.625, 27.499780654907227} <= values
