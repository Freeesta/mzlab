"""Acceptance checks and smoothness measures of the Flow prototype (docs/studi/flow_grafica/prototipo.html), for docs/studi/FLOW_GRAFICA.md §9.

Run: python3 docs/studi/flow_grafica/misura.py [--cpu 4]   (Chromium from Playwright; --cpu N = CPU throttling, like a slow student laptop)
Prints one JSON line per check; exit 1 if an acceptance criterion fails. Study probe: not collected by pytest, not part of the site."""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

HERE = Path(__file__).resolve().parent
URL = (HERE / "prototipo.html").as_uri()
FAILS: list[str] = []


def out(name: str, ok: bool, **kw) -> None:
    print(json.dumps({"check": name, "ok": ok, **kw}))
    if not ok:
        FAILS.append(name)


def centre(page, sel: str) -> tuple[float, float]:
    b = page.locator(sel).first.bounding_box()
    return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2


def functional(page) -> None:
    page.goto(URL)
    page.wait_for_function("window.FLOWPROTO")
    c0 = page.evaluate("FLOWPROTO.count()")
    # Drag raw -> index: a proposal (Build index) must appear, not a silent insertion.
    x0, y0 = centre(page, '.port.out[data-port="raw"]')
    x1, y1 = centre(page, '.port.in[data-port="index"]')
    page.mouse.move(x0, y0); page.mouse.down(); page.mouse.move((x0 + x1) / 2, y0, steps=5)
    hint = page.locator('.port.in[data-port="index"]').get_attribute("class")
    page.mouse.move(x1, y1, steps=5); page.mouse.up()
    out("adapter shown while dragging", "adapt" in (hint or ""), cls=hint)
    out("adapter proposed, not inserted", page.locator("#pop").is_visible() and page.evaluate("FLOWPROTO.count()") == c0)
    page.click('#pop [data-a="ins"]')
    c1 = page.evaluate("FLOWPROTO.count()")
    out("adapter inserted with one click", c1["nodes"] == c0["nodes"] + 1 and c1["edges"] == c0["edges"] + 2, before=c0, after=c1)
    # Run twice: second run fully from cache.
    page.click("#b-run")
    page.wait_for_function("document.querySelector('#msg').textContent.startsWith('Done')", timeout=20000)
    page.click("#b-run")
    page.wait_for_function("document.querySelector('#msg').textContent.includes(' 0 step(s) run')", timeout=5000)
    out("second run from cache", True, msg=page.inner_text("#msg"))
    # Keyboard-only link: remove the table->export link, then relink with Tab/Enter only.
    keyboard(page)
    # Save -> open round trip keeps the plan.
    doc = page.evaluate("FLOWPROTO.save()")
    page.evaluate("d => FLOWPROTO.openDoc(d)", doc)
    out("save/open round trip", page.evaluate("FLOWPROTO.save()")["passi"] == doc["passi"])


def keyboard(page) -> None:
    page.evaluate("""() => { const d = FLOWPROTO.save(); d.passi.forEach(p => { if (p.step.startsWith('export.csv')) p.in = {}; }); FLOWPROTO.openDoc(d); }""")
    before = page.evaluate("FLOWPROTO.count()")["edges"]
    page.locator('.node:has(.tt:text("Align samples")) .port.out').focus()
    page.keyboard.press("Enter")
    page.locator('.node:has(.tt:text("Export CSV")) .port.in').focus()
    page.keyboard.press("Enter")
    out("link made with the keyboard only", page.evaluate("FLOWPROTO.count()")["edges"] == before + 1)
    page.locator('.node:has(.tt:text("Export CSV"))').focus()
    x = page.evaluate("FLOWPROTO.save().ui.nodes")
    page.keyboard.press("ArrowRight"); page.wait_for_timeout(50)
    y = page.evaluate("FLOWPROTO.save().ui.nodes")
    moved = [k for k in x if x[k] != y[k]]
    out("node moved with arrow keys", len(moved) == 1, moved=moved)


def smooth(page, n: int) -> dict:
    page.evaluate("n => FLOWPROTO.stress(n)", n)
    page.wait_for_timeout(200)
    t = page.evaluate("n => { const t0 = performance.now(); FLOWPROTO.stress(n); return performance.now() - t0; }", n)
    x, y = centre(page, ".node .hd")
    page.evaluate("FLOWPROTO.recStart()")
    page.mouse.move(x, y); page.mouse.down()
    for i in range(90):
        page.mouse.move(x + 3 * i, y + (i % 20), steps=1)
    page.mouse.up()
    drag = page.evaluate("FLOWPROTO.recStop()")
    vb = page.locator("#view").bounding_box()
    px, py = vb["x"] + vb["width"] - 30, vb["y"] + vb["height"] - 30
    page.evaluate("FLOWPROTO.recStart()")
    page.mouse.move(px, py); page.mouse.down()
    for i in range(90):
        page.mouse.move(px - 4 * i, py - 2 * i, steps=1)
    page.mouse.up()
    pan = page.evaluate("FLOWPROTO.recStop()")
    return {"nodes": page.evaluate("FLOWPROTO.count()")["nodes"], "render_ms": round(t, 1), "drag": drag, "pan": pan}


def axe(page) -> None:
    js = os.environ.get("AXE_JS")
    if not js:
        try:
            import axe_playwright_python
            js = os.path.join(os.path.dirname(axe_playwright_python.__file__), "axe.min.js")
        except ImportError:
            return out("axe", True, skipped="axe-core not installed")
    page.goto(URL); page.wait_for_function("window.FLOWPROTO")
    page.add_script_tag(path=js)
    v = page.evaluate("async () => (await axe.run(document, {runOnly: ['wcag2a','wcag2aa','wcag21aa','wcag22aa']})).violations.map(x => [x.id, x.nodes.length])")
    out("axe WCAG 2.2 AA", not v, violations=v)


def main() -> int:
    cpu = int(sys.argv[sys.argv.index("--cpu") + 1]) if "--cpu" in sys.argv else 1
    with sync_playwright() as p:
        exe = os.environ.get("CHROMIUM") or ("/opt/pw-browsers/chromium" if Path("/opt/pw-browsers/chromium").exists() else None)
        b = p.chromium.launch(executable_path=exe)
        for scheme in ("light", "dark"):
            page = b.new_page(viewport={"width": 1366, "height": 768}, color_scheme=scheme)
            if scheme == "light":
                functional(page)
            axe(page)
            page.close()
        page = b.new_page(viewport={"width": 1366, "height": 768})
        page.goto(URL); page.wait_for_function("window.FLOWPROTO")
        if cpu > 1:
            page.context.new_cdp_session(page).send("Emulation.setCPUThrottlingRate", {"rate": cpu})
        for n in (0, 50, 200, 500):
            m = smooth(page, n)
            # Acceptance (FLOW_GRAFICA.md §9): with 200 extra nodes the median frame stays under 20 ms and no gesture frame passes 50 ms more than twice.
            ok = n > 200 or (m["drag"]["p50"] <= 20 and m["pan"]["p50"] <= 20 and m["drag"]["over50"] <= 2 and m["pan"]["over50"] <= 2)
            out(f"smooth {n}", ok, cpu=cpu, **m)
        b.close()
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main())
