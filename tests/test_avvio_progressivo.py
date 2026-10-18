"""Unit tests for the AVVIO progressive startup state machine and upload gate (PR 1):
- pending -> ready with 0, 1, and 3 uploads in waiting queue (strict execution order)
- pending -> failed (uploads resolve false, no unhandled exceptions, error shown once)
- double segnaPronto (idempotent transition)
- segnaFallito after ready is ignored (state transition locked)
- annulla from waiting state (cancels queue, resolves false)
- duplicate file while waiting -> single upload (via dropDups and INFLIGHT logic)
"""
import json
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


def test_duplicate_file_while_waiting_single_upload():
    code = f"""
    const AVVIO = require('{WEB / "avvio-progressivo.js"}');
    AVVIO._resetTest('pending');

    // Simulate shell.js deduplication logic with INFLIGHT, ST.files and dropDups
    const INFLIGHT = new Set();
    const ST_files = [];
    const fkey = f => f.name + "|" + f.size;
    function dropDups(files) {{
      const dups = files.filter(f => INFLIGHT.has(fkey(f)) || ST_files.some(g => g.name === f.name && g.size === f.size));
      return files.filter(f => !dups.includes(f));
    }}

    const processed = [];
    async function upload(files) {{
      if (!(await AVVIO.pronto())) return;
      files = dropDups(files);
      if (!files.length) return;
      files.forEach(f => INFLIGHT.add(fkey(f)));
      try {{
        // simulate async upload0 (sendFile)
        await new Promise(r => setTimeout(r, 10));
        for (const f of files) {{
          processed.push(f.name);
          ST_files.push(f);
        }}
      }} finally {{
        files.forEach(f => INFLIGHT.delete(fkey(f)));
      }}
    }}

    const fileA = {{ name: 'sample.mzML', size: 1024 }};
    const fileA_dup = {{ name: 'sample.mzML', size: 1024 }};

    // Two drops of the same file while pending
    const call1 = upload([fileA]);
    const call2 = upload([fileA_dup]);

    AVVIO.segnaPronto();
    await Promise.all([call1, call2]);

    console.log(JSON.stringify({{
      processed
    }}));
    """
    res = run_node(code)
    # The first call processes sample.mzML; the second call finds sample.mzML in INFLIGHT/seen and is dropped
    assert res["processed"] == ["sample.mzML"]
