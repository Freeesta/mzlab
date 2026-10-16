"""Catalog of the help texts of Tabui (TP_Mine/js/40_tabui_testi.js): the same keys in Italian and English, no empty text, every {param} in both languages, at most
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
    for lang in ("it", "en"):
        body = re.search(rf"\n  {lang}: \{{(.*?)\n  \}}", src, re.S).group(1)
        out[lang] = {m.group(1): json.loads('"' + m.group(2) + '"') for m in re.finditer(r'"([\w.]+)": "((?:[^"\\]|\\.)*)"', body)}
    return out


def test_same_keys_and_texts():
    t = catalog()
    assert set(t["it"]) == set(t["en"]) and len(t["it"]) > 40
    for lang in ("it", "en"):
        for k, v in t[lang].items():
            assert v.strip(), (lang, k)
            assert len(v) <= 240, (lang, k, len(v))
    for k in t["it"]:
        assert set(re.findall(r"\{(\w+)\}", t["it"][k])) == set(re.findall(r"\{(\w+)\}", t["en"][k])), k


def test_keys_used_in_the_code_exist():
    keys = set(catalog()["en"])
    for f in JS.glob("*.js"):
        if f.name.startswith("40_"):
            continue
        for k in re.findall(r"""(?:data-tt="|dt\(\s*")(ft\.[\w.]+)""", f.read_text(encoding="utf-8")):
            assert k in keys, (f.name, k)
