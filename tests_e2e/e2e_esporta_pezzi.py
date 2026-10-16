"""Exports written piece by piece (export-pezzi.js, export-worker.js): the file downloaded through OPFS has the same bytes as the old Blob path.
Steps: small CSV with provenance, small iterable (no OPFS), a string/iterable over 8 MB with UTF-8 and a surrogate pair cut between pieces,
a synthetic 210 MB file made of pieces (never held in the test), the OPFS folder export/ emptied after the download."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import hashlib
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_esporta_pezzi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
UNIT = "riga é€ 😀 x;"                       # 2-, 3- and 4-byte characters (the emoji is a surrogate pair)
CUT = 1000003                               # a piece length that cuts a surrogate pair somewhere
L16 = 13                                    # UTF-16 length of UNIT: the sizes below are multiples of it, so the text ends between characters
def units(n): return n // L16
def expected_bytes(n): return UNIT.encode("utf-8") * units(n)
# the pieces of the periodic text, made one at a time (the page never holds the whole of it)
JS_GEN = """(n, cut) => async function* () { const U = %r, L = U.length, B = U.repeat(Math.ceil(cut / L) + 2); for (let o = 0; o < n; o += cut) { const st = o %% L; yield B.slice(st, st + Math.min(cut, n - o)); } }""" % UNIT
r = Run(port=8999, wd="/tmp/wdexp")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate("carica(\"m:export-pezzi.js\")"); attendi(pg, "window.EXPORT_PEZZI && EXPORT_PEZZI.available()")
        def save(js, path):
            with pg.expect_download(timeout=300000) as d: pg.evaluate(js)
            d.value.save_as(path); return open(path, "rb").read() if os.path.getsize(path) < 50e6 else None
        def small():
            b = save("dl('s.csv', 'a\\tb\\n1\\t2\\n', 'text/csv', false)", "/tmp/exp_s.csv"); assert b == b"a\tb\n1\t2\n", b
            b = save(f"(async () => {{ const g = ({JS_GEN})(2496, 1000); await dl('p.txt', g(), 'text/plain'); }})()", "/tmp/exp_p.txt")
            assert b == expected_bytes(2496), "short iterable"
            b = save("dl('c.csv', (async function*(){ yield 'a\\n'; yield '1\\n'; })(), 'text/csv', {table:'x'})", "/tmp/exp_c.csv")
            t = b.decode(); assert t.startswith("# mzLab ") and t.split("\n")[1:3] == ["a", "1"], t[:120]
        step("short sources: same bytes, provenance first on an iterable", small)
        def mid():
            n = 8_999_991
            b = save(f"(async () => {{ const g = ({JS_GEN})({n}, {CUT}); await dl('m.txt', g(), 'text/plain'); }})()", "/tmp/exp_m.txt")
            assert b == expected_bytes(n), "9 MB in pieces (surrogate pair cut between pieces)"
            b = save(f"(async () => {{ const g = ({JS_GEN})({n}, {n}); const s = (await g().next()).value; await dl('m2.txt', s, 'text/plain'); }})()", "/tmp/exp_m2.txt")
            assert b == expected_bytes(n), "a plain string over 8 MB"
        step("over 8 MB goes through OPFS with the same bytes", mid)
        def big():
            n = 209_999_998
            save(f"(async () => {{ const g = ({JS_GEN})({n}, {CUT}); await dl('g.txt', g(), 'text/plain'); }})()", "/tmp/exp_g.txt")
            size = os.path.getsize("/tmp/exp_g.txt"); h = hashlib.sha256(); ref = hashlib.sha256(); ub = UNIT.encode("utf-8") * 1000
            with open("/tmp/exp_g.txt", "rb") as f:
                while True:
                    c = f.read(1 << 20)
                    if not c: break
                    h.update(c)
            for _ in range(units(n) // 1000): ref.update(ub)
            ref.update(UNIT.encode("utf-8") * (units(n) % 1000))
            assert h.hexdigest() == ref.hexdigest() and size == len(UNIT.encode("utf-8")) * units(n), (size, "210 MB")
        step("210 MB in pieces: same SHA-256", big)
        def gone():
            pg.evaluate("EXPORT_PEZZI.clean()")
            n = pg.evaluate("""async () => { const r = await navigator.storage.getDirectory(); let c = 0; try { const d = await (await r.getDirectoryHandle('mzlab')).getDirectoryHandle('export'); for await (const _ of d.keys()) c++; } catch (_) {} return c; }""")
            assert n <= 4, n                         # the download of the last ones is still alive (removed 10 minutes later); nothing else is left behind
        step("export/ holds only the files of the last downloads", gone)
        def miss():
            m = pg.evaluate("window.MZLAB_LR_MISS || []"); assert not m, m
        step("no route unavailable", miss)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
