"""Seeded burst stress: python3 burst.py START END [port]  (bursts START..END-1, each burst seed = its index).
Logs every action to burst_<start>.log BEFORE running it. Failures -> fail_<seed>.png/.json."""
from common import *
import traceback
# STRESS_WIFF_POOL = JSON list of .wiff paths (each with its .wiff.scan next to it); they are exposed under neutral names P0, P1, ...
PATHS = json.loads(os.environ["STRESS_WIFF_POOL"])
POOL = {f"P{i}": (p, p + ".scan") for i, p in enumerate(PATHS)}
INITIAL = list(POOL)[:6]
OUT = os.environ.get("STRESS_OUT", "/tmp/stress_race/"); os.makedirs(OUT, exist_ok=True)
LK = OUT + "links/"; os.makedirs(LK, exist_ok=True)
def pl(name):
    out = []
    for suffix, path in ((".wiff", POOL[name][0]), (".wiff.scan", POOL[name][1])):
        l = LK + name + suffix
        if not os.path.lexists(l): os.symlink(path, l)
        out.append(l)
    return out
START, END = int(sys.argv[1]), int(sys.argv[2]); PORT = int(sys.argv[3]) if len(sys.argv) > 3 else 8765
LOG = open(fOUT + "burst_{START}.log", "a", buffering=1)
def lg(*a): LOG.write(" ".join(str(x) for x in a) + "\n")
errs = []
BLANK = """() => { const bad = []; document.querySelectorAll('.pnl').forEach((p, i) => { if (p.classList.contains('map')) { const q = E.panels.find(q => q.el === p); if (q && !q._img) bad.push(['map-noimg', i]); return; } const cs = Array.from(p.querySelectorAll('canvas')).filter(c => c.offsetParent !== null && !c.classList.contains('mzov')); if (!cs.length) { bad.push(['nocanvas', i, p.className]); return; }
  cs.forEach(c => { if (c.classList.contains('mzov')) return; if (!c.width || !c.height) { bad.push(['zero', i]); return; } try { const d = c.getContext('2d').getImageData(0,0,c.width,c.height).data; let n = 0; for (let j = 3; j < d.length; j += 4) if (d[j]) { n++; if (n > 40) break; } if (n <= 40) bad.push(['blank', i, p.className, c.width+'x'+c.height]); } catch (e) { bad.push(['err', i, String(e)]); } }); }); return bad; }"""
STATE = """() => ({ files: E.files.filter(f=>!f.gone).length, vis: E.files.filter(f=>f.vis&&!f.gone).length, panels: E.panels.length, pnl: document.querySelectorAll('.pnl').length, miss: window.MZLAB_LR_MISS || null,
  banner: (document.querySelector('#err')||{}).innerText || '', busy: [UPBUSY, OPENNOW.busy()], conv: CONVROWS.rows.map(r=>r.name+':'+r.state), up: document.querySelectorAll('#flst .fl.up').length, opening: !!S.opening, st: ST.files.length })"""

def settle(pg):
    loaded(pg, 300000)
    try: ready(pg, 60000)
    except TimeoutError:
        d = pg.evaluate("() => ({loadingHidden: (document.querySelector('#loading')||{}).hidden, ldmsg:(document.querySelector('#ldmsg')||{}).innerText, qqF: window.__qqF, pend: E.pending && E.pending.length, sinceT: Date.now()-(window.__qqT||0)})")
        raise TimeoutError("ready stuck " + json.dumps(d))

def check(pg, seed, tag, rng):
    """After quiescence: no errors, no MISS, panels non-blank, engine answers (add a spectrum)."""
    settle(pg)
    st = pg.evaluate(STATE); probs = []
    if errs: probs.append(("errs", list(errs)))
    if st["miss"]: probs.append(("miss", st["miss"]))
    if "ricarica" in st["banner"].lower() or "reload" in st["banner"].lower() or "engine" in st["banner"].lower(): probs.append(("banner", st["banner"][:200]))
    st["conv"] = [x for x in st["conv"] if not x.endswith(":done")]
    if st["up"] or st["conv"]: probs.append(("stuck rows", st["up"], st["conv"]))
    if st["files"] == 0: return st, probs   # all removed: nothing to assert on panels
    if st["vis"] == 0:
        try:
            from lib import open_first; open_first(pg); settle(pg)
        except Exception as e: probs.append(("open_first", str(e)[:200]))
    if pg.evaluate("S.view") == "data" and pg.evaluate("workQ('#wdata').hidden") is not None:
        b = pg.evaluate(BLANK)
        if b: probs.append(("blank", b))
        n0 = pg.evaluate("document.querySelectorAll('.pnl').length")
        try:
            if not pg.evaluate("document.getElementById('np-spec').disabled"):
                pg.evaluate("document.getElementById('np-spec').click()"); settle(pg)
                b2 = pg.evaluate(BLANK)
                if b2: probs.append(("blank-after-new-spec", b2))
        except Exception as e: probs.append(("engine-probe", str(e)[:200]))
    return st, probs

def act(pg, rng, loaded_names):
    a = rng.choice(["spec", "chrom", "xic", "map", "mrm", "alt", "dbl", "row", "nav", "fsel", "keys", "remove", "addfile", "samefile", "resize", "undo", "closeall", "cancel", "reload", "back", "fwd", "nextfile_extract"] )
    return a
def js(pg, expr, arg=None):
    try: return pg.evaluate(expr, arg) if arg is not None else pg.evaluate(expr)
    except Exception as e: lg("   js-exc", str(e)[:120])
def run_action(pg, rng, a, ctx):
    w = lambda: pg.wait_for_timeout(rng.choice([0, 0, 5, 20, 60]))
    clickid = lambda i: js(pg, "i => { const b = document.getElementById(i); if (b && !b.disabled) b.click(); }", i)
    if a in ("spec", "chrom", "map", "mrm"):
        n = rng.choice([1, 2, 5, 10, 25, 50]); lg("  repeat", a, n)
        for _ in range(n): clickid("np-" + a if a != "chrom" else "np-chrom");
        w()
    elif a == "alt":
        n = rng.choice([4, 10, 20]); ids = rng.sample(["np-spec", "np-chrom", "np-map", "np-mrm", "np-xic", "np-undo", "np-redo"], 3); lg("  alt", ids, n)
        for i in range(n): clickid(ids[i % 3]); (w() if i % 4 == 0 else None)
    elif a == "xic":
        mz = rng.choice(["414", "414.2", "300", "429.1", "150", "", "abc"]); lg("  xic", mz)
        clickid("np-xic"); w()
        js(pg, "mz => { const d = document.getElementById('xicdlg'); const i = document.getElementById('xic-mz'); if (i) { i.value = mz; i.dispatchEvent(new Event('input', {bubbles:true})); } const g = document.getElementById('xic-go'); if (g && d.open) { g.click(); g.click(); } }", mz); w()
        js(pg, "() => { const d = document.getElementById('xicdlg'); if (d.open) d.close(); }")
    elif a == "dbl":
        lg("  dblclick row")
        try:
            r = pg.locator("#flst .fl .nm"); n = r.count()
            if n: r.nth(rng.randrange(n)).dblclick(timeout=2000, force=True)
        except Exception as e: lg("   dbl-exc", str(e)[:80])
    elif a == "row":
        n = rng.choice([1, 3, 8]); lg("  rowclicks", n)
        for _ in range(n):
            try:
                r = pg.locator("#flst .fl .nm"); c = r.count()
                if c: r.nth(rng.randrange(c)).click(timeout=1500, force=True)
            except Exception as e: lg("   row-exc", str(e)[:80])
            if rng.random() < .5: w()
    elif a in ("nav", "nextfile_extract"):
        n = rng.choice([5, 15, 30]); lg("  nav", n)
        for i in range(n):
            clickid(rng.choice(["fnext", "fprev"]))
            if a == "nextfile_extract" and i % 2 == 0: clickid(rng.choice(["np-spec", "np-chrom", "np-xic"]))
            if i % 5 == 0: w()
    elif a == "fsel":
        lg("  fsel");
        for _ in range(rng.choice([2, 6])):
            js(pg, "r => { const s = document.getElementById('fsel'); if (s && !s.disabled && s.options.length) { s.selectedIndex = Math.floor(r * s.options.length); s.dispatchEvent(new Event('change', {bubbles:true})); } }", rng.random()); w()
    elif a == "keys":
        ks = [rng.choice(["Escape", "Tab", "Enter", "ArrowLeft", "ArrowRight", "Shift+Tab", "Space"]) for _ in range(rng.choice([5, 15, 30]))]; lg("  keys", ks)
        for k in ks:
            try: pg.keyboard.press(k)
            except Exception as e: lg("   key-exc", str(e)[:60])
    elif a == "remove":
        n = rng.choice([1, 1, 2, 5]); lg("  remove", n)
        for _ in range(n):
            try:
                r = pg.locator("#flst .fl-x"); c = r.count()
                if c > 1: r.nth(rng.randrange(c)).click(timeout=1500, force=True)
            except Exception as e: lg("   rm-exc", str(e)[:80])
            w()
    elif a in ("addfile", "samefile"):
        nm = rng.choice(list(POOL)) if a == "addfile" else (ctx["last"] or INITIAL[0]); ctx["last"] = nm; lg("  load", a, nm)
        try: pg.set_input_files("#pick", pl(nm))
        except Exception as e: lg("   load-exc", str(e)[:80])
        w()
        if rng.random() < .5:
            nm2 = rng.choice(list(POOL)); lg("  load-2nd-immediately", nm2)
            try: pg.set_input_files("#pick", pl(nm2) + (pl(nm) if rng.random() < .3 else []))
            except Exception as e: lg("   load-exc", str(e)[:80])
    elif a == "resize":
        for _ in range(rng.choice([2, 5])):
            vw = (rng.choice([360, 600, 900, 1200, 1500, 1900]), rng.choice([500, 800, 1000, 1400])); lg("  resize", vw); pg.set_viewport_size({"width": vw[0], "height": vw[1]}); w()
    elif a in ("undo", "closeall"):
        lg("  ", a); clickid("np-undo" if a == "undo" else "np-closeall"); w(); clickid("np-redo" if a == "undo" else "np-spec")
    elif a == "cancel":
        lg("  cancel"); js(pg, "() => { document.querySelectorAll('.convprog [data-r=cancel], #flst .fl.up button, #fl-up button, #fl-up [data-x]').forEach(b => b.click()); }"); w()
    elif a == "reload":
        lg("  reload"); ctx["reloaded"] = True
        try: pg.reload(wait_until="domcontentloaded")
        except Exception as e: lg("   reload-exc", str(e)[:80])
        attendi(pg, "window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", 60000)
        errs.clear()   # a reload legitimately kills in-flight work
    elif a == "back":
        lg("  back"); js(pg, "history.back()"); w(); ctx["nav"] = True
    elif a == "fwd":
        lg("  fwd"); js(pg, "history.forward()"); w(); ctx["nav"] = True

def fresh(b):
    c, pg = newpage(b, PORT, errs); errs.clear()
    names = INITIAL
    pg.set_input_files("#pick", sum([pl(n) for n in names], []))
    loaded(pg, 300000)
    return c, pg

with sync_playwright() as p:
    b = p.chromium.launch(); c = pg = None; nfail = 0
    for seed in range(START, END):
        rng = random.Random(seed)
        try:
            if pg is None or seed % 20 == START % 20:
                if c: c.close()
                c, pg = fresh(b)
                from lib import open_first; open_first(pg); settle(pg)
            ctx = {"last": None, "reloaded": False}
            na = rng.choice([4, 8, 14]); lg(f"BURST {seed} actions={na}")
            for _ in range(na):
                a = rng.choice(["spec", "chrom", "xic", "map", "mrm", "alt", "dbl", "row", "nav", "fsel", "keys", "remove", "addfile", "samefile", "resize", "undo", "closeall", "cancel", "reload", "back", "fwd", "nextfile_extract"])
                if a == "reload" and rng.random() < .6: a = "alt"      # reload rarer
                lg(" action", a); run_action(pg, rng, a, ctx)
            if ctx.get("nav"):
                pg.wait_for_timeout(500)
                if not pg.evaluate("typeof E !== 'undefined' && !!window.MZLAB_LR"):
                    lg(" left the app via history; going back to it"); pg.goto(f"http://127.0.0.1:{PORT}/"); attendi(pg, "window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", 60000); ctx["reloaded"] = True
            if ctx["reloaded"]:
                # after a reload the files are gone from the page unless restored; just reload content
                try: pg.wait_for_timeout(500); loaded(pg, 300000)
                except Exception: pass
                if pg.evaluate("E.files.filter(f=>!f.gone).length") == 0:
                    pg.set_input_files("#pick", pl(INITIAL[0]) + pl(INITIAL[1])); loaded(pg, 300000)
            st, probs = check(pg, seed, "", rng)
            lg(f" END {seed} {json.dumps(st)} probs={json.dumps(probs)[:600]}")
            if probs:
                nfail += 1; pg.screenshot(path=fOUT + "fail_{seed}.png")
                json.dump({"seed": seed, "probs": probs, "st": st}, open(fOUT + "fail_{seed}.json", "w"))
                print("FAIL", seed, json.dumps(probs)[:300], flush=True)
                errs.clear(); pg = None      # new context next burst
        except Exception as e:
            nfail += 1; lg(f" CRASH {seed} {traceback.format_exc()[-800:]}"); print("CRASH", seed, str(e)[:300], flush=True)
            try: pg.screenshot(path=fOUT + "crash_{seed}.png")
            except Exception: pass
            errs.clear(); pg = None
            if not all(os.path.exists(p) for p in PATHS): print("DRIVE GONE"); break
    print("DONE", START, END, "fails", nfail)
