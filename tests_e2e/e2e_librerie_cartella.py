"""Loading a folder of spectral libraries (HR «Identification with the libraries» → Load folder…): a synthetic folder with a big NIST MSP, small mixes, damaged files and
many other files. Every readable library is imported, the damaged ones are counted, and the page never blocks for more than 200 ms (the import runs in the worker)."""
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from lib import *
import subprocess
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
D = Path(tempfile.mkdtemp(prefix="mzlab-cart-"))
subprocess.run([sys.executable, str(ROOT / "tools" / "librerie_cartella_sintetica.py"), str(D), "--n", "4000", "--altri", "60"], check=True, stdout=subprocess.DEVNULL)
JS = """async () => {
  const files = [...document.getElementById('cartdir').files], ok = files.filter(f => /\\.(msp|mgf|json|jsonl|ndjson|mzlib)$/i.test(f.name));
  const lt = []; new PerformanceObserver(l => { for (const e of l.getEntries()) lt.push(Math.round(e.duration)); }).observe({ entryTypes: ['longtask'] });
  let done = 0, bad = files.length - ok.length;
  for (const f of ok) { try { await LIB.call('add', { file: f }); done++; } catch (e) { bad++; } }
  await new Promise(r => setTimeout(r, 300));
  return { done, bad, libs: ((await LIB.call('list')).libs || []).length, longtasks: lt };
}"""
r = Run(port=8830 + 140, wd="/tmp/wd_cartella")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context().new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_function("window.LIB !== undefined && LIB.engine !== undefined", timeout=120000)
        pg.evaluate("(() => { const i = document.createElement('input'); i.type = 'file'; i.id = 'cartdir'; i.multiple = true; i.setAttribute('webkitdirectory', ''); document.body.appendChild(i); })()")
        pg.set_input_files("#cartdir", str(D)); out = {}
        def load(): out.update(pg.evaluate(JS))
        step("load the folder", load)
        step("5 libraries read, 3 damaged + 60 other files counted", lambda: (_ for _ in ()).throw(AssertionError(out)) if (out.get("done"), out.get("bad"), out.get("libs")) != (5, 63, 5) else None)
        step("no main-thread task over 200 ms", lambda: (_ for _ in ()).throw(AssertionError(out["longtasks"])) if any(t > 200 for t in out.get("longtasks", [])) else None)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
