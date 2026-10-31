# e2e_toolbar_wrap: the second toolbar of Data (#tools) is ONE row at every width (900-3000 px, side panel open or closed, upload view and loaded file, long file names, UI zoom 1 and 1.5),
# its height is reserved (the work area below never moves, not after first paint, not when the panel toggles), the groups keep the order file | add | undo-redo-close ... Method, and no divider sits at a row edge
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
WIDTHS = (900, 1200, 1600, 2200, 3000)
LONG = "20240912_Lab_inquinanti_FullMass_positivo_flufenacet_campione_degradato_TiO2_replica_"
# geometry of the toolbar: vertical centres of every visible control, the groups left to right, and the bottom edge of the row (= top of what follows)
PROBE = """(() => {
  const vis = e => e && !e.hidden && e.offsetParent !== null && e.getBoundingClientRect().width > 0;
  const t = document.querySelector('#tools'), tb = t.getBoundingClientRect();
  const ctl = [...t.querySelectorAll('button,select')].filter(vis).map(e => { const r = e.getBoundingClientRect(); return {id: e.id, mid: (r.top + r.bottom) / 2, left: r.left, right: r.right}; });
  const grp = ['g-file', 'g-add', 'g-tools', 'np-method'].map(id => document.getElementById(id)).filter(vis).map(e => { const r = e.getBoundingClientRect(); return {id: e.id, left: r.left, right: r.right, div: getComputedStyle(e).borderLeftWidth !== '0px' && e.id !== 'np-method'}; });
  return {top: tb.top + scrollY, bottom: tb.bottom + scrollY, h: tb.height, left: tb.left, right: tb.right, scrollW: t.scrollWidth, clientW: t.clientWidth, ctl, grp};
})()"""
def geo(pg): return pg.evaluate(PROBE)
def check_row(g, tag):
    mids = [c["mid"] for c in g["ctl"]]
    if mids: assert max(mids) - min(mids) <= 2, (tag, "controls on more than one row", [(c["id"], round(c["mid"])) for c in g["ctl"]])
    assert g["scrollW"] <= g["clientW"] + 1, (tag, "toolbar overflows its box", g["scrollW"], g["clientW"])
    ids = [x["id"] for x in g["grp"]]
    assert ids == [i for i in ("g-file", "g-add", "g-tools", "np-method") if i in ids], (tag, "group order", ids)
    for a, b in zip(g["grp"], g["grp"][1:]): assert a["right"] <= b["left"] + 1, (tag, "groups overlap", a["id"], b["id"])
    if g["grp"]:
        assert g["grp"][-1]["id"] == "np-method" and g["right"] - g["grp"][-1]["right"] < 24, (tag, "Method is not pinned right", g["grp"][-1])
        assert not g["grp"][0]["div"], (tag, "divider at the start of the row", g["grp"][0]["id"])       # a divider belongs to the group that follows another one
        for x in g["grp"][1:]: assert x["left"] > g["left"] + 40, (tag, "a divider group sits at the row start", x["id"])
r = Run(port=8854, wd="/tmp/wd_toolbarwrap")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1400, "height": 900}); pg.wait_for_selector("#tools", state="attached")
        base = {}
        def upload_view():
            for w in WIDTHS:
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 3)
                g = geo(pg); check_row(g, ("upload", w)); base[w] = g["h"]
                assert g["h"] >= 31, (w, "row height not reserved", g["h"])
        step("upload view: one reserved row at 5 widths", upload_view)
        pg.set_input_files("#pick", [{"name": LONG + "%d.mzML" % i, "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t%d" % t), "rb").read()} for i, t in enumerate((0, 10, 15))])
        frames(pg, 3); pg.click("text=Carica dati"); ready(pg)
        def loaded():
            for z in (1, 1.5):
                pg.evaluate("z => document.documentElement.style.setProperty('--z', z)", z)
                for w in WIDTHS:
                    pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 4)
                    g = geo(pg); check_row(g, ("loaded", z, w))
                    assert g["grp"] and len(g["grp"]) == 4, (w, "groups", [x["id"] for x in g["grp"]])
                    assert abs(g["h"] - 32 * z) < 2, (z, w, "row height", g["h"])
            pg.evaluate("document.documentElement.style.removeProperty('--z')")
        step("loaded file, long names, zoom 1 and 1.5: one row, order, dividers, Method right", loaded)
        def stable():
            for w in WIDTHS:
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 2)
                a = geo(pg); pg.wait_for_timeout(500); b = geo(pg)
                assert (a["top"], a["bottom"]) == (b["top"], b["bottom"]), (w, "the row moved after first paint", a["bottom"], b["bottom"])
                pg.click("#sb-toggle"); frames(pg, 3); pg.wait_for_timeout(450)
                c = geo(pg); check_row(c, ("panel toggled", w))
                assert (c["top"], c["bottom"]) == (a["top"], a["bottom"]), (w, "the row moved when the side panel toggled", a["bottom"], c["bottom"])
                pg.click("#sb-toggle"); frames(pg, 3); pg.wait_for_timeout(450)
                d = geo(pg); assert d["bottom"] == a["bottom"], (w, "the row moved when the side panel toggled back")
        step("work area does not move after first paint nor when the side panel toggles", stable)
        def resize_sweep():
            # continuous resize: a row never doubles at any intermediate width
            for w in range(3000, 780, -111):
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 2)
                g = geo(pg); check_row(g, ("sweep", w)); assert g["h"] < 40, (w, "row height", g["h"])
        step("continuous resize 3000 to 800 px keeps one row", resize_sweep)
        def keyboard():
            pg.set_viewport_size({"width": 900, "height": 900}); frames(pg, 3)
            pg.focus("#np-xic"); frames(pg, 2)
            inview = pg.evaluate("(() => { const b = document.getElementById('np-xic').getBoundingClientRect(), t = document.getElementById('tools').getBoundingClientRect(); return b.left >= t.left - 1 && b.right <= t.right + 1; })()")
            assert inview, "a focused button of the scrolling group is not brought into view"
        step("keyboard focus in the narrow row stays inside the toolbar", keyboard)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
