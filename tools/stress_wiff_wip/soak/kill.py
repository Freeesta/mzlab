# usage: kill.py FILEKEY DELAY_S RUNID : load a wiff, kill the page DELAY_S seconds after set_input_files (mid-copy), close the whole context, reopen the same persistent profile, inspect OPFS, reload, load again
import sys, shutil
from common import *
which = sys.argv[1]; delay = float(sys.argv[2]); rid = sys.argv[3]; PROF = f"/tmp/stress_soak/profile_kill{rid}"; shutil.rmtree(PROF, ignore_errors=True)
FIRST = "document.querySelectorAll('#flist input[data-k=use]').length >= 1 && !document.querySelector('#fl-up [aria-busy=true]')"
srv, port = serve()
def launch(p): return p.chromium.launch_persistent_context(PROF, headless=True, viewport={"width":1400,"height":900})
try:
  with sync_playwright() as p:
    ctx = launch(p); ctx.set_default_timeout(120000); errs = []; pg = newpage(ctx, errs); boot(pg, port)
    pg.set_input_files("#pick", pair(which))
    t0 = time.time()
    while time.time() - t0 < delay:
        try: pg.wait_for_function("false", timeout=500)
        except Exception: pass
    r = opfs(pg); print("at kill: opfs", r["files"], r["bytes"], r["names"][:8])
    pg.close(); ctx.close()
    ctx = launch(p); ctx.set_default_timeout(120000); errs = []; pg = newpage(ctx, errs)
    # first page of the new context
    pg.goto(f"http://127.0.0.1:{port}/")
    for _ in range(40):
        try: pg.wait_for_function("false", timeout=500)
        except Exception: pass
    print("relaunch: drop visible", pg.evaluate("(()=>{const d=document.getElementById('drop');return d&&!d.hidden&&d.offsetParent!==null})()"), "has ST", pg.evaluate("typeof ST!=='undefined'"))
    r = opfs(pg); print("after relaunch: opfs", r["files"], r["bytes"], r["names"][:12], "usage", r["usage"])
    print("list on start:", pg.evaluate("typeof ST!=='undefined' ? ST.files.map(f=>f.name) : null"), "toasts", pg.evaluate("Array.from(document.querySelectorAll('[role=status],[role=alert],.toast')).map(e=>e.innerText.trim().slice(0,150)).filter(Boolean)"))
    print("page text head:", pg.evaluate("document.body.innerText.replace(/\\s+/g,' ').slice(0,300)"))
    pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
    pg.set_input_files("#pick", pair(which))
    try:
        pg.wait_for_function(FIRST, timeout=150000); print("reopen ok", pg.evaluate("ST.files.map(f=>f.name)"))
        pg.evaluate("(()=>{const f=E.files[0]; reveal(addPanel(f.kind==='mrm'?'mrm':'chrom', f.kind==='mrm'?{imode:'man',intf:'all',intfFixed:true}:{}))})()")
        pg.wait_for_timeout(2500)
        print("panels", pg.evaluate("E.panels.map(p=>[p.type,p.cv&&p.cv.width])"))
    except Exception as e:
        print("REOPEN FAIL", str(e)[:150]); print(pg.evaluate("Array.from(document.querySelectorAll('#fl-up [data-up]')).map(r=>r.innerText.replace(/\\s+/g,' ').slice(0,150))"))
    r = opfs(pg); print("end: opfs", r["files"], r["bytes"], r["names"][:14])
    print("nat entries:", [n for n in r["names"] if "nat~" in n])
    pg.evaluate("removeAllFiles()")
    for _ in range(16):
        try: pg.wait_for_function("false", timeout=500)
        except Exception: pass
    r = opfs(pg); print("after remove all + 8s:", r["files"], r["bytes"], r["names"])
    print("miss", pg.evaluate("window.MZLAB_LR_MISS||null"), "errs", errs[:4])
    ctx.close()
finally: srv.terminate()
