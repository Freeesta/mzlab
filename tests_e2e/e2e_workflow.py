"""Test e2e for reproducibility (.mzflow) and MGF / MSP spectral export."""
# covers: mzlab/web/campioni.js, mzlab/web/campioni-doc.js, mzlab/web/rapporto.js, mzlab/web/workflow.js, mzlab/web/gradiente.js, mzlab/web/fair.js
# covers: mzlab/web/version.js
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

r = Run(port=8997, wd="/tmp/wd_workflow")
steps = []

F_LR = str(ROOT / "mzlab" / "web" / "esempi" / "FullScan_t15.mzML")

try:
    with sync_playwright() as p:
        pg = r.page(p)

        # 1. Load the LR example file
        pg.set_input_files("#pick", [F_LR])
        pg.wait_for_timeout(800)
        pg.click("text=Load data")
        ready(pg)

        # 2. Check the SHA-256 computation in the worker
        sha = pg.evaluate("() => E.files && E.files[0] ? E.files[0].sha256 : ''")
        assert len(sha) == 64, f"SHA-256 invalid or empty: '{sha}'"
        steps.append(("sha256 computed while the file loads", "ok"))

        # 3. Change session parameters
        pg.evaluate("""() => {
            UIP.hrPpm = 8;
            UIP.hrDec = 5;
            UIP.theme = 'dark';
            if (typeof uipSave === 'function') uipSave();
        }""")

        # 4. Export .mzflow and check its content
        with pg.expect_download() as d_info:
            pg.evaluate("() => WORKFLOW.export()")
        download = d_info.value
        wf_path = download.path()
        with open(wf_path, "r", encoding="utf-8") as fh:
            wf_data = json.load(fh)

        assert wf_data.get("formato") == "mzflow/1", f"Wrong format: {wf_data.get('formato')}"
        assert wf_data["parametri"]["ppm"] == 8, f"PPM not exported: {wf_data['parametri']}"
        assert wf_data["parametri"]["dec"] == 5, f"Dec not exported: {wf_data['parametri']}"
        assert wf_data["parametri"]["tema"] == "dark", f"Theme not exported: {wf_data['parametri']}"
        assert len(wf_data["file"]) == 1
        assert wf_data["file"][0]["sha256"] == sha
        assert "programma" in wf_data and "versione" in wf_data["programma"]
        assert len(wf_data.get("operazioni", [])) >= 1
        steps.append(("export of .mzflow with sha256 and parameters", "ok"))

        # 5. Restore the parameters and re-apply the workflow
        pg.evaluate("""() => {
            UIP.hrPpm = 5;
            UIP.hrDec = 4;
            UIP.theme = 'light';
            if (typeof uipSave === 'function') uipSave();
        }""")
        assert pg.evaluate("() => UIP.hrPpm") == 5

        # Reopen and apply the exported workflow
        applied = pg.evaluate("(wf) => WORKFLOW.applyWorkflow(wf, 'test.mzflow')", wf_data)
        assert applied is True
        assert pg.evaluate("() => UIP.hrPpm") == 8
        assert pg.evaluate("() => UIP.hrDec") == 5
        assert pg.evaluate("() => UIP.theme") == "dark"
        steps.append(("reopening .mzflow and restoring parameters", "ok"))

        # 6. Test MGF and MSP formatting in LR (no annotations/names - Principle 1)
        mgf_out = pg.evaluate("""() => {
            const spec = {
                sid: 42,
                rt: 3.1415,
                prec: 285.1234,
                charge: 1,
                polarity: 1,
                file: 'campione.mzML',
                peaks: [[120.08, 1500.0], [285.12, 10000.0]],
                title_extra: 'SECRET_COMPOUND'
            };
            return WORKFLOW.formatMgf([spec], true);
        }""")
        assert "BEGIN IONS" in mgf_out
        assert "TITLE=scan=42 rt=3.1415 file=campione.mzML" in mgf_out
        assert "SECRET_COMPOUND" not in mgf_out, "Principle 1 violated in LR MGF"
        assert "PEPMASS=285.1234" in mgf_out
        assert "CHARGE=1+" in mgf_out
        assert "120.08000 1500.0" in mgf_out
        assert "END IONS" in mgf_out

        msp_out = pg.evaluate("""() => {
            const spec = {
                sid: 42,
                rt: 3.1415,
                prec: 285.1234,
                charge: 1,
                polarity: 1,
                file: 'campione.mzML',
                peaks: [[120.08, 1500.0], [285.12, 10000.0]],
                name_extra: 'SECRET_COMPOUND'
            };
            return WORKFLOW.formatMsp([spec], true);
        }""")
        assert "NAME: campione.mzML_scan_42" in msp_out
        assert "SECRET_COMPOUND" not in msp_out, "Principle 1 violated in LR MSP"
        assert "PRECURSORMZ: 285.1234" in msp_out
        assert "PRECURSORTYPE: [M+H]+" in msp_out
        assert "IONMODE: Positive" in msp_out
        assert "Num Peaks: 2" in msp_out
        steps.append(("MGF and MSP formatting follows Principle 1 (LR)", "ok"))

        # 7. No entry in the interface (the function stays in the code); the old name is still read
        assert pg.query_selector("#session-menu-btn") is None, "the session menu arrow is gone"
        old = dict(wf_data, formato="mzworkflow/1")
        assert pg.evaluate("(wf) => WORKFLOW.applyWorkflow(wf, 'vecchio.mzworkflow')", old) is True
        steps.append(("reading of the old mzworkflow/1 format", "ok"))
        # 8. Unknown operations: ignored (with a notice), the rest is applied
        fut = dict(wf_data, operazioni=[{"op": "export_report", "t": "x"}, {"op": "operazione_futura", "t": "y"}])
        assert pg.evaluate("(wf) => WORKFLOW.applyWorkflow(wf, 'f.mzflow')", fut) is True
        ops = pg.evaluate("() => NB.ops.map(o => o.op)")
        assert "operazione_futura" not in ops and "export_report" in ops, ops
        steps.append(("unknown operations ignored with a notice", "ok"))

        print("OK: all checks passed")
        for desc, res in steps:
            print(f"  ✓ {desc}: {res}")

finally:
    r.close()
