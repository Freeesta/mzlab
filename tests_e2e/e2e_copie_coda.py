"""The copy gate of the engine (rust-carica.js createCopyGate, lr.js copiesOf): in a drop of five mzML at most COPIES (the default of lr.js) copies into OPFS run at
once, the others say «Queued» first and start in the order of the list; with `?copie=0` (every copy at once, the behaviour before the gate) the indexes are byte-identical (SHA-256 of
every `.mzidx`); the × of a queued row cancels that file before its copy starts (lr_cancel), the others load. Synthetic files only (tools/mzml_msconvert_synthetic.py).
Needs the site and the .wasm: python3 tools/prepara_sito.py, then python3 tests_e2e/e2e_copie_coda.py"""
# covers: mzlab/web/rust-carica.js, mzlab/web/rust-worker.js, mzlab/web/lr.js, mzlab/web/shell.js
import os, re, sys, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import mzml_msconvert_synthetic as gen
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + " ".join(str(e).split())[:500]))
N = 5
K = int(re.search(r"^const COPIES = (\d+);", (ROOT / "mzlab" / "web" / "lr.js").read_text(encoding="utf-8"), re.M).group(1))      # the default of the switch
d = tempfile.mkdtemp(prefix="copie_"); fs = []
base = os.path.join(d, "coda_1.mzML"); gen.scrivi(base, scans=1500, peaks1=3000, peaks2=300)
fs.append(base)
for i in range(2, N + 1):
    p = os.path.join(d, f"coda_{i}.mzML"); shutil.copyfile(base, p); fs.append(p)
# every mzlab-index event: [ms, name, phase, wait]; the row states seen
RECORD = """() => { const t0 = performance.now(); window.__ev = []; window.__rows = {};
  addEventListener('mzlab-index', e => window.__ev.push([performance.now() - t0, e.detail.name, e.detail.phase || '', !!e.detail.wait]));
  setInterval(() => { for (const r of CONVROWS.rows) (window.__rows[r.name] = window.__rows[r.name] || new Set()).add(r.state); }, 20); }"""
# SHA-256 of every .mzidx in OPFS, by name
DIGESTS = """async () => { const root = await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), out = {};
  for await (const [n, h] of (await root.getDirectoryHandle('idx')).entries()) if (n.endsWith('.mzidx')) {
    const b = await (await h.getFile()).arrayBuffer(); out[n] = [...new Uint8Array(await crypto.subtle.digest('SHA-256', b))].map(x => x.toString(16).padStart(2, '0')).join(''); }
  return out; }"""
DONE = "UPBUSY === 0 && OPENNOW.busy() === 0"


def load(copie):
    """Loads the five files in a fresh profile with `?copie=` (None = the default); returns the events, the row states and the digests of the indexes."""
    r = Run(port=8861, wd="/tmp/wd_copie_coda")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            if copie is not None: pg.goto(f"http://127.0.0.1:{r.port}/?copie={copie}")
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            pg.evaluate(RECORD)
            pg.set_input_files("#pick", fs)
            wait_for_js(pg, DONE, 300000)
            out = {"ev": pg.evaluate("window.__ev"), "rows": pg.evaluate("Object.fromEntries(Object.entries(window.__rows).map(([k, v]) => [k, [...v]]))"),
                   "sha": pg.evaluate(DIGESTS), "files": pg.evaluate("ST.files.map(f => f.name)")}
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
            return out
    finally:
        r.close()


def copying(ev):
    """Per file the interval of its copy: first `load` event that is not a wait -> first `start` (the copy is over)."""
    a, b = {}, {}
    for t, n, ph, w in ev:
        if ph == "load" and not w and n not in a: a[n] = t
        if ph == "start" and n not in b: b[n] = t
    return {n: (a[n], b.get(n, a[n])) for n in a}


def most_at_once(iv):
    pts = sorted([(s, 1) for s, e in iv.values()] + [(e, -1) for s, e in iv.values()], key=lambda x: (x[0], x[1]))
    k = m = 0
    for _, dlt in pts: k += dlt; m = max(m, k)
    return m


R = {}
def gated():
    R["k2"] = o = load(None)
    names = [os.path.basename(f) for f in fs]
    assert sorted(o["files"]) == sorted(names), o["files"]
    iv = copying(o["ev"]); assert len(iv) == N, iv
    assert most_at_once(iv) <= K, f"more than {K} copies at once: {iv}"
    waited = sorted({n for _, n, ph, w in o["ev"] if w and ph == "load"})
    assert set(names[K:]) <= set(waited), f"the files after the first {K} did not say «Queued» before their copy: {waited}"
    assert all("wait" in o["rows"].get(n, []) for n in names[K:]), o["rows"]
    starts = sorted(iv, key=lambda n: iv[n][0])
    assert set(starts[:K]) == set(names[:K]), f"the first copies were not the first files: {starts}"
    assert starts[K:] == names[K:], f"the queued copies did not start in the order of the list: {starts}"
step("default: at most COPIES copies at once, the others «Queued» and in the order of the list", gated)


def all_at_once():
    R["k0"] = o = load(0)
    assert most_at_once(copying(o["ev"])) > 2, "?copie=0 should copy every file at once"
    assert not [1 for _, n, ph, w in o["ev"] if w and ph == "load"], "?copie=0: no copy is queued"
    a, b = R["k2"]["sha"], o["sha"]
    assert len(a) == N and a == b, f".mzidx differ between the gate and all at once: {a} vs {b}"
step("?copie=0: every copy at once, the .mzidx byte-identical to the gated load", lambda: all_at_once() if "k2" in R else (_ for _ in ()).throw(AssertionError("no gated run")))


def cancel_queued():
    r = Run(port=8862, wd="/tmp/wd_copie_annulla")
    try:
        with sync_playwright() as p:
            pg = r.page(p); pg.goto(f"http://127.0.0.1:{r.port}/?copie=1")
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            pg.evaluate(RECORD)
            victim = os.path.basename(fs[3])
            # × on the fourth row while it is queued and the first file still copies
            pg.evaluate("""v => { window.__hit = null; const t = setInterval(() => { const r = CONVROWS.find(v), b = [...document.querySelectorAll('[data-up-x]')].find(x => x.dataset.upX === v);
              if (r && r.state === 'wait' && b && !window.__ev.some(e => e[1] === v && e[2] === 'load' && !e[3])) { window.__hit = CONVROWS.rows.map(x => x.name + ':' + x.state); b.click(); clearInterval(t); } }, 10); }""", victim)
            pg.set_input_files("#pick", fs)
            wait_for_js(pg, "window.__hit !== null", 120000)
            wait_for_js(pg, DONE, 300000)
            files, ev = pg.evaluate("ST.files.map(f => f.name)"), pg.evaluate("window.__ev")
            assert victim not in files, f"the cancelled file is in the table: {files}"
            assert not [1 for _, n, ph, w in ev if n == victim and ph == "load" and not w], "the cancelled file's copy started"
            sha = pg.evaluate(DIGESTS)
            assert victim + ".mzidx" not in sha, f"the cancelled file has an index: {sorted(sha)}"
            assert len(sha) == N - 1, f"the other files should be indexed: {sorted(sha)}"
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("× on a queued row cancels that file before its copy (lr_cancel), the others load", cancel_queued)

shutil.rmtree(d, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
