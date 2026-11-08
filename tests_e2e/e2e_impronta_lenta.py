"""The copy of a dropped file does not wait for the script of the hash worker (impronta-worker.js): the server holds that script until the first
file has loaded, as a slow first visit queues it behind the page's own downloads. The file must still load (its fingerprints taken in place,
impronta.js), and a second file, dropped once the script has arrived, must load too. Synthetic files only."""
import sys, os, threading, functools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from http.server import SimpleHTTPRequestHandler
from lib import *
import servi
import misura_lista_molti as mlm
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_impronta_lenta.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))
asked, release, served = threading.Event(), threading.Event(), threading.Event()
class Slow(SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path.split("?")[0].endswith("/impronta-worker.js"):
            asked.set(); release.wait(120); served.set()
        super().do_GET()
    def log_message(self, *a): pass
r = Run(port=8973, wd="/tmp/wd_impronta_lenta")
r.srv.terminate(); r.srv.wait()
srv = servi.Server(("127.0.0.1", r.port), functools.partial(Slow, directory=str(ROOT / "site")))
threading.Thread(target=srv.serve_forever, daemon=True).start()
LOADED = "UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.filter(f => !f.gone).length === {}".format
try:
    with sync_playwright() as p:
        a = r.page(p); wait_for_js(a, "window.qqReady === true", 60000)
        f1, f2 = mlm.small_files(Path("/tmp/wd_impronta_lenta_in"), 2)
        def first():
            a.set_input_files("#pick", [f1])
            wait_for_js(a, LOADED(1), 30000)     # on a copy that waits for the worker script this times out: the server holds it
            assert asked.is_set() and not served.is_set(), "the hash worker script was not held while the file loaded"
            assert a.evaluate("document.getElementById('err').textContent") == ""
        step("a file dropped while the hash worker script is held loads at once", first)
        def second():
            release.set(); assert served.wait(30), "the hash worker script was never served"
            a.set_input_files("#pick", [f2]); wait_for_js(a, LOADED(2), 60000)
            assert a.evaluate("document.getElementById('err').textContent") == ""
        step("a file dropped after the script arrived loads too", second)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
release.set()
srv.shutdown()
for s in steps: print(" ", s)
r.report()
