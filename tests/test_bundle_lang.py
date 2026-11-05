"""The site ships each interface catalog as one file (tools/build_site.py bundle_lang): run in Node with a stub I18N, it adds exactly the keys
and texts of the area files, and it writes no <script> (one render-blocking request before the first paint)."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import build_site  # noqa: E402
import lang_catalogo  # noqa: E402

RUN = r"""const fs = require("fs"); const out = {}; let writes = 0;
global.I18N = { add: (l, o) => { for (const k in o) { if (k in out) throw new Error("twice " + k); out[k] = o[k]; } } };
global.document = { write: () => { writes++; }, currentScript: { src: "x/lang/it.js" } };
eval(fs.readFileSync(process.argv[1], "utf8"));
process.stdout.write(JSON.stringify({ writes, out }));"""


@pytest.mark.skipif(not shutil.which("node"), reason="node missing")
def test_bundled_catalog_equals_the_area_files(tmp_path):
    static = tmp_path / "static"
    shutil.copytree(lang_catalogo.LANG, static / "lang")
    build_site.bundle_lang(static)
    for lang in ("it", "en"):
        r = subprocess.run(["node", "-e", RUN, str(static / "lang" / f"{lang}.js")], capture_output=True, text=True, check=True)
        got = json.loads(r.stdout)
        assert got["writes"] == 0, lang
        assert got["out"] == lang_catalogo.catalogo(lang), lang
