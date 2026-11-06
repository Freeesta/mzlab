"""The caps of the safe file reader (file-sicuri.js) shown to the user: a .mzflow over 8 MB opened with «Open workflow» gets one calm notice that names
the limit, says the file was not opened and gives the next step, in English and Italian, without the «invalid format» prefix (the file was never read); a typed ion list over 300 lines says its size and the cap.
Synthetic bytes only."""
import sys
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))

HUGE = b" " * (8 * 1024 * 1024 + 1)
r = Run(port=8992, wd="/tmp/wd_file_tetti")
try:
    with sync_playwright() as p:
        for lang in ("en", "it"):
            pg = r.page(p); pg.goto(f"http://127.0.0.1:{r.port}/?lang={lang}"); attendi(pg, "typeof WORKFLOW !== 'undefined' && typeof FILESICURI !== 'undefined'")
            def huge():
                pg.evaluate("window.__info = []; const o = window.info; window.info = m => { __info.push(m); if (o) o(m); }")
                pg.evaluate("WORKFLOW.open()")
                pg.set_input_files("#wf-open-input", files=[{"name": "huge.mzflow", "mimeType": "application/json", "buffer": HUGE}])
                pg.wait_for_function("__info.filter(Boolean).length > 0", timeout=10000)
                got = [m for m in pg.evaluate("__info") if m]
                want = pg.evaluate("EH(I18N.t('workflow.err.tooBig', { mb: I18N.num(8) }))")
                assert got == [want] and "8" in want and pg.evaluate("I18N.t('workflow.err.invalidFormat')") not in got[0], got
            step(f"{lang}: a .mzflow over 8 MB: one notice with the limit and the next step, no «invalid format»", huge)
            def list_cap():
                msg = pg.evaluate("TABELLA.fromList(Array.from({ length: 301 }, (_, i) => (100 + i) + '; 5').join('\\n')).then(() => 'measured', e => e.message)")
                want = pg.evaluate("I18N.t('feat.err.many', { n: 301, max: 300 })")
                assert msg == want and "301" in msg and "300" in msg, msg
            step(f"{lang}: a typed list over 300 ions names its size and the cap, and nothing is measured", list_cap)
            errs = [e for e in r.errs if e[0] in ("pageerror",)]
            step(f"{lang}: no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
            pg.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
