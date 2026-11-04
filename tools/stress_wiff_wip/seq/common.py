import sys, os, json, time, random, subprocess, urllib.request
R=os.environ.get("MZLAB_REPO") or os.path.abspath(os.path.join(os.path.dirname(__file__),"..","..",".."))
sys.path.insert(0, R+"/tests_e2e")
from playwright.sync_api import sync_playwright
from lib import attendi, free_port
INIT="try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}"
def serve(port):
    p=subprocess.Popen([sys.executable,R+"/tests_e2e/servi.py",str(port),R+"/site"],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    for _ in range(50):
        try: urllib.request.urlopen(f"http://127.0.0.1:{port}/",timeout=1); break
        except Exception: time.sleep(.2)
    return p
def newpage(b, errs, port, ctx=None):
    c = ctx or b.new_context(viewport={"width":1500,"height":1100}); c.set_default_timeout(120000); pg=c.new_page()
    pg.on("pageerror", lambda e: errs.append(("pageerror",str(e)[:300])))
    pg.on("console", lambda m: errs.append(("console."+m.type,m.text[:300])) if m.type=="error" else None)
    pg.add_init_script(INIT)
    pg.goto(f"http://127.0.0.1:{port}/"); pg.wait_for_selector("#drop")
    attendi(pg,"window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange",60000)
    return c,pg
