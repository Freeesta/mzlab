"""Loading a whole folder (cartella-carica.js) on the start screen, synthetic mzML only (no laboratory data, no Pyodide, dev server):
the «Load a folder» input reads the subfolders too, ignores the unknown files with a summary (N files ignored), tells two equal names apart,
and a dropped folder (simulated entries in the real drop handler) is read recursively as well."""
import sys, os, base64, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
D = Path(tempfile.mkdtemp(prefix="cartella_e2e_")); top = D / "serie"
for rel, seed in (("a.mzML", 1), ("sub/b.mzML", 2), ("sub/deeper/c.mzML", 3), ("altra/a.mzML", 4)):
    (top / rel).parent.mkdir(parents=True, exist_ok=True); synth.write_full(top / rel, 5 * seed, seed=seed)
for rel in ("note.txt", "sub/figura.pdf", ".nascosto.mzML"): (top / rel).write_text("x")
src = (top / "a.mzML").read_bytes()
import re
def names(pg): return [re.match(r"(.+?\.mzML)", t.strip()).group(1) for t in pg.evaluate("Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent)")]
def count(pg): return pg.evaluate("document.querySelectorAll('#flist input[data-k=use]').length")
r = Run(port=8830 + 140, wd="/tmp/wd_cartella")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#cartpick", timeout=60000)
        b = pg.locator("#cartpick"); assert b.get_attribute("aria-label") and b.get_attribute("title") and not pg.query_selector("#pickdirbtn, #cartcard"), "icon with label, old button and card gone"
        bx = b.bounding_box(); assert bx["width"] >= 24 and bx["height"] >= 24, bx
        def folder():
            pg.set_input_files("#pickdir", str(top))
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=4", timeout=90000)
            n = sorted(names(pg)); assert n in (["a (altra).mzML", "a.mzML", "b.mzML", "c.mzML"], ["a (serie).mzML", "a.mzML", "b.mzML", "c.mzML"]), n
            t = pg.inner_text("#cartinfo"); assert "serie" in t and "4" in t and "3 file ignorati" in t, t
            assert pg.is_visible("#cartinfo") and pg.get_attribute("#cartinfo", "role") == "status"
        step("folder input: subfolders read, 3 unknown files ignored with a summary", folder)
        def dup():
            n = names(pg); assert len(set(x.lower() for x in n)) == 4, n
            assert sum(1 for x in n if x.startswith("a")) == 2 and any("(" in x for x in n), n
        step("two files named a.mzML in different folders both appear, told apart", dup)
        def drop():
            before = count(pg)
            b64 = base64.b64encode(src).decode()
            pg.evaluate("""b64 => { const bytes = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
              const file = n => ({ isFile: true, isDirectory: false, name: n, file: ok => ok(new File([bytes], n)) });
              const dir = (n, kids) => ({ isFile: false, isDirectory: true, name: n, createReader() { let i = 0; return { readEntries(ok) { const b = kids.slice(i, i + 1); i += 1; ok(b); } }; } });
              const root = dir('caduta', [file('drop-A.mzML'), file('leggimi.txt'), dir('dentro', [file('drop-B.mzML')])]);
              const ev = new Event('drop', { bubbles: true, cancelable: true });
              Object.defineProperty(ev, 'dataTransfer', { value: { files: [], items: [{ kind: 'file', webkitGetAsEntry: () => root }] } });
              document.getElementById('drop').dispatchEvent(ev); }""", b64)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length>={before + 2}", timeout=90000)
            n = names(pg); assert "drop-A.mzML" in n and "drop-B.mzML" in n, n
            t = pg.inner_text("#cartinfo"); assert "caduta" in t and "1 file ignorato" in t, t
        step("dropped folder: read recursively through the drop handler", drop)
        def empty():
            pg.evaluate("CartCarica.load([new File(['x'],'solo.txt')],'vuota')"); pg.wait_for_timeout(300)
            t = pg.inner_text("#cartinfo"); assert "vuota" in t and "nessun file compatibile" in t, t
        step("a folder without compatible files says so", empty)
        r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
for s in steps: print("  ", s)
r.report()
bad = [s for s in steps if s[1] != "ok"]
sys.exit(1 if bad or any(e for e in r.errs if e[0] in ("pageerror",)) else 0)
