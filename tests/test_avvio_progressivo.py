"""Unit and DOM tests for the AVVIO progressive startup state machine and upload gate (Fase 1):
- pending -> ready with 0, 1, and 3 uploads in waiting queue (strict execution order)
- pending -> failed (uploads resolve false, no unhandled exceptions, error shown once)
- double segnaPronto (idempotent transition)
- segnaFallito after ready is ignored (state transition locked)
- annulla from waiting state (cancels queue, resolves false)
- segnaFallito translates catalog key with I18N.t before window.fail (never raw key in #err)
- renderDropStato DOM test with and without parameters (0, 1, 3 files, cancel click, ready/failed)
- duplicate file while waiting via REAL upload and dropDups from mzlab/web/shell.js
"""
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"


def run_node(code: str) -> dict:
    """Execute snippet in Node and return JSON parsed output."""
    full_code = f"""
    const fs = require('fs');
    (async () => {{
      {code}
    }})().catch(err => {{
      console.error(err);
      process.exit(1);
    }});
    """
    r = subprocess.run(["node", "-e", full_code], capture_output=True, text=True)
    assert r.returncode == 0, f"Node script failed:\nSTDOUT: {r.stdout}\nSTDERR: {r.stderr}"
    lines = [line.strip() for line in r.stdout.strip().splitlines() if line.strip()]
    return json.loads(lines[-1])


def test_pending_to_ready_0_1_3_uploads_order():
    code = f"""
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    
    // 1) 0 uploads
    AVVIO._resetTest('pending');
    let st0_before = AVVIO.stato;
    let inAttesa0 = AVVIO.inAttesa;
    AVVIO.segnaPronto();
    let st0_after = AVVIO.stato;

    // 2) 1 upload
    AVVIO._resetTest('pending');
    let p1_done = false;
    let p1 = AVVIO.pronto().then(res => {{ p1_done = res; }});
    let inAttesa1 = AVVIO.inAttesa;
    AVVIO.segnaPronto();
    await p1;

    // 3) 3 uploads with strict order preservation
    AVVIO._resetTest('pending');
    const order = [];
    const pA = AVVIO.pronto().then(res => {{ order.push(['A', res]); }});
    const pB = AVVIO.pronto().then(res => {{ order.push(['B', res]); }});
    const pC = AVVIO.pronto().then(res => {{ order.push(['C', res]); }});
    let inAttesa3 = AVVIO.inAttesa;
    AVVIO.segnaPronto();
    await Promise.all([pA, pB, pC]);

    console.log(JSON.stringify({{
      st0_before, st0_after, inAttesa0,
      inAttesa1, p1_done,
      inAttesa3,
      order,
      final_inAttesa: AVVIO.inAttesa
    }}));
    """
    res = run_node(code)
    assert res["st0_before"] == "pending"
    assert res["st0_after"] == "ready"
    assert res["inAttesa0"] == 0
    assert res["inAttesa1"] == 1
    assert res["p1_done"] is True
    assert res["inAttesa3"] == 3
    assert res["order"] == [["A", True], ["B", True], ["C", True]]
    assert res["final_inAttesa"] == 0


def test_pending_to_failed_no_exceptions_resolves_false():
    code = f"""
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');

    let failReported = [];
    global.window = {{
      fail: (err) => {{ failReported.push(err); }}
    }};

    let outA, outB;
    const pA = AVVIO.pronto().then(res => {{ outA = res; }});
    const pB = AVVIO.pronto().then(res => {{ outB = res; }});

    AVVIO.segnaFallito('boom');
    await Promise.all([pA, pB]);

    // Another pronto call after failed state
    const pC = await AVVIO.pronto();

    console.log(JSON.stringify({{
      stato: AVVIO.stato,
      errore: AVVIO.errore,
      outA,
      outB,
      outC: pC,
      failReported,
      inAttesa: AVVIO.inAttesa
    }}));
    """
    res = run_node(code)
    assert res["stato"] == "failed"
    assert res["errore"] == "boom"
    assert res["outA"] is False
    assert res["outB"] is False
    assert res["outC"] is False
    assert res["failReported"] == ["boom"]
    assert res["inAttesa"] == 0


def test_double_segna_pronto_idempotent():
    code = f"""
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');

    AVVIO.segnaPronto();
    const st1 = AVVIO.stato;

    // Second call
    AVVIO.segnaPronto();
    const st2 = AVVIO.stato;

    console.log(JSON.stringify({{ st1, st2 }}));
    """
    res = run_node(code)
    assert res["st1"] == "ready"
    assert res["st2"] == "ready"


def test_segna_fallito_after_ready_ignored():
    code = f"""
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');

    AVVIO.segnaPronto();
    assertState = AVVIO.stato;

    // Call segnaFallito after ready
    AVVIO.segnaFallito('late-error');

    console.log(JSON.stringify({{
      initial: assertState,
      after_failed_call: AVVIO.stato,
      errore: AVVIO.errore
    }}));
    """
    res = run_node(code)
    assert res["initial"] == "ready"
    assert res["after_failed_call"] == "ready"
    assert res["errore"] is None


def test_annulla_cancels_waiting_uploads():
    code = f"""
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');

    let out1 = null;
    let out2 = null;
    const p1 = AVVIO.pronto().then(res => {{ out1 = res; }});
    const p2 = AVVIO.pronto().then(res => {{ out2 = res; }});

    const inAttesaBefore = AVVIO.inAttesa;
    AVVIO.annulla();
    await Promise.all([p1, p2]);

    console.log(JSON.stringify({{
      inAttesaBefore,
      inAttesaAfter: AVVIO.inAttesa,
      out1,
      out2,
      stato: AVVIO.stato
    }}));
    """
    res = run_node(code)
    assert res["inAttesaBefore"] == 2
    assert res["inAttesaAfter"] == 0
    assert res["out1"] is False
    assert res["out2"] is False
    assert res["stato"] == "pending"


def test_segna_fallito_translates_catalog_key_before_fail():
    code = f"""
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');

    let failedArg = null;
    global.window = {{
      fail: (arg) => {{ failedArg = arg; }},
      I18N: {{
        t: (k, p) => k === 'load.err.engineStart' ? 'Il motore non risponde' : k
      }}
    }};
    global.I18N = global.window.I18N;

    AVVIO.segnaFallito('load.err.engineStart');

    console.log(JSON.stringify({{
      failedArg,
      errore: AVVIO.errore
    }}));
    """
    res = run_node(code)
    assert res["failedArg"] == "Il motore non risponde"
    assert res["errore"] == "Il motore non risponde"


DOM_HELPERS = """
function makeEl(id) {
  let text = '', hidden = false, children = [];
  return {
    id,
    get textContent() { return text + children.map(c => c.textContent).join(''); },
    set textContent(v) { text = v; children = []; },
    get hidden() { return hidden; },
    set hidden(v) { hidden = v; },
    appendChild(c) { children.push(c); },
    get children() { return children; },
    onclick: null, type: '', className: ''
  };
}
const infoEl = makeEl('avvio-drop-info');
global.document = {
  getElementById: (id) => id === 'avvio-drop-info' ? infoEl : null,
  createElement: () => makeEl()
};
global.window = global;
global.I18N = {
  t: (k, p) => {
    if (k === 'load.engine.preparing') return 'Preparazione del motore di calcolo…';
    if (k === 'load.engine.waitingOne') return '1 file in attesa';
    if (k === 'load.engine.waiting') return p.n + ' file in attesa';
    if (k === 'load.engine.cancel') return 'Annulla';
    return k;
  }
};
"""


def test_render_drop_stato_coda_e_annulla_dom():
    """DOM test of renderDropStato queue counts with parameters and cancel button."""
    code = f"""
    {DOM_HELPERS}
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');

    // 0 file (senza parametri)
    AVVIO._resetTest('pending');
    AVVIO.renderDropStato();
    const txt0 = infoEl.textContent, hidden0 = infoEl.hidden;

    // 1 file (n = 1)
    let ok1 = null;
    const p1 = AVVIO.pronto().then(r => {{ ok1 = r; }});
    const txt1 = infoEl.textContent;
    const btn1 = infoEl.children.find(c => c.id === 'avvio-annulla');

    // 3 file (n = 3)
    let ok2 = null, ok3 = null;
    const p2 = AVVIO.pronto().then(r => {{ ok2 = r; }});
    const p3 = AVVIO.pronto().then(r => {{ ok3 = r; }});
    const txt3 = infoEl.textContent;
    const btn3 = infoEl.children.find(c => c.id === 'avvio-annulla');

    // Clic annulla
    btn3.onclick();
    await Promise.all([p1, p2, p3]);
    const txtCancel = infoEl.textContent;
    const btnCancel = infoEl.children.find(c => c.id === 'avvio-annulla');

    console.log(JSON.stringify({{
      txt0, hidden0, txt1, hasBtn1: !!btn1, ok1, txt3, hasBtn3: !!btn3, ok2, ok3,
      txtCancel, hasBtnCancel: !!btnCancel
    }}));
    """
    res = run_node(code)
    assert res["hidden0"] is False and "Preparazione del motore di calcolo…" in res["txt0"]
    assert res["hasBtn1"] is True and "1 file in attesa" in res["txt1"]
    assert "3 file in attesa" in res["txt3"]
    assert res["ok1"] is False and res["ok2"] is False and res["ok3"] is False
    assert res["hasBtnCancel"] is False and "Preparazione del motore di calcolo…" in res["txtCancel"]


def test_render_drop_stato_transizioni_e_fallback_dom():
    """DOM test of renderDropStato state transitions and fallback without I18N."""
    code = f"""
    {DOM_HELPERS}
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');

    // Transizione ready
    AVVIO._resetTest('pending');
    AVVIO.renderDropStato();
    AVVIO.segnaPronto();
    const hiddenReady = infoEl.hidden, txtReady = infoEl.textContent;

    // Transizione failed
    AVVIO._resetTest('pending');
    AVVIO.renderDropStato();
    AVVIO.segnaFallito('load.err.engineStart');
    const hiddenFail = infoEl.hidden, txtFail = infoEl.textContent;

    // Senza I18N (fallback)
    global.window.I18N = null;
    AVVIO._resetTest('pending');
    AVVIO.renderDropStato();
    const txtFallback0 = infoEl.textContent;
    AVVIO.pronto();
    const txtFallback1 = infoEl.textContent;

    console.log(JSON.stringify({{
      hiddenReady, txtReady, hiddenFail, txtFail, txtFallback0, txtFallback1
    }}));
    """
    res = run_node(code)
    assert res["hiddenReady"] is True and res["txtReady"] == ""
    assert res["hiddenFail"] is True and res["txtFail"] == ""
    assert res["txtFallback0"] == "load.engine.preparing"
    assert "load.engine.waitingOne" in res["txtFallback1"]


def test_duplicate_file_while_waiting_real_upload_drop_dups():
    """Test duplicate files while pending and after ready via real code extracted from shell.js."""
    shell_src = (WEB / "shell.js").read_text(encoding="utf-8")
    m = re.search(r"const INFLIGHT=new Set\(\)[\s\S]*?async function upload\(files,into\)\{[\s\S]*?finally\{[\s\S]*?\}\}\}", shell_src)
    assert m is not None, "Could not extract upload and dropDups from shell.js"

    code = f"""
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');

    global.window = global;
    global.I18N = {{ t: (k, p) => k }};
    global.document = {{
      getElementById: () => null,
      querySelector: () => ({{ appendChild: () => {{}} }}),
      querySelectorAll: () => []
    }};
    const $ = s => document.querySelector(s);
    const ST = {{ files: [] }};
    const CartCarica = {{ note: () => {{}} }};
    let UPBUSY = 0;
    const CONVROWS = {{ clearDone: () => {{}}, paint: () => {{}}, onCancel: null }};
    function checkGo() {{}}
    function fail() {{}}
    const IDLE = [];
    const processed = [];
    async function upload0(files, into, box, signal) {{
      await new Promise(r => setTimeout(r, 10));
      for (const f of files) {{
        processed.push(f.name);
        ST.files.push(f);
      }}
    }}

    {m.group(0)}

    AVVIO._resetTest('pending');
    const fileA = {{ name: 'sample.mzML', size: 1024 }};
    const fileA_dup = {{ name: 'sample.mzML', size: 1024 }};

    const c1 = upload([fileA]);
    const c2 = upload([fileA_dup]);

    AVVIO.segnaPronto();
    await Promise.all([c1, c2]);

    const c3 = upload([fileA_dup]);
    await c3;

    console.log(JSON.stringify({{ processed, inflight: INFLIGHT.size, st_files: ST.files.length }}));
    """
    res = run_node(code)
    assert res["processed"] == ["sample.mzML"]
    assert res["inflight"] == 0
    assert res["st_files"] == 1


def test_segna_fallito_object_and_fallback_without_i18n():
    code = f"""
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');
    global.window = {{
      fail: () => {{}},
      I18N: {{ t: (k, p) => `translated: ${{k}} (${{p.p}})` }}
    }};
    AVVIO.segnaFallito({{ key: 'load.err.code', params: {{ p: 'abc' }} }});
    const withI18N = AVVIO.errore;

    AVVIO._resetTest('pending');
    global.window.I18N = null;
    AVVIO.segnaFallito({{ key: 'load.err.code' }});
    const withoutI18N = AVVIO.errore;

    console.log(JSON.stringify({{ withI18N, withoutI18N }}));
    """
    res = run_node(code)
    assert res["withI18N"] == "translated: load.err.code (abc)"
    assert res["withoutI18N"] == "Calculation engine startup failed"


def test_render_fase_dom_and_sessione_scheletro():
    code = f"""
    {DOM_HELPERS}
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    const faseEl = makeEl('avvio-fase');
    const dropEl = makeEl('drop');
    const skelEl = makeEl('drop-skeleton');
    const store = {{}};
    global.window.localStorage = {{
      getItem: k => store[k] || null,
      setItem: (k, v) => {{ store[k] = String(v); }},
      removeItem: k => {{ delete store[k]; }}
    }};
    global.document.getElementById = id => {{
      if (id === 'avvio-fase') return faseEl;
      if (id === 'drop') return dropEl;
      if (id === 'drop-skeleton') return skelEl;
      return null;
    }};

    // 1) renderFase con phase tracking e percentuale
    AVVIO._resetTest('pending');
    global.window.qqPhaseCur = {{ key: 'load.ph.index', pct: 45.6 }};
    AVVIO.renderFase();
    const faseTxt = faseEl.textContent;
    const faseHidden = faseEl.hidden;

    // Ready nasconde la linea di stato
    AVVIO.segnaPronto();
    const faseAfterReady = faseEl.hidden;

    // 2) Indizio di sessione e scheletro
    AVVIO.segnaSessione(true);
    const haSess1 = AVVIO.haSessione();
    AVVIO.applicaIndizioSessione();
    const dropHidden1 = dropEl.hidden, skelHidden1 = skelEl.hidden;

    AVVIO.rimuoviScheletro();
    const dropHidden2 = dropEl.hidden, skelHidden2 = skelEl.hidden;

    AVVIO.segnaSessione(false);
    const haSess2 = AVVIO.haSessione();

    console.log(JSON.stringify({{
      faseTxt, faseHidden, faseAfterReady,
      haSess1, dropHidden1, skelHidden1,
      dropHidden2, skelHidden2, haSess2
    }}));
    """
    res = run_node(code)
    assert "46%" in res["faseTxt"] and not res["faseHidden"]
    assert res["faseAfterReady"] is True
    assert res["haSess1"] is True and res["dropHidden1"] is True and res["skelHidden1"] is False
    assert res["dropHidden2"] is False and res["skelHidden2"] is True
    assert res["haSess2"] is False

