"""Marked-up labels are drawn once in their final form after a FULL reset (QQRESET.run("all")): the reset reloads the page twice (?riparti=1 without
the service worker, then the one cross-origin-isolation reload) and every frame of every load is recorded from the first one: the «m/z» of
«+ RT-m/z map» must be italic in the first painted frame and never change (it was upright until shell.js marked it, ~0.3-0.5 s later, on
each load: upright, italic, upright, italic), the «MS2» of the demo button keeps its superscript from the first apply on, no node is
rewritten with the same text, and nothing shifts (CLS 0).
Persistent Chromium profile (the service worker and the isolation reload are real). Rule: docs/agenti/languages.md «Marked-up texts»."""
# covers: mzlab/web/i18n.js, mzlab/web/reset.js
import sys, os, json, tempfile, time; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

# per frame (requestAnimationFrame from the first one): [time, load, kind, value]; a state is "<m/z italic?> <painted?> <MS2 superscript?> <i18n applied?>"
REC = r"""(() => { const T0 = performance.timeOrigin, now = () => +(T0 + performance.now()).toFixed(1), load = Math.round(T0 * 10);
  const send = (k, v) => { try { window.__rec(JSON.stringify([now(), load, k, v])); } catch (_) {} };
  const state = () => { const e = document.getElementById('np-map'), b = document.querySelector('button[data-demo=ms2]'); if (!e || !b) return null;
    const i = e.querySelector('i'), cs = getComputedStyle(e), r = e.getBoundingClientRect();
    return [i && getComputedStyle(i).fontStyle === 'italic' ? 'italic' : 'upright', cs.visibility === 'visible' && r.width > 0 ? 'painted' : 'unpainted',
            b.querySelector('sup') ? 'sup' : 'nosup', document.documentElement.classList.contains('i18n-wait') ? 'wait' : 'applied', e.textContent].join('|'); };
  let last; const tick = () => { const s = state(); if (s !== last) { send('frame', s); last = s; } requestAnimationFrame(tick); }; requestAnimationFrame(tick);
  send('url', location.href);
  // a write of the same text over the label: the node would be rebuilt (and, before the fix, drawn upright)
  const d = Object.getOwnPropertyDescriptor(Node.prototype, 'textContent');
  Object.defineProperty(Node.prototype, 'textContent', { get: d.get, set(v) { if (this.id === 'np-map' && d.get.call(this) === v) send('same-write', v); d.set.call(this, v); }, configurable: true });
  new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) send('shift', e.value); }).observe({ type: 'layout-shift', buffered: true });
})()"""

def loads(log, after):
    """the records of each page load that started after `after`, in order; only the real page (not about:blank)"""
    out, order = {}, []
    for t, ld, k, v in log:
        if t < after: continue
        if ld not in out: out[ld] = []; order.append(ld)
        out[ld].append((t, k, v))
    return [out[ld] for ld in order if any(k == "url" and v.startswith("http") for _, k, v in out[ld])]

def check(recs, label):
    """one load: the first painted frame already italic, never upright after it, the MS2 superscript in every applied frame, no same-text write, no shift"""
    fr = [v.split("|") for _, k, v in recs if k == "frame" and v]
    painted = [f for f in fr if f[1] == "painted"]
    if not painted: return None          # reloaded before its first paint: nothing was drawn
    seq = [f[0] for f in painted]; flips = sum(1 for a, b in zip(seq, seq[1:]) if a != b)
    assert seq[0] == "italic" and flips == 0, f"{label}: m/z states of the painted frames {seq} (text {painted[0][4]!r})"
    assert all(f[2] == "sup" for f in fr if f[3] == "applied"), f"{label}: MS2 without superscript in an applied frame: {[f[2] for f in fr]}"
    same = [v for _, k, v in recs if k == "same-write"]; assert not same, f"{label}: the label was rewritten with the same text {same}"
    cls = sum(v for _, k, v in recs if k == "shift"); assert cls == 0, f"{label}: CLS {cls}"
    return painted[0][4]

r = Run(port=8830 + 213, wd="/tmp/wd_corsivo"); log = []
try:
    with sync_playwright() as p:
        kw = dict(viewport={"width": 1400, "height": 900}, locale="en-US", reduced_motion="reduce")
        prof = tempfile.mkdtemp(prefix="corsivo_prof_")
        try: r.b = p.chromium.launch_persistent_context(prof, **kw)
        except Exception:          # a Playwright without its own headless shell (the cloud image): the Chromium that is installed
            exe = next(Path("/opt/pw-browsers").glob("chromium-*/chrome-linux64/chrome"), None); assert exe, "no Chromium"
            r.b = p.chromium.launch_persistent_context(prof, executable_path=str(exe), **kw)
        r.b.expose_function("__rec", lambda s: log.append(json.loads(s)))
        r.b.add_init_script(REC)
        pg = r.b.pages[0] if r.b.pages else r.b.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        URL = f"http://127.0.0.1:{r.port}/"

        # a new page (not the one marked __old before the action) ready (engine up; isolated, or its one isolation reload already spent) and still
        # the current document 2.5 s later. In headless Chromium a load is sometimes left without its service worker (not isolated, the reload guard
        # already set): a race of isolateOnce (browser.js) outside this test, so a reset gives 1 or 2 loads and every one of them is checked.
        SETTLED = ("!window.__old && window.qqReady && (window.crossOriginIsolated || sessionStorage.getItem('mzlab.coiReload'))"
                   " && performance.now() - (window.__readyAt = window.__readyAt || performance.now()) > 2500")
        def settled():
            wait_for_js(pg, SETTLED, timeout=90000)
        def act(js):
            pg.evaluate("window.__old = true"); t = time.time() * 1000
            try: pg.evaluate(js)
            except Exception as e: assert "context was destroyed" in str(e), e          # the page navigated away while answering
            settled(); return t

        pg.goto(URL); settled()
        if not pg.evaluate("crossOriginIsolated"):          # the race above on the first visit: one more load, now under the active worker, before the reset
            pg.evaluate("window.__old = true"); pg.reload(); settled()
        assert pg.evaluate("crossOriginIsolated"), "the page before the reset is not isolated"
        def full_reset():
            t = act("QQRESET.run('all')")
            L = loads(log, t)
            assert L, "no load after the full reset"
            texts = [check(rec, f"load {i + 1} after the reset") for i, rec in enumerate(L)]
            assert texts[-1] and all(x in (None, "+ RT-m/z map") for x in texts), texts
        step("full reset, every load: m/z italic in the first painted frame and never upright, MS2 superscript, no same-text write, CLS 0", full_reset)

        r.b.close()
finally:
    r.close()
for s in steps: print("  ", s)
errs = [e for e in r.errs if e[0] == "pageerror"]
if errs: print("page errors:", errs[:5])
if any(s[1] != "ok" for s in steps) or errs: print("FAIL"); sys.exit(1)
print("OK")
