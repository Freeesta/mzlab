import sys
from common import *
# usage: soak.py N seed [quota_bytes]
N = int(sys.argv[1]); seed = int(sys.argv[2]); QUOTA = int(sys.argv[3]) if len(sys.argv) > 3 else 0
rnd = random.Random(seed); LOG = open(f"/tmp/stress_soak/soak_{seed}.log", "w")
def log(*a):
    s = " ".join(str(x) for x in a); LOG.write(s + "\n"); LOG.flush(); print(s, flush=True)
PAINT = """() => E.panels.map(p => { const cv = p.cv || (p.el && p.el.querySelector('canvas')); let nz = 0, w = 0, h = 0;
  if (cv && cv.width && cv.height) { w = cv.width; h = cv.height; try { const d = cv.getContext('2d').getImageData(0,0,w,h).data; for (let i = 3; i < d.length; i += 4*37) if (d[i]) nz++; } catch(e) { nz = -1 } }
  const ds = p.el && p.el.querySelector('table, [data-tdati], .tdati'); return [p.id, p.type, p.tab, w, h, nz, !!(p.el && p.el.hidden)]; })"""
def check(pg, tag, errs, want_panels=True):
    miss = pg.evaluate("window.MZLAB_LR_MISS ? JSON.stringify(window.MZLAB_LR_MISS) : ''")
    bad = []
    if miss and miss not in ("[]", "{}"): bad.append("LR_MISS " + miss[:200])
    if errs: bad.append("errs " + str(errs[:3]))
    ban = pg.evaluate("Array.from(document.querySelectorAll('[class*=banner],[role=alert],.err')).map(e=>e.innerText.trim()).filter(t=>t&&/engine|fallback|unavail|error|errore/i.test(t)).slice(0,3)")
    if ban: bad.append("banner " + str(ban))
    if want_panels:
        for r in pg.evaluate(PAINT):
            if r[5] == 0 and not r[6]: bad.append("EMPTY panel " + str(r))
    if bad: log("FAIL", tag, bad)
    return bad
srv, port = serve(); errs = []; fails = 0
try:
  with sync_playwright() as p:
    b = p.chromium.launch(args=["--enable-precise-memory-info"]); c = b.new_context(viewport={"width": 1500, "height": 1100}); c.set_default_timeout(120000)
    pg = newpage(c, errs); boot(pg, port)
    cdp = c.new_cdp_session(pg)
    if QUOTA:
        cdp.send("Storage.overrideQuotaForOrigin", {"origin": f"http://127.0.0.1:{port}", "quotaSize": QUOTA}); log("quota override", QUOTA)
    keys = list(FILES)
    for it in range(1, N + 1):
        k = rnd.choice(keys); t0 = time.time()
        log(f"it={it} open {k}")
        errs.clear()
        pg.set_input_files("#pick", pair(k))
        try: pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1 && !document.querySelector('#fl-up [aria-busy=true]')", timeout=120000)
        except Exception as e: log("FAIL open timeout", it, k, diag_s if False else str(e)[:100]); pg.screenshot(path=f"/tmp/stress_soak/fail_{seed}_{it}.png"); fails += 1; break
        pg.wait_for_timeout(1500)
        info = pg.evaluate("ST.files.map(f=>[f.name,f.kind])"); log("  files", info)
        fails += bool(check(pg, f"it{it} {k} after-open", errs))
        act = rnd.choice(["spec", "chrom", "both", "xic"])
        try:
            if k in ("mrm", "sim"):
                pg.evaluate("reveal(addPanel('mrm', { imode: 'man', intf: 'all', intfFixed: true }))"); act = "mrm"
            else:
                if act in ("spec", "both"): pg.evaluate("(()=>{const f = scanFiles(vis())[0] || scanFiles(tabFiles())[0]; reveal(addPanel('spec', { k: f?.k ?? 0, level: f?.lv || 1 }))})()")
                if act in ("chrom", "both"): pg.evaluate("reveal(addPanel('chrom', {}))")
                if act == "xic":
                    pg.evaluate("(()=>{const c = E.panels.find(q=>q.type==='chrom'); if (c) openXic(c, {})})()")
            pg.wait_for_timeout(1500)
        except Exception as e: log("  act err", str(e)[:200])
        log("  act", act, pg.evaluate(PAINT))
        fails += bool(check(pg, f"it{it} {k} after-{act}", errs))
        # remove all files as user (x of the row), then wait for the 6 s expiry to free
        try: pg.evaluate("removeAllFiles()")
        except Exception as e: log("  rm err", str(e)[:150])
        try: pg.wait_for_function("window.MZLAB_BROWSER && !document.querySelector('#flst .fl')", timeout=10000)
        except Exception: pass
        pg.wait_for_timeout(6600)
        # panels of removed file should be gone/closed
        if it % 10 == 0 or it <= 3:
            r = opfs(pg)
            log(f"MEASURE it={it} opfs files={r['files']} bytes={r['bytes']} byDir={r['byDir']} usage={r['usage']} idb={r['idb']} heap={r['heap']} t={time.time()-t0:.1f}")
            if r["files"] > 6: log("  names", r["names"])
            if r["files"] != 0: log("  LEAK? after removal", r["names"][:12]); fails += 1
        else:
            r = opfs(pg)
            if r["files"] != 0: log("LEAK it", it, r["names"][:12]); fails += 1
        pg.evaluate("E.panels.slice().forEach(p=>{ try { p.el.querySelector('.x').click() } catch(e){} })"); pg.wait_for_timeout(200)
        log("  workers", pg.evaluate("performance.getEntriesByType('resource').filter(r=>/worker/i.test(r.name)).length"), "dt", round(time.time()-t0,1))
    log("DONE fails", fails, "errs", errs[:5])
finally: srv.terminate()
