"""The text of the «Report a bug» dialog (mzlab/web/bug-report-testo.js, run by node): no file name or label in the diagnostics, errors cut to one line, the link of the new issue
(form bug.yml, label bug, at most 6000 characters), the translation keys."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
pytestmark = pytest.mark.skipif(not shutil.which("node"), reason="node missing")


def nodo(js: str):
    src = f"import * as B from {(ROOT / 'mzlab/web/bug-report-testo.js').as_uri()!r};process.stdout.write(JSON.stringify({js}))"
    r = subprocess.run(["node", "--input-type=module", "-e", src], capture_output=True, text=True, check=True)
    return json.loads(r.stdout)


def test_anonimizza_toglie_nomi_e_file():
    out = nodo("B.anonimizza('Cannot read Paziente_Rossi_t0.mzML and rossi_t0.raw\\nsecond line', ['Paziente_Rossi_t0.mzML','Campione Rossi'])")
    assert "Rossi" not in out and ".mzML" not in out and ".raw" not in out and "second" not in out and "<file>" in out
    assert nodo("B.anonimizza('x'.repeat(500), [], s=>s, 50).length") == 51


def test_riassunto_file_senza_nomi():
    out = nodo("B.riassuntoFile([{file:'Segreto_Mario.mzML',size:2097152},{file:'altro.wiff.scan#2',size:512,format:'wiff'}])")
    assert out == "2 (mzML 2.0 MB, wiff 512 B)" and "Segreto" not in out


def test_rapporto_e_diagnostica():
    out = nodo("(()=>{const d=B.righeDiagnostica({version:'1.0.1',commit:'abc',date:'D',ua:'UA',mode:'web',view:'data (#/dati)',files:'0',sw:'none',errors:[]});return B.rapporto({what:'boom',expected:'',diag:d},k=>k)})()")
    assert "## bug.r.what\n\nboom" in out and "bug.r.none" in out and "- Mode: web" in out and "- Recent errors: none" in out
    assert "## bug.r.diag" not in nodo("B.rapporto({what:'a',expected:'b',diag:null},k=>k)")


def test_url_issue_attesa_e_limite():
    u = nodo("B.urlIssue({title:'[Bug] x',body:'hello',ambiente:'UA',cut:'CUT'})")
    assert u["url"].startswith("https://github.com/Freeesta/mzlab/issues/new?template=bug.yml&labels=bug&title=%5BBug%5D%20x&ambiente=UA&cosa=hello") and not u["troncato"]
    big = nodo("B.urlIssue({title:'t',body:'à€😀'.repeat(3000),ambiente:'UA',cut:'CUT'})")
    assert big["troncato"] and len(big["url"]) <= 6000 and big["url"].endswith("CUT")


def test_titolo():
    assert nodo("B.titolo('', 'vuoto')") == "[Bug] vuoto"
    assert nodo("B.titolo('linea uno\\nlinea due', 'v')") == "[Bug] linea uno"
    assert len(nodo("B.titolo('parola '.repeat(40), 'v')")) < 90
