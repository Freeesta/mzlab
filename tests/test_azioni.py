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


def test_double_click_takes_the_first_available_claim():
    r = run("""const log=[];
      const reg=(id,o)=>AZIONI.registra({id,gruppo:'xic',viste:['v'],nome:()=>id,esegui:c=>log.push(id),...o});
      reg('main',{doppio:()=>true,doppioOrd:9});reg('zoom',{doppio:c=>c.z,disponibile:c=>c.z,doppioOrd:2});reg('ann',{doppio:c=>c.a,doppioOrd:1,disponibile:c=>c.a});reg('quiet',{});
      const out=[AZIONI.doppio({vista:'v'}),AZIONI.doppio({vista:'v',z:true}),AZIONI.doppio({vista:'v',z:true,a:true}),AZIONI.doppio({vista:'w'})];
      console.log(JSON.stringify({out,log}))""")
    assert r["log"] == ["main", "zoom", "ann"] and r["out"] == [True, True, True, False]


def test_reset_zoom_first_and_annotations_last():
    r = run("""const reg=(id,g)=>AZIONI.registra({id,gruppo:g,viste:['v'],nome:()=>id,esegui:()=>0});
      reg('ann','ann');reg('x','xic');reg('help','help');reg('reset','reset');
      console.log(JSON.stringify(AZIONI.per({vista:'v'}).map(i=>i==='-'?'-':i.id)))""")
    assert r == ["reset", "-", "x", "-", "help", "-", "ann"]


def test_icons_of_the_actions_exist_and_are_decorative():
    """Every key of the id -> icon table is a drawing; the drawings are decorative (aria-hidden) and follow the theme (currentColor); an action gets its icon from the table."""
    js = (f"const fs=require('fs');const I=new Function(fs.readFileSync({str(WEB / 'azioni-icone.js')!r},'utf8')+';return AZIONI_ICONE;')();"
          f"const AZIONI=new Function('AZIONI_ICONE',fs.readFileSync({str(WEB / 'azioni.js')!r},'utf8')+';return AZIONI;')(I);"
          "const a=AZIONI.registra({id:'spec.copy',gruppo:'copy',viste:['v'],nome:()=>'x',esegui:()=>0});const b=AZIONI.registra({id:'zz',gruppo:'copy',viste:['v'],icona:'xic',nome:()=>'x',esegui:()=>0});"
          "const c=AZIONI.registra({id:'yy',gruppo:'copy',viste:['v'],nome:()=>'x',esegui:()=>0});const m=AZIONI.per({vista:'v'});"
          "console.log(JSON.stringify({missing:Object.entries(I.POR_ID).filter(([k,v])=>!I.DISEGNI[v]).map(([k])=>k),svg:Object.keys(I.DISEGNI).map(k=>I.svg(k)),got:[a.icona,b.icona,c.icona||null],menu:m.map(i=>i.icon?1:0)}));")
    r = json.loads(subprocess.run(["node", "-e", js], check=True, capture_output=True, encoding="utf-8").stdout)
    assert not r["missing"], r["missing"]
    assert all('aria-hidden="true"' in v and "currentColor" in v and 'role="img"' not in v and "style=" not in v for v in r["svg"])
    assert r["got"] == ["copy", "xic", None] and r["menu"] == [1, 1, 0]


def test_every_id_of_the_icon_table_is_a_registered_action():
    src = "\n".join(p.read_text(encoding="utf-8") for p in WEB.glob("*.js") if p.name != "azioni-icone.js")
    ids = json.loads(subprocess.run(["node", "-e", f"const fs=require('fs');const I=new Function(fs.readFileSync({str(WEB / 'azioni-icone.js')!r},'utf8')+';return AZIONI_ICONE;')();console.log(JSON.stringify(Object.keys(I.POR_ID)))"],
                                    check=True, capture_output=True, encoding="utf-8").stdout)
    for i in ids:
        assert f'"{i}"' in src or f'over("{i.split(".")[-1]}"' in src, f"{i}: no action with this id in mzlab/web"


def test_arrangement_actions_show_in_every_view_and_have_texts_in_both_languages():
    """Actions with viste ['*'] show in any view; the keys of the arrangement actions (affianca.js) exist in it and en; layout.js never names an experiment."""
    r = run("""AZIONI.registra({id:'t.all',gruppo:'disp',viste:['*'],nome:()=>'x',esegui:()=>0});
      console.log(JSON.stringify(['spec','chrom','mrm'].map(v=>AZIONI.per({vista:v}).some(i=>i.id==='t.all'))))""")
    assert r == [True, True, True]
    src = (WEB / "affianca.js").read_text(encoding="utf-8")
    keys = set(re.findall(r'"(aff\.menu\.[\w.]+)"', src))
    assert len(keys) >= 6, keys
    for lang in ("it", "en"):
        cat = (WEB / "lang" / f"{lang}.js").read_text(encoding="utf-8")
        for k in keys | {f"cmd.disp.{i}.{f}" for i in ("affianca", "su", "giu", "intera", "uguali", "ripristina") for f in ("lab", "name", "tip", "kw")}:
            assert f'"{k}"' in cat, (lang, k)
    assert not re.search(r"""["'](?:full|ms2|mrm|spec|chrom)["']""", (WEB / "layout.js").read_text(encoding="utf-8"))
