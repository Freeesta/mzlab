"""Desktop update notice (mzlab/web/aggiorna.js) with a simulated Tauri: silent check, manual check, download with progress, restart, new app, errors, once a day."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_aggiorna.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
FAKE = """window.MZLAB_DESKTOP = true; window.__calls = []; window.__reply = {}; window.__handlers = {};
window.__TAURI__ = { core: { invoke: async (c) => { window.__calls.push(c); const r = window.__reply[c]; if (r === 'throw') throw new Error('x');
    if (c === 'update_apply' && window.__handlers['update-progress']) window.__handlers['update-progress']({ payload: { done: 1, total: 2, bytes: 3e6 } });
    return JSON.stringify(r || {}); } },
  event: { listen: async (n, f) => { window.__handlers[n] = f; return () => { delete window.__handlers[n]; }; } } };"""
r = Run(port=8925, wd="/tmp/wd_aggiorna")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 1400, "height": 900})
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.upd.last',String(Date.now()));}catch(_){}")
        pg.add_init_script(FAKE)
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        def box(): return pg.inner_text("#updbox") if pg.query_selector("#updbox") else ""
        def run(reply, silent, cmd="update_check"):
            pg.evaluate("document.getElementById('updbox')?.remove()")
            pg.evaluate("r=>{window.__calls=[];window.__reply={update_check:r}}", reply)
            pg.evaluate(f"DESKUPD.check({str(silent).lower()})"); pg.wait_for_timeout(200)
        def silent_quiet():
            run({"status": "uptodate"}, True); assert box() == "", box()
            run({"status": "error", "error_key": "update.error.offline"}, True); assert box() == "", box()
            assert pg.evaluate("__calls") == ["update_check"]
        step("the silent check says nothing when up to date or on error", silent_quiet)
        def manual_uptodate():
            run({"status": "uptodate"}, False); assert pg.query_selector("#updbox [data-a=close]")
            pg.click("#updbox [data-a=close]"); assert pg.query_selector("#updbox") is None
        step("the manual check says it is up to date and closes", manual_uptodate)
        def available():
            run({"status": "available", "files": 3, "bytes": 2500000}, True)
            t = box(); assert "3" in t and "2.5" in t.replace(",", "."), t
            assert pg.query_selector("#updbox [data-a=go]") and pg.query_selector("#updbox [data-a=close]")
        step("a silent check that finds an update shows the notice with Download", available)
        def apply():
            pg.evaluate("window.__reply.update_apply={status:'ready'}"); pg.click("#updbox [data-a=go]"); pg.wait_for_selector("#updbox [data-a=restart]")
            assert "update_apply" in pg.evaluate("__calls")
            pg.click("#updbox [data-a=restart]"); pg.wait_for_timeout(100)
            assert pg.evaluate("__calls").count("update_restart") == 1
            assert pg.evaluate("Object.keys(__handlers)") == [], "the progress listener must be removed"
        step("Download applies the update, then Restart restarts", apply)
        def apply_error():
            run({"status": "available", "files": 1, "bytes": 1000}, False)
            pg.evaluate("window.__reply.update_apply={status:'error',error_key:'update.error.hash'}"); pg.click("#updbox [data-a=go]"); pg.wait_for_timeout(200)
            assert pg.query_selector("#updbox [data-a=restart]") is None and pg.query_selector("#updbox [data-a=close]")
            pg.evaluate("window.__reply.update_apply='throw'"); pg.click("#updbox [data-a=close]")
            run({"status": "available", "files": 1, "bytes": 1000}, False); pg.click("#updbox [data-a=go]"); pg.wait_for_timeout(200)
            assert pg.query_selector("#updbox [data-a=close]") and not pg.query_selector("#updbox [data-a=restart]")
        step("errors on download show only Close", apply_error)
        def newapp():
            run({"status": "newapp", "min": "9.9"}, False)
            a = pg.query_selector("#updbox a"); assert a and "github.com/Freeesta/mzlab" in a.get_attribute("href") and a.get_attribute("rel") == "noopener noreferrer"
            assert "9.9" in box()
        step("a newer app is needed: link to the releases", newapp)
        def upd(): return pg.evaluate("__calls.filter(c => c.startsWith('update'))")       # the page makes other desktop calls on load (the data folder)
        def once_a_day():
            pg.evaluate("window.__calls=[]"); pg.evaluate("localStorage.setItem('qqq.upd.last',String(Date.now()))")
            pg.evaluate("window.dispatchEvent(new Event('load'))"); pg.wait_for_timeout(4600)
            assert upd() == [], "checked twice in a day: " + str(upd())
            pg.evaluate("localStorage.setItem('qqq.upd.last','1')"); pg.evaluate("window.__reply={update_check:{status:'uptodate'}}")
            pg.evaluate("window.dispatchEvent(new Event('load'))"); pg.wait_for_function("__calls.some(c => c === 'update_check')", timeout=15000)
            assert upd() == ["update_check"], upd()
        step("the automatic check runs once a day", once_a_day)
        def rel(reply, force=True):
            pg.evaluate("document.getElementById('relbox')?.remove()")
            pg.evaluate("r=>{window.__calls=[];window.__reply={release_check:r}}", reply)
            pg.evaluate(f"DESKUPD.release({str(force).lower()})")
        def rel_calls(): return pg.evaluate("__calls.filter(c => c === 'release_check')")
        def release_notice():
            pg.evaluate("localStorage.removeItem('qqq.rel.seen')")
            rel({"status": "none"}); pg.wait_for_function("__calls.includes('release_check')"); assert pg.query_selector("#relbox") is None
            rel("throw"); pg.wait_for_function("__calls.includes('release_check')"); assert pg.query_selector("#relbox") is None
            rel({"status": "newer", "version": "9.9.9"}); pg.wait_for_selector("#relbox")
            t = pg.inner_text("#relbox"); assert "9.9.9" in t, t
            a = pg.query_selector("#relbox a"); assert a.get_attribute("href") == "https://github.com/Freeesta/mzlab/releases/latest" and a.get_attribute("rel") == "noopener noreferrer"
        step("a newer release shows a notice with the link; none or an error shows nothing", release_notice)
        def release_seen():
            pg.click("#relbox button"); assert pg.query_selector("#relbox") is None
            assert pg.evaluate("localStorage.getItem('qqq.rel.seen')") == "9.9.9"
            rel({"status": "newer", "version": "9.9.9"}); pg.wait_for_function("__calls.includes('release_check')"); pg.evaluate("0")
            assert pg.query_selector("#relbox") is None, "closed for that version: not shown again"
            rel({"status": "newer", "version": "9.9.10"}); pg.wait_for_selector("#relbox")
        step("a notice closed for a version does not come back for that version, only for a newer one", release_seen)
        def release_daily():
            pg.evaluate("localStorage.setItem('qqq.rel.last',String(Date.now()))")
            rel({"status": "newer", "version": "99.0.0"}, False); pg.evaluate("new Promise(r => setTimeout(r, 0))")
            assert rel_calls() == [], "asked twice in a day"
            pg.evaluate("localStorage.setItem('qqq.rel.last','1')")
            rel({"status": "newer", "version": "99.0.0"}, False); pg.wait_for_selector("#relbox")
            assert rel_calls() == ["release_check"]
        step("the release check runs at most once a day", release_daily)
        def row():
            assert 'id="uip-upd"' in pg.evaluate("DESKUPD.row()")
        step("the settings row exists in the desktop app", row)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
