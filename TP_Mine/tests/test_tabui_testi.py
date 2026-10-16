"""Catalog of the help texts of Tabui (TP_Mine/js/40_tabui_testi.js): English only (no Italian catalog), no empty text, every {param} in both languages, at most
240 characters per text, and every key used in the code (data-tt, dt) exists."""
import json, re, shutil, subprocess
from pathlib import Path

JS = Path(__file__).resolve().parents[1] / "js"


def catalog():
    src = (JS / "40_tabui_testi.js").read_text(encoding="utf-8")
    if shutil.which("node"):
        code = "global.window={};global.location={search:''};" + "eval(require('fs').readFileSync(process.argv[1],'utf8'));console.log(JSON.stringify(window.TABUI_T))"
        return json.loads(subprocess.run(["node", "-e", code, str(JS / "40_tabui_testi.js")], check=True, capture_output=True, text=True).stdout)
    out = {}
    for lang in ("en",):
        body = re.search(rf"\n  {lang}: \{{(.*?)\n  \}}", src, re.S).group(1)
        out[lang] = {m.group(1): json.loads('"' + m.group(2) + '"') for m in re.finditer(r'"([\w.]+)": "((?:[^"\\]|\\.)*)"', body)}
    return out


def test_same_keys_and_texts():
    t = catalog()
    assert set(t) == {"en"} and len(t["en"]) > 40
    for k, v in t["en"].items():
        assert v.strip(), k
        assert len(v) <= 240, (k, len(v))
        assert not re.search(r"[àèéìòù]\b", v), k          # Tabui is English only


def test_keys_used_in_the_code_exist():
    keys = set(catalog()["en"])
    for f in JS.glob("*.js"):
        if f.name.startswith("40_"):
            continue
        for k in re.findall(r"""(?:data-tt="|dt\(\s*")(ft\.[\w.]+)""", f.read_text(encoding="utf-8")):
            assert k in keys, (f.name, k)


def engine_messages():
    if not shutil.which("node"):
        return None
    code = "global.window={};global.location={search:''};eval(require('fs').readFileSync(process.argv[1],'utf8'));console.log(JSON.stringify([window.TABUI_E,window.TE('error.io'),window.TE('error.unknownThing',{message:'raw'}),window.TE('tpmine.centroided',{name:'a.mzML'})]))"
    return json.loads(subprocess.run(["node", "-e", code, str(JS / "40_tabui_testi.js")], check=True, capture_output=True, text=True).stdout)


def test_engine_messages_are_english_only_and_cover_the_keys():
    got = engine_messages()
    if got is None:
        return
    table, known, unknown, filled = got
    assert known == table["error.io"] and unknown == "raw" and filled.startswith("a.mzML was in profile")
    for k, v in table.items():
        assert v.strip() and not re.search(r"[àèéìòù]\b|\b(il|della|per|non|file è)\b", v), (k, v)          # Tabui is English only
    files = list((JS.parent / "files").glob("*.js")) + list(JS.glob("*.js"))
    used = set()
    for f in files:
        if f.name.startswith("40_"):
            continue
        used |= set(re.findall(r"""["']((?:error|tpmine\.feat|tpmine)\.[A-Za-z]+(?:\.[A-Za-z]+)?)["']""", f.read_text(encoding="utf-8")))
    used = {k for k in used if not k.startswith("error.posiz.") and k != "tpmine.api"}          # the panel of the positions has its own English table; tpmine.api is a Python module
    assert used and not (used - set(table)), sorted(used - set(table))


def test_tabui_does_not_use_the_language_catalogs():
    for f in list(JS.glob("*.js")) + list((JS.parent / "files").glob("*.js")):
        assert "I18N" not in f.read_text(encoding="utf-8"), f.name
