"""Pure parts of the didactic tools of the Teoria (strumenti/massa.js) against reference values, with node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

W = Path(__file__).resolve().parent.parent / "mzlab" / "web"


@pytest.mark.skipif(not shutil.which("node"), reason="node missing")
def test_massa_values():
    js = r"""
const fs=require('fs'),vm=require('vm'),w=process.argv[1];
const ctx={module:{exports:{}},console};vm.createContext(ctx);
vm.runInContext(fs.readFileSync(w+'/elements.js','utf8')+'\nthis.ELEMENTS=ELEMENTS;',ctx);
vm.runInContext(fs.readFileSync(w+'/teoria/pratica/motore.js','utf8')+'\nthis.PAL=PAL;',ctx);
vm.runInContext(fs.readFileSync(w+'/teoria/strumenti/massa.js','utf8'),ctx);
const m=ctx.module.exports, r=m.calcola('C8H14ClN5',ctx.PAL,ctx.ELEMENTS);
console.log(JSON.stringify({nom:r.Mnom,M:r.M,mh:r.ioni[0].mz,na:r.ioni[1].mz,mhn:r.ioni[2].mz,nomi:r.ioni.map(x=>x.nom),
 righe:r.righe.map(x=>x.e+x.n),bad:m.calcola('Xx2',ctx.PAL,ctx.ELEMENTS),r1:m.fmt(0.12345,4),r2:m.fmt(2.00005,4),r3:m.fmt(1.5,0),r4:m.fmt(215.09378,4)}));
"""
    r = subprocess.run(["node", "-e", js, str(W)], capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    o = json.loads(r.stdout)
    assert o["nom"] == 215 and o["nomi"] == [216, 238, 214]
    assert abs(o["M"] - 215.09378) < 2e-5
    assert abs(o["mh"] - 216.10105) < 2e-5               # chapter 11: 216,1010
    assert abs(o["na"] - (o["M"] + 22.989770 - 0.000549)) < 2e-5
    assert abs(o["mhn"] - (o["M"] - 1.007825 + 0.000549)) < 2e-5
    assert o["righe"] == ["C8", "H14", "Cl1", "N5"]
    assert o["bad"] is None
    assert o["r1"] == "0,1235" and o["r2"] == "2,0001" and o["r3"] == "2" and o["r4"] == "215,0938"
