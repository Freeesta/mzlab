"""Dynamic tooltip guard (docs/inventari/TOOLTIP.md, rule 20): on the built site, with synthetic files of every kind loaded (Full Scan, MS2, MRM, mixed IDA,
high-resolution DDA, .wiff Q1 / PI / EPI and SIM), every element that carries a tooltip (title, data-tip, data-tiph: the three things the program's own tooltip reads) is
collected in Italian and in English, and the real hover is measured for the group headers, every HTML tooltip and the longest texts. Fails on a tooltip longer than 60
characters, with more than one sentence, wider than 28ch or taller than 3 lines when it is drawn. The static twin over the catalogs is tests/test_tooltip_corti.py.
Needs the site (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_tooltip_dinamici.py"""
import json, os, re, subprocess, sys, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

MAX_CHARS, MAX_CH, MAX_LINES, HOVER_LONGEST = 60, 28.5, 3, 25
# every tooltip element of the page, whatever carries the text (the program moves `title` to data-tip on the first hover)
COLLECT = """() => [...document.querySelectorAll('[title],[data-tip],[data-tiph]')].map((e, i) => {
  const r = e.getBoundingClientRect(), hid = !(r.width || r.height) || getComputedStyle(e).visibility === 'hidden' || !!e.closest('[hidden]');
  const html = e.dataset.tiph || '', txt = html ? new DOMParser().parseFromString(html, 'text/html').body.textContent : (e.getAttribute('title') || e.dataset.tip || '');
  const k = e.id ? '#' + e.id : e.tagName.toLowerCase() + (e.className && typeof e.className === 'string' ? '.' + e.className.trim().split(/\\s+/)[0] : '');
  return { k, txt, html: !!html, hid, i, where: (e.closest('[id]') || {}).id || '' };
}).filter(x => x.txt)"""
# a hovered tooltip as drawn: content width in ch (the width of «0» in its own font) and lines
RENDER = """() => { const q = document.getElementById('qtip'); if (q.hidden) return null; const cs = getComputedStyle(q), pr = document.createElement('span');
  pr.style.cssText = 'position:absolute;visibility:hidden;width:1ch;height:1px;font:' + cs.font; q.appendChild(pr); const ch = pr.getBoundingClientRect().width; pr.remove();
  const w = q.getBoundingClientRect().width - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight), h = q.getBoundingClientRect().height - parseFloat(cs.paddingTop) - parseFloat(cs.paddingBottom);
  return { ch: w / ch, lines: Math.round(h / parseFloat(cs.lineHeight)), txt: q.textContent }; }"""
SENT = re.compile(r"[.!?]\s+[A-ZÀ-ÖØ-Þ0-9«\"“]")

def sentences(t): return 1 + len(SENT.findall(re.sub(r"\b(ca|es|vs|cfr|Fig|n|nr)\.", r"\1", t)))
def problems(txt): 
    out = []
    if len(txt) > MAX_CHARS: out.append(f"{len(txt)} chars")
    if sentences(txt) > 1: out.append("more than one sentence")
    return out

def files(D):
    sys.path.insert(0, str(ROOT / "tools"))
    import dati_sintetici as DS
    DS.make(D / "mz"); subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_modi.py"), str(D)], check=True, capture_output=True)
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
    pick = [D / "mz" / n for n in ("B_FullMass-t0.mzML", "B_MS2-t15.mzML", "B_MRM-t0.mzML", "C_IDA-t0.mzML", DS.HR_NAMES["exploris"])]
    for s in ("Q1", "PI", "EPI"): pick += [D / "modi" / f"{s}.wiff", D / "modi" / f"{s}.wiff.scan"]
    return pick + [D / "sim.wiff", D / "sim.wiff.scan"]

bad, seen, steps = [], {}, []
D = Path(tempfile.mkdtemp(prefix="tip_dyn_")); FILES = files(D)
for lang in ("it", "en"):
    r = Run(port=8890, wd="/tmp/wd_tip_dyn")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.goto(f"http://127.0.0.1:{r.port}/?lang={lang}"); pg.wait_for_selector("#drop")
            stage(pg, FILES); pg.click("text=Carica dati" if lang == "it" else "text=Load data"); ready(pg); attendi(pg, "E.files.length >= 8", timeout=180000)
            items = {}
            def scan():
                for x in pg.evaluate(COLLECT):
                    if not x["hid"]: items[(x["k"], x["txt"])] = x
            scan()
            for kind in pg.evaluate("[...new Set(E.files.map(f => f.kind))]"):       # every experiment type has its own block of panels
                pg.evaluate(f"setTab('{kind}')"); ready(pg); scan()
            # hover: the group headers and every HTML tooltip first, then the longest texts
            heads = pg.evaluate("[...document.querySelectorAll('#flst .fgh')].length")
            todo = [("#flst .fgh", n) for n in range(heads)]
            by_len = sorted(items.values(), key=lambda x: -len(x["txt"]))
            def hover(sel, n):
                el = pg.locator(sel).nth(n)
                el.scroll_into_view_if_needed(); el.hover(position={"x": 30, "y": 5}) if sel.endswith(".fgh") else el.hover()
                try: attendi(pg, "!document.getElementById('qtip').hidden", 3000)
                except Exception: pg.mouse.move(2, 2); return None
                v = pg.evaluate(RENDER); pg.mouse.move(2, 2); return v
            for sel, n in todo:
                v = hover(sel, n)
                if v: seen[(lang, v["txt"])] = v
            for x in [x for x in by_len if x["html"]] + by_len[:HOVER_LONGEST]:
                sel = f'{x["k"]}' if x["k"].startswith("#") else None
                if not sel: continue
                try: v = hover(sel, 0)
                except Exception: v = None
                if v: seen[(lang, v["txt"])] = v
            for x in items.values():
                for why in problems(x["txt"]): bad.append((lang, x["k"], x["where"], why, x["txt"][:90]))
            steps.append((f"[{lang}] {len(items)} tooltip elements, {sum(1 for k in seen if k[0] == lang)} hovered", "ok"))
            if not any(k[0] == lang for k in seen): bad.append((lang, "hover", "", "no tooltip could be hovered", ""))
        r.close()
    except Exception as e:
        steps.append((f"[{lang}] run", "FAIL " + str(e)[:300]))
        try: r.close()
        except Exception: pass
for (lang, txt), v in seen.items():
    if v["ch"] > MAX_CH: bad.append((lang, "drawn", "", f"{v['ch']:.1f}ch wide", txt[:90]))
    if v["lines"] > MAX_LINES: bad.append((lang, "drawn", "", f"{v['lines']} lines", txt[:90]))
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
print("OFFENDERS", len(bad))
for b in bad: print("  ", b)
sys.exit(1 if bad or any(s.startswith("FAIL") for _, s in steps) else 0)
