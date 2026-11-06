"""Seeded interaction fuzzer of the site: random but reproducible keys, clicks, drags, wheel, resizes, views, dialogs, files, language, theme and reloads.
After every step it checks the page (errors, rejections, stuck busy state, focus lost to the body after a dialog, duplicated rows, NaN shown, broken
invariants) and logs long tasks; `--cicli N` measures DOM nodes, listeners and JS heap over N open/close cycles. A failure is minimised to a short
replay (seed + steps, JSON) that `--replay` runs again. Needs the built site (tools/prepara_sito.py); synthetic files only (tools/dati_sintetici.py).

    python3 tools/fuzz_ui.py --seed 7 --passi 150 [--vista data|draw|flow|hr|mix] [--tablet] [--cicli 12] [--minimizza] [--out DIR]
    python3 tools/fuzz_ui.py --replay DIR/fail-7.json
"""
from __future__ import annotations

import argparse
import json
import os
import random
import re
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e"))
sys.path.insert(0, str(ROOT / "tools"))
import lib  # noqa: E402  (Run, the fetch tracking and the cloud Chromium switches)
from playwright.sync_api import sync_playwright  # noqa: E402

DESKTOP, TABLET = (1440, 900), (1024, 768)
SIZES = [(1440, 900), (1280, 800), (1024, 768), (800, 900), (390, 844)]
KEYS = ["Tab", "Tab", "Tab", "Shift+Tab", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Enter", "Space", "Escape", "Escape", "Delete",
        "Backspace", "+", "-", "Home", "End", "PageUp", "PageDown", "Control+z", "Control+y", "Control+Shift+z", "Control+a", "Shift+F10",
        "Alt+1", "Alt+2", "Shift+?", "Shift+H", "x", "i", "r", "f", "Control+e", "Control+o", "[", "]", "0", "1"]
VIEWS = ["data", "draw", "flow"]

# Page instrumentation: unhandled rejections, long tasks, console.error with the first stack line (to tell our code from the browser's)
INSTR = r"""(() => { if (window.__fz) return; const F = window.__fz = {rej: [], long: [], cerr: []};
 addEventListener('unhandledrejection', e => { try { F.rej.push(String(e.reason && (e.reason.stack || e.reason.message) || e.reason).slice(0, 600)); } catch (_) {} });
 try { new PerformanceObserver(l => { for (const e of l.getEntries()) if (e.duration > 200) F.long.push([Math.round(e.startTime), Math.round(e.duration)]); }).observe({type: 'longtask', buffered: true}); } catch (_) {}
})()"""

# One look at the page after a step: what is open, where the focus is, what is busy, what is shown wrong
STATE = r"""() => {
 const vis = e => { if (!e || !e.isConnected) return false; const b = e.getBoundingClientRect(); if (!b.width || !b.height) return false; const s = getComputedStyle(e); return s.visibility !== 'hidden' && s.display !== 'none'; };
 const lab = e => !e ? null : e === document.body ? 'body' : (e.id ? '#' + e.id : e.tagName.toLowerCase() + (typeof e.className === 'string' && e.className.trim() ? '.' + e.className.trim().split(/\s+/)[0] : ''));
 const dlg = [...document.querySelectorAll('dialog[open], [role=dialog], [aria-modal=true], [role=menu], [role=alertdialog]')].filter(vis).map(lab);
 let a = document.activeElement; while (a && a.shadowRoot && a.shadowRoot.activeElement) a = a.shadowRoot.activeElement;
 const fr = document.querySelector('iframe:focus');
 const busy = [];
 if (typeof UPBUSY !== 'undefined' && UPBUSY > 0) busy.push('UPBUSY');
 if (typeof OPENNOW !== 'undefined' && OPENNOW.busy && OPENNOW.busy() > 0) busy.push('OPENNOW');
 if ((window.__qqF || 0) > 0) busy.push('fetch:' + (window.__qqU || []).slice(-1)[0]);
 for (const e of document.querySelectorAll('[aria-busy=true], .spin, .spinner, #loading:not([hidden])')) if (vis(e)) busy.push('spin:' + lab(e));
 const bad = []; const RX = /(^|[^A-Za-z])(NaN|Infinity|undefined|\[object Object\])([^A-Za-z]|$)|(^|[\s(:=])-0([.,]0+)?(?![\d.,])/;
 const w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT); let n;
 while ((n = w.nextNode())) { const t = n.nodeValue; if (!t || t.length > 2000 || !RX.test(t)) continue; const el = n.parentElement; if (!el || el.closest('script,style,code,pre,textarea,.mono,#fz-skip') || !vis(el)) continue; bad.push(lab(el) + ': ' + t.trim().slice(0, 80)); }
 for (const e of document.querySelectorAll('[title],[aria-label],input[type=number],input[type=text]')) { if (!vis(e)) continue; for (const v of [e.getAttribute('title'), e.getAttribute('aria-label'), e.tagName === 'INPUT' ? e.value : null]) if (v && /(^|[^A-Za-z])(NaN|Infinity|undefined)([^A-Za-z]|$)/.test(v) && !(e.tagName === 'INPUT' && (e === document.activeElement || e.dataset.fzTyped))) bad.push(lab(e) + ' attr: ' + v.slice(0, 80)); }
 const names = [...document.querySelectorAll('#flst .fl')].filter(vis).map(r => (r.dataset.k || '') + '|' + (r.querySelector('.fn, .nm, label') || r).textContent.trim().slice(0, 80));
 const dup = names.filter((x, i) => names.indexOf(x) !== i);
 const inv = typeof STATO !== 'undefined' && STATO.verifica ? STATO.verifica() : [];
 const pn = typeof E !== 'undefined' && E.panels ? E.panels.filter(p => !p.el || !p.el.isConnected).map(p => p.id + ':' + p.type) : [];
 return {page: performance.timeOrigin, dlg: [...new Set(dlg)].sort(), focus: lab(a), iframe: !!fr, busy, bad: [...new Set(bad)].slice(0, 6), dup, inv, orphan: pn,
   view: typeof S !== 'undefined' ? S.view : null, nodes: document.getElementsByTagName('*').length};
}"""

# The interactive elements a user can reach, in DOM order, with a selector that finds each again on replay
TARGETS = r"""(touch) => {
 const vis = e => { const b = e.getBoundingClientRect(); if (b.width < 2 || b.height < 2 || b.bottom < 0 || b.right < 0 || b.top > innerHeight || b.left > innerWidth) return false; const s = getComputedStyle(e); return s.visibility !== 'hidden' && s.pointerEvents !== 'none'; };
 const path = e => { const p = []; while (e && e !== document.body && e.nodeType === 1) { if (e.id && document.querySelectorAll('#' + CSS.escape(e.id)).length === 1) { p.unshift('#' + CSS.escape(e.id)); break; }
   let i = 1, s = e; while ((s = s.previousElementSibling)) if (s.tagName === e.tagName) i++; p.unshift(e.tagName.toLowerCase() + ':nth-of-type(' + i + ')'); e = e.parentElement; } return p.join(' > '); };
 const q = 'button, a[href], input, select, textarea, summary, [role=button], [role=tab], [role=menuitem], [role=option], [role=checkbox], [role=switch], [tabindex="0"], canvas, label[for]';
 const out = [];
 for (const e of document.querySelectorAll(q)) {
  if (e.disabled || e.closest('[inert], [hidden], #fz-skip') || !vis(e)) continue;
  if (e.tagName === 'A' && (e.target === '_blank' || /^(https?:|mailto:)/i.test(e.getAttribute('href') || ''))) continue;   // leaves the site
  const r = e.getBoundingClientRect();
  out.push({sel: path(e), tag: e.tagName.toLowerCase(), type: e.type || '', w: Math.round(r.width), h: Math.round(r.height), chart: e.tagName === 'CANVAS'});
 }
 return out; }"""

CYCLE_DIALOGS = ["#np-set", "#sb-help"]


class Page:
    """The browser page: launch with the instrumentation, wait for quiet, CDP counters."""
    def __init__(s, args, files):
        s.a, s.files, s.log, s.fails, s.long = args, files, [], [], []
        s.chooser_files = files[:1]

    def open(s, p, r):
        s.r = r
        s.br = p.chromium.launch()
        vp = TABLET if s.a.tablet else DESKTOP
        ctx = s.br.new_context(viewport={"width": vp[0], "height": vp[1]}, has_touch=s.a.tablet, is_mobile=s.a.tablet, reduced_motion="reduce",
                               accept_downloads=True)
        ctx.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));"
                            "localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        ctx.add_init_script(INSTR)
        pg = s.pg = ctx.new_page()
        s.errs = []
        pg.on("pageerror", lambda e: s.errs.append(("pageerror", str(e)[:400] + " @ " + " | ".join((getattr(e, "stack", "") or "").split("\n")[1:4]))))
        pg.on("console", lambda m: s.errs.append(("console.error", m.text[:400], (m.location or {}).get("url", ""))) if m.type == "error" else None)
        pg.on("dialog", lambda d: (s.errs.append(("native-dialog", d.type, d.message[:200])), d.dismiss()))
        pg.on("filechooser", lambda fc: fc.set_files([str(f) for f in s.chooser_files]))
        ctx.on("page", lambda q: q.close() if q is not pg else None)
        s.cdp = ctx.new_cdp_session(pg)
        s.cdp.send("Performance.enable")
        pg.goto(f"http://127.0.0.1:{r.port}/")
        s.wait_ready()

    def wait_ready(s, t=60):
        lib.attendi(s.pg, "window.qqReady === true", t * 1000)

    def quiet(s, limit=30.0):
        """Wait until nothing is in flight; returns what is still busy after `limit` seconds (empty = quiet)."""
        t0, st, free = time.time(), None, None
        while time.time() - t0 < limit:
            try:
                st = s.pg.evaluate(STATE)
                if not st["busy"]:
                    free = free or time.time()          # an endless animation or a ticking clock never lets the page be quiet: 1.5 s free of work is enough
                    if time.time() - free > 1.5 or s.pg.evaluate(lib._QUIET, 120): return [], st
                else:
                    free = None
                lib.frames(s.pg, 2)
            except Exception:
                try: s.pg.wait_for_load_state("load")
                except Exception: pass
        return (st or {}).get("busy", ["?"]), st

    def counters(s):
        s.cdp.send("HeapProfiler.collectGarbage")
        d = s.cdp.send("Memory.getDOMCounters")
        m = {x["name"]: x["value"] for x in s.cdp.send("Performance.getMetrics")["metrics"]}
        return {"nodes": d["nodes"], "listeners": d["jsEventListeners"], "docs": d["documents"], "heap": int(m.get("JSHeapUsedSize", 0))}



class Pointer(Page):
    """Mouse, touch and typing on one element."""
    def pointer(s, st):
        """A mouse, touch or typing step on the element of `st["sel"]`, at the fractions fx, fy of its box."""
        pg, a = s.pg, st["a"]
        loc, b = s.box(st["sel"])
        if not b or b["width"] < 1: return False
        x, y = b["x"] + b["width"] * st["fx"], b["y"] + b["height"] * st["fy"]
        s.in_dlg = a in ("click", "tap") and pg.evaluate("([x, y]) => { const e = document.elementFromPoint(x, y); return !!(e && e.closest('dialog, [role=dialog], [aria-modal=true], [role=menu]') && e.closest('button, [role=menuitem], a')); }", [x, y])
        if a == "click": pg.mouse.click(x, y)
        elif a == "dbl": pg.mouse.dblclick(x, y)
        elif a == "ctx": pg.mouse.click(x, y, button="right")
        elif a == "hover": pg.mouse.move(x, y, steps=4)
        elif a == "tap": pg.touchscreen.tap(x, y)
        elif a == "wheel":
            pg.mouse.move(x, y)
            if st["ctrl"]: pg.keyboard.down("Control")
            pg.mouse.wheel(0, st["dy"])
            if st["ctrl"]: pg.keyboard.up("Control")
        elif a == "drag":
            tx, ty = b["x"] + b["width"] * st["tx"], b["y"] + b["height"] * st["ty"]
            if st["shift"]: pg.keyboard.down("Shift")
            pg.mouse.move(x, y); pg.mouse.down(); pg.mouse.move((x + tx) / 2, (y + ty) / 2, steps=3); pg.mouse.move(tx, ty, steps=3); pg.mouse.up()
            if st["shift"]: pg.keyboard.up("Shift")
        elif a == "type":
            try:
                loc.evaluate("e => { e.dataset.fzTyped = '1'; }"); loc.fill(st["text"], timeout=1500); pg.keyboard.press("Enter")
            except Exception:
                return False
        return True

    def box(s, sel):
        loc = s.pg.locator(sel).first
        if not loc.count(): return None, None
        try: loc.scroll_into_view_if_needed(timeout=1000)   # a target scrolled out of its pane: a user scrolls to it first
        except Exception: pass
        b = loc.bounding_box()
        return loc, b


class Steps(Pointer):
    """Drawing a step from the seed and running it."""
    def gen(s, rng, step_no):
        """Draw the next step from the seed. Element picks keep the index draw and resolve to a selector at run time."""
        pg = s.pg
        k = rng.choices(["key", "click", "click", "dbl", "drag", "wheel", "hover", "ctx", "resize", "view", "lang", "theme", "files", "remove", "reload", "tap",
                         "type"], weights=[30, 30, 0, 4, 8, 6, 4, 3, 2, 4, 1, 1, 2, 2, 1, 5 if s.a.tablet else 0, 3])[0]
        if k == "key": return {"a": "key", "k": rng.choice(KEYS)}
        if k in ("click", "dbl", "drag", "wheel", "hover", "ctx", "tap", "type"):
            t = pg.evaluate(TARGETS, s.a.tablet)
            if k in ("drag", "wheel", "hover", "ctx"): t = [x for x in t if x["chart"]] or t
            if k == "type": t = [x for x in t if x["tag"] in ("input", "textarea") and x["type"] not in ("file", "checkbox", "radio", "range", "color")] or t
            if not t: return {"a": "key", "k": "Tab"}
            e = t[rng.randrange(len(t))]
            st = {"a": k, "sel": e["sel"], "fx": round(rng.uniform(0.1, 0.9), 3), "fy": round(rng.uniform(0.1, 0.9), 3)}
            if k == "drag": st.update(tx=round(rng.uniform(0.02, 0.98), 3), ty=round(rng.uniform(0.02, 0.98), 3), shift=rng.random() < 0.2)
            if k == "wheel": st.update(dy=rng.choice([-480, -120, -40, 40, 120, 480]), ctrl=rng.random() < 0.5)
            if k == "type": st["text"] = rng.choice(["0", "-1", "1e9", "abc", "288.2+16", "C10H12N2O", "", "999999", "0.0001", "NaN", "  ", "1,5"])
            if e["type"] == "file": st = {"a": "files", "n": rng.randint(1, len(s.files))}
            return st
        if k == "resize": return {"a": "resize", "w": rng.choice(SIZES)[0], "h": rng.choice(SIZES)[1]}
        if k == "view": return {"a": "view", "v": rng.choice(VIEWS)}
        if k == "lang": return {"a": "lang", "v": rng.choice(["it", "en"])}
        if k == "theme": return {"a": "theme", "v": rng.choice(["light", "dark"])}
        if k == "files": return {"a": "files", "n": rng.randint(1, len(s.files))}
        if k == "remove": return {"a": "remove", "i": rng.randrange(8)}
        return {"a": "reload", "mid": rng.random() < 0.5}

    def run(s, st):
        """Run one step. Returns False when its target is gone (a skipped step, not a failure)."""
        pg, a = s.pg, st["a"]
        if a == "key":
            pg.keyboard.press(st["k"]); return True
        if a in ("click", "dbl", "drag", "wheel", "hover", "ctx", "tap", "type"):
            return s.pointer(st)
        if a == "resize":
            pg.set_viewport_size({"width": st["w"], "height": st["h"]}); return True
        if a == "view":
            b = pg.locator(f"#nav [data-v={st['v']}]")
            if not b.count() or not b.first.is_visible(): return False
            b.first.click(); return True
        if a == "lang":
            pg.evaluate("v => I18N.set(v)", st["v"]); return True
        if a == "theme":
            pg.evaluate("v => { document.documentElement.setAttribute('data-theme', v); try { localStorage.setItem('mzlab-theme', v); } catch (_) {} }", st["v"]); return True
        if a == "files":
            sel = "#addf-input" if pg.locator("#addf-input").count() else "#pick"
            if not pg.locator(sel).count(): return False
            lib._real_sif(pg, sel, [str(f) for f in s.files[:st["n"]]]); return True
        if a == "remove":
            xs = pg.locator("#flst .fl [data-act=rm], #flst .fl .rm, #flst .fl button[aria-label*=emov], #flst .fl button[aria-label*=ogli]")
            if not xs.count(): return False
            x = xs.nth(st["i"] % xs.count())
            if not x.is_visible(): return False
            x.click(); return True
        if a == "reload":
            if st["mid"]:
                pg.evaluate("setTimeout(() => location.reload(), 30)")
                lib.frames(pg, 1)
            else:
                pg.reload()
            pg.wait_for_load_state("load"); s.wait_ready(); return True
        raise ValueError(a)



class Fuzz(Steps):
    """Checks after every step, the starting state of a view and the runs."""
    def check(s, st, before, n0):
        """What is wrong after the step: a list of short strings (empty = fine)."""
        busy, after = s.quiet(s.a.busy)
        out = []
        if busy: out.append("stuck busy after %ss: %s" % (s.a.busy, busy[:3]))
        for e in s.errs[n0:]:
            if e[0] == "pageerror": out.append("pageerror: " + e[1])
            elif e[0] == "console.error" and not _foreign(e): out.append("console.error: " + e[1] + " @ " + e[2])
            elif e[0] == "native-dialog": out.append("native %s(): %s" % (e[1], e[2]))
        if after is None: return out
        F = s.pg.evaluate("(() => { const F = window.__fz || {rej: [], long: []}; return {rej: F.rej.splice(0), long: F.long.splice(0)}; })()")
        out += ["unhandled rejection: " + x for x in F["rej"]]
        s.long += [(len(s.log), st, d) for _, d in F["long"]]
        out += ["shown: " + x for x in after["bad"]]
        if after["dup"]: out.append("duplicated file rows: %s" % after["dup"][:3])
        if after["inv"]: out.append("state invariants: %s" % after["inv"][:3])
        if after["orphan"]: out.append("panels without element: %s" % after["orphan"][:3])
        closed = before and before["dlg"] and not after["dlg"] and before["page"] == after["page"]   # a reload (a reset of the session) starts on the body
        by_user = st["a"] == "key" and st["k"] in ("Escape", "Enter", "Space") or st["a"] in ("click", "tap") and getattr(s, "in_dlg", False)
        if closed and by_user and before["focus"] != "body" and after["focus"] == "body" and not after["iframe"]:  # already on the body: a click on nothing, not the dialog
            out.append("focus fell to the body after closing %s" % before["dlg"])
        if s.a.ignora: out = [x for x in out if not re.search(s.a.ignora, x)]
        return out, after

    def prepare(s):
        """The starting state of the chosen view: files loaded for Data and HR, the view open for Draw and Flow."""
        v = s.a.vista
        if v in ("data", "mix", "hr"):
            lib._real_sif(s.pg, "#pick", [str(f) for f in s.files])
            lib.attendi(s.pg, f"typeof E !== 'undefined' && E.files.filter(f => !f.gone).length >= {len(s.files)}", 120000)
            lib.loaded(s.pg)
        if v in ("draw", "flow"):
            s.pg.click(f"#nav [data-v={v}]")
        s.quiet(60)

    def play(s, p, steps=None, seed=0):
        """Run the steps given (replay) or draw `--passi` from the seed. Returns (steps run, index of the first failing step or None, problems)."""
        r = lib.Run(port=lib.free_port(), wd=f"/tmp/wd_fuzz_{os.getpid()}")
        try:
            s.open(p, r); s.prepare()
            rng, done = random.Random(seed), []
            total = len(steps) if steps is not None else s.a.passi
            st_before = s.pg.evaluate(STATE)
            for i in range(total):
                st = steps[i] if steps is not None else s.gen(rng, i)
                if st["a"] == "files": s.chooser_files = s.files[:st["n"]]
                n0, t_step, s.in_dlg = len(s.errs), time.time(), False
                try:
                    ran = s.run(st)
                except Exception as e:
                    ran = None
                    if "Target" in str(e) or "closed" in str(e): return done + [st], i, ["the page or the browser died: " + str(e)[:200]]
                done.append(st)
                res = s.check(st, st_before, n0)
                probs, st_after = res if isinstance(res, tuple) else (res, None)
                if ran is not None or probs: s.log.append({"i": i, "st": st, "ran": ran, "p": probs})
                if s.a.v: print(f"  {i:3d} {time.time() - t_step:5.1f}s {'ran ' if ran else 'skip'} {json.dumps(st)[:150]}", flush=True)
                if probs:
                    try: s.pg.screenshot(path=str(Path(s.a.out) / f"fail-{s.a.vista}-{seed}.png"))
                    except Exception: pass
                    return done, i, probs
                st_before = st_after or st_before
            return done, None, []
        finally:
            try: s.br.close()
            except Exception: pass
            r.close()

def leak(s, p):
    """N cycles of open/close (settings, help, views, drawer tabs, language): DOM nodes, listeners and heap at cycle 2 and N."""
    r = lib.Run(port=lib.free_port(), wd=f"/tmp/wd_fuzz_{os.getpid()}")
    try:
        s.open(p, r); s.prepare()
        pg, rows = s.pg, []
        for c in range(s.a.cicli):
            for sel in CYCLE_DIALOGS:
                if pg.locator(sel).count() and pg.locator(sel).first.is_visible():
                    pg.click(sel); s.quiet(10); pg.keyboard.press("Escape"); s.quiet(10)
            for v in ("draw", "flow", "data"):
                b = pg.locator(f"#nav [data-v={v}]")
                if b.count() and b.first.is_visible(): b.first.click(); s.quiet(20)
            pg.keyboard.press("Shift+?"); s.quiet(10); pg.keyboard.press("Escape"); s.quiet(10)          # the shortcuts panel
            pg.evaluate("carica('m:molecole-ui.js').then(() => window.MOLDB.apri())"); s.quiet(10); pg.keyboard.press("Escape"); s.quiet(10)
            n = pg.evaluate("E.panels.length")                  # a spectrum panel added and closed again
            if pg.locator("#np-spec").first.is_visible():
                pg.click("#np-spec"); s.quiet(20)
                pg.evaluate(f"E.panels.length > {n} && closePanel(E.panels[E.panels.length - 1])"); s.quiet(10)
            for t in ("addloss", "lists", "stat", "file"):
                pg.evaluate(f"typeof BARRA !== 'undefined' && BARRA.setTab('{t}', {{noFocus: true, instant: true}})"); s.quiet(10)
            pg.evaluate("I18N.set(I18N.lang === 'en' ? 'it' : 'en')"); s.wait_ready(); s.quiet(20)
            rows.append(s.counters())
        return rows
    finally:
        try: s.br.close()
        except Exception: pass
        r.close()


def _foreign(e):
    """console.error lines that are not from our code: resources the browser could not load (the e2e servers answer 404 to probes)."""
    return e[1].startswith("Failed to load resource") or "favicon" in e[1]


def files_for(view, out):
    """Synthetic files for the view: three LR full scans for Data/Draw/Flow, one HR DDA file for the HR bench."""
    import numpy as np
    import dati_sintetici as ds
    out.mkdir(parents=True, exist_ok=True)
    if view == "hr":
        f = out / "Z_hr_dda.mzML"
        if not f.exists(): ds.hr_dda(f, np.random.default_rng(3))
        return [f]
    fs = [out / f"Z_t{t}.mzML" for t in (0, 30, 60)]
    for i, (f, t) in enumerate(zip(fs, (0, 30, 60))):
        if not f.exists(): ds.full_scan(f, t, np.random.default_rng(i + 11))
    return fs


def minimise(args, files, p, steps, probs):
    """Delta debugging (ddmin) on the steps: the shortest list that still fails with a problem of the same kind."""
    kind = probs[0].split(":")[0]
    def fails(sub):
        f = Fuzz(args, files)
        _, i, pr = f.play(p, steps=sub)
        return i is not None and pr and pr[0].split(":")[0] == kind
    n = 2
    while len(steps) >= 2:
        chunk = max(1, len(steps) // n); hit = False
        for k in range(0, len(steps), chunk):
            sub = steps[:k] + steps[k + chunk:]
            if sub and fails(sub):
                steps, n, hit = sub, max(n - 1, 2), True
                print(f"  minimise: {len(steps)} steps", flush=True)
                break
        if not hit:
            if chunk == 1: break
            n = min(len(steps), n * 2)
    return steps


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--passi", type=int, default=120)
    ap.add_argument("--vista", default="data", choices=["data", "draw", "flow", "hr", "mix"])
    ap.add_argument("--tablet", action="store_true", help="1024x768 touch tablet")
    ap.add_argument("--busy", type=float, default=30.0, help="seconds after which a busy state counts as stuck")
    ap.add_argument("--cicli", type=int, default=0, help="leak run: N open/close cycles instead of random steps")
    ap.add_argument("--minimizza", action="store_true")
    ap.add_argument("--ignora", help="regular expression of known problems not to stop at (a fix in flight)")
    ap.add_argument("-v", action="store_true", help="print every step")
    ap.add_argument("--replay")
    ap.add_argument("--out", default=str(ROOT / "tests_e2e" / "shots" / "fuzz"))
    a = ap.parse_args()
    out = Path(a.out); out.mkdir(parents=True, exist_ok=True)
    if a.replay:
        rep = json.loads(Path(a.replay).read_text())
        for k in ("vista", "tablet", "seed"): setattr(a, k, rep[k])
    files = files_for(a.vista, out / "dati")
    with sync_playwright() as p:
        if a.cicli:
            rows = leak(Fuzz(a, files), p)
            for i, r_ in enumerate(rows): print(i + 1, r_)
            d = {k: rows[-1][k] - rows[1][k] for k in ("nodes", "listeners")} if len(rows) > 2 else {}
            per = {k: v / (len(rows) - 2) for k, v in d.items()} if d else {}
            print("growth per cycle after the warm-up:", per)
            return 1 if per and (per["nodes"] > 50 or per["listeners"] > 10) else 0
        f = Fuzz(a, files)
        done, i, probs = f.play(p, steps=rep["steps"] if a.replay else None, seed=a.seed)
        for d_ in f.long[:20]: print("long task %d ms at step %d: %s" % (d_[2], d_[0], json.dumps(d_[1])))
        if i is None:
            print(f"OK seed {a.seed} view {a.vista}{' tablet' if a.tablet else ''}: {len(done)} steps, {sum(1 for x in f.log if x['ran'] is False)} skipped, {len(f.long)} long tasks")
            return 0
        print(f"FAIL seed {a.seed} at step {i}: {probs}")
        steps = done
        if a.minimizza:
            steps = minimise(a, files, p, done, probs)
        rep = {"seed": a.seed, "vista": a.vista, "tablet": a.tablet, "problems": probs, "steps": steps}
        fp = out / f"fail-{a.vista}-{a.seed}{'-tab' if a.tablet else ''}.json"
        fp.write_text(json.dumps(rep, indent=1))
        print("replay:", fp, f"({len(steps)} steps)")
        return 1


if __name__ == "__main__":
    sys.exit(main())
