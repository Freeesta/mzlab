"""The arrow of every drop-down menu (select) leaves room to the text: right padding for the arrow, ellipsis for long names; light and dark theme, 1280 and 390 px, long file names."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
r = Run(port=8898, wd="/tmp/wd_select_freccia")
LONG = "Campione_molto_lungo_FullMass_s1_replicato_A"
CHECK = """() => [...document.querySelectorAll('select')].filter(s => s.offsetParent && !s.multiple && !s.size).map(s => {
  const c = getComputedStyle(s), r = s.getBoundingClientRect(); const o = s.options[s.selectedIndex];
  return { id: s.dataset.o || s.id || s.className, pr: parseFloat(c.paddingRight), te: c.textOverflow, w: r.width, arrow: c.backgroundImage.includes('gradient'), txt: o ? o.textContent : '' } })"""
def check(pg, tag):
    rows = pg.evaluate(CHECK); assert len(rows) >= 3, rows
    for x in rows:
        assert x["pr"] >= 24, (tag, "the text would run under the arrow", x)
        assert x["te"] == "ellipsis", (tag, x)
        assert x["arrow"] or x["id"] == "ttlsel", (tag, "arrow missing", x)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        for w, theme in ((1280, "light"), (1280, "dark"), (390, "light")):
            ctx = b.new_context(locale="en-US", viewport={"width": w, "height": 900})
            ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1')}catch(_){}")
            pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/?telefono=0"); pg.wait_for_timeout(800)
            pg.evaluate(f"document.documentElement.dataset.theme='{theme}'")
            pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); pg.wait_for_timeout(800)
            pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); pg.wait_for_timeout(500)
            pg.evaluate("(l) => { E.files.forEach(f => { f.label = l + '_' + f.k; }); E.panels.forEach(q => { if (q.el && typeof ctl === 'function') ctl(q); }); if (typeof renderFileList === 'function') renderFileList(); }", LONG)
            pg.wait_for_timeout(400)
            check(pg, f"{w}px {theme}")
            pg.click("#np-set"); pg.wait_for_timeout(300); check(pg, f"{w}px {theme} settings"); pg.keyboard.press("Escape")
            steps.append((f"{w}px, {theme}: room for the arrow and ellipsis on every menu", "ok"))
            ctx.close()
        b.close()
finally:
    pass
for s in steps: print(s)
print("ERRORS", r.errs)
assert len(steps) == 3 and not r.errs
