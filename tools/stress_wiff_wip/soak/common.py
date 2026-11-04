import sys, os, json, time, subprocess, urllib.request, random
sys.path.insert(0, os.path.join(os.environ.get("MZLAB_REPO", os.getcwd()), "tests_e2e"))
from lib import free_port, ROOT
from playwright.sync_api import sync_playwright
D = os.environ["STRESS_WIFF_DIR"]   # directory holding FullScan.wiff, MRM.wiff, MS2.wiff, SIM.wiff (+ .wiff.scan)
FILES = {"full": D + "/FullScan.wiff", "mrm": D + "/MRM.wiff", "ms2": D + "/MS2.wiff", "sim": D + "/SIM.wiff"}
def pair(k): return [FILES[k], FILES[k] + ".scan"]
INIT = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}"
def serve():
    port = free_port()
    p = subprocess.Popen([sys.executable, str(ROOT/"tests_e2e"/"servi.py"), str(port), str(ROOT/"site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(50):
        try: urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1); break
        except Exception: time.sleep(0.2)
    return p, port
OPFS = """async () => {
  const out = {files:0, bytes:0, byDir:{}, names:[]};
  async function walk(d, path) { for await (const [n, h] of d.entries()) { const p = path + '/' + n;
    if (h.kind === 'directory') await walk(h, p); else { let s = 0; try { s = (await h.getFile()).size } catch(e){ s = -1 } out.files++; if (s>0) out.bytes += s; const top = p.split('/').slice(0,3).join('/'); out.byDir[top] = (out.byDir[top]||0)+1; out.names.push(p+':'+s); } } }
  await walk(await navigator.storage.getDirectory(), '');
  const est = await navigator.storage.estimate(); out.usage = est.usage; out.quota = est.quota;
  try { out.idb = (await indexedDB.databases()).map(d=>d.name+'@'+d.version) } catch(e){}
  out.heap = performance.memory ? performance.memory.usedJSHeapSize : null;
  return out; }"""
def opfs(pg): return pg.evaluate(OPFS)
def newpage(c, errs):
    pg = c.new_page()
    pg.on("pageerror", lambda e: errs.append(("pageerror", str(e)[:300])))
    pg.on("console", lambda m: errs.append(("console." + m.type, m.text[:300])) if m.type == "error" else None)
    pg.add_init_script(INIT)
    return pg
def boot(pg, port):
    pg.goto(f"http://127.0.0.1:{port}/")
    pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
