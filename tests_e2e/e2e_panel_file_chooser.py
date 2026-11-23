"""File chooser of an XIC panel header (the «n/tot ▾» chip), with 8 synthetic files, at iPad widths (810×1080 and 1024×1366, touch) and on a 1280 desktop:
the header stays on ONE row (the caret never wraps under the chip), the chip has a 44×44 px hit area on touch, the list opens above every panel (top layer,
not clipped), fully inside the viewport, scrolls inside when it is long, flips above the chip when there is no room below, selecting a name switches the
panel's file, Esc and a tap outside close it."""
# covers: mzlab/web/lista-file-menu.js, mzlab/web/app.css, mzlab/web/pannelli.js
import sys, os, shutil
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

EX = ROOT / "mzlab" / "web" / "esempi"
SRC = ["FullScan_t0", "FullScan_t5", "FullScan_t10", "FullScan_t15", "FullScan_t30"]
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_panel_file_chooser.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))

# 8 distinct names from the 5 anonymous example files (the file names carry the time, so each copy gets its own)
TMP = Path(tempfile.mkdtemp(prefix="fchooser_"))
FILES = []
for i in range(8):
    dst = TMP / f"Sample_{i + 1}_t{i * 3}.mzML"; shutil.copy(EX / (SRC[i % 5] + ".mzML"), dst); FILES.append(str(dst))

XIC = "E.panels.find(p => p.type === 'xic')"
CHIP = f"{XIC}.el.querySelector('[data-o=fpop]')"
ROW = f"""(() => {{ const p = {XIC}, hd = p.el.querySelector('.hd'), hr = hd.getBoundingClientRect(), c = p.el.querySelector('[data-o=fpop]');
  const mids = [...hd.querySelectorAll(':scope > *, .ctl > *')].filter(e => e.offsetParent && !e.closest('.hmore') && e.getBoundingClientRect().width).map(e => {{ const q = e.getBoundingClientRect(); return q.top + q.height / 2; }});
  const cr = c.getBoundingClientRect();
  return {{ hh: hr.height, spread: Math.max(...mids) - Math.min(...mids), chipInside: cr.left >= hr.left - 1 && cr.right <= hr.right + 1 && cr.top >= hr.top - 1 && cr.bottom <= hr.bottom + 1,
    chipWrapped: (() => {{ const g = document.createRange(); g.selectNodeContents(c); return new Set([...g.getClientRects()].map(q => Math.round(q.top))).size > 1; }})(), text: c.textContent.trim() }}; }})()"""
# the list: its box inside the viewport and, at its four corners (a little inside the rounded ones) and at the centre of each name, the element on top belongs to the list
LIST = """(() => { const d = document.querySelector('#fpop'); if (!d) return null; const r = d.getBoundingClientRect(), vw = document.documentElement.clientWidth, vh = innerHeight;
  const on = (x, y) => { const e = document.elementFromPoint(x, y); return !!e && d.contains(e); }, k = 6;
  const items = [...d.querySelectorAll('label')].map(l => { const q = l.querySelector('.fn').getBoundingClientRect(); return { top: q.top, bottom: q.bottom, h: l.getBoundingClientRect().height,
    seen: q.top >= r.top - 1 && q.bottom <= r.bottom + 1, ok: on(q.left + q.width / 2, q.top + q.height / 2) && document.elementFromPoint(q.left + q.width / 2, q.top + q.height / 2).closest('label') === l }; });
  return { r: [r.left, r.top, r.right, r.bottom], vw, vh, inside: r.left >= 0 && r.top >= 0 && r.right <= vw && r.bottom <= vh,
    corners: [on(r.left + k, r.top + k), on(r.right - k, r.top + k), on(r.left + k, r.bottom - k), on(r.right - k, r.bottom - k)], items, scroll: d.scrollHeight > d.clientHeight + 1, z: +getComputedStyle(d).zIndex,
    chip: (() => { const c = """ + CHIP + """.getBoundingClientRect(); return [c.left, c.top, c.right, c.bottom]; })() }; })()"""

def open_chooser(q):
    q.evaluate(f"{CHIP}.scrollIntoView({{block: 'center'}})"); frames(q, 2)
    q.evaluate(f"{CHIP}.click()"); wait_for_js(q, "!!document.querySelector('#fpop')"); frames(q, 2)
    return q.evaluate(LIST)

def prepare(q):
    wait_for_js(q, "window.qqReady === true", 60000)
    q.set_input_files("#pick", FILES)
    wait_for_js(q, "UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.filter(f => !f.gone).length === 8", 120000)
    q.evaluate("document.querySelector('#flst .fl .nm').click()"); q.wait_for_selector(".pnl.chrom canvas", timeout=60000); ready(q)
    if q.is_visible("#sb-x") or q.is_visible("#dfiles"): close_side(q); ready(q)
    q.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})")
    wait_for_js(q, f"(() => {{ const p = {XIC}; return !!(p && p._a && p._a.sr && p._a.sr.length); }})()"); frames(q, 4)

r = Run(port=8994, wd="/tmp/wd_panel_file_chooser")
try:
    with sync_playwright() as p:
        for w, h, touch in ((810, 1080, True), (1024, 1366, True), (1280, 900, False)):
            tag = f"{w}×{h}{' touch' if touch else ''}"
            q = r.page(p, viewport={"width": w, "height": h}, has_touch=touch, is_mobile=touch); prepare(q)

            def one_row():
                x = q.evaluate(ROW)
                assert x["text"] == "1/8 ▾", x
                assert x["spread"] <= x["hh"] * 0.5 and not x["chipWrapped"] and x["chipInside"], x      # one row, the caret inside the chip
                assert x["hh"] <= (48 if touch else 28), x                                             # 44 px controls on touch; the wrapped header was 50 px high on a 810 px iPad plus a second row
            step(f"{tag}: the header is one row, the caret stays inside the chip", one_row)

            if touch:
                def hit_area():
                    # the 44 px floor of the panel header controls (app.css, coarse pointer) makes the chip itself 44×44
                    r_ = q.evaluate(f"(() => {{ const c = {CHIP}; c.scrollIntoView({{block: 'center'}}); const r = c.getBoundingClientRect(); return [r.left, r.top, r.right, r.bottom]; }})()")
                    assert r_[2] - r_[0] >= 43.5 and r_[3] - r_[1] >= 43.5, ("the chip is too small", r_)
                    cx, cy = (r_[0] + r_[2]) / 2, (r_[1] + r_[3]) / 2
                    for x, y in ((cx, r_[1] + 1), (cx, r_[3] - 1), (r_[0] + 1, cy), (r_[2] - 1, cy), (r_[0] + 1, r_[3] - 1), (r_[2] - 1, r_[1] + 1)):
                        assert q.evaluate(f"(() => {{ const e = document.elementFromPoint({x}, {y}); return !!e && {CHIP}.contains(e); }})()"), ("the 44×44 hit area of the chip does not reach", x, y, r_)
                step(f"{tag}: the chip has a 44×44 px hit area", hit_area)

            def lists():
                L = open_chooser(q)
                assert L and L["inside"], L
                assert L["z"] > 1000, L                                                              # above the panels, whose z-index only grows
                assert all(L["corners"]), ("a corner of the list is covered", L)
                assert len(L["items"]) == 8, L
                for it in L["items"]:
                    assert it["seen"] and it["ok"], ("a name is hidden or covered", it, L["r"])
                    if touch: assert it["h"] >= 43.5, ("a touch row under 44 px", it)
                # the panel being raised above the list's old level does not bury it
                q.evaluate("E.z += 9000; E.panels.forEach(p => { if (p.el) p.el.style.zIndex = String(++E.z); })")
                assert all(q.evaluate(LIST)["corners"]), "a panel raised after opening covers the list"
            step(f"{tag}: the list opens above every panel, inside the viewport, no name covered", lists)

            def select():
                k = 4
                q.evaluate(f"document.querySelector('#fpop .fn[data-k=\"{k}\"]').click()"); frames(q, 3)
                st = q.evaluate("tabFiles().filter(f => f.vis).map(f => f.k)")
                assert st == [k], st                                                                   # the panel now shows only that file
                wait_for_js(q, f"{CHIP}.textContent.trim() === '1/8 ▾'")
                q.keyboard.press("Escape"); wait_for_js(q, "!document.querySelector('#fpop')")
                assert q.evaluate(f"document.activeElement === {CHIP}"), "the focus goes back to the chip after Esc"
            step(f"{tag}: choosing a name switches the panel's file; Esc closes and returns the focus", select)

            def outside():
                open_chooser(q)
                q.mouse.click(5, h - 5) if not touch else q.touchscreen.tap(5, h - 5)
                wait_for_js(q, "!document.querySelector('#fpop')")
                open_chooser(q); q.evaluate(f"{CHIP}.click()"); wait_for_js(q, "!document.querySelector('#fpop')")      # the chip again closes its own list
            step(f"{tag}: a tap outside, or on the chip again, closes the list", outside)

            def flip_and_scroll():
                q.set_viewport_size({"width": w, "height": 260}); frames(q, 3)
                # the chip at the bottom of the window: the list opens above it, shorter than its content, and scrolls inside
                q.evaluate(f"(() => {{ const c = {CHIP}; const r = c.getBoundingClientRect(); window.scrollBy(0, r.top - (innerHeight - 40)); const m = document.querySelector('#v-data') || document.scrollingElement; }})()")
                q.evaluate(f"{CHIP}.click()"); wait_for_js(q, "!!document.querySelector('#fpop')"); frames(q, 2)
                L = q.evaluate(LIST)
                assert L["inside"] and all(L["corners"]), L
                assert L["r"][3] <= L["chip"][1] + 1 or L["r"][1] >= L["chip"][3] - 1, ("the list overlaps its own chip", L)
                assert L["scroll"], ("a long list scrolls inside", L)
                q.keyboard.press("Escape"); wait_for_js(q, "!document.querySelector('#fpop')")
                q.set_viewport_size({"width": w, "height": h}); frames(q, 3)
            step(f"{tag}: in a short window the list stays in the viewport, beside its chip, and scrolls inside", flip_and_scroll)
            q.context.close()

    print("\n".join(f"{n}: {s}" for n, s in steps))
    bad = [s for s in steps if s[1] != "ok"]
    for n, s in steps: print(("OK " if s == "ok" else "FAIL ") + n + (" " + s if s != "ok" else ""))
    assert not bad, bad
finally:
    r.close(); shutil.rmtree(TMP, ignore_errors=True)
