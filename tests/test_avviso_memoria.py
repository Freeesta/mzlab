"""A memory refusal says the same thing in the desktop app and in the CLI (tests/dati/avvisi_memoria.json; the CLI half is
testo_errore_tests.rs), and on the site it keeps its browser text with the numbers of the refusal."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
CASI = json.loads((ROOT / "tests" / "dati" / "avvisi_memoria.json").read_text(encoding="utf-8"))["casi"]
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

# the English catalogs, then I18N.err on every case, in the app (MZLAB_DESKTOP) or on the site
JS = """
const fs = require("fs"), path = require("path"), web = process.argv[1], desktop = process.argv[2] === "1";
for (const f of fs.readdirSync(path.join(web, "lang", "en")).sort()) new Function("I18N", fs.readFileSync(path.join(web, "lang", "en", f), "utf8"))(I18N);
I18N.set("en"); globalThis.window = desktop ? { MZLAB_DESKTOP: true } : {};
const casi = JSON.parse(process.argv[3]);
console.log(JSON.stringify(casi.map(c => I18N.err({ error_key: "error.memoryBudget", params: Object.fromEntries(c.params) }).message)));
"""


def rese(desktop: bool):
    out = subprocess.run(["node", "-e", JS, str(ROOT / "mzlab" / "web"), "1" if desktop else "0", json.dumps(CASI)],
                         check=True, capture_output=True, text=True, encoding="utf-8").stdout
    return json.loads(out)


def test_the_desktop_app_shows_the_text_of_the_cli():
    assert rese(True) == [c["testo"] for c in CASI]


def test_the_site_keeps_the_browser_text_with_the_numbers():
    sito = rese(False)
    assert "browser" in sito[0] and "computer" not in sito[0]
    assert "«scan table»" in sito[1] and "900 MB" in sito[1] and "about 500 MB short" in sito[1] and "desktop app" in sito[1]
    assert "300 MB of peaks" in sito[2] and "256 MB" in sito[2] and "browser" in sito[2]
    assert "300 MB" in sito[3] and "browser gives the engine 192 MB" in sito[3]
