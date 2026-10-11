"""Measurement (not a test; run from the repository root): how many draw() calls a burst of 30 window resizes costs.
    python3 tools/misura_redraw.py"""
import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tests_e2e"))
from lib import *
r = Run(port=8893, wd="/tmp/wd_redraw")
F = FILES + ["B_FullMass-t10"]
with sync_playwright() as p:
    pg = r.page(p)
    pg.set_input_files("#pick", [mz(f) for f in F]); pg.wait_for_timeout(2500)
    pg.click("text=Carica dati"); ready(pg)
    pg.evaluate("window.__n = 0; const d = window.draw; window.draw = function () { window.__n++; return d.apply(this, arguments); }; 0")
    print(pg.evaluate("(() => { try { for (let i = 0; i < 30; i++) dispatchEvent(new Event('resize')); return 'ok'; } catch (e) { return e.stack.split('\\n').slice(0, 3).join(' / '); } })()"))
    pg.wait_for_timeout(1500)
    print("draw calls for 30 resize events:", pg.evaluate("window.__n"), "panels:", pg.evaluate("tabPanels().length"))
