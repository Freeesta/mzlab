"""Layout sweep of the Dati view: with synthetic data, at 1000/1280/1600/2000 px and on iPad (portrait and landscape, touch), in light, dark and high contrast,
in Italian and English, it opens the panels (MRM after an integration; chromatogram, spectrum, XIC and map of a full-scan file), every menu and pop-up of
the page and of the panel headers, the settings, the method and the peak dialogs, and checks from the boxes on screen: no horizontal page scroll, no control
over another or covered by something that is not its own, no control cut by a box that does not scroll, labels next to their control, no text cut with an
ellipsis and no full text at hand, targets of at least 24 px (or the WCAG 2.5.8 spacing)."""
# covers: mzlab/web/app.css, mzlab/web/pannelli.js, mzlab/web/barra.js, mzlab/web/settings.js, mzlab/web/impostazioni, mzlab/web/canali
# covers: mzlab/web/help.js, mzlab/web/affianca.js, mzlab/web/barre-maniglie.js, mzlab/web/explore.js
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_impaginazione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:1500]))

r = Run(port=8973, wd="/tmp/wd_impaginazione")
SKIP = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1')}catch(_){}"

# The check of one state. `scope`: the layer just opened (a menu, a pop-up); an open modal dialog is the scope by itself (the rest of the page is inert under it).
CHECK = r"""((scope) => {
  const out = [], de = document.documentElement, vw = innerWidth, vh = innerHeight;
  const who = e => { if (!e || !e.tagName) return String(e); const id = e.id ? "#" + e.id : "", cl = typeof e.className === "string" && e.className.trim() ? "." + e.className.trim().split(/\s+/).slice(0, 2).join(".") : "";
    const nm = (e.getAttribute("aria-label") || e.getAttribute("title") || e.textContent || e.getAttribute("data-a") || "").trim().replace(/\s+/g, " ").slice(0, 30);
    return `${e.tagName.toLowerCase()}${id}${cl}«${nm}»`; };
  const add = (k, e, msg) => out.push(k + " " + who(e) + (msg ? " " + msg : ""));
  if (de.scrollWidth > de.clientWidth + 1) out.push(`page scrolls sideways: ${de.scrollWidth} > ${de.clientWidth} px`);
  const SEL = "button,a[href],input:not([type=hidden]),select,textarea,summary,[role=button],[role=tab],[role=menuitem],[role=menuitemradio],[role=checkbox],[role=radio],[role=switch],[role=slider],[role=option],[role=treeitem]";
  const shown = e => { if (!e.getClientRects().length || getComputedStyle(e).visibility !== "visible") return false;
    for (let a = e; a && a !== de; a = a.parentElement) if (+getComputedStyle(a).opacity < 0.05) return false; return true; };
  // the part of a box on screen: the viewport and every ancestor that clips (overflow other than visible)
  // and, for what scrolls with the page, the bottom of the sticky header (what went under it is out of sight, not overlapping)
  const hd = document.querySelector("header"), hb = hd ? hd.getBoundingClientRect().bottom : 0;
  const clipOf = e => { let r = { l: 0, t: 0, r: vw, b: vh }, fixed = !!hd && hd.contains(e);
    for (let a = e.parentElement; a && a !== de && a !== document.body; a = a.parentElement) { const s = getComputedStyle(a);
      if (s.overflowX !== "visible" || s.overflowY !== "visible" || s.contain.includes("paint")) { const q = a.getBoundingClientRect(); r = { l: Math.max(r.l, q.left), t: Math.max(r.t, q.top), r: Math.min(r.r, q.right), b: Math.min(r.b, q.bottom) }; }
      if (s.position === "fixed" || a.tagName === "DIALOG") { fixed = true; break; } }
    if (!fixed) r.t = Math.max(r.t, hb);
    return r; };
  const modal = [...document.querySelectorAll("dialog[open]")].filter(d => d.matches(":modal")).pop();
  const root = (scope && document.querySelector(scope)) || modal || document;
  const T = [];
  for (const e of [...root.querySelectorAll(SEL)].filter(shown)) { const b = e.getBoundingClientRect(); if (b.width < 1 || b.height < 1) continue; const c = clipOf(e);
    const v = { l: Math.max(b.left, c.l), t: Math.max(b.top, c.t), r: Math.min(b.right, c.r), b: Math.min(b.bottom, c.b) };
    if (v.r - v.l < 1 || v.b - v.t < 1) continue;            // scrolled away or folded: not on screen
    T.push({ e, b, v, np: getComputedStyle(e).pointerEvents === "none" });
    const cutW = b.width - (v.r - v.l), cutH = b.height - (v.b - v.t);
    if (cutW > 2 || cutH > 2) {                               // cut: fine when a box that scrolls (or the page, vertically) can bring the rest into view
      let scroller = b.bottom > vh || b.top < Math.max(0, c.t === hb ? hb : 0);
      for (let a = e.parentElement; a && a !== de && !scroller; a = a.parentElement) { const s = getComputedStyle(a); if (/(auto|scroll)/.test(s.overflowX + s.overflowY) && (a.scrollWidth > a.clientWidth + 1 || a.scrollHeight > a.clientHeight + 1)) scroller = true; }
      if (!scroller) add("cut", e, `${Math.round(cutW)}x${Math.round(cutH)} px`); }
  }
  // the control with the focus is in sight: not under the sticky header
  const f = document.activeElement; if (f && f !== document.body && hd && !hd.contains(f) && root.contains(f) && shown(f)) { const fb = f.getBoundingClientRect(), fc = clipOf(f); if (fc.t === hb && fb.bottom <= hb + 2) add("under the header", f, "has the focus"); }
  const same = (a, b) => a.contains(b) || b.contains(a) || (a.labels && [...a.labels].some(l => l.contains(b))) || (b.labels && [...b.labels].some(l => l.contains(a))) || (!!a.closest("label") && a.closest("label") === b.closest("label"));
  // the 44 px handles of two integration bars that sit close on the data share their place: the handle is where the bar is (keyboard: ← →)
  const bars = (a, b) => a.classList.contains("ibh") && b.classList.contains("ibh") && a.parentElement === b.parentElement;
  const inter = (p, q) => Math.min(p.r, q.r) - Math.max(p.l, q.l) > 2 && Math.min(p.b, q.b) - Math.max(p.t, q.t) > 2;
  for (let i = 0; i < T.length; i++) for (let j = i + 1; j < T.length; j++) if (!T[i].np && !T[j].np && inter(T[i].v, T[j].v) && !same(T[i].e, T[j].e) && !bars(T[i].e, T[j].e)) add("overlap", T[i].e, "and " + who(T[j].e));
  // covered: what is on top at the centre of the part on screen is not the control (a sticky bar, a pop-up that is not its own)
  for (const t of T) { const h = document.elementFromPoint((t.v.l + t.v.r) / 2, (t.v.t + t.v.b) / 2);
    if (t.np || !h || same(t.e, h) || (h.classList.contains("ibh") && bars(t.e, h))) continue;
    add("covered", t.e, "by " + who(h)); }
  // targets under 24 px, unless the 24 px circle around their centre touches no other target (WCAG 2.5.8) or they are links inside a sentence
  for (const t of T) { const w = t.b.width, h = t.b.height; if (t.np || (w >= 23.5 && h >= 23.5) || (t.e.tagName === "A" && t.e.closest("p,li,dd,td"))) continue;
    const cx = (t.b.left + t.b.right) / 2, cy = (t.b.top + t.b.bottom) / 2;
    if (T.some(o => o !== t && !same(o.e, t.e) && Math.hypot(Math.max(o.b.left - cx, 0, cx - o.b.right), Math.max(o.b.top - cy, 0, cy - o.b.bottom)) < 12)) add("small", t.e, `${Math.round(w)}x${Math.round(h)} px`); }
  // labels: the control on the row of its text (a radio or checkbox right before it), or right under it
  for (const l of root.querySelectorAll("label")) { const c = l.control; if (!c || !shown(l) || !shown(c) || c.type === "hidden") continue;
    const rg = document.createRange(), rs = []; const w = document.createTreeWalker(l, NodeFilter.SHOW_TEXT);
    for (let t; (t = w.nextNode());) { if (!t.textContent.trim() || c.contains(t) || t.parentElement.closest("select,button,input")) continue; rg.selectNodeContents(t); rs.push(...rg.getClientRects()); }
    if (!rs.length) continue;
    const X = { l: Math.min(...rs.map(q => q.left)), r: Math.max(...rs.map(q => q.right)), t: Math.min(...rs.map(q => q.top)), b: Math.max(...rs.map(q => q.bottom)) };
    const C = c.getBoundingClientRect(), box = (c.type === "radio" || c.type === "checkbox") && c.getAttribute("role") !== "switch";
    if (c.getAttribute("role") === "switch" && l.contains(c) && Math.min(X.b, C.bottom) - Math.max(X.t, C.top) > 0) continue;      // a settings row: text, then its switch at the end
    if (Math.min(X.b, C.bottom) - Math.max(X.t, C.top) > 0) { const gap = box ? X.l - (C.left + C.right) / 2 : Math.max(C.left - X.r, X.l - C.right, 0);
      if (box ? gap < 0 || gap > 24 : gap > 120) add("label", c, `${Math.round(gap)} px from its text`); }
    else if (Math.abs(C.left - X.l) > 48 || C.top - X.b > 24 || X.t - C.bottom > 24) add("label", c, "away from its text"); }
  // text cut with an ellipsis and no full text at hand (a title or aria-label on it or a close ancestor)
  for (const e of (root === document ? document.body : root).querySelectorAll("*")) { if (e.children.length > 3) continue; const s = getComputedStyle(e);
    if (s.textOverflow !== "ellipsis" || e.scrollWidth <= e.clientWidth + 1 || !shown(e)) continue;
    let ok = false; for (let a = e, k = 0; a && k < 4 && !ok; a = a.parentElement, k++) ok = !!(a.title || a.getAttribute("aria-label"));
    if (!ok) add("ellipsis", e, ""); }
  return out;
})"""
# the layer a click has shown: an open dialog, else the largest new absolute/fixed box that holds controls; marked with data-sweep
SNAP = "() => { window.__lay = new Set([...document.querySelectorAll('body *')].filter(e => e.offsetWidth && /(absolute|fixed)/.test(getComputedStyle(e).position))); }"
NEW = """() => { document.querySelectorAll('[data-sweep]').forEach(e => e.removeAttribute('data-sweep'));
  const d = [...document.querySelectorAll('dialog[open]')].pop(); if (d && !window.__lay.has(d)) { d.setAttribute('data-sweep', ''); return 'dialog#' + d.id; }
  const n = [...document.querySelectorAll('body *')].filter(e => e.offsetWidth && /(absolute|fixed)/.test(getComputedStyle(e).position) && !window.__lay.has(e) && e.querySelector('button,input,select,a[href],[role=menuitem],[role=option],[role=treeitem]'));
  const top = n.filter(e => !n.some(o => o !== e && o.contains(e))).sort((a, b) => b.offsetWidth * b.offsetHeight - a.offsetWidth * a.offsetHeight)[0];
  if (!top) return ''; top.setAttribute('data-sweep', ''); return (top.id ? '#' + top.id : '.' + String(top.className).split(' ')[0]); }"""
# what opens a menu, a pop-up or a dialog: the buttons that say so, plus the settings, the method, the help and the tools of the toolbar
OPENERS = "[aria-haspopup]:not(:disabled),#np-set,#np-method,.hq,#tb-search,#np-addloss,#np-liste,#np-pt,#samptab"
LO = (1000, 900), (1280, 800), (1600, 1000), (2000, 1100)
IPAD = (768, 1024), (1024, 768)
# every size in Italian and light; English, dark and high contrast where they change the most (long words, narrow widths, borders)
CONFIGS = [(s, False, "it", "light") for s in LO] + [(s, True, "it", "light") for s in IPAD] + [
    ((1000, 900), False, "en", "light"), ((768, 1024), True, "en", "light"), ((1280, 800), False, "it", "dark"), ((1280, 800), False, "en", "hc"), ((1024, 768), True, "it", "hc")]
FULL = [((1000, 900), False, "it", "light"), ((2000, 1100), False, "en", "light"), ((768, 1024), True, "it", "dark")]
found = []


def page(size, touch, lang, theme):
    k = dict(viewport={"width": size[0], "height": size[1]}, locale="it-IT" if lang == "it" else "en-US", reduced_motion="reduce")
    if touch: k.update(has_touch=True, is_mobile=True)
    c = r.b.new_context(**k); c.add_init_script(SKIP + f"try{{localStorage.setItem('mzlab.lang','{lang}')}}catch(_){{}}")
    q = c.new_page(); q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_selector("#pick", state="attached"); ready(q)
    q.evaluate("t => { UIP.theme = t === 'light' ? 'light' : 'dark'; uipApply(); QA11Y.write(t === 'hc' ? { bg: 'hc' } : {}); }", theme)
    assert q.evaluate("I18N.lang") == lang, ("language", lang)
    return q


def look(q, tag, scope=None):
    frames(q, 3)
    for f in q.evaluate(CHECK, scope): found.append(f"{tag}: {f}")


def close_layer(q, opener):
    for _ in range(2):
        q.keyboard.press("Escape"); frames(q, 2)
        if not q.evaluate("!!document.querySelector('[data-sweep]')?.offsetWidth || !!document.querySelector('dialog[open]')"): return
    if q.evaluate(f"(e => !!e && e.getAttribute('aria-expanded') === 'true')(document.querySelector('{opener}'))"): q.click(opener); frames(q, 2)


def sweep(q, tag):
    """Every opener on screen (outside dialogs): click, check the layer it showed (or the page, for a tool docked beside the charts), close."""
    n = q.evaluate(f"[...document.querySelectorAll('{OPENERS}')].filter(e => e.offsetWidth && !e.closest('dialog')).map((e, i) => (e.setAttribute('data-sw-b', i), i))")
    for i in n:
        sel = f"[data-sw-b='{i}']"
        if not q.evaluate(f"(e => !!e && !!e.offsetWidth && !e.disabled)(document.querySelector(\"{sel}\"))"): continue
        nm = q.evaluate(f"(e => e.id || (e.getAttribute('aria-label') || e.textContent).trim().slice(0, 24))(document.querySelector(\"{sel}\"))")
        q.evaluate(SNAP); q.click(sel); frames(q, 3); hold(q, 250)
        lay = q.evaluate(NEW)
        look(q, f"{tag} › {nm}", "[data-sweep]" if lay else None)
        close_layer(q, sel)


def mrm(size, touch, lang, theme):
    tag = f"{size[0]}x{size[1]}{' touch' if touch else ''} {lang} {theme} MRM"
    q = page(size, touch, lang, theme)
    try:
        look(q, tag + " empty page")
        q.set_input_files("#pick", [mz("B_MRM-t0"), mz("B_MRM-t5")]); hold(q, 1000)
        q.evaluate("document.getElementById('colsbtn')?.click()")
        if q.evaluate("!!document.querySelector('dialog[open]')"): look(q, tag + " sample columns"); q.keyboard.press("Escape"); frames(q, 2)
        look(q, tag + " files picked")
        q.click("text=" + ("Carica dati" if lang == "it" else "Load data")); ready(q)
        wait_for_js(q, "!!document.querySelector('.pnl.mrm canvas') && E.panels.some(p => p.type === 'mrm' && p._a)", 60000)
        q.evaluate("document.querySelector('.pnl.mrm [data-a=iauto]').click()"); frames(q, 2)
        b = q.locator(".pnl.mrm canvas").first.bounding_box()
        for f in (0.70, 0.71, 0.69, 0.72):
            q.mouse.click(b["x"] + b["width"] * f, b["y"] + b["height"] * 0.5); frames(q, 3)
            if q.evaluate("E.panels.some(p => p.type === 'mrm' && p.ints && p.ints.length)"): break
        assert q.evaluate("E.panels.some(p => p.type === 'mrm' && p.ints && p.ints.length)"), (tag, "no integration")
        hold(q, 400); look(q, tag)
        sweep(q, tag)
        for nm, js, ready_js in (("peak detection", "peakParams()", "[data-pk]"), ("integrations", "showInts()", "#ig-sn"),
                                 ("baseline", "carica('integra-base.js').then(() => { const p = E.panels.find(p => p.type === 'mrm' && p.ints.length); INTBASE.open(p, p.ints[0]); })", "#ib-pts")):
            q.evaluate(js); wait_for_js(q, f"!!document.querySelector('dialog[open] {ready_js}')", 30000)
            look(q, f"{tag} › {nm}"); q.keyboard.press("Escape"); wait_for_js(q, "!document.querySelector('dialog[open]')")
    finally:
        q.context.close()


def full(size, touch, lang, theme):
    tag = f"{size[0]}x{size[1]}{' touch' if touch else ''} {lang} {theme} full scan"
    q = page(size, touch, lang, theme)
    try:
        q.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t5")]); hold(q, 1000)
        q.click("text=" + ("Carica dati" if lang == "it" else "Load data")); ready(q)
        wait_for_js(q, "E.panels.some(p => p.type === 'chrom' && p._a) && E.panels.some(p => p.type === 'spec')", 60000)
        q.evaluate("(()=>{const f=E.files.find(x=>x.kind==='full');addPanel('xic',{tab:E.tab,traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}],x:0,y:700,w:hostWidth(),h:300,full:true});addPanel('map',{tab:'full',k:f.k,x:0,y:1020,w:hostWidth(),h:380,full:true});relayout();fitHost()})()")
        wait_for_js(q, "E.panels.some(p => p.type === 'xic') && E.panels.some(p => p.type === 'map')", 30000); ready(q); hold(q, 400)
        look(q, tag)
        sweep(q, tag)
    finally:
        q.context.close()


def verdict(what):
    out = sorted(set(found)); found.clear()
    for f in out: print("  " + f)      # the whole list, one per line (the step line keeps only the count)
    assert not out, f"{len(out)} layout problems ({what}):\n" + "\n".join(out[:80])


bad = ["not run"]
try:
    with sync_playwright() as p:
        r.page(p).context.close()      # the browser of the run (Chromium, or WebKit with --browser)
        step("MRM after an integration, every menu, pop-up and dialog: no sideways scroll, overlap, cut or covered control, detached label, bare ellipsis or small target "
             "(1000/1280/1600/2000 px, iPad portrait and landscape; English, dark and high contrast)", lambda: ([mrm(*c) for c in CONFIGS], verdict("MRM")))
        step("full scan: chromatogram, spectrum, XIC and map with every menu of their headers (same checks; 1000 px, 2000 px in English, iPad portrait in dark)",
             lambda: ([full(*c) for c in FULL], verdict("full scan")))
    bad = [s for s in steps if s[1] != "ok"]
    for n, s in steps: print(("OK " if s == "ok" else "FAIL ") + n + (" " + s if s != "ok" else ""))
finally:
    r.close()
sys.exit(1 if bad else 0)
