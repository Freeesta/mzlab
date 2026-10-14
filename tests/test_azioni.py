"""The registry of the actions of the panels (azioni.js, format azioni/1): order of the groups, grey entries, unique ids, and the texts it/en of the map actions."""
import json, re, shutil, subprocess
from pathlib import Path
import pytest

WEB = Path(__file__).resolve().parents[1] / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    js = f"const fs=require('fs');const AZIONI=new Function(fs.readFileSync({str(WEB / 'azioni.js')!r},'utf8')+';return AZIONI;')();{code}"
    return json.loads(subprocess.run(["node", "-e", js], check=True, capture_output=True, encoding="utf-8").stdout)


def test_menu_order_separators_and_grey_entries():
    r = run("""const log=[];
      const reg=(id,gruppo,o)=>AZIONI.registra({id,gruppo,viste:['v'],nome:()=>id,esegui:()=>log.push(id),...o});
      reg('h','help');reg('x','extract',{disponibile:c=>c.ok});reg('l','lock',{attiva:()=>true});reg('z','zoom',{menu:false,barra:true});reg('o','copy',{viste:['w']});
      const m=AZIONI.per({vista:'v',ok:false});
      m.filter(i=>i!=='-').forEach(i=>i.fn());
      AZIONI.esegui('x',{vista:'v',ok:true});AZIONI.esegui('o',{vista:'v'});
      console.log(JSON.stringify({m:m.map(i=>i==='-'?'-':[i.id,i.dim,i.check]),bar:AZIONI.elenco({vista:'v'},'barra').map(a=>a.id),log}));""")
    assert r["m"] == [["x", True, None], "-", ["l", False, True], "-", ["h", False, None]]
    assert r["bar"] == ["z"]
    assert r["log"] == ["l", "h", "x"], "a grey entry does nothing; an action of another view does nothing"


@pytest.mark.parametrize("bad", ["AZIONI.registra({id:'a',gruppo:'help',viste:[],nome:()=>'',esegui:()=>0})",
                                 "AZIONI.registra({id:'b',gruppo:'nope',viste:[],nome:()=>'',esegui:()=>0})"])
def test_duplicate_id_or_unknown_group_is_refused(bad):
    r = run(f"AZIONI.registra({{id:'a',gruppo:'help',viste:[],nome:()=>'',esegui:()=>0}});let e=null;try{{{bad}}}catch(x){{e=x.message}};console.log(JSON.stringify(e))")
    assert r and r.startswith("AZIONI:")


def test_map_actions_have_texts_in_both_languages():
    ids = re.findall(r'cmd\("(\w+)"', (WEB / "mappa.js").read_text(encoding="utf-8"))
    assert len(ids) == len(set(ids)) >= 13
    for lang in ("it", "en"):
        cat = (WEB / "lang" / f"{lang}.js").read_text(encoding="utf-8")
        for i in ids:
            for k in ("name", "tip"):
                assert f'"cmd.{i}.{k}"' in cat, (lang, i, k)


def test_rows_of_one_action_and_order_inside_a_group():
    r = run("""const reg=(id,o)=>AZIONI.registra({id,gruppo:'xic',viste:['v'],nome:()=>id,esegui:()=>0,...o});
      reg('b',{ordine:2});reg('a',{ordine:1});reg('n',{voci:c=>c.xs.map(x=>({nome:'n'+x,dim:x===2,esegui:()=>0}))});reg('e',{voci:()=>[]});
      AZIONI.registra({id:'h',gruppo:'help',viste:['v'],nome:()=>'h',esegui:()=>0});
      const m=AZIONI.per({vista:'v',xs:[1,2]});
      console.log(JSON.stringify(m.map(i=>i==='-'?'-':[i.id,i.label,i.dim])))""")
    assert r == [["n", "n1", False], ["n", "n2", True], ["a", "a", False], ["b", "b", False], "-", ["h", "h", False]]


def test_spectrum_actions_have_texts_in_both_languages():
    """Every i18n key named by a registered action of the spectrum exists in it and en."""
    src = "".join((WEB / f).read_text(encoding="utf-8") for f in ("spettro.js", "dda.js", "liste.js", "elementare.js", "visualizza.js", "libreria.js", "workflow.js", "fair.js"))
    blocks = re.findall(r'id: "spec\.\w+".*', src)
    keys = set(k for b in blocks for k in re.findall(r'"((?:ctx|cx|disp)\.[\w.]+)"', b))
    assert len(blocks) >= 24 and len(keys) >= 20, (len(blocks), len(keys))
    for lang in ("it", "en"):
        cat = (WEB / "lang" / f"{lang}.js").read_text(encoding="utf-8")
        for k in keys:
            assert f'"{k}"' in cat, (lang, k)


def test_trace_actions_have_texts_in_both_languages():
    """Every i18n key named by a registered action of the chromatogram menu exists in it and en."""
    src = "".join((WEB / f).read_text(encoding="utf-8") for f in ("eventi.js", "gradiente.js", "dda.js", "libreria.js", "composizione.js", "visualizza.js", "picchi.js", "grafici.js"))
    blocks = re.findall(r'id: "(?:chrom|xic)\.\w+"[^\n]*(?:\n[^\n]*)?|over\("\w+"[^\n]*', src)
    keys = set(k for b in blocks for k in re.findall(r'"((?:ctx|disp)\.[\w.]+)"', b))
    assert len(blocks) >= 11 and len(keys) >= 9, (len(blocks), len(keys))
    for lang in ("it", "en"):
        cat = (WEB / "lang" / f"{lang}.js").read_text(encoding="utf-8")
        for k in keys:
            assert f'"{k}"' in cat, (lang, k)
