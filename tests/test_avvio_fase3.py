"""Phase 3 of the progressive start: a failed engine start shows a banner (role=alert) and leaves the page usable.
Node with a fake DOM: banner text == AVVIO.errore, «Copy details» without file names, «Full reset», skeleton removed,
a real message with a full stop is not taken for a catalog key, a failure that happened before the module loaded is replayed."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"

DOM = """
function makeEl(id) {
  let text = '', children = [], attrs = {};
  const el = { id, hidden: false, onclick: null, type: '', className: '', disabled: false,
    get textContent() { return text + children.map(c => c.textContent).join(''); },
    set textContent(v) { text = v; children = []; },
    appendChild(c) { children.push(c); reg[c.id] = c; },
    setAttribute(k, v) { attrs[k] = v; }, getAttribute(k) { return attrs[k]; },
    get children() { return children; } };
  return el;
}
const reg = {};
['drop', 'drop-skeleton', 'avvio-fase', 'avvio-drop-info'].forEach(i => { reg[i] = makeEl(i); });
reg['drop-skeleton'].hidden = false; reg['drop'].hidden = true;
const body = makeEl('body'); body.insertBefore = (el) => { reg[el.id] = el; body.kids = (body.kids || 0) + 1; };
global.document = { body, readyState: 'complete', addEventListener() {}, getElementById: id => reg[id] || null, createElement: () => makeEl('') };
global.window = global;
Object.defineProperty(global, 'navigator', { value: { userAgent: 'test-agent', clipboard: { writeText: async t => { global.copied = t; } } }, configurable: true });
global.I18N = { t: (k) => ({ 'load.err.engineStart': 'Impossibile avviare il motore', 'load.fail.copy': 'Copia dettagli', 'reset.all.btn': 'Reset completo' })[k] || k };
"""


def run_node(code: str) -> dict:
    full = f"(async () => {{ {code} }})().catch(e => {{ console.error(e); process.exit(1); }});"
    r = subprocess.run(["node", "-e", full], capture_output=True, text=True)
    assert r.returncode == 0, f"Node failed:\n{r.stdout}\n{r.stderr}"
    return json.loads([ln for ln in r.stdout.strip().splitlines() if ln.strip()][-1])


def test_banner_alert_same_text_as_errore_and_skeleton_removed():
    res = run_node(f"""
    {DOM}
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');
    const pA = AVVIO.pronto();
    AVVIO.segnaFallito({{ key: 'load.err.engineStart' }});
    AVVIO.segnaFallito({{ key: 'other.key' }});   // a second failure is ignored: one banner, one message
    const b = reg['avvio-errore'];
    console.log(JSON.stringify({{ role: b.getAttribute('role'), msg: reg['avvio-errore-msg'].textContent, errore: AVVIO.errore,
      shown: body.kids, copia: reg['avvio-copia'].textContent, reset: reg['avvio-reset'].textContent,
      skelHidden: reg['drop-skeleton'].hidden, dropHidden: reg['drop'].hidden, faseHidden: reg['avvio-fase'].hidden,
      waiting: await pA, after: await AVVIO.pronto() }}));
    """)
    assert res["role"] == "alert"
    assert res["msg"] == res["errore"] == "Impossibile avviare il motore"
    assert res["shown"] == 1
    assert res["copia"] == "Copia dettagli" and res["reset"] == "Reset completo"
    assert res["skelHidden"] is True and res["dropHidden"] is False and res["faseHidden"] is True
    assert res["waiting"] is False and res["after"] is False


def test_copy_details_has_no_paths_or_file_names_and_reset_runs():
    res = run_node(f"""
    {DOM}
    global.MZLAB_LR_ERR = 'cannot open /Users/anna/Documents/campione_riservato.mzML: boom';
    let hard = 0; global.qqHardReset = () => {{ hard++; }};
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');
    AVVIO.segnaFallito({{ key: 'load.err.engineStart' }});
    await reg['avvio-copia'].onclick();
    reg['avvio-reset'].onclick();
    console.log(JSON.stringify({{ copied: global.copied, hard, resetDisabled: reg['avvio-reset'].disabled }}));
    """)
    assert "engine:" in res["copied"] and "load.err.engineStart" in res["copied"]
    assert "/Users" not in res["copied"] and "anna" not in res["copied"] and "campione_riservato" not in res["copied"]
    assert res["hard"] == 1 and res["resetDisabled"] is True


def test_real_message_with_full_stop_is_not_a_key_and_unknown_key_is_generic():
    res = run_node(f"""
    {DOM}
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');
    AVVIO.segnaFallito('Worker did not start. Reload the page.');
    const real = AVVIO.errore;
    AVVIO._resetTest('pending');
    AVVIO.segnaFallito({{ key: 'load.err.notInCatalog' }});
    const unknown = AVVIO.errore;
    AVVIO._resetTest('pending');
    global.I18N = null;
    AVVIO.segnaFallito('load.err.engineStart');
    console.log(JSON.stringify({{ real, unknown, noCatalog: AVVIO.errore }}));
    """)
    assert res["real"] == "Worker did not start. Reload the page."
    assert res["unknown"] == res["noCatalog"] == "Calculation engine startup failed"


def test_failure_before_the_module_loaded_is_replayed():
    res = run_node(f"""
    {DOM}
    global.QQQ_BROWSER = true; global.qqFailInfo = {{ key: 'load.err.engineStart' }};
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    console.log(JSON.stringify({{ stato: AVVIO.stato, msg: reg['avvio-errore-msg'] && reg['avvio-errore-msg'].textContent }}));
    """)
    assert res["stato"] == "failed" and res["msg"] == "Impossibile avviare il motore"
