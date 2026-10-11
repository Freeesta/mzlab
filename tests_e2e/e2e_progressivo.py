"""Opening a list file by file: the view opens on the FIRST file at once, the others are in the list as grey, disabled rows (aria-busy) and light up one by one
(the bar of the list stays while one is missing); a file that fails stays red with its message and the others go on; a file taken off the list is not read.
The server of development answers the appends slowly (a route of the test), so there is time to look. ?progressivo=1 turns it on outside the site."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_progressivo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
FILES = [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")]
r = Run(port=8931, wd="/tmp/wdprogressivo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.add_init_script("""(()=>{window.__hold=new Promise(r=>{window.__release=r});window.__appends=0;const f=window.fetch;
          window.fetch=function(u,o){if(typeof u==='string'&&u.includes('api/explore')&&o&&typeof o.body==='string'&&o.body.includes('"append":true')){window.__appends++;return window.__hold.then(()=>f.call(this,u,o))}return f.call(this,u,o)}})()""")
        pg.goto(f"http://127.0.0.1:{r.port}/?progressivo=1"); pg.wait_for_selector("#drop")
        stage(pg, FILES)
        pg.click("#opbtn")
        pg.wait_for_selector(".pnl.chrom canvas", timeout=60000)
        def first_open():
            assert pg.evaluate("E.files.length") == 1, pg.evaluate("E.files.map(f=>f.file)")
            assert pg.evaluate("E.pending.map(x=>x.file+'|'+x.state).join()").count("wait") == 2
        step("the view opens on the first file while two are still to come", first_open)
        def grey_rows():
            if not pg.is_visible("#flst"): pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(400)
            rows = pg.locator("#flst .fl.pend[aria-busy=true]")
            assert rows.count() == 2, rows.count()
            assert rows.nth(0).locator("input").is_disabled()
            assert "B_FullMass-t15" in rows.nth(0).inner_text() and "B_FullMass-t60" in rows.nth(1).inner_text()
            assert pg.locator("#flst .pendbar[role=progressbar]").count() == 1
            assert pg.get_attribute("#flst", "aria-busy") == "true"
        step("the others are grey, disabled and aria-busy, with the progress bar of the list", grey_rows)
        def first_selectable():
            pg.click("#flst .fl:not(.pend) .nm"); pg.wait_for_timeout(300)
            assert pg.evaluate("E.cur") == 0
            pg.click("#flst .fl.pend >> nth=0"); pg.wait_for_timeout(200)
            assert pg.evaluate("E.cur") == 0 and pg.evaluate("E.files.length") == 1
        step("the first file can be selected while the others are busy; a grey row cannot", first_selectable)
        def remove_one():
            pg.click("#flst [data-pend-rm] >> nth=1"); pg.wait_for_timeout(200)
            assert pg.locator("#flst .fl.pend").count() == 1
        step("a file taken off the list is not read", remove_one)
        def release():
            pg.evaluate("window.__release()")
            pg.wait_for_function("E.files.length===2 && E.pending.length===0", timeout=60000)
            assert pg.locator("#flst .pendbar").count() == 0 and pg.get_attribute("#flst", "aria-busy") == "false"
            assert pg.evaluate("window.__appends") == 1, pg.evaluate("window.__appends")
            assert pg.evaluate("E.files.map(f=>f.file).join()").endswith("B_FullMass-t15.mzML"), pg.evaluate("E.files.map(f=>f.file)")
        step("the files light up when read; the bar goes when none is missing; the removed one was never asked for", release)
        def same_as_all_at_once():
            a = pg.evaluate("fetch('api/session').then(r=>r.json()).then(j=>j.session.map(s=>[s.file,s.kind,s.ms1,s.ms2]))")
            pg.evaluate("fetch('api/explore',{method:'POST',body:JSON.stringify({samples:%s})})" % pg.evaluate("JSON.stringify(E.files.map(f=>({file:f.file,time:f.time,type:f.type,conc:f.conc,cunit:f.cunit,label:f.label})))"))
            pg.wait_for_timeout(500)
            b = pg.evaluate("fetch('api/session').then(r=>r.json()).then(j=>j.session.map(s=>[s.file,s.kind,s.ms1,s.ms2]))")
            assert a == b, (a, b)
        step("the session is the same as opening the files in one call", same_as_all_at_once)
        def failing():
            pg.evaluate("S.adding=true")
            import shutil; shutil.copy(FILES[0], r.wd + "/ok.mzML"); shutil.copy(FILES[1], r.wd + "/ok2.mzML")
            open(r.wd + "/broken.mzML", "w").write("<mzML><spectrum")
            pg.evaluate("exploreList([{file:'ok.mzML',time:null,type:'sample',conc:null,cunit:null,label:null},{file:'broken.mzML',time:null,type:'sample',conc:null,cunit:null,label:null},{file:'ok2.mzML',time:null,type:'sample',conc:null,cunit:null,label:null}]).then(f=>f&&f())")
            pg.wait_for_function("E.pending.some(x=>x.state==='err')", timeout=60000)
            assert pg.locator("#flst .fl.pend.err").count() == 1
            assert pg.locator("#flst .fl.pend.err small").inner_text().strip() != ""
        step("a file that fails stays in red with its message and the others go on", failing)
        b = None
        r.b.close()
finally:
    r.srv.terminate()
for s_ in steps: print("  ", s_)
fails = [s_ for s_ in steps if s_[1] != "ok"]
sys.exit(1 if fails else 0)
