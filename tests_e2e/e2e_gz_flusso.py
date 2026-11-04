# e2e_gz_flusso: a .mzML.gz and a .mzXML.gz picked on the site are inflated piece by piece into OPFS by the engine's worker (gz-flusso.js, rust-carica.js putGz / gunzipTo), never whole in memory.
# The .mzML.gz opens with the TIC of its plain mzML, the side list shows a percentage, «Cancel» leaves no part file, `?gz=memoria` takes the in-memory path of the readers module with the same TIC,
# a .mzXML.gz opens with the TIC of its .mzXML, and the files a closed tab left go at the next start. Synthetic files only (mzml_grande.py and the mzXML written here).
# Needs the built site and the readers modules: python3 tools/prepara_sito.py, tools/build_lettori_wasm.sh site/static
import base64, gzip, math, os, struct, sys, tempfile, time, zlib; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mzml_grande   # before lib: lib puts tools/ first on the path, where a script of the same name lives
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

def two_members(src, dst):      # as bgzip or `cat a.gz b.gz` make: the stream must not stop at the end of the first member
    b = open(src, "rb").read(); k = len(b) // 3
    with open(dst, "wb") as f: f.write(gzip.compress(b[:k], 6) + gzip.compress(b[k:], 6))

def mzxml_file(path, n=60):      # n MS1 scans, 64-bit big-endian zlib peaks, with the <index> at the end
    text = '<?xml version="1.0" encoding="ISO-8859-1"?>\n<mzXML xmlns="http://sashimi.sourceforge.net/schema_revision/mzXML_3.2"><msRun scanCount="%d" startTime="PT0S" endTime="PT%dS">\n' % (n, 3 * n); pos = []
    for k in range(n):
        mz = [100 + 0.37 * i for i in range(30)]; it = [1000 * (1 + math.sin(i + k)) ** 2 + 50 for i in range(30)]
        z = zlib.compress(b"".join(struct.pack(">dd", a, b) for a, b in zip(mz, it))); pos.append(len(text.encode("latin-1")))
        text += ('<scan num="%d" msLevel="1" peaksCount="30" polarity="+" scanType="Full" centroided="0" retentionTime="PT%.1fS" totIonCurrent="%r">' % (k + 1, 3.0 * k, sum(it))
                 + '<peaks precision="64" byteOrder="network" contentType="m/z-int" compressionType="zlib" compressedLen="%d">%s</peaks></scan>\n' % (len(z), base64.b64encode(z).decode()))
    text += "</msRun>\n"; ix = len(text.encode("latin-1"))
    text += '<index name="scan">' + "".join('<offset id="%d">%d</offset>' % (i + 1, o) for i, o in enumerate(pos)) + '</index>\n<indexOffset>%d</indexOffset>\n</mzXML>\n' % ix
    Path(path).write_bytes(text.encode("latin-1"))

OPFS = """async () => { const out = [], root = await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab");
  for (const k of ["files", "conv"]) { try { for await (const [n] of (await root.getDirectoryHandle(k)).entries()) out.push(k + "/" + n); } catch (_) {} } return out.sort(); }"""
TIC = "async () => (await (await fetch('api/chrom?k=0&level=1')).json())"
SEEN = "window.__seen = []; new MutationObserver(() => window.__seen.push([...document.querySelectorAll('#fl-up [data-uptxt]')].map(t => t.textContent).join(' '))).observe(document.getElementById('flst'), {childList: true, characterData: true, subtree: true})"

def opened(pg, n=1):
    pg.wait_for_function(f"UPBUSY === 0 && ST.files.length === {n}", timeout=240000)
    open_first(pg, timeout=240000); ready(pg, timeout=240000)

d = Path(tempfile.mkdtemp(prefix="gzflusso_"))
plain = d / "s.mzML"; mzml_grande.write(str(plain), n_ms1=300, n_ms2=600, peaks=200, seed=3, minutes=5.0)
small = d / "gz" / "s.mzML.gz"; small.parent.mkdir(); two_members(plain, small)
big_src = d / "c.mzML"; mzml_grande.write(str(big_src), n_ms1=1500, n_ms2=14500, peaks=420, seed=5)
big = d / "big" / "c.mzML.gz"; big.parent.mkdir(); two_members(big_src, big); big_src.unlink()
(d / "x").mkdir(); mzxml_file(d / "x" / "x.mzXML"); xgz = d / "xgz" / "x.mzXML.gz"; xgz.parent.mkdir(); two_members(d / "x" / "x.mzXML", xgz)
res = {}
def run_page(query, fn, port):
    r = Run(port=port, wd=f"/tmp/wd_gz_{port}")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            if query: pg.goto(f"http://127.0.0.1:{r.port}/{query}"); attendi(pg, "window.MZLAB_RUST_READY !== undefined", timeout=60000)
            fn(pg)
    finally:
        r.close()
        if r.errs and any(e[0] == "pageerror" for e in r.errs): r.report()

def plain_tic(pg):
    pg.set_input_files("#pick", str(plain)); opened(pg); res["tic"] = pg.evaluate(TIC)
def stream(pg):
    pg.evaluate(SEEN)
    pg.set_input_files("#pick", str(small)); opened(pg)
    assert pg.evaluate("ST.files.map(f => f.name)") == ["s.mzML"], pg.evaluate("ST.files.map(f => f.name)")
    assert not pg.evaluate("!!(window.LETTORI_MOD && window.LETTORI_MOD.loaded)"), "the readers module was started for a streamed .mzML.gz"
    assert pg.evaluate(TIC) == res["tic"], "the TIC of the .mzML.gz is not the one of its mzML"
    o = pg.evaluate(OPFS)
    assert "files/s.mzML" in o and not any(n.endswith(".gz.part") for n in o), o
    assert any("%" in m for m in pg.evaluate("window.__seen")), "no percentage in the side list"
def cancel(pg):
    pg.set_input_files("#pick", str(big))
    pg.wait_for_function("document.querySelector('#fl-up [data-uptxt]') && /\\d+\\s*%/.test(document.querySelector('#fl-up').textContent)", timeout=120000)
    pg.evaluate("cancelLoads()")
    attendi(pg, "UPBUSY === 0", timeout=120000)
    assert pg.evaluate("ST.files.length") == 0, pg.evaluate("ST.files.map(f => f.name)")
    attendi(pg, OPFS.replace("return out.sort(); }", "return !out.some(n => n.startsWith('files/c.mzML')); }"))
def memory(pg):
    pg.set_input_files("#pick", str(small)); opened(pg)
    assert pg.evaluate("!!(window.LETTORI_MOD && window.LETTORI_MOD.loaded)"), "?gz=memoria did not take the readers module"
    assert pg.evaluate(TIC) == res["tic"], "the TIC of the in-memory path is not the one of the mzML"
    t = pg.evaluate("GZFLUSSO.memoryNote({ name: 'big.mzML.gz', size: 300 << 20 }) + ' | ' + I18N.t('err.file.gzStream')")
    assert "big.mzML.gz" in t and "300" in t and "load.note" not in t and "err.file" not in t, t
def mzxml(pg):
    pg.set_input_files("#pick", str(d / "x" / "x.mzXML")); opened(pg); res["xtic"] = pg.evaluate(TIC)
def mzxml_gz(pg):
    pg.set_input_files("#pick", str(xgz)); opened(pg)
    assert pg.evaluate(TIC) == res["xtic"], "the TIC of the .mzXML.gz is not the one of its .mzXML"
    attendi(pg, OPFS.replace("return out.sort(); }", "return !out.some(n => n.startsWith('conv/gz~')); }"))
def orphans(pg):
    pg.evaluate("""async () => { const root = await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab", { create: true });
      for (const [k, n] of [["files", "o.mzML.gz.part"], ["conv", "gz~deadpage~ab.mzXML"]]) {
        const w = await (await (await root.getDirectoryHandle(k, { create: true })).getFileHandle(n, { create: true })).createWritable(); await w.write("x"); await w.close(); } }""")
    pg.reload(); attendi(pg, "window.MZLAB_RUST_READY !== undefined", timeout=60000)
    attendi(pg, OPFS.replace("return out.sort(); }", "return !out.some(n => n.includes('o.mzML.gz.part') || n.includes('gz~deadpage')); }"), timeout=60000)

try:
    step("the plain mzML opens (reference TIC)", lambda: run_page("", plain_tic, 8861))
    step("a .mzML.gz opens through the stream: same TIC, no module, no part file, a percentage", lambda: run_page("", stream, 8862))
    step("«Cancel» during the inflate leaves neither the sample nor its part file", lambda: run_page("", cancel, 8863))
    step("?gz=memoria takes the module's path with the same TIC; the notices are in words", lambda: run_page("?gz=memoria", memory, 8864))
    step("the plain mzXML opens (reference TIC)", lambda: run_page("", mzxml, 8865))
    step("a .mzXML.gz opens with the TIC of its mzXML and its temporary file goes", lambda: run_page("", mzxml_gz, 8866))
    step("the files of a closed tab go at the next start", lambda: run_page("", orphans, 8867))
finally:
    import shutil; shutil.rmtree(d, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
