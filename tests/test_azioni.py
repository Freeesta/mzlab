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
