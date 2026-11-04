import sys, os, json, random, time
ROOT_ = os.environ.get("MZLAB_ROOT", os.getcwd()); sys.path.insert(0, ROOT_ + "/tests_e2e")
from playwright.sync_api import sync_playwright
from lib import attendi, ready, diag, loaded
INIT = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
def newpage(b, port, errs, vw=(1500,1000)):
    c = b.new_context(viewport={"width":vw[0],"height":vw[1]}); c.set_default_timeout(120000); pg = c.new_page()
    pg.on("pageerror", lambda e: errs.append(("pageerror", str(e)[:300])))
    pg.on("console", lambda m: errs.append(("console."+m.type, m.text[:300])) if m.type=="error" else None)
    pg.add_init_script(INIT)
    pg.goto(f"http://127.0.0.1:{port}/")
    pg.wait_for_selector("#drop"); attendi(pg, "window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", 60000)
    return c, pg
