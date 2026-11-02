"""Empty states of the side panels (lists, losses, adducts): the file-list leaf and one short line, no layout shift while it falls, still under reduced motion, light/dark, phone width, axe clean."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

bad = []
def check(name, ok, info=""):
    print(("OK  " if ok else "FAIL"), name, "" if ok else info)
    if not ok: bad.append(name)

# the three panels in their empty state: the lists one is reached with a search that finds nothing, the other two get the same markup
# the quiet way their code uses (the catalogues are never empty on their own)
HOLD = {"lists": "#sb-panel-lists #lst-tbody", "losses": "#sb-panel-losses #nl-list", "adducts": "#sb-panel-adducts .cq-out"}
KEY = {"lists": "empty.lists", "losses": "empty.losses", "adducts": "empty.adducts"}

def show(pg, which):
    if which == "lists":
        pg.click('#stbar [data-cmd="lists"]'); frames(pg, 6)
        pg.fill("#lst-txt", "zzzzqqqq"); frames(pg, 20)
    else:
        pg.click('#stbar [data-cmd="addloss"]'); frames(pg, 6)
        pg.click("#al-t-" + ("losses" if which == "losses" else "adducts")); frames(pg, 4)
        pg.evaluate("([q, k]) => { document.querySelector(q).innerHTML = window.EMPTYLEAF(k); }", [HOLD[which], KEY[which]]); frames(pg, 3)

if __name__ == "__main__":
    r = Run(port=9124, wd="/tmp/wd_foglia_pannelli")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "window.qqReady && document.querySelector('#fl-hint')"); frames(pg, 5)
            for which in ("lists", "losses", "adducts"):
                show(pg, which)
                S = HOLD[which] + " .empty-leaf"
                v = pg.evaluate("""s => { const e = document.querySelector(s); if (!e) return null; const l = e.querySelector('svg.leaf'), t = e.querySelector('span');
                  return {svg: !!l, hid: l && l.getAttribute('aria-hidden'), line: t ? t.textContent.trim() : '', lines: e.querySelectorAll('span').length, vis: e.offsetParent !== null}; }""", S)
                check(f"{which}: leaf and one line", bool(v) and v["svg"] and v["hid"] == "true" and 0 < len(v["line"]) <= 40 and v["lines"] == 1 and v["vis"], v)
                a = pg.evaluate("s => { const l = document.querySelector(s + ' svg.leaf'), c = getComputedStyle(l); return [c.animationName, l.getAnimations().length]; }", S)
                check(f"{which}: still under reduced motion", a[0] == "none" and a[1] == 0, a)
                for th in ("light", "dark"):
                    pg.evaluate(f"document.documentElement.setAttribute('data-theme', '{th}')"); frames(pg, 2)
                    c = pg.evaluate("s => { const f = getComputedStyle(document.querySelector(s + ' .lf')); return [f.fill, +f.fillOpacity]; }", S)
                    check(f"{which}: tinted in {th}", c[0] != "none" and 0 < c[1] <= .15, c)
                pg.evaluate("document.documentElement.removeAttribute('data-theme')")
                ax = axe_path()
                if ax:
                    pg.add_script_tag(path=ax)
                    n = pg.evaluate("s => axe.run(document.querySelector(s)).then(r => r.violations.map(v => v.id))", S)
                    check(f"{which}: axe", not n, n)
                pg.click('#stbar [data-cmd="%s"]' % ("lists" if which == "lists" else "addloss")); frames(pg, 6)
            # motion allowed: the leaf falls and nothing around it moves (transform and opacity only)
            ctx = pg.context.browser.new_context(viewport={"width": 1100, "height": 800}, reduced_motion="no-preference")
            pm = ctx.new_page()
            pm.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');}catch(_){}")
            pm.goto(f"http://127.0.0.1:{r.port}/?telefono=0"); attendi(pm, "window.qqReady && document.querySelector('#fl-hint')"); frames(pm, 5)
            pm.evaluate("document.getElementById('fl-hint').hidden = false; document.getElementById('dfiles').style.width = '280px'")      # the narrowest side panel (below 900 px the panel is not shown at all)
            samples = []
            for _ in range(6):
                samples.append(pm.evaluate("() => { const h = document.getElementById('fl-hint'), l = h.querySelector('svg.leaf'), r = h.getBoundingClientRect(), b = l.getBoundingClientRect(), p = document.getElementById('dfiles').getBoundingClientRect(); return [r.height, r.top, getComputedStyle(l).animationName, b.top, b.left, b.left >= p.left && b.right <= p.right]; }"))
                frames(pm, 40)      # about 0.7 s of animation frames
            check("motion: the box never moves", len({(s[0], s[1]) for s in samples}) == 1, samples)
            check("motion: the leaf falls (animation runs, position changes)", samples[0][2] == "leaf-fall" and len({(s[3], s[4]) for s in samples}) > 1, samples)
            check("narrow panel (280 px): the swing stays inside", all(s[5] for s in samples), samples)
            ctx.close()
            check("page errors", not [e for e in r.errs if e[0] == "pageerror"], r.errs[:3])
    finally:
        r.close()
    sys.exit(1 if bad else 0)
