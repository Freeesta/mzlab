# usage: two.py FILEKEY RUNID  -- two tabs of one persistent profile load the same wiff at the same time; dumps what each tab shows
import sys, shutil
from common import *
which = sys.argv[1]; rid = sys.argv[2]; PROF = f"/tmp/stress_soak/profile_two{rid}"; shutil.rmtree(PROF, ignore_errors=True)
srv, port = serve()
DUMP = """() => ({files: ST.files.map(f=>f.name), busy: Array.from(document.querySelectorAll('#fl-up [data-up]')).map(r=>r.dataset.up+'|'+r.getAttribute('aria-busy')+'|'+r.innerText.replace(/\\s+/g,' ').slice(0,90)),
  unopen: (document.querySelector('#unopen')||{}).innerText||'', toasts: Array.from(document.querySelectorAll('[role=status],[role=alert],.toast')).map(e=>e.innerText.trim().slice(0,150)).filter(Boolean), miss: window.MZLAB_LR_MISS||null})"""
try:
  with sync_playwright() as p:
    ctx = p.chromium.launch_persistent_context(PROF, headless=True, viewport={"width":1400,"height":900}); ctx.set_default_timeout(120000)
    NT = int(os.environ.get('NT','2')); errs = [[] for _ in range(NT)]; pgs = [newpage(ctx, errs[i]) for i in range(NT)]
    for pg in pgs: boot(pg, port)
    for pg in pgs: pg.set_input_files("#pick", pair(which))
    if os.environ.get('WF'): [pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1 && !document.querySelector('#fl-up [aria-busy=true]')", timeout=180000) for pg in pgs]
    if os.environ.get('BF'): [pg.bring_to_front() for pg in pgs]
    # keep every tab driven with rAF-polling waits in turn (a headless tab only advances while a rAF poll runs), dump every 20 s
    for t in range(1, 10):
        for pg in pgs:
            try: pg.wait_for_function("false", timeout=2000)
            except Exception: pass
        if t % 4 == 0 or t == 9:
            for i, pg in enumerate(pgs): print(f"round{t} tab{i}", json.dumps(pg.evaluate(DUMP))[:1400], errs[i][:3])
    r = opfs(pgs[0]); print("opfs", r["files"], r["names"][:40])
    ctx.close()
finally: srv.terminate()
