# e2e_tour_ancoraggio: in every step of the guided tour (both chapters) the arrow of the bubble points at the target (centre of the arrow inside the highlighted rectangle), at 1280, 1440 and 1024 px
import os, sys, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
TMP = "/tmp/Esempio_FullScan_anc.mzML"
shutil.copy(str(ROOT / "mzlab" / "web" / "esempi" / "FullScan_t15.mzML"), TMP)
GEO = """(() => {
  const d = document.getElementById('tour-dialog'), s = document.getElementById('tour-spotlight'); if (!d || !s) return null;
  const a = d.querySelector('.tour-arrow'), ar = a.getBoundingClientRect(), sr = s.getBoundingClientRect(), side = d.className.replace('tour-side-', '');
  return {side, hd: d.querySelector('.tour-hd').textContent, ax: ar.left + ar.width / 2, ay: ar.top + ar.height / 2, hidden: getComputedStyle(a).display === 'none',
    l: sr.left, r: sr.right, t: sr.top, b: sr.bottom};
})()"""
def run(port, W, H):
    r = Run(port=port, wd="/tmp/wd_tour_anc%d" % W)
    try:
        with sync_playwright() as p:
            pg = r.page(p, tour=True)
            try:
                pg.set_input_files("#pick", [TMP]); pg.wait_for_timeout(800); pg.click("text=Carica dati"); ready(pg)
                pg.set_viewport_size({"width": W, "height": H}); pg.wait_for_timeout(500)
                pg.evaluate("TOUR.inizia(1, 0)"); pg.wait_for_timeout(500)
                bad, seen = [], 0
                for _ in range(30):
                    if not pg.evaluate("!!TOUR.active"): break
                    g = pg.evaluate(GEO)
                    if g and not g["hidden"]:
                        seen += 1
                        horiz = g["side"] in ("bottom", "top"); c, lo, hi = (g["ax"], g["l"], g["r"]) if horiz else (g["ay"], g["t"], g["b"])
                        if not (lo - 3 <= c <= hi + 3): bad.append((g["hd"], g["side"], round(c), round(lo), round(hi)))
                    pg.evaluate("TOUR.avanti()"); pg.wait_for_timeout(350)
                assert seen >= 10 and not bad, (W, seen, bad)
                steps.append((f"{W}x{H}: arrow on the target in {seen} steps", "ok"))
            except Exception as e:
                steps.append((f"{W}x{H}", "FAIL " + str(e).split("\n")[0][:400]))
    finally:
        r.close(); r.report()
for port, (W, H) in zip((8840, 8841, 8842), ((1280, 800), (1440, 900), (1024, 768))): run(port, W, H)
print("\nSTEPS"); [print(" ", s) for s in steps]
