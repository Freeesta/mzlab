"""A big plain mzML with no room for its copy in the browser is read in place from the user's disk: it loads and draws, files/ holds no copy, the index
(the only thing written) is byte-identical to the one built from a copy, the page says so calmly, and after a reload a calm notice says to load it
again while its index is removed. The page's storage.estimate() shows room for the index alone and the browser (CDP Storage.overrideQuotaForOrigin)
refuses the copy halfway, as on a nearly full disk; the worker's own estimate does not see the override. Synthetic file (mzml_grande.py)."""
import sys, os, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mzml_grande   # before lib: lib puts tools/ first on the path
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_sul_posto.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))
r = Run(port=8977, wd="/tmp/wd_sul_posto")
d = tempfile.mkdtemp(prefix="sulposto_"); SRC = os.path.join(d, "grande.mzML")
mzml_grande.write(SRC, n_ms1=3000, n_ms2=27000, peaks=10, seed=3)      # many scans of few peaks: the index is about a fifth of the file, so it fits where the copy does not
SIZE = os.path.getsize(SRC); assert SIZE >= 52428800, SIZE        # above ARCH.BIG: a smaller file is always copied
WHY = "[ST.files.map(f => [f.name, f.kind]), UPBUSY, document.getElementById('unopen')?.innerText.slice(0, 400), window.__notes]"
DONE = "ST.files.length === 1 && ST.files[0].kind && UPBUSY === 0 && OPENNOW.busy() === 0"
NOTES = "new MutationObserver(() => { const e = document.getElementById('rust-note'); if (e && !e.hidden) (window.__notes = window.__notes || []).includes(e.textContent) || window.__notes.push(e.textContent); }).observe(document.documentElement, { subtree: true, childList: true, characterData: true, attributes: true })"
WALK = """(async () => { const out = {}; const walk = async (h, p) => { for await (const [n, c] of h.entries()) { if (c.kind === 'file') out[p + n] = (await c.getFile()).size; else await walk(c, p + n + '/'); } };
  await walk(await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), ''); return out; })()"""
SHA = """async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx');
  const b = await (await (await d.getFileHandle('grande.mzML.mzidx')).getFile()).arrayBuffer();
  return [...new Uint8Array(await crypto.subtle.digest('SHA-256', b))].map(x => x.toString(16).padStart(2, '0')).join(''); }"""
try:
    with sync_playwright() as p:
        a = r.page(p); a.add_init_script("document.addEventListener('DOMContentLoaded', () => { " + NOTES + " })"); a.reload()
        wait_for_js(a, "window.qqReady === true", 60000)
        cdp = a.context.new_cdp_session(a)
        def room(free, real):          # the room the page sees (estimate) and the one the browser enforces on writes (the worker's estimate does not see it)
            u = a.evaluate("navigator.storage.estimate().then(e => e.usage)")
            cdp.send("Storage.overrideQuotaForOrigin", {"origin": f"http://127.0.0.1:{r.port}", "quotaSize": u + real})
            a.evaluate("""q => { const real = window.__realEst || (window.__realEst = navigator.storage.estimate.bind(navigator.storage));
                navigator.storage.estimate = async () => { const e = await real(); return { usage: e.usage, quota: q }; }; }""", u + free)
        ref = {}
        def copied():
            room(4 * SIZE, 4 * SIZE)
            a.set_input_files("#pick", [SRC]); wait_for_js(a, DONE, 240000)
            o = a.evaluate(WALK)
            assert o.get("files/grande.mzML") == SIZE, o
            ref["sha"] = a.evaluate(SHA)
            a.evaluate("removeAllFiles()"); wait_for_js(a, "ST.files.length === 0 && UPBUSY === 0", 30000)
            wait_for_js(a, "(" + WALK + ").then(o => !Object.keys(o).some(k => k.includes('grande')))", 30000)
        step("with room the file is copied, as before (the reference index)", copied)
        def in_place():
            room(int(SIZE * 1.2), int(SIZE * 0.6))         # the page sees room for the index alone; the browser refuses the copy halfway, as with a full disk
            a.evaluate("window.__notes = []")
            a.set_input_files("#pick", [SRC])
            try: wait_for_js(a, DONE, 240000)
            except Exception as e: raise AssertionError((a.evaluate(WALK), str(e)[:80], a.evaluate(WHY)))
            assert not a.evaluate("document.getElementById('quotadlg')?.open === true"), "the question was asked though the index fits"
            o = a.evaluate(WALK)
            assert "files/grande.mzML" not in o, ("a copy was written", o)
            assert "idx/grande.mzML.mzidx" in o and "idx/grande.mzML.sul" in o, o
            assert not [k for k in o if k.endswith(".part")], o
            assert a.evaluate("document.querySelectorAll('#unopen .fail, #unopen .quota-note, #flst .fl.up.err').length") == 0, a.evaluate("document.getElementById('unopen')?.innerText || ''")
            want = a.evaluate("I18N.t('load.sulPosto', { name: 'grande.mzML' })")
            wait_for_js(a, "(window.__notes || []).includes(%r)" % want, 30000)
        step("the copy refused halfway: it loads in place, no copy in files/, its index and marker in idx/, one calm notice", in_place)
        def same():
            assert a.evaluate(SHA) == ref["sha"], "the index read in place differs from the one built from the copy"
        step("the index built from the user's File is byte-identical to the one built from the copy", same)
        def reads():
            wait_for_js(a, "typeof E !== 'undefined' && E.files && E.files.some(f => f.k != null)", 60000)
            j = a.evaluate("(async () => { const k = E.files.find(f => f.k != null).k; const s = await J(`api/spectra?k=${k}&i0=0&i1=5&level=1`); return [s.scans.length, s.scans[2].mz.length]; })()")
            assert j[0] > 0 and j[1] > 0, j
        step("the scans are read from the user's File (the spectra of the first scans)", reads)
        def reload():
            a.reload(); wait_for_js(a, "window.qqReady === true && window.MZLAB_LR !== undefined", 60000)
            want = a.evaluate("I18N.t('load.sulRiapri', { name: 'grande.mzML' })")
            wait_for_js(a, "(window.__notes || []).includes(%r)" % want, 30000)
            wait_for_js(a, "ST.files.length === 0", 30000)
            o = a.evaluate(WALK)
            assert not [k for k in o if "grande" in k], ("the index of a file read in place stayed after the reload", o)
        step("after a reload: a calm notice says to load it again (the file is on the disk) and its index is removed", reload)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
os.remove(SRC); os.rmdir(d)
for s in steps: print(" ", s)
r.report()
if any(s[1] != "ok" for s in steps): sys.exit(1)
