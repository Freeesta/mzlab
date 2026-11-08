"""A batch of files bigger than the room the browser gives this site: the page says so once BEFORE copying (how many files, how much room, how many fit)
and loads the first ones that fit; when the copy runs out of room anyway the queue stops and ONE message counts the files loaded and not loaded,
instead of one red error per file. The room is set over CDP (Storage.overrideQuotaForOrigin: real writes fail beyond it) and the page's
storage.estimate() reports the same room. Synthetic files only (mzml_grande.py)."""
import sys, os, re, tempfile
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
        a = r.page(p); wait_for_js(a, "window.qqReady === true", 60000)
        cdp = a.context.new_cdp_session(a)
        def room(free, slack):          # the room the page sees (estimate) and the one the browser enforces on writes
            u = a.evaluate("navigator.storage.estimate().then(e => e.usage)")
            cdp.send("Storage.overrideQuotaForOrigin", {"origin": f"http://127.0.0.1:{r.port}", "quotaSize": u + free + slack})
            a.evaluate("""q => { const real = window.__realEst || (window.__realEst = navigator.storage.estimate.bind(navigator.storage));
                navigator.storage.estimate = async () => { const e = await real(); return { usage: e.usage, quota: q }; }; }""", u + free)
        room(FREE, SLACK)
        def ask():
            a.set_input_files("#pick", names)
            wait_for_js(a, "document.getElementById('quotadlg')?.open === true", 30000)
            t = a.evaluate("document.getElementById('quotadlg').textContent")
            assert str(N) in t and "2" in t, t
            assert a.evaluate("document.querySelectorAll('#flst [data-up]').length") == 0, "a file started copying before the question"
        step("a batch bigger than the room asks once, before copying: N files, the room, how many fit", ask)
        def first():
            a.click("#quotadlg [data-q=first]"); wait_for_js(a, DONE, 240000)
            why = "[ST.files.map(f => f.name), [...MZLAB_LOADFAIL], document.getElementById('unopen').innerText.slice(0, 600), navigator.storage.estimate().then(e => e.usage)]"
            assert a.evaluate("ST.files.length") == 2, a.evaluate(why)
            assert a.evaluate(NOTE + ".length") == 1, a.evaluate(why)
            t = a.evaluate(NOTE + "[0].textContent"); assert "4" in t, t
            assert a.evaluate("document.querySelectorAll('#unopen .fail').length") == 0, a.evaluate(why)
            assert a.evaluate("!!document.querySelector('#unopen [data-a=quota-retry]')")
        step("«Load the first ones»: two load, one message says four were not loaded, with «Retry the rest»", first)
        def full():
            room(0, 10 * 2**20)
            a.click("#unopen [data-a=quota-retry]"); wait_for_js(a, "document.getElementById('quotadlg')?.open === true", 30000)
            assert a.evaluate("document.querySelector('#quotadlg [data-q=first]').hidden"), "none fits: no «Load the first ones»"
            a.click("#quotadlg [data-q=all]"); wait_for_js(a, DONE, 240000)
            assert a.evaluate("ST.files.length") == 2, a.evaluate("ST.files.map(f => f.name)")
            assert a.evaluate(NOTE + ".length") == 1, a.evaluate("document.getElementById('unopen').innerText")
            t = a.evaluate(NOTE + "[0].textContent"); assert "4" in t, t
            assert a.evaluate("document.querySelectorAll('#unopen .fail').length") == 0, "one error per file: " + a.evaluate("document.getElementById('unopen').innerText")[:300]
            assert a.evaluate("document.querySelectorAll('#flst .fl.up.err').length") == 0, "error rows left in the side list"
        step("«Try all» with no room: the queue stops at the first refusal, ONE message, no error per file", full)
        def panel():
            a.click("#unopen [data-a=quota-free]"); wait_for_js(a, "document.getElementById('spaziodlg')?.open === true", 30000)
            wait_for_js(a, "document.querySelectorAll('#spaziovoci li').length === 3", 30000)
            t = a.evaluate("document.getElementById('spaziodlg').innerText")
            assert re.search(r"\d[\d.]* (MB|GB)", a.evaluate("document.getElementById('spaziouso').textContent")), t
            assert "2" in a.evaluate("document.querySelector('#spaziovoci li').textContent"), t       # the two files loaded
            assert a.evaluate("document.getElementById('spaziopriv').textContent") == "", "a normal window is not called private: " + t
            assert not a.evaluate("document.querySelector('#spaziodlg [data-s=all]').hidden")
            a.keyboard.press("Escape"); wait_for_js(a, "!document.getElementById('spaziodlg').open")
            # a quota under 2 GB, as a private window gives: the panel says so and points to a normal window
            a.evaluate("""() => { const f = window.fetch; window.__f0 = f; window.fetch = async (u, o) => { const r = await f(u, o); if (!String(u).startsWith('api/spazio')) return r;
                const j = await r.json(); return new Response(JSON.stringify({ ...j, quota: 300 * 1048576 })); }; }""")
            a.click("#unopen [data-a=quota-free]"); wait_for_js(a, "document.getElementById('spaziopriv').textContent.length > 0", 30000)
            assert "300 MB" in a.evaluate("document.getElementById('spaziopriv').textContent")
            a.evaluate("window.fetch = window.__f0; 0"); a.keyboard.press("Escape")
        step("«Free space»: granted and used in real numbers, files, indexes, leftovers; a quota under 2 GB names the private window", panel)
        WALK = """(async () => { const out = {}; const walk = async (h, p) => { for await (const [n, c] of h.entries()) { if (c.kind === 'file') out[p + n] = (await c.getFile()).size; else await walk(c, p + n + '/'); } };
          await walk(await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), ''); return out; })()"""
        def clean():
            o = a.evaluate(WALK)
            assert not [k for k in o if k.endswith(".part")], ("a refused file left a partial copy or index", o)
            src = {k.split("/", 1)[1] for k in o if k.startswith("files/")}
            assert sorted(k for k in o if k.startswith("files/")) == sorted("files/" + n for n in a.evaluate("ST.files.map(f => f.name)")), o
            assert all(k.split("/", 1)[1].rsplit(".mzidx", 1)[0] in src for k in o if k.startswith("idx/") and k.endswith(".mzidx")), ("an index without its file", o)
        step("the refused files leave no partial copy, no half-written index: only the two loaded files and their indexes", clean)
        def freed():
            u0 = a.evaluate("navigator.storage.estimate().then(e => e.usage)")
            a.evaluate("removeAllFiles()"); wait_for_js(a, "ST.files.length === 0 && " + DONE, 30000)
            wait_for_js(a, "navigator.storage.estimate().then(e => e.usage < %d)" % (u0 - 40 * 2**20), 30000)
        step("«Remove all» gives the room back at once (the estimate falls by the two files)", freed)
        def one():
            room(0, 10 * 2**20)
            a.set_input_files("#pick", names[:1]); wait_for_js(a, DONE + " && document.querySelectorAll('#unopen .quota-note').length === 1", 240000)     # under 50 MB: no question, the write is refused
            assert a.evaluate(NOTE + ".length") == 1, a.evaluate("document.getElementById('unopen').innerText")
            t = a.evaluate(NOTE + "[0].textContent")
            assert len(re.findall(r"\d+ MB", t)) >= 2, "a single file says the MB it needs and the room: " + t
            assert a.evaluate("document.querySelectorAll('#flst .fl.up.err').length + document.querySelectorAll('#unopen .fail').length") == 0, t
        step("one big file with no room: the same calm notice, with the MB needed and the room the browser gives", one)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for f in names: os.remove(f)
os.rmdir(d)
for s in steps: print(" ", s)
r.report()
if any(s[1] != "ok" for s in steps): sys.exit(1)
