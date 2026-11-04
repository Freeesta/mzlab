import sys, shutil
from common import *
N = int(sys.argv[1]); which = sys.argv[2]; PROF = f"/tmp/stress_soak/profile_min{N}"; shutil.rmtree(PROF, ignore_errors=True)
FIRST = "document.querySelectorAll('#flist input[data-k=use]').length >= 1 && !document.querySelector('#fl-up [aria-busy=true]')"
srv, port = serve()
try:
  with sync_playwright() as p:
    ctx = p.chromium.launch_persistent_context(PROF, headless=True, viewport={"width":1400,"height":900}); ctx.set_default_timeout(120000)
    errs = [[] for _ in range(N)]; pgs = [newpage(ctx, errs[i]) for i in range(N)]
    for pg in pgs: boot(pg, port)
    for pg in pgs: pg.set_input_files("#pick", pair(which))
    for pg in pgs: pg.wait_for_function(FIRST, timeout=180000)
    pgs[0].wait_for_timeout(5000)
    out = [(pg.evaluate("JSON.stringify(window.MZLAB_LR_MISS||null)"), pg.evaluate("ST.files.length"), errs[i][:2]) for i, pg in enumerate(pgs)]
    print(N, which, out)
    ctx.close()
finally: srv.terminate()
