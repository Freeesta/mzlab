"""Files open one by one as soon as they are read: the first file starts the session, the others are appended in order; while one is still being opened its grey row
(.fl.pend.up, aria-busy) waits at the bottom of the side list; the first file opens by itself as soon as it is read, the others open when read and are shown by a click; × on a waiting row leaves that file unticked and unread;
a file with no data stays in the table unopened and the others go on. The appends are held back by a wrapper of fetch, so there is time to look."""
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_progressivo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
FILES = [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")]
BAD = os.path.join(tempfile.mkdtemp(prefix="progr_"), "rotto.mzML"); open(BAD, "w").write("<?xml version='1.0'?><mzML><run><spectrumList count='1'><spectrum>")
r = Run(port=8931, wd="/tmp/wdprogressivo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#drop")
        pg.evaluate("""(()=>{window.__hold=new Promise(r=>{window.__release=r});window.__appends=0;const f=window.fetch;
          window.fetch=function(u,o){if(typeof u==='string'&&u.includes('api/explore')&&o&&typeof o.body==='string'&&o.body.includes('"append":true')){window.__appends++;(window.__sent=window.__sent||[]).push(JSON.parse(o.body).samples[0].file);return window.__hold.then(()=>f.call(this,u,o))}return f.call(this,u,o)}})()""")       # on the outermost fetch, after the page is up: browser.js answers api/ routes itself and never reaches an earlier wrapper
        pg.evaluate("window.__t = []; new MutationObserver(() => { if (document.querySelector('#flst .fl.up')) window.__t.push(performance.now()); }).observe(document.getElementById('flst'), {childList: true, subtree: true})")
        pg.evaluate("window.__t0 = performance.now()")
        stage(pg, FILES)
        attendi(pg, "E.files.filter(f => !f.gone).length === 1 && window.__appends >= 1", 60000)
        def first_open():
            assert pg.evaluate("OPENNOW.busy()") >= 2, pg.evaluate("OPENNOW.busy()")
            attendi(pg, "document.getElementById('v-workspace').dataset.work === 'panels'", 30000)
            assert pg.evaluate("E.files.filter(f => f.vis && !f.gone).length") == 1, "only the first file is shown while the others are read"
            pg.wait_for_selector("#dpanels .pnl canvas", timeout=60000)
        step("the first file is open and shown by itself, the others wait", first_open)
        names = [os.path.basename(f) for f in FILES]
        state = lambda: pg.evaluate("[E.files.filter(f => !f.gone).map(f => f.file), window.__sent]")
        def grey_rows():
            if not pg.is_visible("#flst"): pg.click("#stbar [data-cmd=file]"); attendi(pg, "!!document.querySelector('#flst') && document.querySelector('#flst').offsetParent")
            rows = pg.locator("#flst .fl.pend.up[aria-busy=true]")
            assert rows.count() == 2, rows.count()
            opened, sent = state()
            txt = " ".join(rows.nth(i).inner_text() for i in range(2))
            assert all((n in txt) != (n in opened) for n in names), (txt, opened)      # a row for each file not open yet
            assert rows.nth(0).locator(".prog[role=progressbar]").count() == 1
            assert pg.get_attribute("#flst", "aria-busy") == "true"
            assert pg.evaluate("window.__t.length && window.__t[0] - window.__t0") < 5000
        step("the others are grey rows with their bar, aria-busy, under the open file", grey_rows)
        def first_click():
            pg.click("#flst .fl:not(.up) .nm"); attendi(pg, "E.files.some(f => f.vis && !f.gone)")
            pg.wait_for_selector(".pnl.chrom canvas", timeout=60000)
            assert pg.evaluate("E.cur") == 0
        step("the first click on a file shows it while the others are still being opened", first_click)
        opened, sent = state()
        queued = [n for n in names if n not in opened and n not in sent]           # the file still in the queue (the other one is held in its append)
        def remove_one():
            assert len(queued) == 1, (opened, sent)
            pg.click(f"#flst [data-up-x='{queued[0]}']")
            attendi(pg, f"!document.querySelector(\"#flst [data-up='{queued[0]}']\")", 10000)
            assert pg.locator("#flst .fl.up").count() == 1
        step("× on a waiting row: that file is not opened", remove_one)
        def release():
            pg.evaluate("window.__release()")
            attendi(pg, "E.files.filter(f => !f.gone).length === 2 && OPENNOW.busy() === 0 && !document.querySelector('#flst .fl.up')", 60000)
            assert pg.get_attribute("#flst", "aria-busy") == "false"
            assert pg.evaluate("window.__appends") == 1, pg.evaluate("window.__appends")
            assert queued[0] not in pg.evaluate("E.files.map(f=>f.file).join()"), pg.evaluate("E.files.map(f=>f.file)")
            assert pg.evaluate(f"ST.files.find(f => f.name === '{queued[0]}').use") is False, "the removed file stays unticked in the table"
        step("the files open when read; the removed one was never asked for", release)
        def same_as_all_at_once():
            a = pg.evaluate("fetch('api/session').then(r=>r.json()).then(j=>j.session.map(s=>[s.file,s.kind,s.ms1,s.ms2]))")
            pg.evaluate("fetch('api/explore',{method:'POST',body:JSON.stringify({samples:%s})})" % pg.evaluate("JSON.stringify(E.files.map(f=>({file:f.file,time:f.time,type:f.type,conc:f.conc,cunit:f.cunit,label:f.label})))"))
            pg.wait_for_timeout(500)
            b = pg.evaluate("fetch('api/session').then(r=>r.json()).then(j=>j.session.map(s=>[s.file,s.kind,s.ms1,s.ms2]))")
            assert a == b, (a, b)
        step("the session is the same as opening the files in one call", same_as_all_at_once)
        def failing():
            pg.set_input_files("#pick", [BAD])
            attendi(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && ST.files.some(f => f.name === 'rotto.mzML') && !document.querySelector('#flst .fl.up')", 60000)
            assert pg.evaluate("!E.files.some(f => !f.gone && f.file === 'rotto.mzML')"), "a file with no data is not opened"
            assert pg.evaluate("E.files.filter(f => !f.gone).length") == 2, "the open files go on"
        step("a file with no data stays in the table, unopened, and its row goes; the others go on", failing)
        b = None
        r.b.close()
finally:
    r.srv.terminate()
for s_ in steps: print("  ", s_)
fails = [s_ for s_ in steps if s_[1] != "ok"]
sys.exit(1 if fails else 0)
