"""The shortcuts of the registry azioni/1 (field `scorciatoia`): no key twice in the same view, none reserved by the browser, text for Mac and Windows."""
import json
import re
import subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
# a key may be on several actions only when their views are disjoint (the key belongs to the view of the chart in focus)
DISJOINT = {"x": {"spec.xic", "chrom.xic", "xic.addIon"}, "enter": {"spec.peak", "chrom.specAtRt"}}
RESERVED = {("t", True, False), ("n", True, False), ("w", True, False), ("q", True, False), ("l", True, False), ("r", True, False), ("p", True, False), ("s", True, False), ("f", True, False)}


def tasti():
    out = []
    for f in sorted(WEB.glob("*.js")):
        for m in re.finditer(r'id: "([\w.]+)"[^\n]*?scorciatoia: (\{[^}]*\})', f.read_text(encoding="utf8")):
            out.append((m.group(1), dict(re.findall(r'(\w+): ("[^"]*"|true|false)', m.group(2)))))
    src = (WEB / "azioni-tasti.js").read_text(encoding="utf8")
    for k, v, code in re.findall(r'vista\("(\w+)", "(\w+)", "(Digit\d)"', src):
        out.append(("app." + k, {"code": f'"{code}"', "alt": "true", "dove": '"view"'}))
    return out


def chiave(s):
    q = lambda k: s.get(k, "").strip('"')
    return ((q("tasto") or q("code")).lower(), s.get("mod") == "true", s.get("alt") == "true")


def test_found_the_wanted_keys():
    ids = {i for i, _ in tasti()}
    for want in ("spec.xic", "chrom.specAtRt", "spec.peak", "chrom.intAuto", "spec.measureKey", "view.fullscreen", "app.apri", "app.esporta", "app.dati", "app.disegno", "app.teoria", "app.scorciatoie"):
        assert want in ids, want


def test_no_key_twice_in_the_same_view():
    seen = {}
    for i, s in tasti():
        seen.setdefault(chiave(s), []).append(i)
    for k, ids in seen.items():
        if len(ids) > 1:
            assert set(ids) <= DISJOINT.get(k[0], set()), (k, ids)


def test_no_key_reserved_by_the_browser():
    for i, s in tasti():
        assert chiave(s) not in RESERVED, i
        if s.get("alt") == "true" and "code" in s:
            assert s["code"].strip('"') in ("Digit1", "Digit2", "Digit3"), i
        if s.get("dove") == '"chart"':
            assert s.get("mod") != "true" and s.get("alt") != "true", i      # the letters of a chart have no modifiers


def test_text_mac_and_windows():
    js = (f"const fs=require('fs');const A=new Function(fs.readFileSync({str(WEB / 'azioni.js')!r},'utf8')+';return AZIONI;')();"
          "const r=[];for(const p of ['MacIntel','Linux x86_64']){Object.defineProperty(globalThis,'navigator',{value:{platform:p},configurable:true});"
          "r.push([{tasto:'o',mod:true,dove:'view'},{code:'Digit1',alt:true,dove:'view'},{tasto:'x',dove:'chart'},{tasto:'Enter',dove:'chart'}].map(s=>A.testo(s)));}console.log(JSON.stringify(r));")
    mac, win = json.loads(subprocess.run(["node", "-e", js], capture_output=True, text=True, encoding="utf-8", check=True).stdout)
    assert mac == ["⌘O", "⌥1", "X", "Enter"] and win == ["Ctrl+O", "Alt+1", "X", "Enter"]


def test_premuto():
    js = (f"const fs=require('fs');const A=new Function(fs.readFileSync({str(WEB / 'azioni.js')!r},'utf8')+';return AZIONI;')();"
          "const e=(o)=>Object.assign({key:'',code:'',ctrlKey:false,metaKey:false,altKey:false,shiftKey:false},o);"
          "console.log(JSON.stringify([A.premuto({tasto:'x'},e({key:'x'})),A.premuto({tasto:'x'},e({key:'X',shiftKey:true})),A.premuto({tasto:'x'},e({key:'x',ctrlKey:true})),"
          "A.premuto({tasto:'o',mod:true},e({key:'o',metaKey:true})),A.premuto({code:'Digit1',alt:true},e({code:'Digit1',key:'¡',altKey:true})),A.premuto({tasto:'?'},e({key:'?',shiftKey:true})),A.premuto({tasto:'Enter'},e({key:'Enter'}))]));")
    assert json.loads(subprocess.run(["node", "-e", js], capture_output=True, text=True, encoding="utf-8", check=True).stdout) == [True, False, False, True, True, True, True]
