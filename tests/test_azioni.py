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


# the header buttons of a panel (azioni-barra.js): the ids and the order of every kind are those of the old hand-written header
HEADER = {
    "chrom": "izoom fit tlink iauto iall iman idel intf iclr smooth log xic xwcl dl more max close",
    "xic": "izoom fit tlink iauto iall iman idel intf iclr smooth log dl more max close",
    "mrm": "izoom fit tlink iauto iall iman idel intf iclr smooth log dl more max close",
    "spec": "xicb xicl fit rul par rel lock dl xicp tabp more max close",
    "map": "fit dl more max close",
}
# the actions drawn only by the shared panel bar (AZIONI.pbar): the zoom and gestures buttons of the map, whose header keeps its own .mznav group
BAR_ONLY = {"map": "mapzin mapzout maphelp"}


def bar(view, extra="", fn="bar"):
    js = ("const fs=require('fs');const AZIONI=new Function(fs.readFileSync(%r,'utf8')+';return AZIONI;')();" % str(WEB / "azioni.js")
          + "global.AZIONI=AZIONI;global.I18N={t:k=>k};global.EH=s=>s;for(const k of ['ZOOM','FIT','SYNC','RULER','UNLOCK','AUTO','MAN','DEL_INT','SMOOTH','DL','FS'])global['IC_'+k]='<svg/>';"
          + "global.window=global;global.XICBTN={lens:{ok:p=>!p.nolens}};global.UIP={xicLens:true};global.ms2Chrom=p=>!!p.ms2;global.specRel=p=>!!p.rel;global.ESP={mrm:p=>p.type==='mrm'};global.MAPPA={SV:s=>s,ICO:{plus:'+'},ad2:()=>({}),runCmd:()=>{}};for(const id of ['mapzin','mapzout','maphelp'])AZIONI.register({id,group:'zoom',ic:'plus',views:['map'],menu:false,name:()=>id,svg:()=>''});eval(fs.readFileSync(%r,'utf8')+';global.h=h;global.raw=raw;');" % str(WEB / "html-sicuro.js") + "" + "eval(fs.readFileSync(%r,'utf8'));" % str(WEB / "azioni-barra.js")
          + "const host={innerHTML:'',querySelector:()=>null};const p={type:%r,tl:true,smooth:true,log:false,imode:null};" % view + extra
          + "AZIONI.bar({view:%r,p},host%s);console.log(JSON.stringify({html:host.innerHTML,ids:AZIONI.list({view:%r,p},'barra').map(a=>a.id),runs:AZIONI.list({view:%r,p},'barra').filter(a=>typeof a.run==='function').map(a=>a.id)}));" % (view, "," + fn if fn != "bar" else "", view, view))
    return json.loads(subprocess.run(["node", "-e", js], check=True, capture_output=True, encoding="utf-8").stdout)


@pytest.mark.parametrize("view", sorted(HEADER))
def test_header_buttons_are_registry_actions_drawn_in_order(view):
    r = bar(view)
    ids = re.findall(r'data-a="(\w+)"', r["html"]) + (["close"] if 'class="x"' in r["html"] else [])
    assert ids == HEADER[view].split(), "the ids and the order of the old header"
    assert sorted(i[len("panel."):] for i in r["ids"]) == sorted(HEADER[view].split() + BAR_ONLY.get(view, "").split())
    assert all(i.startswith("panel.") for i in r["ids"])


# the buttons the modules used to append by hand are registry actions too; the UV spectrum has the XWC split button instead of the XIC lens
def test_buttons_of_the_modules_are_actions_with_a_run():
    r = bar("spec", "p.uv=true;")
    ids = re.findall(r'data-a="(\w+)"', r["html"])
    assert ids == "xicb xicl xwc xwcl fit rul par rel lock dl tabp more max".split(), "XWC in the XIC group, no lens switch"
    assert {"panel.xwc", "panel.xwcs"} <= set(r["ids"]) and "panel.xicp" not in r["ids"]
    for view in ("spec", "chrom"):
        r = bar(view)
        assert {i for i in r["ids"] if i != "panel.more"} <= set(r["runs"]), "every header button has a run"
    tag = lambda html, i: re.search(r'<button[^>]*data-a="%s"[^>]*>' % i, html).group(0)
    spec = bar("spec", "p.nolens=true;")["html"]
    assert "hidden" in tag(spec, "xicp") and "hidden" in tag(spec, "xicl") and "hidden" in tag(bar("chrom")["html"], "xwcl")
    assert 'aria-pressed="true"' in tag(bar("spec")["html"], "xicp") and "hidden" not in tag(bar("spec")["html"], "xicp")


def test_header_button_state_comes_from_the_panel():
    tag = lambda html, i: re.search(r'<button[^>]*data-a="%s"[^>]*>' % i, html).group(0)
    r = bar("chrom", "p.ms2=true;p.log=true;p.tl=false;")
    assert "disabled" in tag(r["html"], "log") and "disabled" in tag(r["html"], "xic")
    assert 'aria-pressed="false"' in tag(r["html"], "tlink") and 'class="bt tgs on"' in tag(r["html"], "smooth")
    r = bar("chrom")
    assert "disabled" not in tag(r["html"], "log") and 'aria-pressed="true"' in tag(r["html"], "tlink")


# the shared panel bar (AZIONI.bar with groups, as pbar.js passes them): bar-only rows are drawn, an unavailable action is hidden (not greyed), a group is drawn only when it has a button
PB = "{of:a=>a.bid==='more'||a.nodata?'':a.bar==='o'||a.bar==='e'?'o':a.bar==='i'?'a':'v',groups:[['v','V'],['a','A'],['o','O']],wrap:(k,t,s)=>`<g ${k}>${s}</g>`}"


def test_shared_bar_draws_groups_with_the_bar_only_buttons():
    group = lambda html, g: re.search(r"<g %s>(.*?)</g>" % g, html, re.S)
    ids = lambda html, g: re.findall(r'data-a="(\w+)"', group(html, g).group(1))
    m = bar("map", fn=PB)["html"]
    assert ids(m, "v") == ["fit", "mapzin", "mapzout", "maphelp"] and ids(m, "o") == ["dl", "max"] and not group(m, "a"), "the map zoom buttons go through the same renderer; no empty group"
    assert 'data-a="close"' not in m and 'data-a="more"' not in m, "close and the overflow stay in the header"
    c = bar("chrom", fn=PB)["html"]
    assert ids(c, "a") == ["iauto", "iall", "iman", "idel", "intf", "iclr"] and ids(c, "o") == ["dl", "max"]


def test_shared_bar_hides_what_is_unavailable():
    tag = lambda html, i: re.search(r'<button[^>]*data-a="%s"[^>]*>' % i, html).group(0)
    h = bar("chrom", "p.ms2=true;", fn=PB)["html"]
    assert " hidden" in tag(h, "log") and "disabled" not in tag(h, "log")
    h = bar("chrom", fn=PB)["html"]
    assert " hidden" not in tag(h, "log")
