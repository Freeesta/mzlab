"""Periodic table cells (dialog #refdlg), light and dark, at the iPad width (1024) and on a desktop (1440):
- a cell shows only the atomic number, the symbol and the mass; the class code is gone, the class stays in the legend (swatch + full name),
  the tooltip, the accessible name and the card; the isotope marker is CSS-drawn and shows on hover or focus only;
- no child of any cell overlaps another or is clipped (bounding boxes);
- colour is never the only cue (WCAG 1.4.1): the nine class patterns differ from each other (spectrum of the rendered pattern), and every pair of
  classes is told apart by hue under simulated deuteranopia and protanopia or by its pattern;
- text keeps 4.5:1 over the darkest/lightest pixel of every pattern (not only over the flat colour)."""
# covers: mzlab/web/tavola-periodica.js, mzlab/web/tables.js
import sys, os, io, itertools; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
from PIL import Image
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tavola_celle.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).replace("\n", " ")[:700]))
# Boxes of every child of every cell, in CSS px: z (atomic number), marker, symbol, mass.
BOXES = """()=>{const out=[], R=e=>{const r=e.getBoundingClientRect();return {x:r.left,y:r.top,r:r.right,b:r.bottom}};
 for(const c of document.querySelectorAll('#refbody .pt-c')){
   const kids=[['z','.pt-t > span:first-child'],['marker','.pt-m'],['symbol','b'],['mass','small']].map(([n,s])=>[n,c.querySelector(s)]).filter(([n,e])=>e&&(n==='marker'||e.textContent.trim()));
   out.push({s:c.dataset.s,cell:R(c),text:c.innerText.replace(/\\s+/g,' ').trim(),kids:kids.map(([n,e])=>({n,...R(e),sw:e.scrollWidth,cw:e.clientWidth}))})}
 return out}"""
def overlaps(a, b): return min(a["r"], b["r"]) - max(a["x"], b["x"]) > 0.5 and min(a["b"], b["b"]) - max(a["y"], b["y"]) > 0.5
def open_dialog(pg, w, h, th):
    pg.set_viewport_size({"width": w, "height": h})
    pg.evaluate("document.querySelector('#refdlg') && document.querySelector('#refdlg').close()")
    pg.evaluate(f"document.documentElement.setAttribute('data-theme','{th}')")
    pg.evaluate("document.querySelector('#np-pt').click()"); wait_for_js(pg, "document.querySelectorAll('#refbody .pt-c').length >= 118")
    wait_for_js(pg, "document.querySelector('#refdlg').open && !document.getAnimations().length")
def cells(pg, w, h, th):
    open_dialog(pg, w, h, th)
    boxes = pg.evaluate(BOXES); assert len(boxes) == 118, len(boxes); bad = []
    for c in boxes:
        cell = c["cell"]
        for k in c["kids"]:
            # inside the cell (1 px border) and not cut: its own content fits
            if k["x"] < cell["x"] + 0.5 or k["r"] > cell["r"] - 0.5 or k["y"] < cell["y"] + 0.5 or k["b"] > cell["b"] - 0.5 or k["sw"] > k["cw"] + 1: bad.append((c["s"], "clipped", k["n"]))
            if k["n"] == "marker" and k["r"] - k["x"] < 2: bad.append((c["s"], "marker has no width"))
        for a, b in itertools.combinations(c["kids"], 2):
            if overlaps(a, b): bad.append((c["s"], "overlap", a["n"], b["n"]))
    assert not bad, (w, th, bad[:6], len(bad))
def content(pg):
    """Only the number, the symbol and the mass on a cell; the class is in the name, the tooltip, the legend and the card."""
    open_dialog(pg, 1024, 768, "light")
    r = pg.evaluate("""()=>{const bad=[]; for(const c of document.querySelectorAll('#refbody .pt-c')){
        const z=c.querySelector('.pt-t > span:first-child').textContent, sym=c.querySelector('b').textContent, m=(c.querySelector('small')||{}).textContent||'';
        const hasCode=c.querySelectorAll('.pt-t > span').length!==1, txt=c.innerText.replace(/\\s+/g,' ').trim();
        if(hasCode||txt!==[z,sym,m].filter(Boolean).join(' ')) bad.push([c.dataset.s,txt]);
        const cls=document.querySelector('#refbody .pt-key[data-k="'+c.dataset.k+'"]').innerText.trim();
        if(!c.title.includes(cls)||!c.getAttribute('aria-label').includes(cls)) bad.push([c.dataset.s,'class missing from title/name',cls]);}
      const keys=[...document.querySelectorAll('#refbody .pt-key')].map(k=>[k.innerText.trim(),!!k.querySelector('.pt-sw')]);
      return {bad,keys,leg:document.querySelector('#refbody .pt-leg').innerText}}""")
    assert not r["bad"], r["bad"][:5]
    assert len(r["keys"]) == 9 and all(sw and not (len(t) == 2 and t.isupper()) and not t.split(" ")[0].isupper() for t, sw in r["keys"]), r["keys"]
    assert "Boxed: used in formulas" in r["leg"] and "single isotope" in r["leg"] and "M+2 pattern" in r["leg"], r["leg"]
    # the marker shows on hover and the card names the class and the marker
    pg.mouse.move(0, 0)
    assert pg.evaluate("getComputedStyle(document.querySelector('#refbody .pt-c[data-s=\"Cl\"] .pt-m')).opacity") == "0"
    pg.locator('#refbody .pt-c[data-s="Cl"]').hover()
    wait_for_js(pg, "getComputedStyle(document.querySelector('#refbody .pt-c[data-s=Cl] .pt-m')).opacity === '1'")
    t = pg.inner_text("#pt-info"); assert "Class: Reactive nonmetal" in t and "Notable M+2 isotope peak" in t, t
    pg.locator('#refbody .pt-c[data-s="F"]').hover(); wait_for_js(pg, "document.querySelector('#pt-info').innerText.includes('Only one natural')")
    t = pg.inner_text("#pt-info"); assert "Only one natural isotope" in t, t
# --- colour vision ---------------------------------------------------------------------------------------------------------------------
# Machado, Oliveira & Fernandes (2009), severity 1.0, applied in linear RGB.
CVD = {"deuteranopia": [[.367322, .860646, -.227968], [.280085, .672501, .047413], [-.011820, .042940, .968881]],
       "protanopia": [[.152286, 1.052583, -.204868], [.114503, .786281, .099216], [-.003882, -.048116, 1.051998]]}
def lin(c): c = np.asarray(c, float) / 255; return np.where(c <= .04045, c / 12.92, ((c + .055) / 1.055) ** 2.4)
def delin(c): c = np.clip(c, 0, 1); return np.where(c <= .0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - .055) * 255
def lab(rgb):
    m = np.array([[.4124, .3576, .1805], [.2126, .7152, .0722], [.0193, .1192, .9505]]); xyz = m @ lin(rgb) / np.array([.95047, 1, 1.08883])
    f = np.where(xyz > .008856, np.cbrt(xyz), 7.787 * xyz + 16 / 116); return np.array([116 * f[1] - 16, 500 * (f[0] - f[1]), 200 * (f[1] - f[2])])
def sim(rgb, kind): return delin(np.array(CVD[kind]) @ lin(rgb))
def lum(rgb): l = lin(rgb); return .2126 * l[0] + .7152 * l[1] + .0722 * l[2]
def contrast(a, b): la, lb = lum(a), lum(b); return (max(la, lb) + .05) / (min(la, lb) + .05)
RENDER = """k=>{const sw=document.querySelector('#refbody .pt-sw[data-k="'+k+'"]'), n=sw.cloneNode(); n.removeAttribute('aria-hidden');
  n.style.cssText='position:fixed;left:8px;top:8px;width:96px;height:96px;z-index:99999;border:0';n.id='pt-render';document.querySelector('#refdlg').appendChild(n);
  const cs=getComputedStyle(sw); return {bar:cs.borderLeftColor,ink:getComputedStyle(document.querySelector('#refbody .pt-c')).color}}"""
def rgb_of(css):
    import re; v = [float(x) for x in re.findall(r"[\d.]+", css)[:3]]; return v
def render(pg, k):
    info = pg.evaluate(RENDER, k)
    png = pg.locator("#pt-render").screenshot(); pg.evaluate("document.querySelector('#pt-render').remove()")
    return np.array(Image.open(io.BytesIO(png)).convert("RGB"), float), info
def spectrum(img):
    g = img.mean(axis=2)[8:-8, 8:-8]
    if g.std() < .5: return np.zeros(g.size)          # a flat fill: no pattern
    g = g - g.mean(); s = np.abs(np.fft.fft2(g)); s[0, 0] = 0; v = s.ravel(); return v / np.linalg.norm(v)
def colour_vision(pg):
    keys = pg.evaluate("[...document.querySelectorAll('#refbody .pt-key')].map(k=>k.dataset.k)"); assert len(keys) == 9, keys
    for th in ("light", "dark"):
        open_dialog(pg, 1024, 768, th); R = {k: render(pg, k) for k in keys}
        # 1) the patterns: all but one class have one, and no two look alike (spectrum of the rendered fill, colour removed)
        spec = {k: spectrum(R[k][0]) for k in keys}; flat = [k for k in keys if not spec[k].any()]; assert flat == ["transition"], ("classes without a pattern", flat)
        sim_pat = {(a, b): float(spec[a] @ spec[b]) for a, b in itertools.combinations([k for k in keys if k not in flat], 2)}
        worst = max(sim_pat.items(), key=lambda kv: kv[1]); assert worst[1] < .6, (th, "two patterns look alike", worst)
        # 2) colour: under deuteranopia / protanopia a pair of classes is told apart by the hue of its bar (Delta E >= 15) or, failing that, by its pattern
        bars = {k: rgb_of(R[k][1]["bar"]) for k in keys}; weak = []
        for a, b in itertools.combinations(keys, 2):
            de = min(np.linalg.norm(lab(sim(bars[a], kd)) - lab(sim(bars[b], kd))) for kd in CVD)
            by_pattern = (a in flat) != (b in flat) or (a not in flat and b not in flat and sim_pat[(a, b)] < .6)
            if de < 15: weak.append((a, b, round(float(de), 1), by_pattern))
        assert all(w[3] for w in weak), (th, [w for w in weak if not w[3]])
        print(f"  {th}: most alike patterns {worst[0]} {worst[1]:.2f}; hue pairs that need the pattern under CVD: {len(weak)}")
        # 3) text over the pattern: ink against the pixel of the fill nearest to the ink in luminance
        ink = rgb_of(R[keys[0]][1]["ink"]); low = []
        for k in keys:
            px = R[k][0][8:-8, 8:-8].reshape(-1, 3); c = min(contrast(ink, p) for p in px[::7]); low.append((round(c, 2), k))
        print("  " + th + ": lowest text contrast over a pattern", min(low)); assert min(low)[0] >= 4.5, (th, sorted(low)[:3])
    pg.evaluate("document.documentElement.removeAttribute('data-theme')")
r = Run(port=8984, wd="/tmp/wd_tavola_celle")
try:
    with sync_playwright() as p:
        pg = r.page(p, device_scale_factor=1); ready(pg)
        step("cell content: number, symbol, mass only; class in legend, tooltip, name and card; markers on hover", lambda: content(pg))
        for w, h in ((1024, 768), (1440, 900)):
            for th in ("light", "dark"):
                step(f"{w}px {th}: no overlap, no clipping in any of the 118 cells", lambda w=w, h=h, th=th: cells(pg, w, h, th))
        step("colour vision: distinct patterns, hue pairs under CVD, text contrast over the patterns", lambda: colour_vision(pg))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any("FAIL" in s for _, s in steps) else 0)
