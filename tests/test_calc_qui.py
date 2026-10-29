"""«What can this number be?» (mzlab/web/calc-qui.js): the argument of the field (a number, the result of a sum, a formula), the groups of candidates
(lists, adducts, differences between adducts, neutral losses) with the tolerances of high and low resolution, and the low-resolution table without ppm."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

PRELUDE = f"""const fs=require('fs');
global.window={{}};global.I18N={{lang:'it',t:(k,p)=>k+(p?JSON.stringify(p):'')}};global.EH=s=>String(s);global.fmtFormula=s=>s;global.fmtAdduct=s=>s;global.NOMI={{KEY:k=>k}};
const calc=require({str(WEB / 'calcola.js')!r});global.calcIsExpr=calc.calcIsExpr;global.calcEval=calc.calcEval;
const P=require({str(WEB / 'perdite.js')!r});const LISTE=require({str(WEB / 'liste.js')!r});const CQ=require({str(WEB / 'calc-qui.js')!r});
const AH=new Function(fs.readFileSync({str(WEB / 'calc-addotti-html.js')!r},'utf8')+';return CALCADDHTML')();
const E=new Function(fs.readFileSync({str(WEB / 'elements.js')!r},'utf8')+';return ELEMENTS')();
const EL=Object.fromEntries(E.map(e=>[e.s,e]));
const mono=s=>EL[s].iso.reduce((a,b)=>b[2]>a[2]?b:a)[1];
const mass=f=>{{let m=0;for(const [,s,n] of f.matchAll(/([A-Z][a-z]?)(\\d*)/g))m+=mono(s)*(n?+n:1);return m}};
// the list of the test: one known contaminant (N-butylbenzenesulfonamide [M+H]+)
const idx=LISTE.buildIndex([{{id:'t',name:'test',builtin:true,items:[{{id:'nbbs',name:'N-butylbenzenesulfonamide',mz:214.0896,adduct:'[M+H]+',pol:1}}]}}],null);
// stand-in for api/adduct_table: the Na-H difference only, with the window the page asks for
const asked=[];
const table=async(f,obs,tol,delta)=>{{asked.push({{f,obs,tol,delta}});const ref=+obs||400,win=+tol*1e-6*ref,d=+delta,hits=[];
  if(d&&Math.abs(21.98194-d)<=win)hits.push({{a:'[M+Na]+',b:'[M+H]+',d:21.98194,err_mda:(21.98194-d)*1000}});
  return {{formula:f||null,neutral:null,tol:+tol,rows:[],best:null,diffs:[{{pair:'Na',a:'[M+Na]+',b:'[M+H]+',pol:1,d:21.98194}}],delta:delta?{{value:d,hits}}:undefined}}}};
const dep={{idx,find:LISTE.find,findNominal:LISTE.findNominal,table,mass,losses:P.LOSSES,lossCombos:P.lossCombos}};
const HR={{hr:true,pol:1,tol:5,dec:4}},LR={{hr:false,pol:1,tol:0.5,dec:1}};
"""


def run(code):
    js = PRELUDE + "(async()=>{" + code + "})()"
    out = subprocess.run(["node", "-e", js], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_the_argument_is_the_number_the_result_or_the_formula():
    r = run("console.log(JSON.stringify([CQ.arg('214',0),CQ.arg('100+114',0),CQ.arg('ans',214),CQ.arg('C9H10N2O',0),CQ.arg('',0),CQ.arg('1+',0)]))")
    assert [(a["kind"], a["v"] if a["kind"] == "num" else None) for a in r] == [("num", 214), ("num", 214), ("num", 214), ("formula", None), ("none", None), ("none", None)]


def test_214_in_low_resolution_finds_the_known_contaminant():
    r = run("const g=await CQ.gruppi(CQ.arg('214',0),LR,dep);console.log(JSON.stringify(g.filter(x=>x.id==='liste').map(x=>[x.n,x.items[0].name,x.items[0].diffDa])))")
    assert r[0][0] >= 1 and r[0][1] == "N-butylbenzenesulfonamide" and abs(r[0][2] + 0.0896) < 1e-3


def test_214_in_high_resolution_uses_ppm_not_nominal():
    r = run("const g=await CQ.gruppi(CQ.arg('214.0896',0),HR,dep);const l=g.find(x=>x.id==='liste');const g2=await CQ.gruppi(CQ.arg('214.2',0),HR,dep);console.log(JSON.stringify([l.n,Math.abs(l.items[0].err)<0.1,!!g2.find(x=>x.id==='liste')]))")
    assert r == [1, True, False]            # 214.2 is 0.11 Da away: a match in low resolution, none within 5 ppm


def test_30_in_low_resolution_explains_it_as_ch2o_and_other_losses():
    r = run("const g=await CQ.gruppi(CQ.arg('30',0),LR,dep);const p=g.find(x=>x.id==='perdite');console.log(JSON.stringify(p.items.map(x=>x.ls.map(l=>l.f).join('+')+(x.kind==='rep'?'x'+x.n:''))))")
    assert "CH2O" in r and "H2O+H2O" not in r
    r = run("const g=await CQ.gruppi(CQ.arg('18',0),LR,dep);console.log(JSON.stringify(g.find(x=>x.id==='perdite').items[0].ls[0].f))")
    assert r == "H2O"


def test_the_difference_21_98_is_na_minus_h_in_high_resolution():
    r = run("asked.length=0;const g=await CQ.gruppi(CQ.arg('21.98',0),HR,dep);const d=g.find(x=>x.id==='diff');console.log(JSON.stringify([d.items[0].a,d.items[0].b,asked.find(a=>a.delta).tol]))")
    assert r[:2] == ["[M+Na]+", "[M+H]+"] and float(r[2]) == pytest.approx(7.5)        # 0.003 Da at m/z 400: the narrowest window of a difference


def test_a_sum_is_the_argument_of_the_groups():
    r = run("const g=await CQ.gruppi(CQ.arg('15.9949+2.0157',0),HR,dep);console.log(JSON.stringify(g.map(x=>x.id)))")
    assert "perdite" in r                     # 18.0106 = H2O


def test_low_resolution_gives_the_tolerance_in_da_to_the_adduct_engine():
    r = run("asked.length=0;await CQ.gruppi(CQ.arg('21.98',0),LR,dep);console.log(JSON.stringify(asked.find(a=>a.delta).tol))")
    assert float(r) == pytest.approx(1250)     # 0.5 Da at m/z 400


def test_no_match_drops_the_group():
    r = run("const g=await CQ.gruppi(CQ.arg('9999',0),LR,dep);console.log(JSON.stringify(g.map(x=>x.id)))")
    assert "liste" not in r and "perdite" not in r


def test_the_adduct_table_has_ppm_in_high_resolution_only():
    tab = ("{formula:'C9H10N2O',neutral:176.0950,tol:5,best:0,diffs:[],rows:[{n:'[M+H]+',id:'mh',pol:1,common:true,k:1,z:1,shift:1.00728,mz4:177.1023,mz5:177.10226,mz1:177.1,nominal:177,ppm:1.2,m_if:null}]}")
    hr = run(f"console.log(JSON.stringify(AH.mainHtml({tab},true,true,{{pol:1,more:false}})))")
    lr = run(f"console.log(JSON.stringify(AH.mainHtml({tab},true,true,{{pol:1,more:false,lr:true,tolDa:0.5}})))")
    assert "calc.col.ppm" in hr and "177.1023" in hr and "calc.add.hit" in hr and "calc.add.hitDa" not in hr
    assert "calc.col.da" in lr and "calc.col.ppm" not in lr and "177.1023" not in lr and "177.1<" in lr and "calc.add.hitDa" in lr


def test_the_nearest_group_comes_first():
    r = run("const g=await CQ.gruppi(CQ.arg('18.0106',0),HR,dep);console.log(JSON.stringify(g[0].id))")
    assert r == "perdite"


def test_the_resolution_comes_from_what_is_typed_when_no_file_is_open():
    r = run("const m=t=>{const a=CQ.arg(t,0);return [CQ.mode(a).hr,CQ.mode(a).src,CQ.opts(a).tol]};console.log(JSON.stringify([m('214'),m('214.1'),m('214.0896'),m('214,0896'),m('C9H10N2O'),m('')]))")
    assert r == [[False, "short", 0.5], [False, "short", 0.5], [True, "decimals", 5], [True, "decimals", 5], [True, "formula", 5], [False, "none", 0.5]]


def test_the_open_files_decide_the_resolution_and_name_the_instrument():
    r = run("CQ.follow(()=>({hr:false,name:'QTRAP 3200'}),()=>1);const a=CQ.arg('214.0896',0);const lo=CQ.opts(a);CQ.follow(()=>({hr:true,name:'Orbitrap'}),()=>1);const hi=CQ.opts(CQ.arg('214',0));console.log(JSON.stringify([lo.hr,lo.mode,lo.tol,hi.hr,hi.mode.name,hi.tol]))")
    assert r == [False, {"hr": False, "src": "files", "name": "QTRAP 3200"}, 0.5, True, "Orbitrap", 5]


def test_there_is_no_resolution_switch_and_no_observed_value_in_the_choices():
    r = run("console.log(JSON.stringify([Object.keys(CQ.st),Object.keys(CQ.opts(CQ.arg('214',0)))]))")
    assert r[0] == ["ppm", "da", "pol"] and "obs" not in r[1] and "res" not in r[0]


def test_the_typed_number_is_the_observed_value_of_the_adduct_table():
    r = run("asked.length=0;await CQ.gruppi(CQ.arg('177.1023',0),HR,dep);console.log(JSON.stringify(asked.find(a=>!a.delta)))")
    assert float(r["obs"]) == pytest.approx(177.1023) and r["f"] == ""
