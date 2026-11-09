"""A maker's file (synthetic SCIEX .wiff + .wiff.scan pair of tools/wiff_misto.py, no laboratory data) whose members the browser refuses to copy is
read in place from the user's Files: every sample loads, no member is copied, the stubs are marked `sul`, the index of a sample is byte-identical to
the one built from the copy, the members read in place are hashed by the hash workers (each stub records their SHA-1: the builder does not read them
once more for it), and after a reload a calm notice per sample says to load it again while stubs, members and indexes go (no «interrupted
copy» notice). The browser's limit is set over CDP (Storage.overrideQuotaForOrigin). Needs the site: python3 tools/prepara_sito.py"""
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_nat_sul_posto.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:600]))
D = Path(tempfile.mkdtemp(prefix="nat_sul_posto_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_misto.py"), str(D)], check=True, capture_output=True)
PAIR = [str(D / "misto.wiff"), str(D / "misto.wiff.scan")]
with open(PAIR[1], "ab") as f: f.write(bytes(4 << 20))      # bytes past the last scan, which the reader never reads: the members outweigh the indexes, as in a real file
N = len(json.loads((D / "tipi.json").read_text()))
r = Run(port=8979, wd="/tmp/wd_nat_sul_posto")
DONE = f"ST.files.length === {N} && ST.files.every(f => f.kind) && UPBUSY === 0 && OPENNOW.busy() === 0"
NOTES = "new MutationObserver(() => { const e = document.getElementById('rust-note'); if (e && !e.hidden) (window.__notes = window.__notes || []).includes(e.textContent) || window.__notes.push(e.textContent); }).observe(document.documentElement, { subtree: true, childList: true, characterData: true, attributes: true })"
WALK = """(async () => { const out = {}; const walk = async (h, p) => { for await (const [n, c] of h.entries()) { if (c.kind === 'file') out[p + n] = (await c.getFile()).size; else await walk(c, p + n + '/'); } };
  await walk(await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), ''); return out; })()"""
SHAS = """async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx'); const o = {};
  for await (const [n, h] of d.entries()) if (n.endsWith('.mzidx')) o[n] = [...new Uint8Array(await crypto.subtle.digest('SHA-256', await (await h.getFile()).arrayBuffer()))].map(x => x.toString(16).padStart(2, '0')).join('');
  return o; }"""
STUBS = """async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('files'); const o = {};
  for await (const [n, h] of d.entries()) if (/\\.mzml$/i.test(n)) { try { o[n] = !!JSON.parse(await (await h.getFile()).text()).sul; } catch (_) { o[n] = null; } } return o; }"""
PRINTS = """async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('files'); const o = {};
  for await (const [n, h] of d.entries()) if (/\\.mzml$/i.test(n)) { try { o[n] = JSON.parse(await (await h.getFile()).text()).members.map(m => !!m.sha1); } catch (_) { o[n] = null; } } return o; }"""
CALM = "!window.qqFailed && document.getElementById('rust-note')?.getAttribute('role') === 'status' && ![...document.querySelectorAll('[role=alert]')].some(e => e.offsetParent && e.textContent.trim())"      # a notice, never the failure banner
WHY = "[ST.files.map(f => [f.name, f.kind]), UPBUSY, document.getElementById('unopen')?.innerText.slice(0, 400), window.__notes]"
try:
    with sync_playwright() as p:
        a = r.page(p); a.add_init_script("document.addEventListener('DOMContentLoaded', () => { " + NOTES + " })"); a.reload()
        wait_for_js(a, "window.qqReady === true", 60000)
        cdp = a.context.new_cdp_session(a)
        def limit(real):          # the room the browser enforces on writes (the page and the worker see their own estimate)
            u = a.evaluate("navigator.storage.estimate().then(e => e.usage)")
            cdp.send("Storage.overrideQuotaForOrigin", {"origin": f"http://127.0.0.1:{r.port}", "quotaSize": u + real})
        ref = {}
        def copied():
            a.set_input_files("#pick", PAIR); wait_for_js(a, DONE, 120000)
            o = a.evaluate(WALK); ref["walk"] = o
            assert [k for k in o if k.startswith("files/nat~")], o
            assert not any(a.evaluate(STUBS).values()), "a stub of a copied sample is marked sul"
            ref["sha"] = a.evaluate(SHAS); assert len(ref["sha"]) >= N - 1, ref["sha"]
            a.evaluate("removeAllFiles()"); wait_for_js(a, "ST.files.length === 0 && UPBUSY === 0", 30000)
            wait_for_js(a, "(" + WALK + ").then(o => !Object.keys(o).some(k => k.startsWith('files/') || k.startsWith('idx/')))", 30000)
        step("with room the members are copied, as before (the reference indexes)", copied)
        def in_place():
            members = sum(os.path.getsize(f) for f in PAIR); room = sum(v for k, v in ref["walk"].items() if not k.startswith("files/nat~"))
            limit(room + (1 << 20))          # the stubs and indexes fit, the members do not
            a.evaluate("window.__notes = []")
            a.set_input_files("#pick", PAIR)
            try: wait_for_js(a, DONE, 120000)
            except Exception as e: raise AssertionError((a.evaluate(WALK), str(e)[:60], a.evaluate(WHY)))
            o = a.evaluate(WALK)
            assert not [k for k in o if k.startswith("files/nat~") and k.endswith(".scan")], ("the .wiff.scan was copied", o)
            st = a.evaluate(STUBS); assert len(st) == N and all(st.values()), st
            assert a.evaluate("document.querySelectorAll('#unopen .fail, #unopen .quota-note, #flst .fl.up.err').length") == 0, a.evaluate(WHY)
            assert any(t == a.evaluate("n => I18N.t('load.sulPosto', { name: n })", n) for n in st for t in a.evaluate("window.__notes || []")), a.evaluate("window.__notes")
            assert a.evaluate(CALM), ("the notice looks like a failure", a.evaluate(WHY))
        step("the members refused: every sample loads in place, the .wiff is not copied, the stubs are marked sul, a calm notice", in_place)
        def hashed():
            o = a.evaluate(PRINTS)
            assert len(o) == N and all(v and all(v) for v in o.values()), ("a member read in place has no SHA-1 in its stub: the builder reads it all once more", o)
        step("the members read in place get their SHA-1 from the hash workers, not from one more read in the builder", hashed)
        def same():
            now = a.evaluate(SHAS)
            assert now == ref["sha"], ("the indexes read in place differ from the ones built from the copy", sorted(set(now) ^ set(ref["sha"])))
        step("the indexes built from the user's Files are byte-identical to the ones built from the copy", same)
        def reload():
            names = a.evaluate("ST.files.map(f => f.name)")
            a.evaluate("window.__notes = []"); a.reload(); wait_for_js(a, "window.qqReady === true && window.MZLAB_LR !== undefined", 60000)
            wait_for_js(a, "ST.files.length === 0 && (window.__notes || []).length >= %d" % len(names), 30000)
            wait_for_js(a, "(" + WALK + ").then(o => !Object.keys(o).some(k => k.startsWith('files/') || k.endsWith('.mzidx')))", 30000)
            notes = a.evaluate("window.__notes")
            calm = [a.evaluate("n => I18N.t('load.sulRiapri', { name: n })", n) for n in names]
            assert notes and all(t in calm for t in notes), ("only the calm notice of the samples read in place, no «interrupted copy»", notes)
            assert a.evaluate("n => I18N.t('load.sulRiapri', { name: n })", names[0]) in notes, notes
            o = a.evaluate(WALK)
            assert not [k for k in o if k.startswith("files/") or k.endswith(".mzidx")], ("stubs, members or indexes stayed after the reload", o)
            assert a.evaluate(CALM), ("the reload notice looks like a failure", a.evaluate(WHY))
        step("after a reload: a calm notice says to load it again, no «interrupted copy» notice, stubs, members and indexes are gone", reload)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for f in D.iterdir(): f.unlink()
D.rmdir()
for s in steps: print(" ", s)
r.report()
if any(s[1] != "ok" for s in steps): sys.exit(1)
