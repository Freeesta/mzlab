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


def test_net_change_cell_has_subscripts_and_the_export_stays_plain():
    src = (JS / "20_tpmine.js").read_text(encoding="utf-8")
    assert '.replace(/([A-Za-z)])(\\d+)/g, "$1<sub>$2</sub>")' in src          # the cell of «Net change (formula)»
    assert 'r.net_change ?? ""' in src                                      # the export keeps the plain text
    if shutil.which("node"):
        out = subprocess.run(["node", "-e", 'console.log("\\u2212C2H2 +O3".replace(/([A-Za-z)])(\\d+)/g, "$1<sub>$2</sub>"))'], check=True, capture_output=True, text=True).stdout.strip()
        assert out == "−C<sub>2</sub>H<sub>2</sub> +O<sub>3</sub>"


def _lum(h):
    if len(h) == 4:
        h = "#" + "".join(ch * 2 for ch in h[1:])
    c = [int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    c = [v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4 for v in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]


def test_green_scale_keeps_3_to_1_for_the_symbol_at_every_step():
    css = (JS / "44_tp_riga.js").read_text(encoding="utf-8")
    g = [re.search(rf"--tpr-g{i}:(#[0-9a-f]{{6}})", css).group(1) for i in range(5)]
    dark, white = re.search(r"--tpr-k:(#[0-9a-f]{3,6})", css).group(1), re.search(r"--tpr-w:(#[0-9a-f]{3,6})", css).group(1)
    assert "--tpr-c0" not in css and "--tpr-g-none:#fff" in css
    lums = [_lum(x) for x in g]
    assert lums == sorted(lums, reverse=True), "light to dark, one direction"
    cr = lambda a, b: (max(a, b) + 0.05) / (min(a, b) + 0.05)
    for k in range(101):                                   # u = 0..1; on-k below 0.6, on-w from 0.6 (TPTAB.paint)
        u = k / 100
        x = u * 4; i = min(3, int(x)); f = round((x - i) * 100) / 100
        mix = "#%02x%02x%02x" % tuple(round(int(g[i][j:j + 2], 16) * (1 - f) + int(g[i + 1][j:j + 2], 16) * f) for j in (1, 3, 5))
        fg = _lum(dark) if u < 0.6 else _lum(white)
        assert cr(fg, _lum(mix)) >= 3, (u, mix)
    assert cr(_lum(dark), _lum("#ffffff")) >= 3          # the empty (white) square


def test_cells_uses_the_shared_area_scale_and_white_for_none():
    t = (JS / "42_tp_table.js").read_text(encoding="utf-8")
    assert not re.search(r"--tpr-c[0-4]", t) and "areaScale" in t and "AREA_NOTE" in t
    assert not re.search(r"--tpr-c[0-4]", (JS / "44_tp_riga.js").read_text(encoding="utf-8"))
