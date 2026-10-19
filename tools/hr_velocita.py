"""Measurement (not a test; run from the repository root): cost of the data of scan-by-scan navigation on a high-resolution file, JSON (api/spectra) against the binary block (api/scanbin).
Local server by default; the built site (Pyodide) with MZLAB_SITE=1 or QQQ_SITE=1 (python tools/build_site.py first). File: $HR_FILE or the Exploris file of the data repository."""
import os, subprocess, sys, time, json
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tests_e2e"))
from lib import *
from dati_percorsi import percorso
f = os.environ.get("HR_FILE") or str(percorso("dati/HRMS/Orbitrap_Exploris120_DDApos_10-15min.mzML") or "dati/HRMS/Orbitrap_Exploris120_DDApos_10-15min.mzML")
SITE = os.environ.get("MZLAB_SITE") == "1" or os.environ.get("QQQ_SITE") == "1"
MEAS = """async()=>{
  const k=E.files.find(x=>x.kind==='full').k, out={};
  const T=async(name,fn)=>{const ts=[];for(let r=0;r<4;r++){const t=performance.now();await fn(r*60+10);ts.push(performance.now()-t)}ts.sort((a,b)=>a-b);out[name]=Math.round(ts[1])};
  await J(`api/spectra?k=${k}&i0=0&i1=2&level=1`);                                   // warm
  await T('json60', i=>J(`api/spectra?k=${k}&i0=${i}&i1=${i+59}&level=1`));
  await T('bin60', i=>scBin(`api/scanbin?k=${k}&i0=${i+300}&i1=${i+359}&level=1`));
  let n=0;const r=await fetch(`api/spectra?k=${k}&i0=700&i1=759&level=1`);n=(await r.text()).length;out.jsonKB=Math.round(n/1024);
  const b=await fetch(`api/scanbin?k=${k}&i0=700&i1=759&level=1`);out.binKB=Math.round((await b.arrayBuffer()).byteLength/1024);
  return out}"""
if SITE:
    srv = subprocess.Popen([sys.executable, "-m", "http.server", "8871", "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); time.sleep(1)
    try:
        with sync_playwright() as p:
            b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1500, "height": 1400}).new_page()
            pg.goto("http://127.0.0.1:8871/"); pg.wait_for_selector("#drop", timeout=180000)
            pg.set_input_files("#pick", [f]); pg.wait_for_function("document.querySelectorAll('#flist tr').length >= 2", timeout=300000)
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=300000); pg.wait_for_timeout(3000)
            print("SITE (Pyodide):", json.dumps(pg.evaluate(MEAS)))
    finally:
        srv.terminate()
else:
    r = Run(port=8872, wd="/tmp/wd_vel")
    try:
        with sync_playwright() as p:
            pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
            pg.set_input_files("#pick", [f]); pg.wait_for_timeout(2000); pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(2000)
            print("LOCAL SERVER:", json.dumps(pg.evaluate(MEAS)))
        r.close()
    except Exception as e:
        print("FAIL", e); r.close()
