import sys, shutil
from common import *
PROF = "/tmp/stress_soak/profile_tabs"; shutil.rmtree(PROF, ignore_errors=True)
LOG = open("/tmp/stress_soak/tabs.log", "w")
def log(*a):
    s = " ".join(str(x) for x in a); LOG.write(s + "\n"); LOG.flush(); print(s, flush=True)
FIRST = "document.querySelectorAll('#flist input[data-k=use]').length >= 1 && !document.querySelector('#fl-up [aria-busy=true]')"
srv, port = serve()
try:
  with sync_playwright() as p:
    ctx = p.chromium.launch_persistent_context(PROF, headless=True, viewport={"width":1400,"height":900}, args=["--enable-precise-memory-info"]); ctx.set_default_timeout(120000)
    errs = [[], [], []]; pgs = [newpage(ctx, errs[i]) for i in range(3)]
    for pg in pgs: boot(pg, port)
    which = sys.argv[1] if len(sys.argv) > 1 else "full"
    # A: all three tabs open the same file at once
    for pg in pgs: pg.set_input_files("#pick", pair(which))
    for i, pg in enumerate(pgs):
        try: pg.wait_for_function(FIRST, timeout=180000); log("tab", i, "opened", pg.evaluate("ST.files.map(f=>[f.name,f.kind])")[:3])
        except Exception as e: log("tab", i, "OPEN FAIL", str(e)[:100]); pg.screenshot(path=f"/tmp/stress_soak/tabs_fail{i}.png")
    pgs[0].wait_for_timeout(3000)
    r = opfs(pgs[0]); log("opfs after concurrent open", r["files"], r["names"])
    for i, pg in enumerate(pgs):
        log("tab", i, "panels", pg.evaluate("E.panels.map(p=>[p.type, p.cv&&p.cv.width])"), "miss", pg.evaluate("JSON.stringify(window.MZLAB_LR_MISS||null)"), "errs", errs[i][:4])
    # B: tab 0 removes the file (frees OPFS) while tabs 1,2 still use it; then tabs 1,2 draw
    pgs[0].evaluate("removeAllFiles()"); pgs[0].wait_for_timeout(7500)
    r = opfs(pgs[0]); log("opfs after tab0 removal", r["files"], r["names"])
    for i in (1, 2):
        pg = pgs[i]
        try:
            pg.evaluate("(()=>{ if (window.E && E.files.length) reveal(addPanel(E.files[0].kind==='mrm'?'mrm':'spec', E.files[0].kind==='mrm'?{imode:'man',intf:'all',intfFixed:true}:{k:0,level:1})) })()"); pg.wait_for_timeout(2500)
            log("tab", i, "after removal in tab0: panels", pg.evaluate("E.panels.map(p=>[p.type, p.cv&&p.cv.width])"), "errs", errs[i][-4:])
        except Exception as e: log("tab", i, "act err", str(e)[:150])
    # C: tab 0 reopens while 1,2 are sitting on it
    pgs[0].set_input_files("#pick", pair(which))
    try: pgs[0].wait_for_function(FIRST, timeout=180000); log("tab0 reopened ok")
    except Exception as e: log("tab0 reopen FAIL", str(e)[:100])
    pgs[0].wait_for_timeout(2000)
    r = opfs(pgs[0]); log("opfs end", r["files"], r["names"])
    for i in range(3): log("errs tab", i, errs[i])
    ctx.close()
finally: srv.terminate()
