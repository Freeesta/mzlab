"""A batch of files bigger than the room the browser gives this site: the page says so once BEFORE copying (how many files, how much room, how many fit)
and loads the first ones that fit; when the copy runs out of room anyway the queue stops and ONE message counts the files loaded and not loaded,
instead of one red error per file. The room is set over CDP (Storage.overrideQuotaForOrigin: real writes fail beyond it) and the page's
storage.estimate() reports the same room. Synthetic files only (mzml_grande.py)."""
import sys, os, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mzml_grande   # before lib: lib puts tools/ first on the path
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_quota_lotto.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))
N, FREE, SLACK = 6, 100 * 2**20, 200 * 2**20       # six files of ~22 MB, ~1.8x each while loading: the page sees room for two; the real quota adds room for the scratch file of the index
r = Run(port=8975, wd="/tmp/wd_quota_lotto")
d = tempfile.mkdtemp(prefix="quota_"); names = []
for k in range(N):
    f = os.path.join(d, f"lotto{k + 1}.mzML"); mzml_grande.write(f, n_ms1=300, n_ms2=3000, seed=k + 1); names.append(f)
DONE = "UPBUSY === 0 && OPENNOW.busy() === 0"
NOTE = "[...document.querySelectorAll('#unopen .quota-note')]"
try:
    with sync_playwright() as p:
        a = r.page(p); attendi(a, "window.qqReady === true", 60000)
        cdp = a.context.new_cdp_session(a)
        def room(free, slack):          # the room the page sees (estimate) and the one the browser enforces on writes
            u = a.evaluate("navigator.storage.estimate().then(e => e.usage)")
            cdp.send("Storage.overrideQuotaForOrigin", {"origin": f"http://127.0.0.1:{r.port}", "quotaSize": u + free + slack})
            a.evaluate("""q => { const real = window.__realEst || (window.__realEst = navigator.storage.estimate.bind(navigator.storage));
                navigator.storage.estimate = async () => { const e = await real(); return { usage: e.usage, quota: q }; }; }""", u + free)
        room(FREE, SLACK)
        def ask():
            a.set_input_files("#pick", names)
            attendi(a, "document.getElementById('quotadlg')?.open === true", 30000)
            t = a.evaluate("document.getElementById('quotadlg').textContent")
            assert str(N) in t and "2" in t, t
            assert a.evaluate("document.querySelectorAll('#flst [data-up]').length") == 0, "a file started copying before the question"
        step("a batch bigger than the room asks once, before copying: N files, the room, how many fit", ask)
        def first():
            a.click("#quotadlg [data-q=first]"); attendi(a, DONE, 240000)
            why = "[ST.files.map(f => f.name), [...MZLAB_LOADFAIL], document.getElementById('unopen').innerText.slice(0, 600), navigator.storage.estimate().then(e => e.usage)]"
            assert a.evaluate("ST.files.length") == 2, a.evaluate(why)
            assert a.evaluate(NOTE + ".length") == 1, a.evaluate(why)
            t = a.evaluate(NOTE + "[0].textContent"); assert "4" in t, t
            assert a.evaluate("document.querySelectorAll('#unopen .fail').length") == 0, a.evaluate(why)
            assert a.evaluate("!!document.querySelector('#unopen [data-a=quota-retry]')")
        step("«Load the first ones»: two load, one message says four were not loaded, with «Retry the rest»", first)
        def full():
            room(0, 10 * 2**20)
            a.click("#unopen [data-a=quota-retry]"); attendi(a, "document.getElementById('quotadlg')?.open === true", 30000)
            assert a.evaluate("document.querySelector('#quotadlg [data-q=first]').hidden"), "none fits: no «Load the first ones»"
            a.click("#quotadlg [data-q=all]"); attendi(a, DONE, 240000)
            assert a.evaluate("ST.files.length") == 2, a.evaluate("ST.files.map(f => f.name)")
            assert a.evaluate(NOTE + ".length") == 1, a.evaluate("document.getElementById('unopen').innerText")
            t = a.evaluate(NOTE + "[0].textContent"); assert "4" in t, t
            assert a.evaluate("document.querySelectorAll('#unopen .fail').length") == 0, "one error per file: " + a.evaluate("document.getElementById('unopen').innerText")[:300]
            assert a.evaluate("document.querySelectorAll('#flst .fl.up.err').length") == 0, "error rows left in the side list"
        step("«Try all» with no room: the queue stops at the first refusal, ONE message, no error per file", full)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for f in names: os.remove(f)
os.rmdir(d)
for s in steps: print(" ", s)
r.report()
if any(s[1] != "ok" for s in steps): sys.exit(1)
