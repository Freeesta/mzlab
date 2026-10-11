"""Feature table of the series (mzlab/web/tabella.js, pure parsers run by node)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
LOADER = """
const vm=require('vm'),fs=require('fs');
const ctx={module:{exports:{}},console};vm.createContext(ctx);
vm.runInContext(fs.readFileSync(%r,'utf8')+';this.O=module.exports',ctx);
vm.runInContext(fs.readFileSync(%r,'utf8')+';this.T=module.exports',ctx);
const T=ctx.T;
"""


def js(code):
    src = LOADER % (str(WEB / "ordine.js"), str(WEB / "tabella.js")) + code
    return json.loads(subprocess.run(["node", "-e", src], check=True, capture_output=True, encoding="utf-8").stdout)


@needs_node
def test_parse_list():
    txt = "# comment\n305.1234; 6,02\n250.0\t4.01\n\n412.1 8.5\n305.1234, 7\nabc; 3\n100;\n"
    r = js(f"console.log(JSON.stringify(T.parseList({json.dumps(txt)})))")
    assert r["rows"] == [{"mz": 305.1234, "rt": 6.02}, {"mz": 250.0, "rt": 4.01}, {"mz": 412.1, "rt": 8.5}, {"mz": 305.1234, "rt": 7}]
    assert r["bad"] == [7, 8]


MZMINE = ("row ID,row m/z,row retention time,run_A.mzML Peak area,run_B.mzML Peak area,run_A.mzML Peak height,other.mzML Peak area\n"
          "1,305.1234,6.02,1000,2000,50,7\n2,250.0000,4.01,,300,10,8\n")


@needs_node
def test_parse_mzmine_table_matches_columns_to_open_files():
    r = js(f"console.log(JSON.stringify(T.parseTable({json.dumps(MZMINE)}, ['run_A.mzML','run_B.mzML','run_C.mzML'])))")
    assert r["columns"] == ["run_A.mzML", "run_B.mzML"]                      # the height column and the unknown file are ignored
    assert r["features"][0] == {"key": "1", "id": "1", "mz": 305.1234, "rt": 6.02, "areas": {"run_A.mzML": 1000, "run_B.mzML": 2000}}
    assert r["features"][1]["areas"] == {"run_A.mzML": 0, "run_B.mzML": 300}   # an empty cell is 0 (not detected)


@needs_node
def test_parse_table_mzmine3_headers_and_errors():
    t3 = "id\tmz\trt\tdatafile:run_A.mzML:area\n7\t100.5\t1.5\t42\n"
    r = js(f"console.log(JSON.stringify(T.parseTable({json.dumps(t3)}, ['run_A.mzML'])))")
    assert r["features"][0]["areas"] == {"run_A.mzML": 42} and r["features"][0]["id"] == "7"
    assert js("console.log(JSON.stringify(T.parseTable('a,b\\n1,2', ['x'])))")["error"] == "columns"
    assert js("console.log(JSON.stringify(T.toMatrix([{areas:{a:1,b:2}},{areas:{b:5}}],['b','a'])))") == [[2, 1], [5, 0]]
