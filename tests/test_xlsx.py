"""The home-made .xlsx writer (mzlab/web/xlsx.js): build a workbook with node, read it back with zipfile + XML (and openpyxl if installed)."""
import json
import re
import shutil
import subprocess
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "xlsx.js"
NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def build(tmp_path):
    out = tmp_path / "t.xlsx"
    code = f"""const x=require({str(JS)!r});
const u8=x.xlsxBuild([{{name:"Dati: 1/2 [prova]",head:["RT (min)","Intensità (cps)","Nome"],rows:[[1.5,3400.25,"a & b <c>"],[2,null,"  spazi  "],[NaN,1e-7,{{v:"grassetto",b:true}}]]}},
 {{name:"Dati: 1/2 [prova]",rows:[[{{v:"Pendenza",b:true}},123456789.123]]}}]);
require("fs").writeFileSync({str(out)!r},u8);"""
    subprocess.run(["node", "-e", code], check=True)
    return out


def test_xlsx_zip_and_cells(tmp_path):
    f = build(tmp_path)
    z = zipfile.ZipFile(f)
    assert z.testzip() is None                                   # CRC32 of every stored file is right
    assert all(i.compress_type == zipfile.ZIP_STORED for i in z.infolist())
    assert {"[Content_Types].xml", "_rels/.rels", "xl/workbook.xml", "xl/styles.xml", "xl/worksheets/sheet1.xml", "xl/worksheets/sheet2.xml"} <= set(z.namelist())
    names = re.findall(r'<sheet name="([^"]*)"', z.read("xl/workbook.xml").decode())
    assert names == ["Dati- 1-2 -prova-", "Dati- 1-2 -prova-_2"]      # forbidden characters replaced, names unique
    root = ET.fromstring(z.read("xl/worksheets/sheet1.xml"))
    cells = {c.get("r"): c for c in root.iter("{%s}c" % NS["m"])}
    assert cells["A2"].find("m:v", NS).text == "1.5" and cells["A2"].get("t") is None   # number, not text
    assert cells["B2"].find("m:v", NS).text == "3400.25"
    assert "".join(cells["C2"].itertext()) == "a & b <c>"           # escaped in the XML, intact when read
    assert "B3" not in cells and "A4" not in cells                  # null and NaN: empty cells
    assert cells["B4"].find("m:v", NS).text == "1e-7"
    assert cells["A1"].get("s") == "1" and cells["C4"].get("s") == "1"   # bold header and bold cell


def test_xlsx_openpyxl_reads_it(tmp_path):
    openpyxl = pytest.importorskip("openpyxl")
    ws = openpyxl.load_workbook(build(tmp_path)).worksheets[0]
    assert ws["A1"].value == "RT (min)" and ws["A1"].font.b and ws["A2"].value == 1.5 and ws["B2"].value == 3400.25
    assert ws["C3"].value == "  spazi  " and ws["B4"].value == pytest.approx(1e-7)
    assert openpyxl.load_workbook(build(tmp_path)).worksheets[1]["B1"].value == pytest.approx(123456789.123)


def node(code):
    return subprocess.run(["node", "-e", f"const x=require({str(JS)!r});{code}"], check=True, capture_output=True, encoding="utf-8").stdout


def sheet_xml(tmp_path, sheet_js, n=1):
    out = tmp_path / "s.xlsx"
    node(f"require('fs').writeFileSync({str(out)!r},x.xlsxBuild([{sheet_js}]))")
    return zipfile.ZipFile(out).read(f"xl/worksheets/sheet{n}.xml").decode()


def test_crc32_known_vector():
    assert node("console.log(x.xlsxCrc32(new TextEncoder().encode('123456789')))").strip() == str(0xCBF43926)


def test_header_frozen_and_widths(tmp_path):
    xml = sheet_xml(tmp_path, '{name:"a",head:["x","y"],rows:[["corto",1]],widths:[7,33]}')
    assert 'state="frozen"' in xml and 'ySplit="1"' in xml
    assert re.findall(r'<col min="(\d)" max="\d" width="([\d.]+)"', xml) == [("1", "7"), ("2", "33")]
    plain = sheet_xml(tmp_path, '{name:"a",rows:[["x".repeat(200),1]]}')
    assert 'state="frozen"' not in plain
    w = [float(v) for v in re.findall(r'width="([\d.]+)"', plain)]
    assert w == [60, 12]                                         # automatic width: capped at 60, numbers 12


def test_sheet_names_and_text_cleanup(tmp_path):
    out = tmp_path / "n.xlsx"
    node(f"""require('fs').writeFileSync({str(out)!r},x.xlsxBuild([{{rows:[["a\\u0001b\\nc"]]}},{{name:"x".repeat(40),rows:[]}},{{name:"x".repeat(40),rows:[]}},{{name:"X",rows:[]}},{{name:"x",rows:[]}}]))""")
    z = zipfile.ZipFile(out)
    names = re.findall(r'<sheet name="([^"]*)"', z.read("xl/workbook.xml").decode())
    assert names[0] == "Foglio1" and names[1] == "x" * 31 and names[2] == "x" * 29 + "_2"
    assert len({n.lower() for n in names}) == 5 and all(len(n) <= 31 for n in names)
    xml = z.read("xl/worksheets/sheet1.xml").decode()
    assert "\x01" not in xml and 'xml:space="preserve"' in xml and ">ab\nc<" in xml


def test_dlx_completes_the_extension():
    code = "const calls=[];global.dl=(n,d,m)=>calls.push([n,d.constructor.name,m]);" \
           "const fs=require('fs');const src=fs.readFileSync(" + repr(str(JS)) + ",'utf8');" \
           "(0,eval)(src+';globalThis.dlx=dlx');dlx('r',[{name:'a',rows:[[1]]}]);dlx('r.XLSX',[{name:'a',rows:[]}]);console.log(JSON.stringify(calls))"
    calls = json.loads(subprocess.run(["node", "-e", code], check=True, capture_output=True, encoding="utf-8").stdout)
    mime = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    assert calls == [["r.xlsx", "Uint8Array", mime], ["r.XLSX", "Uint8Array", mime]]
