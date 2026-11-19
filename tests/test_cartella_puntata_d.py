"""«Open folder» and folder datasets (.D / .d / Waters .raw): the list of the page (CartPP.collect, run by node over a fake handle built from the disk) is the list of
`mzlab-cli file list` (the scan of the desktop app) on one synthetic tree, and a pointed .D walks and groups exactly like a dropped one (CartCarica.entryOfHandle)."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tests"))
import agilent_sintetico as ag  # noqa: E402
import cargo_util  # noqa: E402

WEB = ROOT / "mzlab" / "web"


def need_tools():
    """node and cargo are guaranteed by the CI jobs: a missing one is a failure, never a skip."""
    for tool in ("node", "cargo"):
        assert shutil.which(tool), f"{tool} is required by this test and was not found"

NODE_HANDLE = """
const fs=require('fs'),path=require('path');
const H=(p)=>{const st=fs.statSync(p),name=path.basename(p);
  if(st.isDirectory())return {kind:'directory',name,async*entries(){for(const n of fs.readdirSync(p).sort())yield [n,H(path.join(p,n))]}};
  return {kind:'file',name,async getFile(){const b=fs.readFileSync(p);const f=new File([b],name,{lastModified:st.mtimeMs});return f}}};
"""


def node(code):
    need_tools()
    out = subprocess.run(["node", "-e", NODE_HANDLE + "(async()=>{const r=await(" + code + ");console.log(JSON.stringify(r))})()"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


@pytest.fixture(scope="module")
def tree(tmp_path_factory):
    need_tools()
    r = tmp_path_factory.mktemp("lab")
    for f in ["a.mzML", "notes.txt", "sub/w.wiff", "sub/w.wiff.scan", "sub/.hid/x.mzML", "lc/k.d/analysis.tdf", "lc/k.d/analysis.tdf_bin",
              "wat/z.raw/_FUNC001.DAT", "wat/z.raw/_extern.inf", "other/q.raw/readme.csv", "a/b/c/d/deep.mzML"]:
        p = r / f
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(b"xy")
    ag.write_old(r / "gc", "s1.D", gc=True)          # data.ms on top, AcqData with Contents.xml only
    (r / "empty.D").mkdir()
    (r / "empty.D/readme.txt").write_text("x")
    cargo_util.build("-p", "mzlab-cli")
    return r


def cli_rows(r):
    out = subprocess.run(cargo_util.cli_argv("file", "list", str(r), "--json"), check=True, capture_output=True, encoding="utf-8", cwd=ROOT).stdout
    j = json.loads(out)
    assert j["schema"] == "folder/1" and j["truncated"] is False
    return [(i["path"], i["kind"], i["bundle"]) for i in j["items"]]


def test_site_list_equals_cli_list_on_one_tree(tree):
    page = node("(async()=>{const c=require(%r);const it=await c.collect(H(%r));return it.map(x=>[x.path,x.kind,x.bundle||null])})()" % (str(WEB / "cartella-puntata.js"), str(tree)))
    cli = cli_rows(tree)
    assert sorted(map(tuple, page)) == sorted(cli)
    names = [p for p, _, _ in cli]
    assert {"gc/s1.D", "lc/k.d", "empty.D", "wat/z.raw"} <= set(names)       # folder datasets are rows
    assert "other/q.raw/readme.csv" in names and "other/q.raw" not in names   # a .raw folder without _FUNC001.DAT is only a folder
    assert not any(n.startswith("gc/s1.D/") for n in names)                 # a folder dataset is not entered
    assert dict((p, b) for p, _, b in cli)["wat/z.raw"] == "waters-raw"


def test_a_pointed_gc_d_walks_and_groups_like_a_dropped_one(tree):
    code = """(async()=>{const C=require(%r);const A=require(%r);global.CartCarica=C;
      const files=await C.readEntries([C.entryOfHandle(H(%r))],null,'lab/gc/');
      const g=A.group(files);return [files.map(f=>C.dir(f)+f.name).sort(),g.runs.map(r=>[r.name,r.root,Object.keys(r.files).sort(),r.missing,r.found.length])]})()""" % (
        str(WEB / "cartella-carica.js"), str(WEB / "agilent-d.js"), str(tree / "gc" / "s1.D"))
    paths, runs = node(code)
    assert "lab/gc/s1.D/data.ms" in paths and "lab/gc/s1.D/AcqData/Contents.xml" in paths
    assert runs[0][:2] == ["s1.D", "lab/gc/s1.D/"] and runs[0][3] == [] and "datams" in runs[0][2]
