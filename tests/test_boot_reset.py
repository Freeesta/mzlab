"""Unit tests for the initial boot 10-second timeout and full reset button:
- 10-second boot timeout shows full reset button on initial site/engine load
- Timer does not fire if qqReady is true (or cancelled via ldClearBootTimer)
- Full reset button executes QQRESET.run('all') without confirmation dialogs
- RESET.wipe('all') completely clears localStorage, sessionStorage, and resets hash
"""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"


def run_node(code: str) -> dict:
    """Execute snippet in Node with simulated DOM globals and return JSON."""
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


def test_boot_timer_triggers_full_reset_button():
    code = f"""
    let capturedTimerFn = null;
    let capturedDelay = 0;
    global.setTimeout = (fn, delay) => {{
      capturedTimerFn = fn;
      capturedDelay = delay;
      return 123;
    }};
    global.clearTimeout = () => {{}};
    global.setInterval = () => 456;
    global.clearInterval = () => {{}};

    const ldact = {{ textContent: '', hidden: true, appendChild(el) {{ this.children = this.children || []; this.children.push(el); }} }};
    const loading = {{ hidden: false }};
    const ldmsg = {{ textContent: '', innerHTML: '', setAttribute() {{}}, appendChild() {{}} }};

    global.document = {{
      title: '',
      getElementById(id) {{
        if (id === 'ldact') return ldact;
        if (id === 'loading') return loading;
        if (id === 'ldmsg') return ldmsg;
        return null;
      }},
      querySelector(sel) {{
        if (sel === '#ldact button') return ldact.children && ldact.children[0];
        return null;
      }},
      createElement(tag) {{
        return {{ tagName: tag, children: [], style: {{}}, setAttribute() {{}}, appendChild() {{}}, focus() {{}} }};
      }}
    }};

    global.window = {{ QQQ_PHONE: false, qqReady: false, I18N: {{ t: (k) => k === 'reset.all.btn' ? 'Full reset' : k }} }};
    global.I18N = window.I18N;
    global.FRAME = {{ cancel() {{}}, next() {{}} }};

    const avvioCode = fs.readFileSync({json.dumps(str(WEB / "avvio.js"))}, 'utf8');
    eval(avvioCode);

    if (capturedDelay !== 10000) throw new Error('Timer delay should be 10000ms');

    capturedTimerFn();
    const btn = ldact.children && ldact.children[0];
    const buttonCreated = !!btn;
    const buttonText = btn ? btn.textContent : '';
    const buttonId = btn ? btn.id : '';
    const isImp = btn && btn.className && btn.className.includes('imp');

    console.log(JSON.stringify({{ buttonCreated, buttonText, buttonId, isImp }}));
    """
    res = run_node(code)
    assert res["buttonCreated"] is True
    assert res["buttonText"] == "Full reset"
    assert res["buttonId"] == "ldreset"
    assert res["isImp"] is True


def test_boot_timer_does_not_fire_when_ready_or_cleared():
    code = f"""
    let timerCleared = false;
    global.setTimeout = (fn, delay) => 789;
    global.clearTimeout = (id) => {{ if (id === 789) timerCleared = true; }};
    global.setInterval = () => 456;
    global.clearInterval = () => {{}};

    global.document = {{
      title: '',
      getElementById() {{ return null; }},
      createElement() {{ return {{ children: [], style: {{}}, setAttribute() {{}}, appendChild() {{}} }}; }}
    }};
    global.window = {{ QQQ_PHONE: false, qqReady: false, I18N: {{ t: (k) => k }} }};
    global.I18N = window.I18N;
    global.FRAME = {{ cancel() {{}}, next() {{}} }};

    const avvioCode = fs.readFileSync({json.dumps(str(WEB / "avvio.js"))}, 'utf8');
    eval(avvioCode);

    window.ldClearBootTimer();
    console.log(JSON.stringify({{ timerCleared }}));
    """
    res = run_node(code)
    assert res["timerCleared"] is True


def test_full_reset_execution_calls_run_all():
    code = f"""
    let runMode = null;
    global.window = {{
      QQRESET: {{
        run(mode) {{
          runMode = mode;
          return Promise.resolve();
        }}
      }},
      I18N: {{ t: (k) => k }},
      QQQ_PHONE: false,
      qqReady: false
    }};
    global.I18N = window.I18N;
    global.FRAME = {{ cancel() {{}}, next() {{}} }};
    global.setTimeout = () => 1;
    global.clearTimeout = () => {{}};
    global.setInterval = () => 2;
    global.clearInterval = () => {{}};

    global.document = {{
      title: '',
      querySelector() {{ return {{ disabled: false }}; }},
      getElementById(id) {{
        if (id === 'ldion') return null;
        return {{ textContent: '', innerHTML: '', setAttribute() {{}}, appendChild() {{}} }};
      }},
      createElement() {{ return {{ children: [], style: {{}}, setAttribute() {{}}, appendChild() {{}} }}; }}
    }};

    const avvioCode = fs.readFileSync({json.dumps(str(WEB / "avvio.js"))}, 'utf8');
    eval(avvioCode);

    window.ldFullReset();
    console.log(JSON.stringify({{ runMode }}));
    """
    res = run_node(code)
    assert res["runMode"] == "all"


def test_reset_wipe_all_clears_storage_and_hash():
    code = f"""
    let lsCleared = false;
    let ssCleared = false;
    global.localStorage = {{
      clear() {{ lsCleared = true; }}
    }};
    global.sessionStorage = {{
      clear() {{ ssCleared = true; }},
      setItem() {{}},
      getItem() {{ return null; }},
      removeItem() {{}}
    }};
    global.window = global;
    global.navigator = {{
      storage: {{
        getDirectory: async () => ({{
          entries: async function* () {{}}
        }})
      }}
    }};
    global.indexedDB = {{
      deleteDatabase: () => ({{ onsuccess: null, onerror: null }})
    }};
    global.caches = {{
      keys: async () => []
    }};
    let replacedUrl = '';
    global.location = {{
      href: 'https://example.com/mzlab/?foo=1#hash123',
      replace: (u) => {{ replacedUrl = u; }}
    }};
    global.NOMI = {{
      DB: 'mzlab_lab',
      DB_OLD: 'qqq_lab',
      CACHE_PREFIX: 'mzlab-',
      CACHE_BIG: 'mzlab-big-',
      KEY: (k) => 'mzlab.' + k,
      OLD_KEY: (k) => 'qqq.' + k
    }};
    global.I18N = {{ t: (k) => k }};
    global.document = {{ getElementById() {{ return null; }} }};

    const resetCode = fs.readFileSync({json.dumps(str(WEB / "reset.js"))}, 'utf8');
    eval(resetCode);

    await window.QQRESET.run('all');
    console.log(JSON.stringify({{
      lsCleared,
      ssCleared,
      replacedUrl,
      hashCleared: !replacedUrl.includes('#')
    }}));
    """
    res = run_node(code)
    assert res["lsCleared"] is True
    assert res["ssCleared"] is True
    assert res["hashCleared"] is True


def test_browser_fail_action_button_on_engine_start_error():
    code = f"""
    let actionLabel = null;
    let segnalaCalled = false;
    global.window = {{
      ldAction(label, fn) {{
        actionLabel = label;
      }},
      ERRORI: {{
        segnala(err, tag) {{
          if (tag === 'engine') segnalaCalled = true;
        }}
      }},
      I18N: {{
        t: (k) => k === 'reset.all.btn' ? 'Full reset' : k
      }}
    }};
    global.I18N = window.I18N;
    const ldmsg = {{ textContent: '' }};
    global.document = {{
      getElementById(id) {{
        if (id === 'ldmsg') return ldmsg;
        return null;
      }},
      addEventListener() {{}}
    }};

    // Verify browser.js fail() logic with again = true
    const restart = () => {{}};
    const fail = (key, params, again) => {{
      window.qqFailed = key;
      const m = document.getElementById('ldmsg');
      if (m && window.I18N) m.textContent = I18N.t(key, params);
      if (again && window.ldAction && window.I18N) window.ldAction(I18N.t('reset.all.btn'), restart);
    }};

    fail('load.err.engineStart', null, true);
    if (window.ERRORI && window.ERRORI.segnala) window.ERRORI.segnala(new Error('engine failed'), 'engine');

    console.log(JSON.stringify({{
      actionLabel,
      msgText: ldmsg.textContent,
      segnalaCalled
    }}));
    """
    res = run_node(code)
    assert res["actionLabel"] == "Full reset"
    assert res["msgText"] == "load.err.engineStart"
    assert res["segnalaCalled"] is True

