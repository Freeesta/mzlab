"""The text of the «Report a bug» and «Suggest an improvement» groups of the settings (mzlab/web/bug-report-testo.js, run by node): no file name, sample name or path in any field or in the JSON,
errors cut to one line with their stack frames reduced to site scripts, the link of the new issue (form bug.yml / suggestion.yml, label, at most 6000 characters)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
pytestmark = pytest.mark.skipif(not shutil.which("node"), reason="node missing")


def node(js: str):
    src = f"import * as B from {(ROOT / 'mzlab/web/bug-report-testo.js').as_uri()!r};process.stdout.write(JSON.stringify({js}))"
    r = subprocess.run(["node", "--input-type=module", "-e", src], capture_output=True, text=True, encoding="utf-8", check=True)
    return json.loads(r.stdout)


def test_anonymize_strips_names_and_files():
    out = node("B.anonimizza('Cannot read Paziente_Rossi_t0.mzML and rossi_t0.raw\\nsecond line', ['Paziente_Rossi_t0.mzML','Campione Rossi'])")
    assert "Rossi" not in out and ".mzML" not in out and ".raw" not in out and "second" not in out and "<file>" in out
    assert node("B.anonimizza('x'.repeat(500), [], s=>s, 50).length") == 51


def test_file_summary_without_names():
    out = node("B.riassuntoFile([{file:'Segreto_Mario.mzML',size:2097152},{file:'altro.wiff.scan#2',size:512,format:'wiff'},{file:'x.mzML',size:3}])")
    assert out == "3 (mzML ×2, wiff ×1; total <10 MB)" and "Segreto" not in out
    assert node("B.riassuntoFile([])") == "0" and node("B.classeDimensione(2e9)") == ">1 GB"


SNAP = ("{version:'1.0.0',commit:'abc',date:'D',ua:'UA',mode:'web',view:'data (#/data)',lang:'en',screen:{sw:1920,sh:1080,iw:1500,ih:900,dpr:2},memory:8,cores:4,isolated:true,opfs:true,"
        "engine:'wasm ready, engine 1.0.0',tool:'spec',cards:'spec ×2',files:B.riassuntoFile([{file:'Paziente_Rossi_t0.mzML',size:5e6}]),settings:['theme'],sw:'active',storage:'5 settings entries',"
        "errors:[{where:'error',message:'Cannot read Paziente_Rossi_t0.mzML in /home/mrossi/dati/x.raw',stack:'Error: x\\n    at foo (https://h/mzlab/app.js?v=3&n=Rossi:12:5)\\n    at bar (https://h/Paziente_Rossi_t0.mzML:1:2)\\nbaz@https://h/a/b.js#frag:3:4'}],"
        "actions:['#load','#Rossi_t0-open','button[data-v=draw]']}")


def test_report_and_diagnostics():
    out = node(f"(()=>{{const d=B.campiDiagnostica({SNAP},['Paziente_Rossi_t0.mzML']);return B.rapporto({{what:'boom',diag:B.righe(d)}},k=>k)}})()")
    assert "## bug.r.what\n\nboom" in out and "## bug.r.diag" in out and "- mzLab: 1.0.0 (abc)" in out and "- Language: en" in out
    assert "- Screen and window: screen 1920x1080, window 1500x900, pixel ratio 2" in out and "- Device: 8 GB memory, 4 cores" in out and "- Cross-origin isolated: yes" in out
    assert "- Loaded files: 1 (mzML ×1; total <10 MB)" in out and "- Changed settings: theme" in out and "- Open cards: spec ×2" in out and "- Recent actions (oldest first):" in out
    assert "## bug.r.diag" not in node("B.rapporto({what:'a',diag:null},k=>k)")
    assert "bug.r.none" in node("B.rapporto({what:' ',diag:null},k=>k)")
    # no field when the page does not tell it (no deviceMemory, no OPFS answer)
    keys = node("B.campiDiagnostica({version:'1',date:'D',ua:'U',mode:'web',view:'v',lang:'en',files:'0',errors:[]}).map(f=>f.key)")
    assert "device" not in keys and "opfs" not in keys and "errors" in keys


def test_no_name_sample_or_path_in_any_field_or_json():
    names = "['Paziente_Rossi_t0.mzML','Campione Rossi']"
    res = node(f"(()=>{{const d=B.campiDiagnostica({SNAP},{names},s=>s.replace(/\\/home\\/[^\\s]*/g,'<path>'));return [B.righe(d).join('\\n'),B.jsonRapporto('bug','what','',d),B.rapporto({{what:'',diag:B.righe(d)}},k=>k)]}})()")
    for text in res:
        for bad in ("Rossi", "Paziente", "mrossi", "/home", ".mzML", ".raw", "Campione", "frag", "v=3", "n=Rossi"):
            assert bad not in text, (bad, text)
    assert "<file>" in res[0] and "foo app.js:12:5" in res[0] and "baz b.js:3:4" in res[0], res[0]
    # the same fields untick: an unticked key is gone from the text and the JSON
    out = node(f"(()=>{{const d=B.campiDiagnostica({SNAP},{names});const off=new Set(['errors','actions']);return [B.righe(d,off).join('\\n'),JSON.parse(B.jsonRapporto('bug','w','',d,off)).fields]}})()")
    assert "Recent errors" not in out[0] and "Recent actions" not in out[0] and "errors" not in out[1] and "version" in out[1]


def test_stack_frames_keep_only_site_scripts():
    fr = node("B.telai('Error: m\\n    at f (https://x.io/mzlab/app.js?a=1:7:8)\\n    at async g (blob:https://x.io/uuid:1:1)\\nh@https://x.io/dati/Rossi.raw:3:4\\n    at <anonymous>')")
    assert fr == ["f app.js:7:8", "g <file>:1:1", "h <file>:3:4"], fr
    assert len(node("B.telai(Array.from({length:30},(_, i)=>'at f (https://x.io/a.js:'+i+':1)').join('\\n'))")) == 8


def test_suggestion_has_minimal_fields_and_fixed_headings():
    keys = node(f"B.campiDiagnostica({SNAP},[],s=>s,B.TIPI.suggestion.keys).map(f=>f.key)")
    assert keys == ["version", "view", "lang"], keys
    out = node("B.rapporto({what:'idea',doing:'',diag:['- mzLab: 1']},k=>k)")
    assert out.split("\n")[0] == "## sug.r.what" and "\n## sug.r.doing\n\nbug.r.none" in out and "\n## bug.r.diag\n" in out
    j = node("JSON.parse(B.jsonRapporto('suggestion','i','d',[{key:'k',value:1}]))")
    assert j == {"schema": "mzlab-suggestion/1", "kind": "suggestion", "what": "i", "doing": "d", "fields": {"k": 1}}
    assert node("B.titolo('', 'idea', B.TIPI.suggestion.prefix)") == "Suggestion: idea"


def test_issue_url_expected_and_limit():
    u = node("B.urlIssue({title:'[Bug] x',body:'hello',ambiente:'UA',cut:'CUT'})")
    assert u["url"].startswith("https://github.com/Freeesta/mzlab/issues/new?template=bug.yml&labels=bug&title=%5BBug%5D%20x&ambiente=UA&cosa=hello") and not u["troncato"]
    big = node("B.urlIssue({title:'t',body:'à€😀'.repeat(3000),ambiente:'UA',cut:'CUT'})")
    assert big["troncato"] and len(big["url"]) <= 6000 and big["url"].endswith("CUT")


def test_suggestion_url_label_and_limit():
    u = node("B.urlIssue({kind:'suggestion',title:'Suggestion: x',body:'hello',cut:'CUT'})")
    assert u["url"] == "https://github.com/Freeesta/mzlab/issues/new?template=suggestion.yml&labels=suggestion&title=Suggestion%3A%20x&idea=hello" and not u["troncato"]
    big = node("B.urlIssue({kind:'suggestion',title:'t',body:'à€😀'.repeat(3000),cut:'CUT'})")
    assert big["troncato"] and len(big["url"]) <= 6000 and big["url"].endswith("CUT")


def test_email_url_expected_and_limit():
    u = node("B.urlEmail({user:'a.b',host:'c.it',subject:'[Bug] x',body:'hello world',cut:'CUT'})")
    assert u["url"] == "mailto:a.b@c.it?subject=%5BBug%5D%20x&body=hello%20world" and not u["troncato"]
    big = node("B.urlEmail({user:'a.b',host:'c.it',subject:'t',body:'à€😀'.repeat(3000),cut:'CUT'})")
    assert big["troncato"] and len(big["url"]) <= 1900 and big["url"].endswith("CUT")


def test_title():
    assert node("B.titolo('', 'vuoto')") == "[Bug] vuoto"
    assert node("B.titolo('linea uno\\nlinea due', 'v')") == "[Bug] linea uno"
    assert len(node("B.titolo('parola '.repeat(40), 'v')")) < 90
