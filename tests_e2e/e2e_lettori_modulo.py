"""The readers modules, one per family (crates/mzlab-lettori-wasm, mzlab/web/lettori-worker.js, rule R of shell.js upload0) proved with a reader the engine already has: with ?lettori=modulo a synthetic Agilent .D folder
(tools/agilent_sintetico.py, no laboratory data) goes to the module, which writes its mzML in OPFS; the file enters the list as any mzML, gives the same TIC as the engine's own path, «Download .mzML» gives it
byte for byte as the engine writes it and without converting again, and nothing misses the engine (MZLAB_LR_MISS empty). Needs the site and both .wasm: python tools/build_site.py, tools/build_lettori_wasm.sh
(MZLAB_PIPE_SITE=<built site> reuses a site). Also the «aperti» module: an mzML with MS-Numpress arrays (built here: linear m/z, slof intensity) goes to it without ?lettori=modulo and gives the TIC and a spectrum
of its zlib twin within the codec's precision, and an mzXML (built here: 64-bit zlib peaks, with its index) that opens in the Dati view with its TIC and a spectrum. Run from the repo root: python3 tests_e2e/e2e_lettori_modulo.py"""
import base64, math, os, shutil, socket, struct, subprocess, sys, tempfile, threading, zlib
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
import agilent_sintetico

def nib(x):   # one integer of the numpress nibble code (halves, least significant first)
    x &= 0xffffffff; top = x & 0xf0000000
    if top == 0:
        l = next((i for i in range(8) if x & (0xf0000000 >> (4 * i))), 8)
        return [l] + [(x >> (4 * (i - l))) & 0xf for i in range(l, 8)]
    if top == 0xf0000000:
        l = next((i for i in range(8) if (x & (0xf0000000 >> (4 * i))) != (0xf0000000 >> (4 * i))), 7)
        return [l + 8] + [(x >> (4 * (i - l))) & 0xf for i in range(l, 8)]
    return [0] + [(x >> (4 * i)) & 0xf for i in range(8)]
def np_linear(v, fp):
    q = [int(x * fp + 0.5) for x in v]; out = struct.pack(">d", fp) + b"".join(struct.pack("<I", x & 0xffffffff) for x in q[:2]); h = []
    for i in range(2, len(q)): h += nib(q[i] - (q[i - 1] + (q[i - 1] - q[i - 2])))
    return out + bytes(((h[i] << 4) | (h[i + 1] if i + 1 < len(h) else 8)) for i in range(0, len(h), 2))
def np_slof(v, fp): return struct.pack(">d", fp) + b"".join(struct.pack("<H", int(math.log(x + 1) * fp + 0.5)) for x in v)
def b64(b): return base64.b64encode(b).decode()
def mzml_file(path, numpress, n=40):
    """n MS1 spectra; the arrays are numpress (linear + slof, zlib on top for the old accessions) or the same values as zlib-compressed doubles; the intensities are rounded to what slof keeps so both read alike"""
    def cv(a, nm): return f'<cvParam cvRef="MS" accession="{a}" name="{nm}" value=""/>'
    out = ['<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><cvList count="1"><cv id="MS" fullName="PSI-MS" version="4.1.0" URI="x"/></cvList>',
           '<fileDescription><fileContent>' + cv("MS:1000579", "MS1 spectrum") + '</fileContent></fileDescription>',
           '<instrumentConfigurationList count="1"><instrumentConfiguration id="IC1">' + cv("MS:1000031", "instrument model") + '</instrumentConfiguration></instrumentConfigurationList>',
           f'<run id="r" defaultInstrumentConfigurationRef="IC1"><spectrumList count="{n}" defaultDataProcessingRef="dp">']
    for k in range(n):
        mz = [100 + 0.37 * i + 0.001 * (i % 7) for i in range(30)]; it = [1000 * (1 + math.sin(i + k)) ** 2 + 50 for i in range(30)]
        tic = sum(it)
        if numpress:
            a_mz = b64(np_linear(mz, 1e7)); a_it = b64(np_slof(it, 1000.0))
            mzc = cv("MS:1002312", "MS-Numpress linear prediction compression") + cv("MS:1000523", "64-bit float")
            itc = cv("MS:1002314", "MS-Numpress short logged float compression") + cv("MS:1000523", "64-bit float")
        else:
            fp = 1000.0; it = [math.exp(int(math.log(x + 1) * fp + 0.5) / fp) - 1 for x in it]
            mz = [round(x * 1e7) / 1e7 for x in mz]
            a_mz = b64(zlib.compress(struct.pack("<30d", *mz))); a_it = b64(zlib.compress(struct.pack("<30d", *it)))
            mzc = cv("MS:1000574", "zlib compression") + cv("MS:1000523", "64-bit float"); itc = mzc
        out.append(f'<spectrum index="{k}" id="scan={k + 1}" defaultArrayLength="30">' + cv("MS:1000511", "ms level").replace('value=""', 'value="1"')
                   + cv("MS:1000130", "positive scan") + cv("MS:1000128", "profile spectrum")
                   + '<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="%.4f" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>' % (k * 0.05)
                   + '<binaryDataArrayList count="2"><binaryDataArray encodedLength="0">' + mzc + cv("MS:1000514", "m/z array") + f'<binary>{a_mz}</binary></binaryDataArray>'
                   + '<binaryDataArray encodedLength="0">' + itc + cv("MS:1000515", "intensity array") + f'<binary>{a_it}</binary></binaryDataArray></binaryDataArrayList></spectrum>')
    out.append('</spectrumList></run></mzML>')
    Path(path).write_text("".join(out), encoding="utf-8")

def mzxml_file(path, n=40):
    """n MS1 scans, 64-bit big-endian peaks in zlib, with the <index> at the end; returns (rt in minutes, TIC) of each"""
    body = ['<?xml version="1.0" encoding="ISO-8859-1"?>\n<mzXML xmlns="http://sashimi.sourceforge.net/schema_revision/mzXML_3.2"><msRun scanCount="%d" startTime="PT0S" endTime="PT%dS">\n' % (n, 3 * n),
            '<parentFile fileName="campione.raw" fileType="RAWData" fileSha1="0123456789abcdef0123456789abcdef01234567"/><msInstrument msInstrumentID="1"><msManufacturer category="msManufacturer" value="Test"/><msModel category="msModel" value="Modello X"/></msInstrument>\n']
    pos = []; tics = []; text = body[0] + body[1]
    for k in range(n):
        mz = [100 + 0.37 * i + 0.001 * (i % 7) for i in range(30)]; it = [1000 * (1 + math.sin(i + k)) ** 2 + 50 for i in range(30)]
        raw = b"".join(struct.pack(">dd", a, b) for a, b in zip(mz, it)); z = zlib.compress(raw); tics.append(sum(it))
        pos.append(len(text.encode("latin-1")))
        text += ('<scan num="%d" msLevel="1" peaksCount="30" polarity="+" scanType="Full" centroided="0" retentionTime="PT%.1fS" totIonCurrent="%r">' % (k + 1, 3.0 * k, sum(it))
                 + '<peaks precision="64" byteOrder="network" contentType="m/z-int" compressionType="zlib" compressedLen="%d">%s</peaks></scan>\n' % (len(z), b64(z)))
    text += "</msRun>\n"; ix = len(text.encode("latin-1"))
    text += '<index name="scan">' + "".join('<offset id="%d">%d</offset>' % (i + 1, o) for i, o in enumerate(pos)) + '</index>\n<indexOffset>%d</indexOffset>\n</mzXML>\n' % ix
    Path(path).write_bytes(text.encode("latin-1")); return tics

def cdf_file(path, n=40):
    """an ANDI-MS file (netCDF-3 classic) of n scans of 30 points, float m/z and intensities as record variables; returns the TIC of each scan"""
    def pad(b): return b + b"\0" * (-len(b) % 4)
    def nm(t): return struct.pack(">I", len(t)) + pad(t.encode())
    def atts(a):
        if not a: return struct.pack(">II", 0, 0)
        return struct.pack(">II", 0x0C, len(a)) + b"".join(nm(k) + struct.pack(">II", 2, len(v)) + pad(v.encode()) for k, v in a)
    tics = []; mz = []; it = []
    for k in range(n):
        y = [1000 * (1 + math.sin(i + k)) ** 2 + 50 for i in range(30)]
        y = [struct.unpack(">f", struct.pack(">f", v))[0] for v in y]
        mz += [100 + 0.37 * i for i in range(30)]; it += y; tics.append(sum(y))
    dims = [("point_number", 0), ("scan_number", n)]
    # (name, dim ids, nc_type, data bytes, record?)
    dbl = lambda v: b"".join(struct.pack(">d", x) for x in v); ints = lambda v: b"".join(struct.pack(">i", x) for x in v)
    flt = lambda v: b"".join(struct.pack(">f", x) for x in v)
    fixed = [("scan_acquisition_time", 6, dbl([k * 3.0 for k in range(n)])), ("total_intensity", 6, dbl(tics)),
             ("scan_index", 4, ints([30 * k for k in range(n)])), ("point_count", 4, ints([30] * n))]
    rec = [("mass_values", flt(mz)), ("intensity_values", flt(it))]
    def header(begins):
        h = b"CDF\x01" + struct.pack(">I", 30 * n) + struct.pack(">II", 0x0A, 2) + b"".join(nm(d) + struct.pack(">I", l) for d, l in dims)
        h += atts([("experiment_type", "Centroided Mass Spectrum"), ("test_ionization_polarity", "Positive Polarity")])
        h += struct.pack(">II", 0x0B, len(fixed) + len(rec))
        for (name, ty, data), b in zip(fixed, begins):
            h += nm(name) + struct.pack(">II", 1, 1) + atts([]) + struct.pack(">III", ty, len(data), b)
        for (name, data), b in zip(rec, begins[len(fixed):]):
            h += nm(name) + struct.pack(">II", 1, 0) + atts([]) + struct.pack(">III", 5, 4, b)
        return h
    pos = len(header([0] * 6)); begins = []
    for _, _, data in fixed: begins.append(pos); pos += len(data)
    begins += [pos, pos + 4]
    out = header(begins) + b"".join(d for _, _, d in fixed)
    for r in range(30 * n): out += rec[0][1][4 * r:4 * r + 4] + rec[1][1][4 * r:4 * r + 4]
    Path(path).write_bytes(out); return tics

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

tmp = Path(tempfile.mkdtemp(prefix="lettori_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
    src = ROOT / "site" / "static" / "wasm-lettori"
    assert (src / "manifest.json").exists(), "build it: tools/build_lettori_wasm.sh site/static"
    shutil.copytree(src, site / "static" / "wasm-lettori", dirs_exist_ok=True)
N = 120
folder = agilent_sintetico.write(tmp / "serie", "uno.D", N)
top = folder.parent
lc_top = tmp / "lc"
agilent_sintetico.write_lcms(lc_top, "mrm.D", "mrm"); agilent_sintetico.write_lcms(lc_top, "perfil.D", "profile")
for nm, flag in (("npa", True), ("npb", False)):   # two folders of one mzML each: numpress and its zlib twin
    (tmp / nm).mkdir(); mzml_file(tmp / nm / "np.mzML", flag)
(tmp / "xml").mkdir(); XTIC = mzxml_file(tmp / "xml" / "x.mzXML")
(tmp / "cdf").mkdir(); CTIC = cdf_file(tmp / "cdf" / "c.cdf")

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):   # the server answers
    try: socket.create_connection(("127.0.0.1", PORT), 1).close(); break
    except OSError: threading.Event().wait(0.1)
errs = []; asked = []
LOAD = """async () => {
  const rb = await window.MZLAB_RUST_READY;
  await new Promise(r => setTimeout(r, 300));
  const tic = await (await fetch("api/chrom?k=0&level=1")).json();
  return { files: ST.files.map(x => x.name), native: rb.isNative("uno.mzML"), tic, miss: (window.MZLAB_LR_MISS || []).length, loaded: !!(window.LETTORI_MOD && window.LETTORI_MOD.loaded), kept: CONV.files.has("uno.mzML") };
}"""
MZML = """async () => {
  const rb = await window.MZLAB_RUST_READY, f = CONV.files.get("uno.mzML") || await rb.nativeToMzml("uno.mzML", 1 << 20, undefined, undefined);
  const b = new Uint8Array(await f.arrayBuffer()); let s = ""; for (let i = 0; i < b.length; i += 32768) s += String.fromCharCode(...b.subarray(i, i + 32768));
  return btoa(s);
}"""

def opened(pg):
    pg.set_input_files("#pickdir", str(top))
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
    pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)

def page(b, query):
    pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.on("request", lambda r: asked.append(r.url) if "wasm-lettori/" in r.url else None)
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1{query}"); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_function("window.MZLAB_RUST_READY !== undefined", timeout=60000)
    return pg
res = {}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        eng = page(b, ""); mod = page(b, "&lettori=modulo")
        def engine_path():
            opened(eng); r = eng.evaluate(LOAD); assert r["files"] == ["uno.mzML"] and r["native"] and not r["loaded"], r
            res["eng"] = r; res["eng_mzml"] = eng.evaluate(MZML)
        step("without the parameter the engine reads the .D folder and the module is not started", engine_path)
        def module_path():
            opened(mod); r = mod.evaluate(LOAD); assert r["files"] == ["uno.mzML"] and not r["native"] and r["loaded"] and r["kept"], r
            res["mod"] = r; res["mod_mzml"] = mod.evaluate(MZML)
        step("with ?lettori=modulo the module writes the mzML and the file is listed as an mzML", module_path)
        def same_tic():
            assert res["mod"]["tic"] == res["eng"]["tic"] and len(res["mod"]["tic"].get("t", res["mod"]["tic"].get("rt", [1] * N))) > 0, (res["mod"]["tic"], res["eng"]["tic"])
        step("the TIC is the one of the engine's path", same_tic)
        def same_bytes():
            assert res["mod_mzml"] == res["eng_mzml"] and len(res["mod_mzml"]) > 1000, (len(res["mod_mzml"]), len(res["eng_mzml"]))
        step("«Download .mzML» is the engine's mzML byte for byte, without converting again", same_bytes)
        def no_miss():
            assert res["mod"]["miss"] == 0 and res["eng"]["miss"] == 0, (res["mod"]["miss"], res["eng"]["miss"])
        step("MZLAB_LR_MISS is empty", no_miss)
        def one_module():
            fams = {u.split("wasm-lettori/")[1].split("/")[0] for u in asked}
            assert "agilent" in fams and fams <= {"agilent", "manifest.json"}, asked
        step("only the module of the family (agilent) is downloaded, not sciex or thermo", one_module)
        def cancel():
            r = mod.evaluate("""async () => { const c = new AbortController(); c.abort(); try { await LETTORI_MOD.toMzml("agilent", [{ name: "a", blob: new Blob([]) }], 0, "x.mzML", undefined, c.signal); return "no"; } catch (e) { return e.cancelled ? "cancelled" : String(e); } }""")
            assert r == "cancelled", r
        step("a cancelled conversion stops", cancel)
        def lc_runs():
            lc = page(b, "&lettori=modulo")
            lc.set_input_files("#pickdir", str(lc_top))
            lc.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
            r = lc.evaluate("""async () => {
              const rb = await window.MZLAB_RUST_READY, out = {};
              for (const n of ["mrm.mzML", "perfil.mzML"]) {
                const f = CONV.files.get(n) || await rb.nativeToMzml(n, 1 << 20, undefined, undefined);
                out[n] = await f.text();
              }
              return { names: [...document.querySelectorAll("#flist tr")].map(t => (t.innerText.match(/\\S+\\.mzML/) || [""])[0]).filter(Boolean).sort(), mrm: out["mrm.mzML"], perfil: out["perfil.mzML"], miss: (window.MZLAB_LR_MISS || []).length };
            }""")
            assert r["names"] == ["mrm.mzML", "perfil.mzML"] and r["miss"] == 0, (r["names"], r["miss"])
            assert "SRM SIC Q1=301.1 Q3=150.05" in r["mrm"] and "SRM SIC Q1=301.1 Q3=200.05" in r["mrm"], "no SRM chromatograms"
            assert 'MS:1000130' in r["perfil"] and 'profile' in r["perfil"].lower(), "profile spectra, positive"
            assert 'spectrumList count="40"' in r["perfil"], "profile spectra"
        step("a QqQ MRM run and a Q-TOF profile run open through the module (SRM chromatograms, profile spectra)", lc_runs)
        # the numpress mzML: the engine refuses it, so the page sends it to the «aperti» module with no parameter
        NP = """async () => { const rb = await window.MZLAB_RUST_READY; await new Promise(r => setTimeout(r, 300));
          const tic = await (await fetch("api/chrom?k=0&level=1")).json(), sp = await (await fetch("api/spectrum?k=0&rt0=0.9&rt1=1.1&level=1")).json();
          return { files: ST.files.map(x => x.name), tic, sp, miss: (window.MZLAB_LR_MISS || []).length }; }"""
        def one(folder):
            pg = page(b, ""); pg.set_input_files("#pickdir", str(tmp / folder))
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
            return pg.evaluate(NP)
        def numpress_opens():
            res["npa"] = one("npa"); res["npb"] = one("npb")
            assert res["npa"]["files"] == ["np.mzML"] and res["npb"]["files"] == ["np.mzML"], (res["npa"]["files"], res["npb"]["files"])
        step("an mzML with numpress arrays opens (through the «aperti» module) like its zlib twin", numpress_opens)
        def numpress_same():
            a, c = res["npa"], res["npb"]
            ta = a["tic"].get("y"); tb = c["tic"].get("y")
            assert ta and tb and len(ta) == len(tb) == 40, (list(a["tic"])[:6], len(ta or []), len(tb or []))
            assert all(abs(x - y) <= 1e-3 * max(abs(y), 1) for x, y in zip(ta, tb)), (ta[:3], tb[:3])
            assert a["miss"] == 0 and c["miss"] == 0, (a["miss"], c["miss"])
            ka = [k for k in a["sp"] if isinstance(a["sp"][k], list) and len(a["sp"][k]) > 5]
            assert ka and ka == [k for k in c["sp"] if isinstance(c["sp"][k], list) and len(c["sp"][k]) > 5], (list(a["sp"]), list(c["sp"]))
            for k in ka:   # the spectrum: m/z to 1e-7, intensity to the precision of slof (1e-3 relative)
                assert len(a["sp"][k]) == len(c["sp"][k]) and all(abs(x - y) <= 1e-3 * max(abs(y), 1) for x, y in zip(a["sp"][k], c["sp"][k])), k
        step("its TIC equals the twin's within the codec precision and nothing misses the engine", numpress_same)
        def mzxml_opens():
            r = one("xml"); assert r["files"] == ["x.mzML"], r["files"]
            y = r["tic"].get("y"); assert y and len(y) == 40 and all(abs(a - b) <= 1e-4 * b for a, b in zip(y, XTIC)), (y and y[:3], XTIC[:3])
            assert r["miss"] == 0, r["miss"]
            ks = [k for k in r["sp"] if isinstance(r["sp"][k], list) and len(r["sp"][k]) > 5]; assert ks and all(len(r["sp"][k]) == 30 for k in ks), list(r["sp"])
        step("an mzXML opens in the Dati view (through the «aperti» module) with its TIC and a spectrum", mzxml_opens)
        def cdf_opens():
            r = one("cdf"); assert r["files"] == ["c.mzML"], r["files"]
            y = r["tic"].get("y"); assert y and len(y) == 40 and all(abs(a - b) <= 1e-4 * b for a, b in zip(y, CTIC)), (y and y[:3], CTIC[:3])
            assert r["miss"] == 0, r["miss"]
            ks = [k for k in r["sp"] if isinstance(r["sp"][k], list) and len(r["sp"][k]) > 5]; assert ks and all(len(r["sp"][k]) == 30 for k in ks), list(r["sp"])
        step("an ANDI-MS .cdf opens in the Dati view (through the «andi» module) with its TIC and a spectrum", cdf_opens)
        def andi_module():
            fams = {u.split("wasm-lettori/")[1].split("/")[0] for u in asked}
            assert "andi" in fams, asked
        step("the «andi» module was downloaded", andi_module)
        def aperti_module():
            fams = {u.split("wasm-lettori/")[1].split("/")[0] for u in asked}
            assert "aperti" in fams, asked
        step("the «aperti» module was downloaded", aperti_module)
        b.close()
finally:
    srv.terminate()
for s in steps: print("  ", s)
errs = [e for e in errs if "GL Driver" not in e]
print("   errors:", errs[:5] if errs else "none")
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
