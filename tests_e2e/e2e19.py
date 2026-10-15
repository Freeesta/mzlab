"""Block 1 (MS2 tab): one loading screen only, arrows jump to the nearest scan with data, precursor chromatogram + spectrum coupled
with an arrow, controls that make no sense are grey (not removed), time next to the file name, precursor list in the sidebar."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8819, wd="/tmp/wd19")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def load():
            assert pg.locator("#qq-load").count() == 0, "second loading screen still there"
            assert pg.locator("#ldmsg").count() == 1
        step("only the loading screen with the funny phrases exists", load)
        stage(pg, [mz(f) for f in FILES])
        pg.click("text=Carica dati"); ready(pg)
        open_kind(pg, "ms2"); ready(pg)
        def pair():
            n = pg.locator("#dpanels .parr").count(); assert n >= 1, n
            c = pg.evaluate("E.panels.find(p=>p.tab==='ms2'&&p.type==='chrom')"); 
            order = pg.evaluate("E.panels.filter(p=>p.tab==='ms2').sort((a,b)=>a.y-b.y).map(p=>p.type)"); assert order[:2] == ["chrom", "spec"], order
        step("MS2: precursor chromatogram above, spectrum below, arrow between", pair)
        def dis():
            h = pg.evaluate("""()=>{const p=E.panels.find(q=>q.tab==='ms2'&&q.type==='chrom');const c=p.el.querySelector('.ctl');
              const d=[...c.querySelectorAll(':disabled')].map(x=>x.dataset.o||x.tagName);
              return {d, kinds:[...c.querySelectorAll('option')].map(o=>o.textContent).join('|'), tips:[...c.querySelectorAll('[title]')].filter(x=>x.disabled||x.closest('label.dis')).length}}""")
            assert "mz0" in h["d"] and "corr" in h["d"], h
            assert pg.evaluate("E.panels.find(q=>q.tab==='ms2'&&q.type==='chrom').el.querySelector('[data-a=log]').disabled"), "log is grey on the precursor chromatogram"
            assert "TIC" not in h["kinds"] and "BPC" not in h["kinds"], h
            assert h["tips"] >= 2, h
            pg.screenshot(path=SH + "19_ms2.png")
        step("precursor chromatogram: kind/m-z/blank/baseline/log grey with tooltips, no TIC/BPC", dis)
        def side():
            drawer(pg); t = pg.inner_text("#flst"); assert "PRODUCT ION" in t.upper() and "m/z 229.1" in t, t      # grouped by experiment: the MS2 group lists its precursors
            r = pg.evaluate("""()=>{const f=document.querySelector('#flst .fl:not(.pr) .fi');const b=f.querySelector('.nm').getBoundingClientRect(),s=f.querySelector('small').getBoundingClientRect();return [b.top,s.top,b.right,s.left]}""")
            assert abs(r[0] - r[1]) < 12 and r[3] >= r[2] - 1, r
        step("sidebar: precursors first, time to the right of the file name", side)
        def toggle():
            n = lambda: pg.evaluate("E.panels.filter(p=>p.tab==='ms2').length")
            assert n() == 2, n()                                                     # ONE pair at the start (chromatogram + spectrum)
            assert pg.locator("#pall").count() == 0
            p0 = pg.evaluate("E.panels.find(p=>p.tab==='ms2'&&p.type==='chrom').prec"); ids = pg.evaluate("E.panels.filter(p=>p.tab==='ms2').map(p=>p.id)")
            others = pg.locator("#flst input[data-pr]:not(:checked)"); assert others.count() >= 1
            name = pg.locator("#flst .fl.pr:has(input:not(:checked)) [data-pg]").first; pr1 = name.get_attribute("data-pg")
            name.click(); pg.wait_for_timeout(2500)
            assert n() == 2 and pg.evaluate("E.panels.filter(p=>p.tab==='ms2').map(p=>p.id)") == ids, "no new panel: the pair switches precursor"
            assert str(pg.evaluate("E.panels.find(p=>p.tab==='ms2'&&p.type==='chrom').prec")) == pr1 != str(p0)
            assert str(pg.evaluate("E.panels.find(p=>p.tab==='ms2'&&p.type==='spec').prec")) == pr1
            sp = pg.evaluate("(()=>{const s=E.panels.find(p=>p.tab==='ms2'&&p.type==='spec');return {r0:s.r0,lock:!!s.lock,zy:s.zoomY}})()"); assert sp["r0"] is not None and not sp["lock"] and sp["zy"] is None, sp
            pg.locator("#flst input[data-pr]:not(:checked)").first.click(); pg.wait_for_timeout(1500); assert n() == 3, n()      # a second chromatogram alone (the spectrum comes on a double click)
            pg.locator("#flst input[data-pr]:checked").nth(1).click(); pg.wait_for_timeout(500); assert n() == 1, n()      # the second precursor in the list is the one with the pair: it closes, the lone chromatogram stays
            pg.evaluate("(()=>{const p=E.panels.find(p=>p.tab==='ms2'&&p.type==='chrom');p._parOpen=true;ctl(p)})()")
            sel = pg.evaluate("(()=>{const c=E.panels.find(p=>p.tab==='ms2'&&p.type==='chrom').el.querySelector('[data-o=prec]');return c?c.getBoundingClientRect().width:0})()"); assert sel > 90, sel     # the precursor menu is wide enough to read
            pg.screenshot(path=SH + "19_ms2b.png")
        step("sidebar precursors: click switches the one pair, tick adds a second pair, readable menu", toggle)
        # MS2 arrows (nearest scan with data): covered by e2e_scroll «MS2: arrows jump over empty scans»
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
