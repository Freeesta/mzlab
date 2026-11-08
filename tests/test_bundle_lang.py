"""The site ships each interface catalog as two files (tools/build_site.py bundle_lang): run in Node with a stub I18N, the first part then the
rest add exactly the keys and texts of the area files, each key once; the first part writes no <script>, names the rest (awaitRest) and is small
(it blocks the first paint); the rest ends with done()."""
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

RUN = r"""const fs = require("fs"); const out = {}, calls = []; let writes = 0, first = 0;
global.I18N = { add: (l, o) => { for (const k in o) { if (k in out) throw new Error("twice " + k); out[k] = o[k]; } },
  awaitRest: p => calls.push("awaitRest " + p), done: () => calls.push("done") };
global.document = { write: () => { writes++; }, currentScript: { src: "x/lang/it.js" } };
eval(fs.readFileSync(process.argv[1], "utf8")); first = Object.keys(out).length;
eval(fs.readFileSync(process.argv[2], "utf8"));
process.stdout.write(JSON.stringify({ writes, out, calls, first }));"""


@pytest.mark.skipif(not shutil.which("node"), reason="node missing")
def test_bundled_catalog_equals_the_area_files(tmp_path):
    static = tmp_path / "static"
    shutil.copytree(lang_catalogo.LANG, static / "lang")
    for f in [lang_catalogo.LANG.parent / "index.html", *lang_catalogo.LANG.parent.glob("*.js")]:      # the first part is chosen from the page and its first scripts
        shutil.copy2(f, static / f.name)
    build_site.bundle_lang(static)
    for lang in ("it", "en"):
        r = subprocess.run(["node", "-e", RUN, str(static / "lang" / f"{lang}.js"), str(static / "lang" / f"{lang}-resto.js")], capture_output=True, text=True, check=True)
        got = json.loads(r.stdout)
        assert got["writes"] == 0, lang
        assert got["out"] == lang_catalogo.catalogo(lang), lang
        assert got["calls"] == [f"awaitRest lang/{lang}-resto.js", "done"], got["calls"]
        assert 0 < got["first"] < len(got["out"]) // 4, (lang, got["first"])
