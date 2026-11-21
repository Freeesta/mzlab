"""The registry of the actions of the panels (azioni.js, format actions/1): order of the groups, grey entries, unique ids, and the texts of the map actions."""
import json, re, shutil, subprocess
from pathlib import Path
import sys
import pytest
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import lang_catalog  # noqa: E402

WEB = Path(__file__).resolve().parents[1] / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    js = f"const fs=require('fs');const AZIONI=new Function(fs.readFileSync({str(WEB / 'azioni.js')!r},'utf8')+';return AZIONI;')();{code}"
    return json.loads(subprocess.run(["node", "-e", js], check=True, capture_output=True, encoding="utf-8").stdout)


def test_menu_order_separators_and_grey_entries():
    r = run("""const log=[];
      const reg=(id,group,o)=>AZIONI.register({id,group,views:['v'],name:()=>id,run:()=>log.push(id),...o});
      reg('h','help');reg('x','extract',{available:c=>c.ok});reg('l','lock',{checked:()=>true});reg('z','zoom',{menu:false,barra:true});reg('o','copy',{views:['w']});
      const m=AZIONI.forContext({view:'v',ok:false});
      m.filter(i=>i!=='-').forEach(i=>i.fn());
      AZIONI.run('x',{view:'v',ok:true});AZIONI.run('o',{view:'v'});
      console.log(JSON.stringify({m:m.map(i=>i==='-'?'-':[i.id,i.dim,i.check]),bar:AZIONI.list({view:'v'},'barra').map(a=>a.id),log}));""")
    assert r["m"] == [["x", True, None], "-", ["l", False, True], "-", ["h", False, None]]
    assert r["bar"] == ["z"]
    assert r["log"] == ["l", "h", "x"], "a grey entry does nothing; an action of another view does nothing"


@pytest.mark.parametrize("bad", ["AZIONI.register({id:'a',group:'help',views:[],name:()=>'',run:()=>0})",
                                 "AZIONI.register({id:'b',group:'nope',views:[],name:()=>'',run:()=>0})"])
def test_duplicate_id_or_unknown_group_is_refused(bad):
    r = run(f"AZIONI.register({{id:'a',group:'help',views:[],name:()=>'',run:()=>0}});let e=null;try{{{bad}}}catch(x){{e=x.message}};console.log(JSON.stringify(e))")
    assert r and r.startswith("AZIONI:")


def test_map_actions_have_texts():
    ids = re.findall(r'cmd\("(\w+)"', (WEB / "mappa.js").read_text(encoding="utf-8"))
    assert len(ids) == len(set(ids)) >= 12
    for lang in ("en",):
        cat = lang_catalog.testo(lang)
        for i in ids:
            for k in ("name", "tip"):
                assert f'"cmd.{i}.{k}"' in cat, (lang, i, k)


def test_rows_of_one_action_and_order_inside_a_group():
    r = run("""const reg=(id,o)=>AZIONI.register({id,group:'xic',views:['v'],name:()=>id,run:()=>0,...o});
      reg('b',{order:2});reg('a',{order:1});reg('n',{rows:c=>c.xs.map(x=>({name:'n'+x,dim:x===2,run:()=>0}))});reg('e',{rows:()=>[]});
      AZIONI.register({id:'h',group:'help',views:['v'],name:()=>'h',run:()=>0});
      const m=AZIONI.forContext({view:'v',xs:[1,2]});
      console.log(JSON.stringify(m.map(i=>i==='-'?'-':[i.id,i.label,i.dim])))""")
    assert r == [["n", "n1", False], ["n", "n2", True], ["a", "a", False], ["b", "b", False], "-", ["h", "h", False]]


def test_spectrum_actions_have_texts():
    """Every i18n key named by a registered action of the spectrum exists in the catalog."""
    src = "".join((WEB / f).read_text(encoding="utf-8") for f in ("spettro.js", "dda.js", "liste.js", "elementare.js", "visualizza.js", "libreria.js", "workflow.js", "fair-azioni.js"))
    blocks = re.findall(r'id: "spec\.\w+".*', src)
    keys = set(k for b in blocks for k in re.findall(r'"((?:ctx|cx|disp)\.[\w.]+)"', b))
    assert len(blocks) >= 24 and len(keys) >= 20, (len(blocks), len(keys))
    for lang in ("en",):
        cat = lang_catalog.testo(lang)
        for k in keys:
            assert f'"{k}"' in cat, (lang, k)


def test_trace_actions_have_texts():
    """Every i18n key named by a registered action of the chromatogram menu exists in the catalog."""
    src = "".join((WEB / f).read_text(encoding="utf-8") for f in ("eventi.js", "gradiente.js", "dda.js", "libreria.js", "composizione.js", "visualizza.js", "picchi.js", "grafici.js"))
    blocks = re.findall(r'id: "(?:chrom|xic)\.\w+"[^\n]*(?:\n[^\n]*)?|over\("\w+"[^\n]*', src)
    keys = set(k for b in blocks for k in re.findall(r'"((?:ctx|disp)\.[\w.]+)"', b))
    assert len(blocks) >= 11 and len(keys) >= 9, (len(blocks), len(keys))
    for lang in ("en",):
        cat = lang_catalog.testo(lang)
        for k in keys:
            assert f'"{k}"' in cat, (lang, k)


def test_double_click_takes_the_first_available_claim():
    r = run("""const log=[];
      const reg=(id,o)=>AZIONI.register({id,group:'xic',views:['v'],name:()=>id,run:c=>log.push(id),...o});
      reg('main',{doubleClick:()=>true,doubleClickOrder:9});reg('zoom',{doubleClick:c=>c.z,available:c=>c.z,doubleClickOrder:2});reg('ann',{doubleClick:c=>c.a,doubleClickOrder:1,available:c=>c.a});reg('quiet',{});
      const out=[AZIONI.doubleClick({view:'v'}),AZIONI.doubleClick({view:'v',z:true}),AZIONI.doubleClick({view:'v',z:true,a:true}),AZIONI.doubleClick({view:'w'})];
      console.log(JSON.stringify({out,log}))""")
    assert r["log"] == ["main", "zoom", "ann"] and r["out"] == [True, True, True, False]


def test_reset_zoom_first_and_annotations_last():
    r = run("""const reg=(id,g)=>AZIONI.register({id,group:g,views:['v'],name:()=>id,run:()=>0});
      reg('ann','ann');reg('x','xic');reg('help','help');reg('reset','reset');
      console.log(JSON.stringify(AZIONI.forContext({view:'v'}).map(i=>i==='-'?'-':i.id)))""")
    assert r == ["reset", "-", "x", "-", "help", "-", "ann"]


def test_icons_of_the_actions_exist_and_are_decorative():
    """Every key of the id -> icon table is a drawing; the drawings are decorative (aria-hidden) and follow the theme (currentColor); an action gets its icon from the table."""
    js = (f"const fs=require('fs');const I=new Function(fs.readFileSync({str(WEB / 'azioni-icone.js')!r},'utf8')+';return AZIONI_ICONE;')();"
          f"const AZIONI=new Function('AZIONI_ICONE',fs.readFileSync({str(WEB / 'azioni.js')!r},'utf8')+';return AZIONI;')(I);"
          "const a=AZIONI.register({id:'spec.copy',group:'copy',views:['v'],name:()=>'x',run:()=>0});const b=AZIONI.register({id:'zz',group:'copy',views:['v'],iconKey:'xic',name:()=>'x',run:()=>0});"
          "const c=AZIONI.register({id:'yy',group:'copy',views:['v'],name:()=>'x',run:()=>0});const m=AZIONI.forContext({view:'v'});"
          "console.log(JSON.stringify({missing:Object.entries(I.POR_ID).filter(([k,v])=>!I.DISEGNI[v]).map(([k])=>k),svg:Object.keys(I.DISEGNI).map(k=>I.svg(k)),got:[a.iconKey,b.iconKey,c.iconKey||null],menu:m.map(i=>i.icon?1:0)}));")
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


def test_arrangement_actions_show_in_every_view_and_have_texts():
    """Actions with views ['*'] show in any view; the keys of the arrangement actions (arrange.js) exist in the catalog; layout.js never names an experiment."""
    r = run("""AZIONI.register({id:'t.all',group:'disp',views:['*'],name:()=>'x',run:()=>0});
      console.log(JSON.stringify(['spec','chrom','mrm'].map(v=>AZIONI.forContext({view:v}).some(i=>i.id==='t.all'))))""")
    assert r == [True, True, True]
    src = (WEB / "arrange.js").read_text(encoding="utf-8")
    keys = set(re.findall(r'"(arrange\.menu\.[\w.]+)"', src))
    assert len(keys) >= 6, keys
    for lang in ("en",):
        cat = lang_catalog.testo(lang)
        for k in keys | {f"cmd.disp.{i}.{f}" for i in ("arrange", "su", "giu", "intera", "uguali", "ripristina") for f in ("lab", "name", "tip", "kw")}:
            assert f'"{k}"' in cat, (lang, k)
    assert not re.search(r"""["'](?:full|ms2|mrm|spec|chrom)["']""", (WEB / "layout.js").read_text(encoding="utf-8"))


def test_unavailable_entries_are_hidden_and_separators_stay_clean():
    """Every menu hides what is not available now (`off`); no separator is left first, last or doubled; info rows stay."""
    r = run("""const reg=(id,group,o)=>AZIONI.register({id,group,views:['v'],name:()=>id,run:()=>0,...o});
      reg('a','reset',{available:c=>c.ok});reg('b','value',{rows:()=>[{name:'info',dim:true,info:true,run:()=>0}]});reg('c','xic',{available:c=>c.ok});
      reg('d','cont',{available:c=>c.ok});reg('e','help');reg('f','ann',{available:c=>c.ok});
      const names=l=>AZIONI.clean(l).map(i=>i==='-'?'-':i.id);
      console.log(JSON.stringify({off:names(AZIONI.forContext({view:'v',ok:false})),on:names(AZIONI.forContext({view:'v',ok:true})),
        fila:AZIONI.clean([{fila:[{id:'x',off:true}]},{label:'m'},'-','-',{label:'n'},'-']).map(i=>i==='-'?'-':i.label||i.fila)}))""")
    assert r["off"] == ["b", "-", "e"], r
    assert r["on"] == ["a", "-", "b", "-", "c", "-", "d", "-", "e", "-", "f"], r
    assert r["fila"] == ["m", "-", "n"], r


def test_downloads_of_a_view_come_from_the_registry_not_from_the_menu():
    r = run("""const reg=(id,o)=>AZIONI.register({id,group:'lib',views:['v'],name:()=>id,run:()=>0,...o});
      reg('d1',{menu:false,download:true});reg('d2',{menu:false,download:true,available:c=>c.ok});reg('m');
      console.log(JSON.stringify({menu:AZIONI.forContext({view:'v',ok:true}).map(i=>i.id),dl:AZIONI.downloads({view:'v',ok:false}).map(i=>i.label),dl2:AZIONI.downloads({view:'v',ok:true}).map(i=>i.label)}))""")
    assert r == {"menu": ["m"], "dl": ["d1"], "dl2": ["d1", "d2"]}
